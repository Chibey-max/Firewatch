import { fixtures } from './fixtures'

// Placeholder until the dashboard components land (D3-7).
export default function App() {
  return (
    <main className="mx-auto max-w-5xl p-6">
      <h1 className="font-display text-4xl font-extrabold uppercase">Firewatch</h1>
      <pre className="mt-4 overflow-auto font-mono text-sm">
        {JSON.stringify(fixtures.mixed, null, 2)}
      </pre>
    </main>
  )
}
