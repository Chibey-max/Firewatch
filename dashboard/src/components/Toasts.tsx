import { AnimatePresence, motion } from 'motion/react'
import { dismiss, useToasts, type ToastTone } from '../lib/toasts'
import { EASE, spring } from '../lib/motion'

const dot: Record<ToastTone, string> = { good: 'bg-green', warn: 'bg-amber', bad: 'bg-red' }

/** Connection changes and failed checks arrive here, bottom right, and leave on their own. */
export function Toasts() {
  const toasts = useToasts()
  return (
    <div
      aria-live="polite"
      className="pointer-events-none fixed right-4 bottom-4 z-50 flex w-[min(380px,calc(100vw-2rem))] flex-col gap-2.5 sm:right-6 sm:bottom-6"
    >
      <AnimatePresence initial={false}>
        {toasts.map((t) => (
          <motion.div
            key={t.id}
            layout
            initial={{ opacity: 0, y: 18, scale: 0.96, filter: 'blur(6px)' }}
            animate={{ opacity: 1, y: 0, scale: 1, filter: 'blur(0px)', transition: { ...spring } }}
            exit={{ opacity: 0, x: 40, transition: { duration: 0.3, ease: EASE } }}
            role={t.tone === 'bad' ? 'alert' : 'status'}
            className="fw-glass pointer-events-auto flex items-start gap-3 rounded-2xl bg-surface-solid px-4 py-3.5 shadow-raised"
          >
            <span aria-hidden className={`mt-1.5 size-2 shrink-0 rounded-full ${dot[t.tone]}`} />
            <div className="min-w-0 flex-1">
              <p className="text-[0.92rem] font-semibold">{t.title}</p>
              {t.body && <p className="mt-0.5 text-[0.85rem] text-ink-2">{t.body}</p>}
            </div>
            <button
              type="button"
              onClick={() => dismiss(t.id)}
              aria-label="Dismiss"
              className="-mt-0.5 -mr-1 grid size-7 place-items-center rounded-full text-ink-3 hover:bg-surface-hover hover:text-ink"
            >
              <svg viewBox="0 0 16 16" className="size-3" aria-hidden>
                <path
                  d="m4 4 8 8M12 4l-8 8"
                  stroke="currentColor"
                  strokeWidth="1.8"
                  strokeLinecap="round"
                />
              </svg>
            </button>
          </motion.div>
        ))}
      </AnimatePresence>
    </div>
  )
}
