import { AnimatePresence, motion } from 'motion/react'
import { useState } from 'react'
import { helpText } from '../lib/helpText'
import { EASE } from '../lib/motion'
import { toast } from '../lib/toasts'
import type { NodeReport } from '../types/report'

async function copy(text: string): Promise<boolean> {
  try {
    await navigator.clipboard.writeText(text)
    return true
  } catch {
    // No clipboard API (or it was refused): the old way, through a selection.
    const area = document.createElement('textarea')
    area.value = text
    area.setAttribute('readonly', '')
    area.style.position = 'fixed'
    area.style.opacity = '0'
    document.body.append(area)
    area.select()
    const ok = document.execCommand('copy')
    area.remove()
    return ok
  }
}

/**
 * "Copy for help": the node's verdict and advisories as plain text, to paste
 * into a community chat. It leaves out what would point at the operator's
 * machine (see lib/helpText.ts).
 */
export function CopyHelp({ node }: { node: NodeReport }) {
  const [done, setDone] = useState(false)

  const click = async () => {
    if (await copy(helpText(node, Math.floor(Date.now() / 1000)))) {
      setDone(true)
      setTimeout(() => setDone(false), 1600)
      toast(
        'good',
        'Copied a summary',
        'Paste it where you ask for help. It leaves out the node’s name, addresses and credentials.',
      )
    } else {
      toast('warn', 'Couldn’t copy', 'This browser blocked the clipboard.')
    }
  }

  return (
    <button
      type="button"
      onClick={click}
      title="Copy this node’s verdict and advisories as text, without its name, addresses or credentials"
      className="fw-glass inline-flex h-8 shrink-0 items-center gap-2 rounded-full px-3 text-[0.82rem] font-semibold text-ink-2 hover:bg-surface-hover hover:text-ink"
    >
      <span aria-hidden className="relative grid size-3.5 place-items-center">
        <AnimatePresence mode="popLayout" initial={false}>
          <motion.svg
            key={done ? 'done' : 'copy'}
            viewBox="0 0 16 16"
            className="size-3.5"
            fill="none"
            stroke="currentColor"
            strokeWidth="1.6"
            strokeLinecap="round"
            strokeLinejoin="round"
            initial={{ opacity: 0, scale: 0.6 }}
            animate={{ opacity: 1, scale: 1 }}
            exit={{ opacity: 0, scale: 0.6 }}
            transition={{ duration: 0.25, ease: EASE }}
          >
            {done ? (
              <path d="M3 8.5 6.5 12 13 4.5" />
            ) : (
              <>
                <rect x="5.5" y="5.5" width="8" height="8" rx="1.6" />
                <path d="M10.5 3.2A1.6 1.6 0 0 0 9 2.5H4.1a1.6 1.6 0 0 0-1.6 1.6V9a1.6 1.6 0 0 0 .7 1.3" />
              </>
            )}
          </motion.svg>
        </AnimatePresence>
      </span>
      {done ? 'Copied' : 'Copy for help'}
    </button>
  )
}
