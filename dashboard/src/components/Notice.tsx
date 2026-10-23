import type { ReactNode } from 'react'

/** A strip above the header for the data's state: sample, stale or starting. */
export function Notice({ tone, children }: { tone: 'plain' | 'amber'; children: ReactNode }) {
  const tones = {
    plain: 'border-line bg-surface text-fg-2',
    amber: 'border-amber/40 bg-amber-soft text-fg',
  }
  return (
    <div
      role="status"
      className={`mb-5 rounded-lg border px-4 py-2.5 text-[0.88rem] ${tones[tone]}`}
    >
      {children}
    </div>
  )
}

/** Placeholder rows while the first report loads. */
export function Loading() {
  return (
    <div className="grid gap-6 lg:grid-cols-[minmax(320px,400px)_1fr]">
      <div className="space-y-2">
        {[0, 1, 2].map((i) => (
          <div
            key={i}
            className="h-[76px] animate-pulse rounded-lg bg-surface motion-reduce:animate-none"
          />
        ))}
      </div>
      <div className="h-[420px] animate-pulse rounded-xl bg-surface motion-reduce:animate-none" />
    </div>
  )
}
