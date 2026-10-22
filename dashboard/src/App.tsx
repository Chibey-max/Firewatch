import { CheckControls } from './components/CheckControls'
import { EmptyState } from './components/EmptyState'
import { Header } from './components/Header'
import { NodeCard } from './components/NodeCard'
import { LoadingCards, Notice } from './components/Notice'
import { isFixtureName } from './fixtures'
import { clockTime } from './lib/format'
import { useNow, useReport } from './lib/useReport'

export default function App() {
  const param = new URLSearchParams(window.location.search).get('fixture')
  const fixture = isFixtureName(param) ? param : null
  const { report, phase, receivedAt, checking, checkError, checkNow } = useReport(fixture)
  const now = useNow()

  return (
    <div className="mx-auto max-w-[1480px] px-4 py-6 sm:px-8">
      {phase === 'fixture' && (
        <Notice tone="ink">
          Sample data from fixture “{fixture}”. Live checks are off.{' '}
          <a href="./" className="font-semibold underline underline-offset-4">
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
          Can't reach the Firewatch daemon. Showing the report received at {clockTime(receivedAt)};
          it updates again once the daemon answers.
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
      </Header>

      <main className="mt-6">
        {report ? (
          report.nodes.length === 0 ? (
            <EmptyState />
          ) : (
            <div className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,350px),1fr))] items-start gap-5">
              {report.nodes.map((node) => (
                <NodeCard key={node.name} node={node} />
              ))}
            </div>
          )
        ) : phase === 'offline' ? (
          <div className="mx-auto mt-16 max-w-lg text-center">
            <p className="font-display text-[2rem] font-bold">Can't reach the Firewatch daemon</p>
            <p className="mt-2 text-[1.02rem] leading-relaxed text-ink-2">
              Start it from the repo root with{' '}
              <code className="font-mono text-ink">cargo run -- watch</code>, or{' '}
              <a
                href="?fixture=mixed"
                className="font-semibold text-ink underline underline-offset-4"
              >
                view sample data
              </a>
              .
            </p>
          </div>
        ) : (
          <LoadingCards />
        )}
      </main>

      <footer className="mt-10 border-t border-rule pt-4 text-[0.88rem] text-ink-2">
        Read-only: Firewatch can't change or spend from your nodes. Every fix is quoted from the
        project's own release notes, with a link.
      </footer>
    </div>
  )
}
