import { AnimatePresence, motion } from 'motion/react'
import type { ReactNode } from 'react'
import { readingWord, type Reading } from '../lib/format'
import { EASE } from '../lib/motion'
import { tone } from '../lib/tone'

// A shape for each verdict, so it reads without colour: a tick, a bang, a
// question mark, a dash.
const glyph: Record<Reading, ReactNode> = {
  affected: (
    <>
      <path d="M6 2.6v4.3" />
      <circle cx="6" cy="9.4" r="0.6" fill="currentColor" />
    </>
  ),
  cant_tell: (
    <>
      <path d="M4.3 4.3a1.8 1.8 0 1 1 2.6 1.6c-.6.3-.9.7-.9 1.3v.2" />
      <circle cx="6" cy="9.5" r="0.6" fill="currentColor" />
    </>
  ),
  not_affected: <path d="M2.8 6.3 5 8.5l4.2-4.8" />,
  none: <path d="M3.4 6h5.2" />,
}

/** The verdict as a word with its icon: colour never carries it alone. The word crossfades on change. */
export function StatusPill({ reading, large = false }: { reading: Reading; large?: boolean }) {
  const t = tone[reading]
  return (
    <motion.span
      layout
      transition={{ duration: 0.45, ease: EASE }}
      className={`inline-flex shrink-0 items-center gap-2 overflow-hidden rounded-full font-semibold whitespace-nowrap transition-colors duration-500 ${t.bg} ${t.text} ${
        large ? 'px-3.5 py-1.5 text-[0.95rem]' : 'px-2.5 py-[3px] text-[0.78rem]'
      }`}
    >
      <span
        aria-hidden
        className={`grid shrink-0 place-items-center rounded-full ${t.dot} text-[var(--fw-on-status)] ${large ? 'size-4' : 'size-3'}`}
      >
        <svg
          viewBox="0 0 12 12"
          className="size-full"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.7"
          strokeLinecap="round"
          strokeLinejoin="round"
        >
          {glyph[reading]}
        </svg>
      </span>
      <AnimatePresence mode="popLayout" initial={false}>
        <motion.span
          key={reading}
          initial={{ opacity: 0, y: 8 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -8 }}
          transition={{ duration: 0.35, ease: EASE }}
        >
          {readingWord[reading]}
        </motion.span>
      </AnimatePresence>
    </motion.span>
  )
}
