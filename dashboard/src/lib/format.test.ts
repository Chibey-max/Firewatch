import { describe, expect, it } from 'vitest'
import { fixtures } from '../fixtures'
import type { Finding, NodeReport } from '../types/report'
import {
  checkedAgo,
  compareVersions,
  describeError,
  fixedTarget,
  needsAction,
  nodeSummary,
  priorityLine,
  readingCounts,
  readingOf,
  ruledOutByCondition,
  sortNodes,
  sourceLabel,
  splitVersion,
  worstReading,
} from './format'

const demo = fixtures.demo.nodes
const byName = (name: string) => demo.find((n) => n.name === name)!

function finding(over: Partial<Finding>): Finding {
  return {
    advisory_id: 'FW-TEST',
    title: 't',
    severity: 'high',
    exploited_in_wild: false,
    verified: true,
    fixed: '1.0.0',
    source: 'https://example.com',
    status: 'affected',
    reason: 'r',
    manual_checks: [],
    in_range: true,
    action: 'upgrade',
    steps: 's',
    ...over,
  }
}

function node(over: Partial<NodeReport>): NodeReport {
  return {
    name: 'n',
    implementation: 'cln',
    version: 'v1.0.0',
    network: 'regtest',
    status: 'not_affected',
    error: null,
    findings: [],
    ...over,
  }
}

describe('readingOf', () => {
  it('reads a node with no findings as "none", never green', () => {
    expect(readingOf(node({ status: 'not_affected', findings: [] }))).toBe('none')
  })
  it('keeps an unreadable node amber even with no findings', () => {
    expect(readingOf(node({ status: 'cant_tell', error: 'down', findings: [] }))).toBe('cant_tell')
  })
})

describe('sortNodes', () => {
  it('puts affected first, exploited before not, then can’t tell, then the rest', () => {
    expect(sortNodes(demo).map((n) => n.name)).toEqual([
      'old-cln', // affected, exploited in the wild
      'alice',
      'lnd-old',
      'lnd-unofficial', // can't tell (build mismatch)
      'current-cln',
    ])
  })
  it('does not reorder the array it was given', () => {
    const before = demo.map((n) => n.name)
    sortNodes(demo)
    expect(demo.map((n) => n.name)).toEqual(before)
  })
})

describe('fleet summaries', () => {
  it('counts the nodes that need attention', () => {
    expect(needsAction(demo)).toBe(4)
    expect(needsAction(fixtures.green.nodes)).toBe(0)
  })
  it('counts readings', () => {
    expect(readingCounts(demo)).toEqual({ affected: 3, cant_tell: 1, not_affected: 1, none: 0 })
  })
  it('lights the scene with the worst reading', () => {
    expect(worstReading(demo)).toBe('affected')
    expect(worstReading(fixtures.green.nodes)).toBe('not_affected')
  })
  it('names the node to start with, and why', () => {
    expect(priorityLine(demo)).toBe(
      'Start with old-cln: an advisory on it is exploited in the wild.',
    )
    expect(priorityLine([])).toMatch(/Add a node/)
  })
})

describe('nodeSummary', () => {
  it('says what a node’s status rests on', () => {
    expect(nodeSummary(byName('old-cln'))).toBe('2 advisories apply · exploited in the wild')
    expect(nodeSummary(byName('lnd-old'))).toBe('2 advisories apply · 1 to check by hand')
    expect(nodeSummary(byName('lnd-unofficial'))).toBe("Build doesn't match the release")
    expect(nodeSummary(byName('current-cln'))).toBe('None of 2 advisories apply')
  })
  it('leads with an unreadable node’s problem', () => {
    expect(nodeSummary(node({ error: 'down', status: 'cant_tell' }))).toBe(
      "Couldn't read this node",
    )
  })
  it('names a Core Lightning build after the release', () => {
    const n = node({
      status: 'cant_tell',
      build_check: { status: 'unreleased', release: 'v26.06.7', build: 'v26.06.7-12-gabc1234' },
    })
    expect(nodeSummary(n)).toBe('Build is a commit after the release')
  })
})

describe('ruledOutByCondition', () => {
  it('is true only for green findings whose version was in range', () => {
    const ruled = byName('alice').findings.find((f) => f.advisory_id === 'FW-2026-009')!
    expect(ruled.status).toBe('not_affected')
    expect(ruledOutByCondition(ruled)).toBe(true)
    expect(ruledOutByCondition(finding({ status: 'not_affected', in_range: false }))).toBe(false)
    expect(ruledOutByCondition(finding({ status: 'affected', in_range: true }))).toBe(false)
  })
})

describe('versions', () => {
  it('compares dotted versions numerically, ignoring suffixes', () => {
    expect(compareVersions('26.06.9', '26.06.8')).toBeGreaterThan(0)
    expect(compareVersions('26.06.10', '26.06.9')).toBeGreaterThan(0)
    expect(compareVersions('0.20.1-beta', '0.20.1')).toBe(0)
    expect(compareVersions('0.19.0', '0.20.1')).toBeLessThan(0)
  })
  it('splits LND’s build tag off the version', () => {
    expect(splitVersion('0.20.0-beta commit=v0.20.0-beta')).toEqual({
      version: '0.20.0-beta',
      build: 'v0.20.0-beta',
    })
    expect(splitVersion('v26.06.7')).toEqual({ version: 'v26.06.7', build: null })
  })
})

describe('fixedTarget', () => {
  it('is the highest fix among the red findings', () => {
    expect(fixedTarget(byName('old-cln'))).toBe('26.06.9')
    expect(fixedTarget(byName('lnd-old'))).toBe('0.20.1')
  })
  it('gives up when a red finding has no fix, or fixes on several lines', () => {
    expect(fixedTarget(node({ findings: [finding({ fixed: null })] }))).toBeNull()
    expect(fixedTarget(node({ findings: [finding({ fixed: '0.20.2, 0.21.1' })] }))).toBeNull()
    expect(fixedTarget(node({ findings: [] }))).toBeNull()
  })
})

describe('describeError', () => {
  it('leaves the connectors’ plain sentences alone', () => {
    expect(describeError("Can't connect to http://127.0.0.1:8183; is the node running?")).toBeNull()
    expect(describeError('TLS certificate not trusted; check tls_cert for alice')).toBeNull()
  })
  it('names the cause of an older daemon’s raw error', () => {
    const raw =
      'request failed: error sending request for url (http://127.0.0.1:9/v1/getinfo): client error (Connect): tcp connect error: Connection refused (os error 111)'
    expect(describeError(raw)).toBe('Nothing answered at 127.0.0.1:9. Is the node running?')
  })
})

describe('sourceLabel', () => {
  it('names the repo on GitHub and the site elsewhere', () => {
    expect(sourceLabel('https://github.com/ElementsProject/lightning/releases/tag/v26.06.8')).toBe(
      'github.com/ElementsProject/lightning',
    )
    expect(sourceLabel('https://security.lightning.engineering/2026/09/21/x.html')).toBe(
      'security.lightning.engineering',
    )
  })
  it('shows a non-URL as given', () => {
    expect(sourceLabel('not a url')).toBe('not a url')
  })
})

describe('checkedAgo', () => {
  const at = 1_800_000_000
  it('rounds to what a glance needs', () => {
    expect(checkedAgo(at, at * 1000 + 2000)).toBe('Checked just now')
    expect(checkedAgo(at, at * 1000 + 42_000)).toBe('Checked 42 s ago')
    expect(checkedAgo(at, at * 1000 + 5 * 60_000)).toBe('Checked 5 min ago')
  })
  it('never counts backwards when the clocks disagree', () => {
    expect(checkedAgo(at, at * 1000 - 9000)).toBe('Checked just now')
  })
})
