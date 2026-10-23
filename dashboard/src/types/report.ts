// Mirrors crates/firewatch-core/src/report.rs. See docs/API.md for field notes.
// When the Rust report changes, change this file in the same PR.

export type Status = 'affected' | 'cant_tell' | 'not_affected'
export type Severity = 'low' | 'medium' | 'high' | 'critical'
export type Action = 'upgrade' | 'offline' | 'disable_feature'
export type Implementation = 'cln' | 'lnd' | 'ldk'

export interface Finding {
  advisory_id: string
  title: string
  severity: Severity
  exploited_in_wild: boolean
  /** false only when the daemon runs with --include-unverified */
  verified: boolean
  /** null means no fix exists yet */
  fixed: string | null
  /** the developers' own notes */
  source: string
  status: Status
  /** one sentence, always shown */
  reason: string
  /** non-empty for amber findings */
  manual_checks: string[]
  /** the version is inside the affected range; with not_affected, a condition ruled it out */
  in_range: boolean
  action: Action
  /** the developers' words, quoted; never reworded */
  steps: string
}

/**
 * Whether the binary is the official release its version claims (docs/API.md).
 * match / mismatch / unknown_release: LND, by commit. unreleased / modified:
 * Core Lightning, from its version string (git describe).
 */
export type BuildCheck =
  | { status: 'match' }
  | { status: 'mismatch'; release: string; expected: string; actual: string; build?: string }
  /** a commit after the tag: every finding is already cant_tell */
  | { status: 'unreleased'; release: string; build: string }
  /** the tag with local changes: reds stay red, would-be-greens are cant_tell */
  | { status: 'modified'; release: string; build: string }
  | { status: 'unknown_release' }
  | { status: 'not_checked' }

export interface NodeReport {
  name: string
  implementation: Implementation
  /** null when the node couldn't be read */
  version: string | null
  network: string | null
  /** worst status across findings; cant_tell when unreadable */
  status: Status
  /** set when the node couldn't be read */
  error: string | null
  /** absent on daemons from before the build check */
  build_check?: BuildCheck
  /** worst first */
  findings: Finding[]
}

export interface Report {
  /** unix seconds */
  checked_at: number
  nodes: NodeReport[]
}
