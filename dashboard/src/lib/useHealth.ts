import { useEffect, useRef, useState } from 'react'
import type { Health } from '../types/report'
import { toast } from './toasts'
import { POLL_MS } from './useReport'

const API = import.meta.env.VITE_API_BASE ?? ''

/**
 * Polls `GET /api/health` alongside the report, for whether alerts are
 * getting through. Quiet when the daemon can't be reached (the live status
 * says so already); null with sample data. Toasts once when waiting alerts
 * go out.
 */
export function useHealth(enabled: boolean): Health | null {
  const [health, setHealth] = useState<Health | null>(null)
  const waiting = useRef(0)

  useEffect(() => {
    if (!enabled) return
    let stopped = false
    let timer: ReturnType<typeof setTimeout>
    const poll = async () => {
      try {
        const res = await fetch(`${API}/api/health`)
        if (res.ok) {
          const next = (await res.json()) as Health
          const pending = next.alerts_pending ?? 0
          if (waiting.current > 0 && pending === 0) {
            toast(
              'good',
              'Waiting alerts sent',
              'The connection is back and your phone is up to date.',
            )
          }
          waiting.current = pending
          if (!stopped) setHealth(next)
        }
      } catch {
        // unreachable: useReport shows that
      }
      if (!stopped) timer = setTimeout(poll, POLL_MS)
    }
    poll()
    return () => {
      stopped = true
      clearTimeout(timer)
    }
  }, [enabled])

  return enabled ? health : null
}
