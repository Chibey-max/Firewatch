import { AnimatePresence, motion, useReducedMotion } from 'motion/react'
import { useState } from 'react'
import { checkedAgo, clockTime } from '../lib/format'
import { EASE } from '../lib/motion'
import { POLL_MS, type Phase } from '../lib/useReport'
import { Logo } from './Logo'
import { MetalLayers } from './MetalLayers'

function Dot({ tone, breathe }: { tone: string; breathe?: boolean }) {
  return (
    <span
      aria-hidden
      className={`relative inline-block size-2 rounded-full bg-current ${tone} ${breathe ? 'fw-breathe' : ''}`}
    />
  )
}

/** A ring that empties until the next retry. Restarts on every failed poll. */
function RetryRing({ failures }: { failures: number }) {
  return (
    <svg key={failures} viewBox="0 0 16 16" className="size-4 -rotate-90 text-amber" aria-hidden>
      <circle
        cx="8"
        cy="8"
        r="7"
        fill="none"
        stroke="currentColor"
        strokeOpacity=".2"
        strokeWidth="2"
      />
      <circle
        cx="8"
        cy="8"
        r="7"
        fill="none"
        stroke="currentColor"
        strokeWidth="2"
        strokeDasharray="44"
        style={{ animation: `fw-countdown ${POLL_MS}ms linear forwards` }}
      />
    </svg>
  )
}

/** What the data is right now: live, starting, reconnecting or sample. */
export function LiveChip(props: {
  phase: Phase
  fixture: string | null
  checkedAt: number | null
  receivedAt: number | null
  failures: number
  now: number
}) {
  const { phase, fixture, checkedAt, receivedAt, failures, now } = props
  let content
  if (phase === 'fixture') {
    content = (
      <>
        <Dot tone="text-neutral" />
        <span>
          Sample data · <span className="font-mono">{fixture}</span>
        </span>
        <a
          href="./"
          className="font-semibold text-ink underline decoration-line-strong underline-offset-4 hover:decoration-ink"
        >
          Go live
        </a>
      </>
    )
  } else if (phase === 'offline') {
    content = (
      <>
        <RetryRing failures={failures} />
        <span>
          Reconnecting
          {receivedAt && <span className="text-ink-3"> · last report {clockTime(receivedAt)}</span>}
        </span>
      </>
    )
  } else if (phase === 'first_check') {
    content = (
      <>
        <Dot tone="text-amber" breathe />
        <span>Starting · first check running</span>
      </>
    )
  } else if (phase === 'loading') {
    content = (
      <>
        <Dot tone="text-neutral" breathe />
        <span>Connecting…</span>
      </>
    )
  } else {
    content = (
      <>
        <Dot tone="text-green" breathe />
        <span>
          Live
          {checkedAt !== null && (
            <span className="text-ink-3 tabular-nums">
              {' '}
              · {checkedAgo(checkedAt, now).toLowerCase()}
            </span>
          )}
        </span>
      </>
    )
  }
  return (
    <motion.div
      layout
      transition={{ duration: 0.45, ease: EASE }}
      role="status"
      className="fw-glass flex h-10 items-center gap-2.5 rounded-full px-4 text-[0.85rem] text-ink-2"
    >
      <AnimatePresence mode="popLayout" initial={false}>
        <motion.span
          key={phase}
          initial={{ opacity: 0, filter: 'blur(4px)' }}
          animate={{ opacity: 1, filter: 'blur(0px)' }}
          exit={{ opacity: 0, filter: 'blur(4px)' }}
          transition={{ duration: 0.3 }}
          className="flex items-center gap-2.5"
        >
          {content}
        </motion.span>
      </AnimatePresence>
    </motion.div>
  )
}

/**
 * Re-check, as Firewatch's one liquid-metal button (after Mirror's): a 2px rim
 * of moving chrome around the face. The chrome idles, runs on hover, bursts on
 * a press and races while a check runs; the label says what's happening and
 * ends on a tick.
 */
export function RecheckButton({
  onCheck,
  checking,
}: {
  onCheck: () => Promise<boolean>
  checking: boolean
}) {
  const reduce = useReducedMotion()
  const [done, setDone] = useState(false)
  const [hover, setHover] = useState(false)
  const [burst, setBurst] = useState(false)

  const click = async () => {
    if (await onCheck()) {
      setDone(true)
      setTimeout(() => setDone(false), 1400)
    }
  }
  const press = () => {
    setBurst(true)
    setTimeout(() => setBurst(false), 300)
  }
  const speed = reduce ? 0 : burst ? 2.4 : checking ? 1.8 : hover ? 1 : 0.45
  const label = checking ? 'Checking…' : done ? 'Checked' : 'Re-check now'

  return (
    <motion.button
      type="button"
      onClick={click}
      onPointerEnter={() => setHover(true)}
      onPointerLeave={() => setHover(false)}
      onPointerDown={press}
      disabled={checking}
      layout
      transition={{ duration: 0.4, ease: EASE }}
      className="group relative isolate flex h-11 items-center gap-3 rounded-full pr-1.5 pl-5 text-[0.9rem] font-semibold tracking-[-0.01em] text-[var(--fw-metal-label)] transition-transform duration-150 active:translate-y-px active:scale-[0.98] disabled:cursor-wait"
    >
      <MetalLayers speed={speed} />
      <AnimatePresence mode="popLayout" initial={false}>
        <motion.span
          key={label}
          className="relative"
          initial={{ opacity: 0, y: 6 }}
          animate={{ opacity: 1, y: 0 }}
          exit={{ opacity: 0, y: -6 }}
          transition={{ duration: 0.25, ease: EASE }}
        >
          {label}
        </motion.span>
      </AnimatePresence>
      <span className="relative grid size-8 place-items-center rounded-full bg-[var(--fw-metal-label)]/10">
        {done ? (
          <motion.svg
            viewBox="0 0 16 16"
            className="size-3.5"
            initial={{ scale: 0.4 }}
            animate={{ scale: 1 }}
          >
            <motion.path
              d="M3 8.5 6.5 12 13 4.5"
              fill="none"
              stroke="currentColor"
              strokeWidth="2"
              strokeLinecap="round"
              strokeLinejoin="round"
              initial={{ pathLength: 0 }}
              animate={{ pathLength: 1 }}
              transition={{ duration: 0.35, ease: EASE }}
            />
          </motion.svg>
        ) : (
          <motion.svg
            viewBox="0 0 16 16"
            className="size-3.5"
            animate={checking ? { rotate: 360 } : { rotate: 0 }}
            transition={
              checking ? { duration: 0.9, ease: 'linear', repeat: Infinity } : { duration: 0.3 }
            }
          >
            <path
              d="M13 8a5 5 0 1 1-1.5-3.55"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.8"
              strokeLinecap="round"
            />
            <path
              d="M13 2.5v3h-3"
              fill="none"
              stroke="currentColor"
              strokeWidth="1.8"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </motion.svg>
        )}
      </span>
    </motion.button>
  )
}

/** The wordmark pill. */
export function Brand() {
  return (
    <div className="fw-glass flex h-10 items-center gap-2.5 rounded-full pr-4 pl-3.5">
      <Logo />
      <span className="text-[0.98rem] font-semibold tracking-[0.01em]">Firewatch</span>
    </div>
  )
}
