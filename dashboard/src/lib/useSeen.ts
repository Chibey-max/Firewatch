import { useEffect, useMemo, useRef, useState } from 'react'
import type { Report } from '../types/report'
import { changesSince, loadSnapshot, saveSnapshot, snapshot, type Snapshot } from './seen'

/**
 * Changes since the last look. The baseline is what this browser last
 * acknowledged (or the first report, on a first visit) and holds while the
 * page is open, so a change that lands mid-visit shows too. Changes stay
 * until "Got it", across visits; leaving the page with nothing to report just
 * moves the baseline up to now.
 */
export function useSinceLastLook(report: Report | null, source: 'live' | 'sample') {
  const [baseline, setBaseline] = useState<Snapshot | null>(() => loadSnapshot(source))
  const latest = useRef(report)
  // A first visit: the first report is the baseline (set while rendering,
  // so it's in place before anything compares against it).
  if (report && !baseline) setBaseline(snapshot(report))

  const changes = useMemo(() => (report ? changesSince(baseline, report) : []), [baseline, report])
  const pending = useRef(false)
  const base = useRef(baseline)

  useEffect(() => {
    latest.current = report
    base.current = baseline
    pending.current = changes.length > 0
  }, [report, baseline, changes])

  useEffect(() => {
    // With changes waiting, keep the baseline they're measured from;
    // otherwise move it up to what's on screen.
    const save = () => {
      const snap = pending.current ? base.current : latest.current && snapshot(latest.current)
      if (snap) saveSnapshot(source, snap)
    }
    const onVisibility = () => {
      if (document.visibilityState === 'hidden') save()
    }
    window.addEventListener('pagehide', save)
    document.addEventListener('visibilitychange', onVisibility)
    return () => {
      window.removeEventListener('pagehide', save)
      document.removeEventListener('visibilitychange', onVisibility)
    }
  }, [source])

  const dismiss = () => {
    if (!report) return
    const now = snapshot(report)
    setBaseline(now)
    saveSnapshot(source, now)
  }

  return { changes, since: baseline?.at ?? null, dismiss }
}
