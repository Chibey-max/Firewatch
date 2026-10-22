import type { Finding, Implementation, NodeReport, Severity, Status } from '../types/report'

export const implementationName: Record<Implementation, string> = {
  cln: 'Core Lightning',
  lnd: 'LND',
  ldk: 'LDK',
}

export const severityName: Record<Severity, string> = {
  critical: 'Critical',
  high: 'High',
  medium: 'Medium',
  low: 'Low',
}

/** What the dial shows. "none" is a node no advisory applies to: not proof of safety. */
export type Reading = Status | 'none'

export function readingOf(node: NodeReport): Reading {
  if (!node.error && node.findings.length === 0) return 'none'
  return node.status
}

export const readingWord: Record<Reading, string> = {
  affected: 'Affected',
  cant_tell: "Can't tell",
  not_affected: 'Not affected',
  none: 'No known advisories',
}

/** LND reports `0.20.1-beta commit=v0.20.1-beta`; split the build tag off for display. */
export function splitVersion(raw: string): { version: string; build: string | null } {
  const [version, ...rest] = raw.trim().split(/\s+/)
  const build = rest.join(' ').replace(/^commit=/, '')
  return { version, build: build || null }
}

export function countBy(findings: Finding[]): Record<Status, number> {
  const counts: Record<Status, number> = { affected: 0, cant_tell: 0, not_affected: 0 }
  for (const f of findings) counts[f.status] += 1
  return counts
}

export function plural(n: number, one: string, many: string): string {
  return `${n} ${n === 1 ? one : many}`
}

/** "github.com/ElementsProject/lightning" from a release URL. */
export function sourceLabel(url: string): string {
  try {
    const u = new URL(url)
    const path = u.pathname.split('/').filter(Boolean).slice(0, 2).join('/')
    return path ? `${u.host}/${path}` : u.host
  } catch {
    return url
  }
}

export function checkedAgo(checkedAt: number, now: number): string {
  const secs = Math.max(0, Math.round(now / 1000 - checkedAt))
  if (secs < 5) return 'Checked just now'
  if (secs < 60) return `Checked ${secs} s ago`
  if (secs < 3600) return `Checked ${Math.floor(secs / 60)} min ago`
  const d = new Date(checkedAt * 1000)
  return `Checked ${d.toLocaleString(undefined, {
    day: 'numeric',
    month: 'short',
    hour: '2-digit',
    minute: '2-digit',
  })}`
}

export function clockTime(ms: number): string {
  return new Date(ms).toLocaleTimeString(undefined, { hour12: false })
}
