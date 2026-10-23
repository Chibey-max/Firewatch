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

/**
 * True when the version is in range but a condition (e.g. "mainnet only") rules
 * the advisory out. These are worth showing: they're where a version check alone
 * would have raised a false alarm. Keyed on the engine's wording in
 * crates/firewatch-core/src/engine.rs; if that changes, these fold away with the
 * other ruled-out findings, which is safe.
 */
export function ruledOutByCondition(f: Finding): boolean {
  return f.status === 'not_affected' && f.reason.includes(' is in the affected range ')
}

export function countBy(findings: Finding[]): Record<Status, number> {
  const counts: Record<Status, number> = { affected: 0, cant_tell: 0, not_affected: 0 }
  for (const f of findings) counts[f.status] += 1
  return counts
}

export function plural(n: number, one: string, many: string): string {
  return `${n} ${n === 1 ? one : many}`
}

/**
 * Who published the source: "github.com/ElementsProject/lightning" for GitHub,
 * otherwise just the site ("security.lightning.engineering"). Long labels are cut.
 */
export function sourceLabel(url: string, max = 44): string {
  let label = url
  try {
    const u = new URL(url)
    const repo = u.pathname.split('/').filter(Boolean).slice(0, 2).join('/')
    label = u.host === 'github.com' && repo ? `${u.host}/${repo}` : u.host
  } catch {
    // not a URL: show it as given
  }
  return label.length > max ? `${label.slice(0, max - 1)}…` : label
}

/**
 * A plain-English first line for a connector error. The raw error is still shown
 * underneath; this only names the likely cause. Returns null when unsure.
 */
export function describeError(error: string): string | null {
  const host = error.match(/url \((?:https?:\/\/)?([^/)]+)/)?.[1]
  const at = host ? ` at ${host}` : ''
  const e = error.toLowerCase()
  if (e.includes('not read-only'))
    return 'Firewatch refused this credential: it can do more than read.'
  if (e.includes('connection refused')) return `Nothing answered${at}. Is the node running?`
  if (e.includes('timed out') || e.includes('timeout'))
    return `The node${at} didn't answer in time.`
  if (e.includes('certificate') || e.includes('tls'))
    return "The node's TLS certificate wasn't accepted. Check tls_cert in firewatch.toml."
  if (e.includes('1502') || e.includes('not permitted') || / 40[13]\b/.test(e))
    return "The node refused Firewatch's credential. Check the rune or macaroon in firewatch.toml."
  return null
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
