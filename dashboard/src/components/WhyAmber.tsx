import { AnimatePresence, motion } from 'motion/react'
import { useEffect, useId, useRef, useState } from 'react'
import { amberReason } from '../lib/format'
import { SOFT } from '../lib/motion'
import type { NodeReport } from '../types/report'

/**
 * "Why amber?": a small note on the rule behind every amber verdict, and
 * what this node's rests on. It closes on a click outside or Escape (which
 * then doesn't also close the sheet around it).
 */
export function WhyAmber({ node }: { node: NodeReport }) {
  const [open, setOpen] = useState(false)
  const root = useRef<HTMLSpanElement>(null)
  const id = useId()

  useEffect(() => {
    if (!open) return
    const outside = (e: PointerEvent) => {
      if (!root.current?.contains(e.target as Node)) setOpen(false)
    }
    document.addEventListener('pointerdown', outside)
    return () => document.removeEventListener('pointerdown', outside)
  }, [open])

  return (
    <span
      ref={root}
      // No position of its own: the note places itself under the whole line
      // the button sits in, so it never runs off a narrow screen.
      className="inline-block"
      onKeyDown={(e) => {
        if (e.key === 'Escape' && open) {
          e.preventDefault()
          setOpen(false)
        }
      }}
    >
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        aria-expanded={open}
        aria-controls={id}
        className="inline-flex items-center gap-1.5 font-semibold text-amber underline decoration-amber/40 underline-offset-4 hover:decoration-amber"
      >
        <svg
          viewBox="0 0 16 16"
          className="size-3.5"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.6"
          aria-hidden
        >
          <circle cx="8" cy="8" r="6.5" />
          <path d="M8 7.2v4" strokeLinecap="round" />
          <circle cx="8" cy="4.9" r="0.6" fill="currentColor" />
        </svg>
        Why amber?
      </button>
      <AnimatePresence>
        {open && (
          <motion.span
            id={id}
            role="note"
            className="absolute top-full left-0 z-20 mt-2.5 block w-[min(360px,100%)]"
            initial={{ opacity: 0, y: -4, scale: 0.98 }}
            animate={{ opacity: 1, y: 0, scale: 1, transition: { duration: 0.3, ease: SOFT } }}
            exit={{ opacity: 0, y: -4, scale: 0.98, transition: { duration: 0.2, ease: SOFT } }}
            style={{ transformOrigin: 'top left' }}
          >
            <span className="fw-note block rounded-2xl px-4 py-3.5 text-[0.88rem] leading-relaxed text-ink-2">
              <span className="mb-1 block font-semibold text-ink">
                Firewatch never shows green when it isn’t sure.
              </span>
              {amberReason(node)}
            </span>
          </motion.span>
        )}
      </AnimatePresence>
    </span>
  )
}
