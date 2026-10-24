import type {
  Exposure,
  Finding,
  Implementation,
  NodeReport,
  Severity,
  Status,
} from '../types/report'

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

/** A node's overall status. "none" is a node no advisory applies to: not proof of safety. */
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
 * would have raised a false alarm.
 */
export function ruledOutByCondition(f: Finding): boolean {
  return f.status === 'not_affected' && f.in_range
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
 * A plain-English first line for a raw connector error. The connectors now
 * write plain sentences themselves ("Can't connect to …; is the node
 * running?"); those are shown as they are. This only steps in for an older
 * daemon's raw transport text, which is then shown underneath. Null when the
 * error is already plain, or when unsure.
 */
export function describeError(error: string): string | null {
  if (!/^request failed:|error sending request|client error \(/.test(error)) return null
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

const rank: Record<Reading, number> = { affected: 0, cant_tell: 1, not_affected: 2, none: 3 }

function exploited(node: NodeReport): number {
  return node.findings.filter((f) => f.status === 'affected' && f.exploited_in_wild).length
}

/** Worst first: affected (exploited first), then can't tell, then the rest; ties by name. */
export function sortNodes(nodes: NodeReport[]): NodeReport[] {
  return [...nodes].sort(
    (a, b) =>
      rank[readingOf(a)] - rank[readingOf(b)] ||
      exploited(b) - exploited(a) ||
      a.name.localeCompare(b.name),
  )
}

/** Nodes that are affected or can't be confirmed. */
export function needsAction(nodes: NodeReport[]): number {
  return nodes.filter((n) => n.status !== 'not_affected').length
}

/** One short line for a node's row: what its status rests on. */
export function nodeSummary(node: NodeReport): string {
  if (node.error) return "Couldn't read this node"
  if (node.build_check?.status === 'mismatch') return "Build doesn't match the release"
  if (node.build_check?.status === 'unreleased') return 'Build is a commit after the release'
  const counts = countBy(node.findings)
  const parts: string[] = []
  if (counts.affected) parts.push(plural(counts.affected, 'advisory applies', 'advisories apply'))
  if (exploited(node)) parts.push('exploited in the wild')
  if (counts.cant_tell) parts.push(`${counts.cant_tell} to check by hand`)
  if (parts.length) return parts.join(' · ')
  if (node.findings.length === 0) return 'No advisory covers this node'
  return `None of ${plural(node.findings.length, 'advisory', 'advisories')} apply`
}

/**
 * Why a node is amber, for the "Why amber?" note: the rule first, then what
 * this node's amber rests on.
 */
export function amberReason(node: NodeReport): string {
  if (node.error) {
    return "It couldn't read this node, so it can't tell which advisories apply. It stays amber until the node answers."
  }
  const build = node.build_check?.status
  if (build === 'mismatch' || build === 'unreleased') {
    return "This build isn't the official release, so its version number can't say which fixes it has."
  }
  if (build === 'modified') {
    return 'This build was made with local changes, so Firewatch can’t confirm a fix is intact.'
  }
  return 'Something an advisory depends on can’t be read from the node, so check it by hand.'
}

/** Whether any of a node's APIs can be reached from other machines. */
export function isExposed(exposure: Exposure[] | undefined): boolean {
  return (exposure ?? []).some((e) => e.status === 'exposed')
}

/** Compare dotted version numbers ("26.06.9", "0.20.1-beta"); suffixes are ignored. */
export function compareVersions(a: string, b: string): number {
  const parts = (v: string) => (v.match(/\d+(\.\d+)*/)?.[0] ?? '').split('.').map(Number)
  const [x, y] = [parts(a), parts(b)]
  for (let i = 0; i < Math.max(x.length, y.length); i++) {
    const d = (x[i] ?? 0) - (y[i] ?? 0)
    if (d) return d
  }
  return 0
}

/**
 * The release that carries every fix this node is affected by: the highest
 * `fixed` among its red findings. Null when any of them has no fix yet, or
 * lists fixes on several release lines (then each finding says its own).
 */
export function fixedTarget(node: NodeReport): string | null {
  const fixes = node.findings.filter((f) => f.status === 'affected').map((f) => f.fixed)
  if (fixes.length === 0 || fixes.some((f) => !f || f.includes(','))) return null
  return (fixes as string[]).reduce((max, f) => (compareVersions(f, max) > 0 ? f : max))
}

export function shortHash(hash: string): string {
  return hash.slice(0, 7)
}

export function readingCounts(nodes: NodeReport[]): Record<Reading, number> {
  const counts: Record<Reading, number> = { affected: 0, cant_tell: 0, not_affected: 0, none: 0 }
  for (const n of nodes) counts[readingOf(n)] += 1
  return counts
}

/** The worst reading across the fleet: what the ambient light shows. */
export function worstReading(nodes: NodeReport[]): Reading {
  return sortNodes(nodes)[0] ? readingOf(sortNodes(nodes)[0]) : 'none'
}

/** Where to start: one sentence naming the node to look at first, and why. */
export function priorityLine(nodes: NodeReport[]): string {
  const first = sortNodes(nodes)[0]
  if (!first) return 'Add a node to firewatch.toml and Firewatch starts watching it.'
  const reading = readingOf(first)
  if (reading === 'affected') {
    return exploited(first)
      ? `Start with ${first.name}: an advisory on it is exploited in the wild.`
      : `Start with ${first.name}: ${nodeSummary(first).toLowerCase()}.`
  }
  if (reading === 'cant_tell') {
    return first.error
      ? `${first.name} can’t be read, so Firewatch won’t call it safe.`
      : `${first.name} can’t be confirmed over RPC. Check it by hand.`
  }
  return 'Nothing the developers have published applies to your nodes right now.'
}
