import type { Report, Status } from '../types/report'
import { readingOf, readingWord, type Reading } from './format'

/**
 * What the dashboard last showed, kept in this browser only, so the next
 * visit can say what changed in between: a verdict that moved, an advisory
 * that newly applies, a node that's new.
 */
export interface Snapshot {
  /** the report's checked_at, unix seconds */
  at: number
  nodes: Record<string, { reading: Reading; findings: Record<string, Status> }>
}

export type Change =
  | { kind: 'verdict'; node: string; from: Reading; to: Reading }
  | { kind: 'advisory'; node: string; id: string; title: string; status: Status }
  | { kind: 'added'; node: string; reading: Reading }

export function snapshot(report: Report): Snapshot {
  const nodes: Snapshot['nodes'] = {}
  for (const n of report.nodes) {
    nodes[n.name] = {
      reading: readingOf(n),
      findings: Object.fromEntries(n.findings.map((f) => [f.advisory_id, f.status])),
    }
  }
  return { at: report.checked_at, nodes }
}

/**
 * What changed from `prev` to `report`, in the report's node order. An
 * advisory counts when it's new to a node and isn't green; a node that left
 * the config isn't news.
 */
export function changesSince(prev: Snapshot | null, report: Report): Change[] {
  if (!prev) return []
  const out: Change[] = []
  for (const n of report.nodes) {
    const was = prev.nodes[n.name]
    const reading = readingOf(n)
    if (!was) {
      out.push({ kind: 'added', node: n.name, reading })
      continue
    }
    if (was.reading !== reading)
      out.push({ kind: 'verdict', node: n.name, from: was.reading, to: reading })
    for (const f of n.findings) {
      if (!(f.advisory_id in was.findings) && f.status !== 'not_affected') {
        out.push({
          kind: 'advisory',
          node: n.name,
          id: f.advisory_id,
          title: f.title,
          status: f.status,
        })
      }
    }
  }
  return out
}

/** One line for a change. */
export function describeChange(c: Change): string {
  if (c.kind === 'added') return `${c.node} is now watched: ${readingWord[c.reading].toLowerCase()}`
  if (c.kind === 'verdict') {
    return `${c.node} is now ${readingWord[c.to].toLowerCase()} (was ${readingWord[c.from].toLowerCase()})`
  }
  return c.status === 'affected'
    ? `${c.id} now applies to ${c.node}: ${c.title}`
    : `${c.id} needs a check by hand on ${c.node}: ${c.title}`
}

/** The verdict colour a change points to: where the node is now. */
export function changeReading(c: Change): Reading {
  if (c.kind === 'advisory') return c.status === 'affected' ? 'affected' : 'cant_tell'
  return c.kind === 'verdict' ? c.to : c.reading
}

/** "2 h ago", for when the last look was. */
export function ago(at: number, now: number): string {
  const secs = Math.max(0, Math.round(now / 1000 - at))
  if (secs < 60) return 'moments ago'
  if (secs < 3600) return `${Math.floor(secs / 60)} min ago`
  if (secs < 86400) return `${Math.floor(secs / 3600)} h ago`
  const days = Math.floor(secs / 86400)
  return days === 1 ? 'yesterday' : `${days} days ago`
}

// Live and sample data keep separate memories; the samples share one, so
// ?fixture=demo then ?fixture=demo-after shows the upgrade as a change.
const key = (source: 'live' | 'sample') => `fw-seen:${source}`

export function loadSnapshot(source: 'live' | 'sample'): Snapshot | null {
  try {
    const raw = localStorage.getItem(key(source))
    const parsed = raw ? (JSON.parse(raw) as Snapshot) : null
    return parsed && typeof parsed.at === 'number' && parsed.nodes ? parsed : null
  } catch {
    return null
  }
}

export function saveSnapshot(source: 'live' | 'sample', snap: Snapshot) {
  try {
    localStorage.setItem(key(source), JSON.stringify(snap))
  } catch {
    // storage blocked: changes just won't carry over to the next visit
  }
}
