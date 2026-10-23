import { MotionConfig } from 'motion/react'
import { useEffect, useState } from 'react'
import { Ambient } from './components/Ambient'
import { EmptyState, Loading } from './components/EmptyState'
import { Hero } from './components/Hero'
import { NodeDetail } from './components/NodeDetail'
import { NodeList } from './components/NodeList'
import { Toasts } from './components/Toasts'
import { TopBar } from './components/TopBar'
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
  }
  return [name, select]
}

const code = 'rounded-md bg-ink/8 px-1.5 py-0.5 font-mono text-[0.88em] text-ink'

export default function App() {
  const param = new URLSearchParams(window.location.search).get('fixture')
  const fixture = isFixtureName(param) ? param : null
  const { report, phase, receivedAt, failures, checking, checkNow } = useReport(fixture)
  const now = useNow()
  const [wanted, select] = useSelectedNode()

  const nodes = report ? sortNodes(report.nodes) : []
  // Fall back to the worst node when nothing (or a node that's gone) is selected.
  const selected = nodes.find((n) => n.name === wanted) ?? nodes[0] ?? null
  const ambient = report ? worstReading(report.nodes) : phase === 'offline' ? 'cant_tell' : 'none'

  return (
    <MotionConfig reducedMotion="user">
      <Ambient reading={ambient} />
      <div className="mx-auto max-w-[1400px] px-4 py-5 sm:px-8 sm:py-6 min-[1600px]:max-w-[1640px]">
        <TopBar
          phase={phase}
          fixture={fixture}
          checkedAt={report?.checked_at ?? null}
          receivedAt={receivedAt}
          failures={failures}
          now={now}
          checking={checking}
          onCheck={checkNow}
        />

        <Hero report={report} phase={phase} />

        <main>
          {report ? (
            selected ? (
              <div className="grid items-start gap-6 lg:grid-cols-[minmax(320px,410px)_1fr]">
                <div className="lg:sticky lg:top-6">
                  <NodeList nodes={nodes} selected={selected.name} onSelect={select} />
                </div>
                <NodeDetail node={selected} />
              </div>
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
        </main>

        <footer className="mt-14 flex flex-wrap justify-between gap-3 border-t border-line pt-5 text-[0.82rem] text-ink-3">
          <span>
            Read-only: Firewatch can’t change or spend from your nodes. Every fix is quoted from the
            project’s own release notes, with a link.
          </span>
          <span className="font-mono">github.com/Chibey-max/Firewatch</span>
        </footer>
      </div>
      <Toasts />
    </MotionConfig>
  )
}
