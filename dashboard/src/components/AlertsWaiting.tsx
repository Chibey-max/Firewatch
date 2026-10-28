import { AnimatePresence, motion } from 'motion/react'
import { pendingLabel } from '../lib/format'
import { SOFT } from '../lib/motion'
import type { Health } from '../types/report'

/**
 * Beside the live status while alerts wait to be sent (no internet, relays
 * down): "1 alert waiting to send · offline since 14:02". It's about
 * delivery, not a node, so it wears no verdict colour. It clears when the
 * alerts go out.
 */
export function AlertsWaiting({ health, now }: { health: Health | null; now: number }) {
  const count = health?.alerts_pending ?? 0
  const label = count > 0 ? pendingLabel(count, health?.oldest_pending_since, now) : null

  return (
    <AnimatePresence initial={false}>
      {label && (
        <motion.div
          key="waiting"
          role="status"
          title="Firewatch retries every check and sends them the moment the connection is back."
          className="fw-glass flex h-10 items-center gap-2.5 rounded-full px-4 text-[0.85rem] text-ink-2"
          initial={{ opacity: 0, scale: 0.96 }}
          animate={{ opacity: 1, scale: 1, transition: { duration: 0.4, ease: SOFT } }}
          exit={{ opacity: 0, scale: 0.96, transition: { duration: 0.3, ease: SOFT } }}
        >
          <svg
            viewBox="0 0 16 16"
            className="size-4 shrink-0"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.5"
            strokeLinejoin="round"
            aria-hidden
          >
            <path d="M14.5 1.5 1.5 6.8l5 1.7 1.7 5z" />
            <path d="M14.5 1.5 6.5 8.5" strokeLinecap="round" />
          </svg>
          <span>
            <span className="font-semibold text-ink">{label.waiting}</span>
            {label.since && <span className="tabular-nums"> · {label.since}</span>}
          </span>
        </motion.div>
      )}
    </AnimatePresence>
  )
}
