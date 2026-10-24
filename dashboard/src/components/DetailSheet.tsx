import { AnimatePresence, motion, useDragControls, type PanInfo } from 'motion/react'
import { useEffect, useRef } from 'react'
import { useMedia } from '../lib/useMedia'
import { EASE, SOFT } from '../lib/motion'
import type { NodeReport } from '../types/report'
import { NodeDetail } from './NodeDetail'

function CloseButton({ onClose }: { onClose: () => void }) {
  return (
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
  )
}

/**
 * Below the side-by-side layout, a node's detail opens over the list instead
 * of sitting below it: on a phone as a sheet that rises from the bottom (and
 * drags down to close), on a tablet as a dialog in the middle of the page.
 * With the tablet's pill (`beside`), the dialog takes the place of the panel
 * on the left and leaves the pill uncovered and usable. Either closes on the
 * backdrop, the close button, Escape or the browser's Back. While it's open the page behind is inert and doesn't scroll; closing
 * hands focus back to the row.
 */
export function DetailSheet({
  node,
  onClose,
  beside = false,
}: {
  node: NodeReport | null
  onClose: () => void
  beside?: boolean
}) {
  const open = node !== null
  const centred = useMedia('(min-width: 640px)')
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

  const dialog = {
    ref: panel,
    role: 'dialog',
    // The pill stays usable beside the dialog, so the page isn't fully modal.
    'aria-modal': !beside,
    'aria-labelledby': 'detail-title',
    tabIndex: -1,
  } as const

  return (
    <AnimatePresence>
      {node && (
        <motion.div
          key={centred ? 'dialog' : 'sheet'}
          className={`fixed inset-0 z-40 ${centred ? `grid place-items-center ${beside ? 'p-5 pr-[92px]' : 'p-8'}` : ''}`}
        >
          <motion.div
            className="fw-scrim absolute inset-0"
            onClick={onClose}
            initial={{ opacity: 0 }}
            animate={{ opacity: 1, transition: { duration: 0.4, ease: SOFT } }}
            exit={{ opacity: 0, transition: { duration: 0.3, ease: SOFT } }}
          />
          {centred ? (
            <motion.div
              {...dialog}
              className={`fw-sheet fw-dialog relative flex max-h-[min(86dvh,960px)] w-full flex-col rounded-[28px] outline-none ${beside ? '' : 'max-w-[720px]'}`}
              initial={{ opacity: 0, scale: 0.94, y: 16 }}
              animate={{ opacity: 1, scale: 1, y: 0, transition: { duration: 0.5, ease: EASE } }}
              exit={{ opacity: 0, scale: 0.97, y: 8, transition: { duration: 0.3, ease: SOFT } }}
            >
              <div className="flex shrink-0 justify-end px-4 pt-4">
                <CloseButton onClose={onClose} />
              </div>
              <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain">
                <NodeDetail node={node} bare />
              </div>
            </motion.div>
          ) : (
            <motion.div
              {...dialog}
              className="fw-sheet absolute inset-x-0 bottom-0 flex max-h-[92dvh] flex-col rounded-t-[28px] outline-none"
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
                <CloseButton onClose={onClose} />
              </div>
              <div className="min-h-0 flex-1 overflow-y-auto overscroll-contain">
                <NodeDetail node={node} bare />
              </div>
            </motion.div>
          )}
        </motion.div>
      )}
    </AnimatePresence>
  )
}
