import { useEffect, useState } from 'react'
import { readingWord, type Reading } from '../lib/format'

// A fire-danger style dial: green (not affected), amber (can't tell), red
// (affected). The needle position, the lit segment and the word below all say
// the same thing, so colour is never the only signal.

const CX = 100
const CY = 100
const R = 78

/** Needle angle in degrees from straight up. */
const angle: Record<Reading, number> = {
  not_affected: -60,
  cant_tell: 0,
  affected: 60,
  none: -86,
}
const REST = -86

const segments = [
  { reading: 'not_affected', from: -90, to: -31, color: 'var(--color-safe)' },
  { reading: 'cant_tell', from: -29, to: 29, color: 'var(--color-canttell)' },
  { reading: 'affected', from: 31, to: 90, color: 'var(--color-affected)' },
] as const

function point(deg: number, r = R) {
  const rad = (deg * Math.PI) / 180
  return `${(CX + r * Math.sin(rad)).toFixed(2)} ${(CY - r * Math.cos(rad)).toFixed(2)}`
}

export function Dial({ reading, size = 200 }: { reading: Reading; size?: number }) {
  // Start at rest and swing to the reading after mount, so the needle moves on
  // load and again whenever the reading changes (e.g. after an upgrade). A timer,
  // not requestAnimationFrame: rAF never fires in a background tab, and a needle
  // left at rest would show the wrong verdict.
  const [deg, setDeg] = useState(REST)
  useEffect(() => {
    const id = setTimeout(() => setDeg(angle[reading]), 30)
    return () => clearTimeout(id)
  }, [reading])

  return (
    <svg
      viewBox="0 0 200 112"
      width={size}
      height={(size * 112) / 200}
      role="img"
      aria-label={`Status: ${readingWord[reading]}`}
      className="block"
    >
      {segments.map((s) => (
        <path
          key={s.reading}
          d={`M ${point(s.from)} A ${R} ${R} 0 0 1 ${point(s.to)}`}
          fill="none"
          stroke={s.color}
          strokeWidth={18}
          opacity={s.reading === reading ? 1 : 0.2}
          className="transition-opacity duration-700 motion-reduce:transition-none"
        />
      ))}
      <g
        style={{ transform: `rotate(${deg}deg)`, transformOrigin: `${CX}px ${CY}px` }}
        className="dial-needle"
      >
        <line
          x1={CX}
          y1={CY}
          x2={CX}
          y2={CY - R + 4}
          stroke={reading === 'none' ? 'var(--color-none)' : 'var(--color-ink)'}
          strokeWidth={5}
          strokeLinecap="round"
        />
      </g>
      <circle cx={CX} cy={CY} r={8} fill="var(--color-ink)" />
      <circle cx={CX} cy={CY} r={3} fill="var(--color-panel)" />
    </svg>
  )
}
