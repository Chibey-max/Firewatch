import { EmptyState } from './components/EmptyState'
import { Header } from './components/Header'
import { NodeCard } from './components/NodeCard'
import { fixtures, isFixtureName } from './fixtures'

export default function App() {
  const param = new URLSearchParams(window.location.search).get('fixture')
  const name = isFixtureName(param) ? param : 'mixed'
  const report = fixtures[name]

  return (
    <div className="mx-auto max-w-[1480px] px-4 py-6 sm:px-8">
      <p className="mb-5 rounded-sm bg-ink px-4 py-2 text-[0.92rem] text-panel">
        Sample data from fixture “{name}”. Live checks are off.
      </p>
      <Header report={report} />
      <main className="mt-6">
        {report.nodes.length === 0 ? (
          <EmptyState />
        ) : (
          <div className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,350px),1fr))] items-start gap-5">
            {report.nodes.map((node) => (
              <NodeCard key={node.name} node={node} />
            ))}
          </div>
        )}
      </main>
      <footer className="mt-10 border-t border-rule pt-4 text-[0.88rem] text-ink-2">
        Read-only: Firewatch can't change or spend from your nodes. Every fix is quoted from the
        project's own release notes, with a link.
      </footer>
    </div>
  )
}
