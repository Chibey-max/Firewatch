import { describe, expect, it } from 'vitest'
import { fixtures } from '../fixtures'
import { ago, changesSince, describeChange, snapshot } from './seen'

const before = fixtures.demo
const after = fixtures['demo-after']

describe('changesSince', () => {
  it('says nothing on a first visit', () => {
    expect(changesSince(null, before)).toEqual([])
  })
  it('says nothing when nothing changed', () => {
    expect(changesSince(snapshot(before), before)).toEqual([])
  })
  it('reports the upgrade as a verdict change', () => {
    const changes = changesSince(snapshot(before), after)
    expect(changes).toContainEqual({
      kind: 'verdict',
      node: 'old-cln',
      from: 'affected',
      to: 'not_affected',
    })
    expect(changes.map(describeChange)).toContain('old-cln is now not affected (was affected)')
  })
  it('reports an advisory new to a node, and a new node', () => {
    const prev = snapshot(before)
    delete prev.nodes['alice']
    const firstRed = before.nodes.find((n) => n.name === 'old-cln')!.findings[0]
    delete prev.nodes['old-cln'].findings[firstRed.advisory_id]
    const changes = changesSince(prev, before)
    expect(changes).toContainEqual({ kind: 'added', node: 'alice', reading: 'affected' })
    expect(changes).toContainEqual(
      expect.objectContaining({ kind: 'advisory', node: 'old-cln', id: firstRed.advisory_id }),
    )
  })
})

describe('ago', () => {
  const at = 1_800_000_000
  it('rounds to what a glance needs', () => {
    expect(ago(at, at * 1000 + 20_000)).toBe('moments ago')
    expect(ago(at, at * 1000 + 2 * 3_600_000)).toBe('2 h ago')
    expect(ago(at, at * 1000 + 30 * 3_600_000)).toBe('yesterday')
  })
})
