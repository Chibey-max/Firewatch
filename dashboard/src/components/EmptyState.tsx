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

function Bar({ className, width }: { className: string; width?: string }) {
  return (
    <span aria-hidden className={`fw-skeleton block rounded-full ${className}`} style={{ width }} />
  )
}

/** Skeleton rows in the node list while the first report loads. */
export function SkeletonRows() {
  return (
    <div aria-hidden>
      <Bar className="mb-4 ml-1 h-2.5 w-36" />
      <ul className="space-y-2.5">
        {[0.62, 0.48, 0.55].map((w, i) => (
          <li
            key={i}
            className="fw-glass grid grid-cols-[auto_1fr_auto] items-center gap-x-3.5 rounded-[18px] px-4 py-3.5"
          >
            <span className="fw-skeleton h-9 w-1 rounded-full" />
            <span className="space-y-2">
              <Bar className="h-3.5" width={`${w * 100}%`} />
              <Bar className="h-2.5 w-2/5" />
            </span>
            <Bar className="h-5 w-20" />
          </li>
        ))}
      </ul>
    </div>
  )
}

/** A skeleton of the detail panel while the first report loads. */
export function Loading() {
  return (
    <div
      role="status"
      aria-label="Loading the report"
      className="fw-glass-strong h-full min-h-[460px] rounded-[28px] p-6 sm:p-9"
    >
      <div className="flex items-start justify-between">
        <span className="space-y-3">
          <Bar className="h-8 w-48" />
          <Bar className="h-3 w-56" />
        </span>
        <Bar className="h-8 w-28" />
      </div>
      <Bar className="mt-6 h-3 w-64" />
      <span aria-hidden className="fw-skeleton mt-6 block h-20 rounded-2xl" />
      <Bar className="mt-9 h-2.5 w-28" />
      {[0, 1].map((i) => (
        <span key={i} aria-hidden className="fw-skeleton mt-3 block h-32 rounded-2xl" />
      ))}
    </div>
  )
}
