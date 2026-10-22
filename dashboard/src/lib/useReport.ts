import { useCallback, useEffect, useState } from 'react'
import { fixtures, type FixtureName } from '../fixtures'
import type { Report } from '../types/report'

const API = import.meta.env.VITE_API_BASE ?? ''
const POLL_MS = 3000

export type Phase =
  /** nothing loaded yet */
  | 'loading'
  /** the daemon is up but its first check hasn't finished (503) */
  | 'first_check'
  | 'live'
  /** the daemon can't be reached; `report` is the last good one, if any */
  | 'offline'
  | 'fixture'

export interface ReportState {
  report: Report | null
  phase: Phase
  /** when the last good live report arrived, in ms */
  receivedAt: number | null
  checking: boolean
  checkError: string | null
  /** runs a check now (POST /api/check) */
  checkNow: () => void
}

class FirstCheckRunning extends Error {}

async function readReport(res: Response): Promise<Report> {
  if (res.status === 503) throw new FirstCheckRunning()
  if (!res.ok) throw new Error(`The daemon answered ${res.status}`)
  return (await res.json()) as Report
}

/** Polls `GET /api/report`, or serves a fixture when one is named. */
export function useReport(fixture: FixtureName | null): ReportState {
  const [report, setReport] = useState<Report | null>(null)
  const [phase, setPhase] = useState<Phase>('loading')
  const [receivedAt, setReceivedAt] = useState<number | null>(null)
  const [checking, setChecking] = useState(false)
  const [checkError, setCheckError] = useState<string | null>(null)

  const accept = useCallback((next: Report) => {
    setReport(next)
    setReceivedAt(Date.now())
    setPhase('live')
  }, [])

  useEffect(() => {
    if (fixture) return
    let stopped = false
    let timer: ReturnType<typeof setTimeout>
    const controller = new AbortController()

    const poll = async () => {
      try {
        const res = await fetch(`${API}/api/report`, { signal: controller.signal })
        accept(await readReport(res))
      } catch (e) {
        if (stopped) return
        setPhase(e instanceof FirstCheckRunning ? 'first_check' : 'offline')
      }
      if (!stopped) timer = setTimeout(poll, POLL_MS)
    }
    poll()

    return () => {
      stopped = true
      controller.abort()
      clearTimeout(timer)
    }
  }, [fixture, accept])

  const checkNow = useCallback(async () => {
    setChecking(true)
    setCheckError(null)
    try {
      const res = await fetch(`${API}/api/check`, { method: 'POST' })
      accept(await readReport(res))
    } catch (e) {
      setCheckError(
        e instanceof FirstCheckRunning
          ? 'The first check is still running.'
          : "Couldn't run the check: the Firewatch daemon didn't answer.",
      )
    } finally {
      setChecking(false)
    }
  }, [accept])

  if (fixture) {
    return {
      report: fixtures[fixture],
      phase: 'fixture',
      receivedAt: null,
      checking: false,
      checkError: null,
      checkNow: () => {},
    }
  }
  return { report, phase, receivedAt, checking, checkError, checkNow }
}

/** Re-renders every second so "Checked N s ago" stays true. */
export function useNow(intervalMs = 1000): number {
  const [now, setNow] = useState(() => Date.now())
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), intervalMs)
    return () => clearInterval(id)
  }, [intervalMs])
  return now
}
