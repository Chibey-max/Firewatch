import { describe, expect, it } from 'vitest'
import { fixtures } from '../fixtures'
import { helpText, scrub } from './helpText'

const byName = (fixture: 'demo' | 'error', name: string) =>
  fixtures[fixture].nodes.find((n) => n.name === name)!

describe('helpText', () => {
  it('names what runs, the verdict, and quotes the developers with their link', () => {
    const text = helpText(byName('demo', 'old-cln'), 1_800_000_000)
    expect(text).toContain('Core Lightning v26.06.7 on regtest: Affected')
    expect(text).toContain('exploited in the wild')
    expect(text).toContain('The developers: "We strongly recommend upgrading to this release."')
    expect(text).toContain('https://github.com/ElementsProject/lightning/releases/tag/')
  })
  it('leaves out the node name, URLs, addresses and ports', () => {
    for (const [fixture, name] of [
      ['demo', 'old-cln'],
      ['demo', 'lnd-unofficial'],
      ['error', 'offline-node'],
    ] as const) {
      const node = byName(fixture, name)
      const text = helpText(node, 1_800_000_000)
      expect(text).not.toContain(name)
      expect(text).not.toMatch(/127\.0\.0\.1|0\.0\.0\.0|localhost|:\d{4,5}\b/)
      expect(text).not.toMatch(/https?:\/\/(127|0\.0|localhost)/)
    }
  })
  it('keeps the reason an unreadable node gives, without its address', () => {
    const text = helpText(byName('error', 'offline-node'), 1_800_000_000)
    expect(text).toMatch(/Couldn't read the node: Can't connect to <node url>/)
  })
})

describe('scrub', () => {
  it('replaces URLs and IP addresses, and keeps versions', () => {
    expect(scrub('Timed out after 10 s waiting for https://10.0.0.5:8080/v1/getinfo')).toBe(
      'Timed out after 10 s waiting for <node url>',
    )
    expect(scrub('listens on 192.168.1.20:10009 and [::1]:8080')).toBe(
      'listens on <address> and <address>',
    )
    expect(scrub('fixed in 26.06.9 and 0.20.1-beta')).toBe('fixed in 26.06.9 and 0.20.1-beta')
  })
})
