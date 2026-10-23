import { motion } from 'motion/react'
import type { ReactNode } from 'react'
import { rise } from '../lib/motion'

export function EmptyState({ title, children }: { title: string; children: ReactNode }) {
  return (
    <motion.div
      initial="hidden"
      animate="show"
      variants={rise}
      className="fw-glass mx-auto mt-4 max-w-lg rounded-[28px] px-8 py-10 text-center"
    >
      <p className="font-serif text-[1.6rem] font-medium tracking-[-0.01em]">{title}</p>
      <p className="mt-2.5 text-[1rem] leading-relaxed text-ink-2">{children}</p>
    </motion.div>
  )
}

/** Placeholder rows and panel while the first report loads. */
export function Loading() {
  return (
    <div className="grid gap-6 lg:grid-cols-[minmax(320px,410px)_1fr]">
      <div className="space-y-2.5">
        {[0, 1, 2, 3].map((i) => (
          <div key={i} className="fw-shimmer h-[84px] rounded-[18px] border border-line" />
        ))}
      </div>
      <div className="fw-shimmer h-[460px] rounded-[28px] border border-line" />
    </div>
  )
}
