import { motion, useReducedMotion } from 'motion/react'
import type { CSSProperties } from 'react'
import type { Reading } from '../lib/format'

// The landscape behind the glass: a hazy valley by day, night peaks in the
// dark theme (photos in public/bg, credited in its README). Both stay mounted
// and crossfade, so switching themes never waits on a download. Over them: a
// veil for legibility, the fleet's worst verdict as light on the horizon,
// embers while something is affected, and a fine grain.

const glow: Record<Reading, string> = {
  affected: 'color-mix(in srgb, var(--fw-red) 60%, transparent)',
  cant_tell: 'color-mix(in srgb, var(--fw-amber) 55%, transparent)',
  not_affected: 'color-mix(in srgb, var(--fw-green) 35%, transparent)',
  none: 'transparent',
}

// Rising embers: only while something is affected. Fixed seeds, so they don't
// jump between renders.
const EMBERS = Array.from({ length: 14 }, (_, i) => ({
  x: 6 + ((i * 37) % 88),
  size: 2 + (i % 3),
  delay: (i * 1.7) % 9,
  secs: 9 + (i % 5) * 2,
  sway: (i % 2 ? 1 : -1) * (10 + (i % 4) * 6),
}))

export function Scene({ reading }: { reading: Reading }) {
  const reduce = useReducedMotion()
  return (
    <div
      aria-hidden
      className="fw-scene"
      style={{ ['--fw-glow' as string]: glow[reading] } as CSSProperties}
    >
      <div className="fw-photo fw-photo-light" />
      <div className="fw-photo fw-photo-dark" />
      <div className="fw-veil" />
      <div className="fw-horizon" />
      {reading === 'affected' && !reduce && (
        <div className="absolute inset-x-0 bottom-0 h-[60%] overflow-hidden">
          {EMBERS.map((e, i) => (
            <motion.span
              key={i}
              className="absolute bottom-0 rounded-full bg-[var(--fw-ember)]"
              style={{ left: `${e.x}%`, width: e.size, height: e.size }}
              initial={{ y: 0, opacity: 0 }}
              animate={{ y: [-10, -420], x: [0, e.sway], opacity: [0, 0.9, 0] }}
              transition={{ duration: e.secs, delay: e.delay, repeat: Infinity, ease: 'easeOut' }}
            />
          ))}
        </div>
      )}
      <div className="fw-grain" />
    </div>
  )
}
