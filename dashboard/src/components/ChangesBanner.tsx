import { AnimatePresence, motion } from 'motion/react'
import { SOFT } from '../lib/motion'
import { ago, changeReading, describeChange, type Change } from '../lib/seen'
import { tone } from '../lib/tone'

const SHOWN = 4

/**
 * "Since you last looked": what moved between visits, or while the page was
 * open. Each line opens its node; "Got it" clears the list.
 */
export function ChangesBanner({
  changes,
  since,
  now,
  onSelect,
  onDismiss,
}: {
  changes: Change[]
  since: number | null
  now: number
  onSelect: (node: string) => void
  onDismiss: () => void
}) {
  const shown = changes.slice(0, SHOWN)
  const more = changes.length - shown.length

  return (
    <AnimatePresence initial={false}>
      {changes.length > 0 && (
        <motion.section
          key="changes"
          aria-label="Since you last looked"
          aria-live="polite"
          className="shrink-0 overflow-hidden"
          initial={{ height: 0, opacity: 0 }}
          animate={{ height: 'auto', opacity: 1, transition: { duration: 0.5, ease: SOFT } }}
          exit={{ height: 0, opacity: 0, transition: { duration: 0.35, ease: SOFT } }}
        >
          <div className="fw-glass rounded-[18px] px-4 py-3.5">
            <div className="flex items-center justify-between gap-3">
              <p className="font-mono text-[0.72rem] tracking-[0.14em] text-ink-3 uppercase">
                Since you last looked{since !== null && ` · ${ago(since, now)}`}
              </p>
              <button
                type="button"
                onClick={onDismiss}
                className="text-[0.82rem] font-semibold text-ink-2 underline decoration-line-strong underline-offset-4 hover:text-ink hover:decoration-ink"
              >
                Got it
              </button>
            </div>
            <ul className="mt-2 space-y-1.5">
              {shown.map((c, i) => (
                <li key={i}>
                  <button
                    type="button"
                    onClick={() => onSelect(c.node)}
                    className="flex items-start gap-2.5 text-left text-[0.92rem] leading-snug text-ink hover:underline hover:underline-offset-4"
                  >
                    <span
                      aria-hidden
                      className={`mt-[0.4em] size-2 shrink-0 rounded-full ${tone[changeReading(c)].dot}`}
                    />
                    {describeChange(c)}
                  </button>
                </li>
              ))}
            </ul>
            {more > 0 && <p className="mt-1.5 text-[0.82rem] text-ink-3">and {more} more</p>}
          </div>
        </motion.section>
      )}
    </AnimatePresence>
  )
}
