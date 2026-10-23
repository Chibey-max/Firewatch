import { useEffect, useState } from 'react'
import { CheckControls } from './components/CheckControls'
import { EmptyState } from './components/EmptyState'
import { Header } from './components/Header'
import { NodeDetail } from './components/NodeDetail'
import { NodeList } from './components/NodeList'
import { Loading, Notice } from './components/Notice'
import { ThemeToggle } from './components/ThemeToggle'
import { isFixtureName } from './fixtures'
import { clockTime, sortNodes } from './lib/format'
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

const code = 'rounded bg-fg/10 px-1.5 font-mono text-[0.9em] text-fg'

export default function App() {
  const param = new URLSearchParams(window.location.search).get('fixture')
  const fixture = isFixtureName(param) ? param : null
  const { report, phase, receivedAt, checking, checkError, checkNow } = useReport(fixture)
  const now = useNow()
  const [wanted, select] = useSelectedNode()

  const nodes = report ? sortNodes(report.nodes) : []
  // Fall back to the worst node when nothing (or a node that's gone) is selected.
  const selected = nodes.find((n) => n.name === wanted) ?? nodes[0] ?? null

  return (
    <div className="mx-auto max-w-[1440px] px-4 py-5 sm:px-8 min-[1600px]:max-w-[1720px]">
      {phase === 'fixture' && (
        <Notice tone="plain">
          Sample data from fixture “{fixture}”. Live checks are off.{' '}
          <a href="./" className="font-semibold text-fg underline underline-offset-4">
            Show live data
          </a>
        </Notice>
      )}
      {phase === 'first_check' && (
        <Notice tone="amber">
          Firewatch is running its first check. Results appear in a few seconds.
        </Notice>
      )}
      {phase === 'offline' && report && receivedAt && (
        <Notice tone="amber">
          Can’t reach the Firewatch daemon. Showing the report from {clockTime(receivedAt)}; it
          updates again once the daemon answers.
        </Notice>
      )}

      <Header report={report}>
        <CheckControls
          checkedAt={report?.checked_at ?? null}
          now={now}
          checking={checking}
          checkError={checkError}
          onCheck={phase === 'fixture' ? null : checkNow}
        />
        <ThemeToggle />
      </Header>

      <main className="mt-6">
        {report ? (
          selected ? (
            <div className="grid items-start gap-6 lg:grid-cols-[minmax(320px,400px)_1fr]">
              <div className="lg:sticky lg:top-5">
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
          <EmptyState title="Can’t reach the Firewatch daemon">
            Start it from the repo root with <code className={code}>cargo run -- watch</code>, or{' '}
            <a href="?fixture=demo" className="font-semibold text-fg underline underline-offset-4">
              view sample data
            </a>
            .
          </EmptyState>
        ) : (
          <Loading />
        )}
      </main>

      <footer className="mt-12 border-t border-line pt-4 text-[0.82rem] text-fg-3">
        Read-only: Firewatch can’t change or spend from your nodes. Every fix is quoted from the
        project’s own release notes, with a link.
      </footer>
    </div>
  )
}
