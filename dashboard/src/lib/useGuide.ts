import { useCallback, useEffect, useState } from 'react'

export type GuideTab = 'setup' | 'alerts' | 'dashboard'
const TABS: readonly string[] = ['setup', 'alerts', 'dashboard']

// The guide lives in the hash next to the node (#node=old-cln&guide=alerts),
// so it can be linked to and the node behind it stays put. #guide alone opens
// the first tab.
function fromHash(): GuideTab | null {
  const params = new URLSearchParams(window.location.hash.slice(1))
  if (!params.has('guide')) return null
  const tab = params.get('guide') ?? ''
  return TABS.includes(tab) ? (tab as GuideTab) : 'setup'
}

function urlWith(tab: GuideTab | null): string {
  const params = new URLSearchParams(window.location.hash.slice(1))
  if (tab) params.set('guide', tab)
  else params.delete('guide')
  const hash = params.toString()
  return hash ? `#${hash}` : window.location.pathname + window.location.search
}

/**
 * The how-to guide's state. Opening it adds a history entry, so Back closes
 * it; switching tabs replaces the entry; closing it any other way steps back
 * over the entry it added.
 */
export function useGuide() {
  const [tab, setTab] = useState(fromHash)
  useEffect(() => {
    const sync = () => setTab(fromHash())
    window.addEventListener('hashchange', sync)
    window.addEventListener('popstate', sync)
    return () => {
      window.removeEventListener('hashchange', sync)
      window.removeEventListener('popstate', sync)
    }
  }, [])

  const open = (next: GuideTab = 'setup') => {
    window.history.pushState({ fwGuide: true }, '', urlWith(next))
    setTab(next)
  }
  const show = (next: GuideTab) => {
    window.history.replaceState(window.history.state, '', urlWith(next))
    setTab(next)
  }
  const close = useCallback(() => {
    if (window.history.state?.fwGuide) {
      window.history.back()
    } else {
      window.history.replaceState(null, '', urlWith(null))
      setTab(null)
    }
  }, [])

  return { tab, open, show, close }
}
