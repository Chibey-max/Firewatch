import { MotionConfig } from 'motion/react'
import { useEffect, useState } from 'react'
import { EmptyState, Loading } from './components/EmptyState'
import { FontPicker } from './components/FontPicker'
import { Hero } from './components/Hero'
import { NodeDetail } from './components/NodeDetail'
import { NodeList } from './components/NodeList'
import { Scene } from './components/Scene'
import { SurfaceMotion } from './components/SurfaceMotion'
import { ThemeToggle } from './components/ThemeToggle'
import { Toasts } from './components/Toasts'
import { Brand, LiveChip, RecheckButton } from './components/TopBar'
import { isFixtureName } from './fixtures'
import { sortNodes, worstReading } from './lib/format'
import { useNow, useReport } from './lib/useReport'

/** The selected node lives in the URL (#node=old-cln), so a presenter can link to it. */
function nodeFromHash(): string | null {
  return new URLSearchParams(window.location.hash.slice(1)).get('node')
}

function useSelectedNode(): [string | null, (name: string) => void] {
  const [name, setName] = useState(nodeFromHash)
  useEffect(() => {
    const onHash = () => setName(nodeFromHash())
    window.addEventListener('hashchange', onHash)
    return () => window.removeEventListener('hashchange', onHash)
  }, [])
  const select = (next: string) => {
    window.history.replaceState(null, '', `#node=${encodeURIComponent(next)}`)
    setName(next)
    // Stacked layout: the detail is below the list, so bring it into view.
    if (window.matchMedia('(max-width: 1023px)').matches) {
      const smooth = !window.matchMedia('(prefers-reduced-motion: reduce)').matches
      requestAnimationFrame(() =>
        document.getElementById('detail')?.scrollIntoView({ behavior: smooth ? 'smooth' : 'auto' }),
      )
    }
  }
  return [name, select]
}

const code = 'rounded-md bg-ink/8 px-1.5 py-0.5 font-mono text-[0.88em] text-ink'

export default function App() {
  const param = new URLSearchParams(window.location.search).get('fixture')
  const fixture = isFixtureName(param) ? param : null
  // The type experiment's picker: open the page with ?fonts.
  const showFonts = new URLSearchParams(window.location.search).has('fonts')
  const { report, phase, receivedAt, failures, checking, checkNow } = useReport(fixture)
  const now = useNow()
  const [wanted, select] = useSelectedNode()

  const nodes = report ? sortNodes(report.nodes) : []
  // Fall back to the worst node when nothing (or a node that's gone) is selected.
  const selected = nodes.find((n) => n.name === wanted) ?? nodes[0] ?? null
  const scene = report ? worstReading(report.nodes) : phase === 'offline' ? 'cant_tell' : 'none'

  return (
    <MotionConfig reducedMotion="user">
      <Scene reading={scene} />
      {/* Two panes of glass over the lookout: the shell (headline and nodes)
          and the slab (the selected node). On large screens each scrolls on
          its own and the page stays put. */}
      <div className="mx-auto grid max-w-[1720px] gap-5 p-3 sm:p-5 lg:h-[100dvh] lg:grid-cols-[minmax(400px,500px)_1fr] lg:p-6 min-[1600px]:grid-cols-[560px_1fr]">
        <aside
          data-surface
          className="fw-shell flex min-h-0 flex-col overflow-hidden rounded-[28px]"
        >
          <div className="flex flex-wrap items-center gap-2.5 p-5 pb-0 sm:p-6 sm:pb-0">
            <Brand />
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
            <Hero report={report} phase={phase} />
            {report && selected && (
              <NodeList nodes={nodes} selected={selected.name} onSelect={select} />
            )}
          </div>
          <p className="border-t border-line px-6 py-3.5 text-[0.78rem] leading-snug text-ink-2">
            Read-only: Firewatch can’t change or spend from your nodes. Every fix is quoted from the
            project’s own release notes.
          </p>
        </aside>

        <section className="flex min-h-0 flex-col gap-4">
          <div className="flex items-center justify-end gap-2.5">
            {phase !== 'fixture' && <RecheckButton onCheck={checkNow} checking={checking} />}
            <ThemeToggle />
          </div>
          <div id="detail" className="min-h-0 flex-1 scroll-mt-3 overflow-y-auto rounded-[28px]">
            {report ? (
              selected ? (
                <NodeDetail node={selected} />
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
      <Toasts />
      <SurfaceMotion />
      {showFonts && <FontPicker />}
    </MotionConfig>
  )
}
