import { useCallback, useEffect, useRef, useState } from 'react'
import { fixtures, type FixtureName } from '../fixtures'
import type { Report } from '../types/report'
import { toast } from './toasts'

const API = import.meta.env.VITE_API_BASE ?? ''
export const POLL_MS = 3000

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
  /** failed polls in a row; restarts the retry countdown */
  failures: number
  checking: boolean
  /** runs a check now (POST /api/check); resolves true on success */
  checkNow: () => Promise<boolean>
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
  const [failures, setFailures] = useState(0)
  const [checking, setChecking] = useState(false)
  // The phase the poller last saw, to toast only on transitions.
  const seen = useRef<Phase>('loading')

  const accept = useCallback((next: Report) => {
    if (seen.current === 'offline') toast('good', 'Reconnected', 'The report is up to date again.')
    seen.current = 'live'
    setReport(next)
    setReceivedAt(Date.now())
    setFailures(0)
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
        const next: Phase = e instanceof FirstCheckRunning ? 'first_check' : 'offline'
        if (next === 'offline' && seen.current === 'live') {
          toast('warn', 'Lost the Firewatch daemon', 'Showing the last report. Retrying every 3 s.')
        }
        seen.current = next
        setPhase(next)
        setFailures((n) => n + 1)
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
    try {
      const res = await fetch(`${API}/api/check`, { method: 'POST' })
      accept(await readReport(res))
      return true
    } catch (e) {
      if (e instanceof FirstCheckRunning) {
        toast('warn', 'The first check is still running', 'Try again in a few seconds.')
      } else {
        toast('bad', 'Couldn’t run the check', 'The Firewatch daemon didn’t answer. Is it running?')
      }
      return false
    } finally {
      setChecking(false)
    }
  }, [accept])

  if (fixture) {
    return {
      report: fixtures[fixture],
      phase: 'fixture',
      receivedAt: null,
      failures: 0,
      checking: false,
      checkNow: async () => false,
    }
  }
  return { report, phase, receivedAt, failures, checking, checkNow }
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
