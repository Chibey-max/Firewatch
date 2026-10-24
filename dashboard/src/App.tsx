import { MotionConfig } from 'motion/react'
import { useCallback, useEffect, useState } from 'react'
import { DetailSheet } from './components/DetailSheet'
import { EmptyState, Loading, SkeletonRows } from './components/EmptyState'
import { FontPicker } from './components/FontPicker'
import { Hero } from './components/Hero'
import { NodeDetail } from './components/NodeDetail'
import { ChangesBanner } from './components/ChangesBanner'
import { NodeList } from './components/NodeList'
import { Scene } from './components/Scene'
import { ThemeToggle } from './components/ThemeToggle'
import { Toasts } from './components/Toasts'
import { Brand, LiveChip, NavBar, RecheckButton, SideRail } from './components/TopBar'
import { isFixtureName } from './fixtures'
import { sortNodes, worstReading } from './lib/format'
import { useMedia, useWide } from './lib/useMedia'
import { useNow, useReport } from './lib/useReport'
import { useSinceLastLook } from './lib/useSeen'

/** The selected node lives in the URL (#node=old-cln), so a presenter can link to it. */
function nodeFromHash(): string | null {
  return new URLSearchParams(window.location.hash.slice(1)).get('node')
}

/**
 * Side by side, choosing a node replaces the hash. On a phone, opening a
 * node's sheet adds a history entry, so Back closes the sheet rather than
 * leaving the page; closing it any other way steps back over that entry.
 */
function useSelectedNode(wide: boolean) {
  const [name, setName] = useState(nodeFromHash)
  useEffect(() => {
    const sync = () => setName(nodeFromHash())
    window.addEventListener('hashchange', sync)
    window.addEventListener('popstate', sync)
    return () => {
      window.removeEventListener('hashchange', sync)
      window.removeEventListener('popstate', sync)
    }
  }, [])
  const select = (next: string) => {
    const url = `#node=${encodeURIComponent(next)}`
    if (wide) window.history.replaceState(null, '', url)
    else window.history.pushState({ fwSheet: true }, '', url)
    setName(next)
  }
  const close = useCallback(() => {
    if (window.history.state?.fwSheet) {
      window.history.back()
    } else {
      window.history.replaceState(null, '', window.location.pathname + window.location.search)
      setName(null)
    }
  }, [])
  return { wanted: name, select, close }
}

const code = 'rounded-md bg-ink/8 px-1.5 py-0.5 font-mono text-[0.88em] text-ink'

export default function App() {
  const param = new URLSearchParams(window.location.search).get('fixture')
  const fixture = isFixtureName(param) ? param : null
  // The type experiment's picker: open the page with ?fonts.
  const showFonts = new URLSearchParams(window.location.search).has('fonts')
  const { report, phase, receivedAt, failures, checking, checkNow } = useReport(fixture)
  const seen = useSinceLastLook(report, fixture ? 'sample' : 'live')
  const now = useNow()
  const wide = useWide()
  const tablet = useMedia('(min-width: 640px)')
  // Tablets get a side rail; ?nav=top tries the phone's bar there instead.
  const rail = tablet && !wide && new URLSearchParams(window.location.search).get('nav') !== 'top'
  const { wanted, select, close } = useSelectedNode(wide)

  const nodes = report ? sortNodes(report.nodes) : []
  const chosen = nodes.find((n) => n.name === wanted) ?? null
  // Side by side, the detail pane always shows a node: the worst one when
  // nothing (or a node that's gone) is chosen. On a phone it's a sheet that
  // only opens when asked.
  const selected = wide ? (chosen ?? nodes[0] ?? null) : chosen
  const sheetOpen = !wide && selected !== null
  const scene = report ? worstReading(report.nodes) : phase === 'offline' ? 'cant_tell' : 'none'

  const controls = (
    <>
      {phase !== 'fixture' && (
        <RecheckButton
          onCheck={checkNow}
          checking={checking}
          size={wide ? 'full' : rail ? 'icon' : 'compact'}
        />
      )}
      <ThemeToggle />
    </>
  )

  return (
    <MotionConfig reducedMotion="user">
      <Scene reading={scene} />
      {!wide &&
        (rail ? (
          <SideRail reading={scene}>{controls}</SideRail>
        ) : (
          <NavBar inert={sheetOpen} narrow={tablet} reading={scene}>
            {controls}
          </NavBar>
        ))}
      {/* Two panes of glass over the lookout: the shell (headline and nodes)
          and the slab (the selected node). On large screens each scrolls on
          its own and the page stays put; on a phone the slab is a sheet. */}
      <div
        inert={sheetOpen}
        className={`mx-auto grid max-w-[1720px] gap-5 p-3 sm:p-5 ${rail ? 'min-h-[100dvh] content-center sm:pr-[92px]' : ''} lg:h-[100dvh] lg:grid-cols-[minmax(400px,500px)_1fr] lg:p-6 min-[1600px]:grid-cols-[560px_1fr]`}
      >
        <aside className="fw-shell flex min-h-0 flex-col overflow-hidden rounded-[28px]">
          <div className="flex flex-wrap items-center gap-2.5 p-5 pb-0 sm:p-6 sm:pb-0">
            {wide && <Brand reading={scene} />}
            <LiveChip
              phase={phase}
              fixture={fixture}
              checkedAt={report?.checked_at ?? null}
              receivedAt={receivedAt}
              failures={failures}
              now={now}
            />
          </div>
          <div className="flex min-h-0 flex-1 flex-col gap-7 overflow-y-auto p-5 sm:p-6 sm:pt-9">
            <ChangesBanner
              changes={seen.changes}
              since={seen.since}
              now={now}
              onSelect={select}
              onDismiss={seen.dismiss}
            />
            <Hero report={report} phase={phase} />
            {!report && phase !== 'offline' && <SkeletonRows />}
            {report && nodes.length > 0 && (
              <NodeList
                nodes={nodes}
                selected={selected?.name ?? null}
                changed={new Set(seen.changes.map((c) => c.node))}
                onSelect={select}
              />
            )}
          </div>
          <p className="border-t border-line px-6 py-3.5 text-[0.78rem] leading-snug text-ink-2">
            Read-only: Firewatch can’t change or spend from your nodes. Every fix is quoted from the
            project’s own release notes.
          </p>
        </aside>

        {/* On a phone or tablet the column only holds the empty and offline
            states: nodes open over the list, and loading shows in the list. */}
        <section
          className={`flex min-h-0 flex-col gap-4 ${!wide && (report ? nodes.length > 0 : phase !== 'offline') ? 'hidden' : ''}`}
        >
          {wide && <div className="flex items-center justify-end gap-2.5">{controls}</div>}
          <div id="detail" className="min-h-0 flex-1 overflow-y-auto rounded-[28px]">
            {report ? (
              nodes.length > 0 ? (
                wide && selected && <NodeDetail node={selected} />
              ) : (
                <EmptyState title="No nodes to watch">
                  Add a node to <code className={code}>firewatch.toml</code>, then restart{' '}
                  <code className={code}>firewatch watch</code>.
                </EmptyState>
              )
            ) : phase === 'offline' ? (
              <EmptyState title="Firewatch isn’t answering">
                It retries every few seconds. Start it from the repo root with{' '}
                <code className={code}>cargo run -- watch</code>, or{' '}
                <a
                  href="?fixture=demo"
                  className="font-semibold text-ink underline underline-offset-4"
                >
                  view sample data
                </a>
                .
              </EmptyState>
            ) : (
              <Loading />
            )}
          </div>
        </section>
      </div>
      {!wide && <DetailSheet node={selected} onClose={close} beside={rail} />}
      <Toasts />
      {showFonts && <FontPicker />}
    </MotionConfig>
  )
}
