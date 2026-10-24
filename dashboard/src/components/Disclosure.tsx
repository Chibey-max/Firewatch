import { AnimatePresence, motion, useReducedMotion } from 'motion/react'
import { useEffect, useId, useRef, useState, type ReactNode } from 'react'
import { SOFT } from '../lib/motion'

type State = 'closed' | 'loading' | 'open'

const LOAD_MS = 480

const list = { hidden: {}, show: { transition: { staggerChildren: 0.07, delayChildren: 0.12 } } }

/**
 * A folded list. Opening shows a short spinner where the arrow was, then the
 * fold eases to its full height while the items rise in one after another.
 * Closing folds it straight away. Under reduced motion it just opens.
 */
export function Disclosure({ label, children }: { label: ReactNode; children: ReactNode }) {
  const reduce = useReducedMotion()
  const [state, setState] = useState<State>('closed')
  const timer = useRef<number>(undefined)
  const id = useId()

  useEffect(() => () => window.clearTimeout(timer.current), [])

  const toggle = () => {
    window.clearTimeout(timer.current)
    if (state !== 'closed') {
      setState('closed')
      return
    }
    if (reduce) {
      setState('open')
      return
    }
    setState('loading')
    timer.current = window.setTimeout(() => setState('open'), LOAD_MS)
  }

  return (
    <div>
      <button
        type="button"
        onClick={toggle}
        aria-expanded={state === 'open'}
        aria-controls={id}
        aria-busy={state === 'loading'}
        className="flex items-center gap-2 text-[0.86rem] text-ink-3 select-none hover:text-ink-2"
      >
        <span aria-hidden className="relative grid size-3.5 place-items-center">
          <AnimatePresence initial={false}>
            {state === 'loading' ? (
              <motion.svg
                key="spin"
                viewBox="0 0 16 16"
                className="absolute size-3.5"
                initial={{ opacity: 0, scale: 0.6 }}
                animate={{ opacity: 1, scale: 1, rotate: 360 }}
                exit={{ opacity: 0, scale: 0.6 }}
                transition={{
                  opacity: { duration: 0.2 },
                  scale: { duration: 0.2 },
                  rotate: { duration: 0.8, ease: 'linear', repeat: Infinity },
                }}
              >
                <circle
                  cx="8"
                  cy="8"
                  r="6"
                  fill="none"
                  stroke="currentColor"
                  strokeOpacity=".25"
                  strokeWidth="2"
                />
                <path
                  d="M8 2a6 6 0 0 1 6 6"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="2"
                  strokeLinecap="round"
                />
              </motion.svg>
            ) : (
              <motion.span
                key="arrow"
                className="absolute leading-none"
                initial={{ opacity: 0, scale: 0.6 }}
                animate={{ opacity: 1, scale: 1, rotate: state === 'open' ? 90 : 0 }}
                exit={{ opacity: 0, scale: 0.6 }}
                transition={{ duration: 0.3, ease: SOFT }}
              >
                ▸
              </motion.span>
            )}
          </AnimatePresence>
        </span>
        {label}
      </button>
      <AnimatePresence initial={false}>
        {state === 'open' && (
          <motion.div
            id={id}
            key="fold"
            className="overflow-hidden"
            initial={{ height: 0, opacity: 0 }}
            animate={{
              height: 'auto',
              opacity: 1,
              transition: { height: { duration: 0.6, ease: SOFT }, opacity: { duration: 0.3 } },
            }}
            exit={{
              height: 0,
              opacity: 0,
              transition: { height: { duration: 0.45, ease: SOFT }, opacity: { duration: 0.25 } },
            }}
          >
            <motion.ul variants={list} initial="hidden" animate="show" className="space-y-3 pt-3">
              {children}
            </motion.ul>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  )
}
