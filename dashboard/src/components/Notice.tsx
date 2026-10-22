import type { ReactNode } from 'react'

/** A full-width strip above the header for the data's state: sample, stale or starting. */
export function Notice({ tone, children }: { tone: 'ink' | 'amber'; children: ReactNode }) {
  const tones = {
    ink: 'bg-ink text-panel',
    amber: 'bg-canttell-tint text-canttell-ink border border-canttell/40',
  }
  return (
    <div role="status" className={`mb-5 rounded-sm px-4 py-2 text-[0.92rem] ${tones[tone]}`}>
      {children}
    </div>
  )
}

/** Placeholder cards while the first report loads. */
export function LoadingCards() {
  return (
    <div className="grid grid-cols-[repeat(auto-fit,minmax(min(100%,350px),1fr))] gap-5">
      {[0, 1, 2].map((i) => (
        <div
          key={i}
          className="h-[420px] animate-pulse rounded-md bg-panel/70 motion-reduce:animate-none"
        />
      ))}
    </div>
  )
}
