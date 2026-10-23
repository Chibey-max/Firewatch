import { AnimatePresence, motion } from 'motion/react'
import { readingWord, type Reading } from '../lib/format'
import { EASE } from '../lib/motion'
import { tone } from '../lib/tone'

/** The verdict as a word with a dot: colour never carries it alone. The word crossfades on change. */
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
      <span aria-hidden className={`rounded-full ${t.dot} ${large ? 'size-2.5' : 'size-[7px]'}`} />
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
