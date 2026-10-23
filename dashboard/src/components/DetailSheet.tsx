import { AnimatePresence, motion, useDragControls, type PanInfo } from 'motion/react'
import { useEffect, useRef } from 'react'
import { EASE, SOFT } from '../lib/motion'
import type { NodeReport } from '../types/report'
import { NodeDetail } from './NodeDetail'

/**
 * On a phone, a node's detail rises over the list as a sheet instead of
 * sitting below it. It closes on the backdrop, the close button, Escape, a
 * drag down from the handle or the browser's Back. While it's open the page
 * behind is inert and doesn't scroll; closing hands focus back to the row.
 */
export function DetailSheet({ node, onClose }: { node: NodeReport | null; onClose: () => void }) {
  const open = node !== null
  const panel = useRef<HTMLDivElement>(null)
  const drag = useDragControls()

  useEffect(() => {
    if (!open) return
    const back = document.activeElement as HTMLElement | null
    const root = document.documentElement
    root.style.overflow = 'hidden'
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') onClose()
    }
    window.addEventListener('keydown', onKey)
    requestAnimationFrame(() => panel.current?.focus({ preventScroll: true }))
    return () => {
      root.style.overflow = ''
      window.removeEventListener('keydown', onKey)
      back?.focus({ preventScroll: true })
    }
  }, [open, onClose])

  const dragEnd = (_: PointerEvent, info: PanInfo) => {
    if (info.offset.y > 110 || info.velocity.y > 600) onClose()
  }

  return (
    <AnimatePresence>
      {node && (
        <motion.div key="sheet" className="fixed inset-0 z-40">
          <motion.div
            className="fw-scrim absolute inset-0"
            onClick={onClose}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1, transition: { duration: 0.4, ease: SOFT } }}
            exit={{ opacity: 0, transition: { duration: 0.3, ease: SOFT } }}
          />
          <motion.div
            ref={panel}
            role="dialog"
            aria-modal="true"
            aria-labelledby="detail-title"
            tabIndex={-1}
            className="fw-sheet absolute inset-x-0 bottom-0 flex max-h-[92dvh] flex-col rounded-t-[28px] outline-none sm:inset-x-5"
            initial={{ y: '100%' }}
            animate={{ y: 0, transition: { duration: 0.55, ease: EASE } }}
            exit={{ y: '100%', transition: { duration: 0.35, ease: SOFT } }}
            drag="y"
            dragControls={drag}
            dragListener={false}
            dragConstraints={{ top: 0, bottom: 0 }}
            dragElastic={{ top: 0, bottom: 0.7 }}
            onDragEnd={dragEnd}
          >
            {/* The grip: drag it down to close. The content below scrolls. */}
            <div
              className="flex shrink-0 cursor-grab touch-none items-center justify-between px-4 pt-2.5 pb-1 select-none active:cursor-grabbing"
              onPointerDown={(e) => {
                // No text selection while dragging with a mouse.
                e.preventDefault()
                drag.start(e)
              }}
            >
              <span className="size-9" />
              <span aria-hidden className="h-1.5 w-11 rounded-full bg-ink/20" />
              <button
                type="button"
                onClick={onClose}
                onPointerDown={(e) => e.stopPropagation()}
                aria-label="Close"
                className="fw-glass grid size-9 place-items-center rounded-full text-ink-2 hover:text-ink"
              >
                <svg viewBox="0 0 16 16" className="size-3.5" aria-hidden>
                  <path
                    d="M4 4l8 8M12 4l-8 8"
                    stroke="currentColor"
                    strokeWidth="1.8"
                    strokeLinecap="round"
                  />
                </svg>
              </button>
            </div>
            <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain">
              <NodeDetail node={node} bare />
            </div>
          </motion.div>
        </motion.div>
      )}
    </AnimatePresence>
  )
}
