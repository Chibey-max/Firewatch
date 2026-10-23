import { motion } from 'motion/react'
import type { ReactNode } from 'react'
import { rise } from '../lib/motion'

export function EmptyState({ title, children }: { title: string; children: ReactNode }) {
  return (
    <motion.div
      initial="hidden"
      animate="show"
      variants={rise}
      className="fw-glass-strong grid min-h-full place-content-center rounded-[28px] px-8 py-14 text-center"
    >
      <p className="fw-display text-[1.6rem]">{title}</p>
      <p className="mx-auto mt-2.5 max-w-md text-[1rem] leading-relaxed text-ink-2">{children}</p>
    </motion.div>
  )
}

/** Placeholder panel while the first report loads. */
export function Loading() {
  return <div className="fw-shimmer h-full min-h-[460px] rounded-[28px] border border-line" />
}
