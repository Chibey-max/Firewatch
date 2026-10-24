import { useId } from 'react'
import type { Reading } from '../lib/format'

// Where the needle points, in degrees from straight up: the middle of each
// arc. With nothing to judge it stands upright.
// The three arcs as tubes of clear tinted glass. Each gets a gradient
// across its width (centred on the dial, so it follows the curve): a firm
// inner edge, a clear body the lens shows through, light caught near the
// outer edge, and a firm outer edge. Offsets are radii over 12.8; the
// stroke runs from 9.2 to 12.8.
const tube: [offset: number, opacity: number, light?: boolean][] = [
  [0.71, 1],
  [0.76, 0.82],
  [0.84, 0.58],
  [0.9, 0.62],
  [0.935, 0.9, true],
  [0.965, 0.72],
  [1, 1],
]
const arcs = [
  { key: 'green', colour: 'var(--fw-green)', d: 'M9.17 23.09A11 11 0 0 1 14.84 15.29' },
  { key: 'amber', colour: 'var(--fw-amber)', d: 'M15.88 14.8A11 11 0 0 1 24.12 14.8' },
  { key: 'red', colour: 'var(--fw-red)', d: 'M25.16 15.29A11 11 0 0 1 30.83 23.09' },
]

const needle: Record<Reading, number> = {
  not_affected: -54,
  cant_tell: 0,
  affected: 54,
  none: 0,
}

/**
 * The lookout dial under a lens of glass, like the panes it sits on: a
 * translucent body in the theme's glass colour, light on the top rim, a
 * gloss across the upper dial so the arcs read as under the glass, a glint,
 * light caught along the bottom edge, and a soft shadow. The needle points at
 * the fleet's worst verdict and swings there on load and on every change.
 */
export function Logo({ reading = 'affected', size = 28 }: { reading?: Reading; size?: number }) {
  const id = useId()
  const rim = `${id}-rim`
  const gloss = `${id}-gloss`
  const body = `${id}-body`

  return (
    <svg
      viewBox="0 0 40 40"
      width={size}
      height={size}
      aria-hidden
      className="shrink-0 drop-shadow-[0_1px_1.5px_rgb(0_0_0/0.2)]"
    >
      <defs>
        <linearGradient id={body} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" style={{ stopColor: 'var(--fw-sheen)', stopOpacity: 0.55 }} />
          <stop offset="0.6" style={{ stopColor: 'var(--fw-surface-solid)' }} />
          <stop offset="1" style={{ stopColor: 'var(--fw-surface-solid)' }} />
        </linearGradient>
        <linearGradient id={rim} x1="0" y1="0" x2="0" y2="1">
          <stop offset="0" style={{ stopColor: 'var(--fw-gloss)' }} />
          <stop offset="0.5" style={{ stopColor: 'var(--fw-line)' }} />
          <stop offset="1" style={{ stopColor: 'var(--fw-line-strong)' }} />
        </linearGradient>
        {arcs.map((a) => (
          <radialGradient
            key={a.key}
            id={`${id}-${a.key}`}
            gradientUnits="userSpaceOnUse"
            cx="20"
            cy="25"
            r="12.8"
          >
            {tube.map(([offset, opacity, light]) => (
              <stop
                key={offset}
                offset={offset}
                style={{ stopColor: light ? '#fff' : a.colour, stopOpacity: opacity }}
              />
            ))}
          </radialGradient>
        ))}
        <radialGradient id={`${id}-hub`} cx="0.36" cy="0.3" r="0.75">
          <stop offset="0" style={{ stopColor: '#fff', stopOpacity: 0.85 }} />
          <stop offset="0.45" style={{ stopColor: 'var(--fw-ink)' }} />
          <stop offset="1" style={{ stopColor: 'var(--fw-ink)' }} />
        </radialGradient>
        <radialGradient id={gloss} cx="0.42" cy="0.18" r="0.62">
          <stop offset="0" style={{ stopColor: 'var(--fw-gloss)', stopOpacity: 0.9 }} />
          <stop offset="1" style={{ stopColor: 'var(--fw-gloss)', stopOpacity: 0 }} />
        </radialGradient>
      </defs>

      {/* the lens */}
      <circle cx="20" cy="20" r="18" fill={`url(#${body})`} />

      {/* the dial: green, amber, red over the top, each a tube of glass */}
      <g fill="none" strokeWidth="3.6">
        {arcs.map((a) => (
          <path key={a.key} d={a.d} stroke={`url(#${id}-${a.key})`} />
        ))}
      </g>
      <g
        className="fw-needle"
        style={{
          transform: `rotate(${needle[reading]}deg)`,
          transformOrigin: '20px 25px',
          transformBox: 'view-box',
        }}
      >
        <line
          x1="20"
          y1="25"
          x2="20"
          y2="15.5"
          stroke="var(--fw-ink)"
          strokeWidth="2.2"
          strokeLinecap="round"
        />
      </g>
      <circle cx="20" cy="25" r="2.3" fill={`url(#${id}-hub)`} />

      {/* glass over it: a gloss on the upper half, the rim, light along the
          bottom edge, a glint */}
      <ellipse cx="20" cy="12.5" rx="14" ry="8.5" fill={`url(#${gloss})`} />
      <circle cx="20" cy="20" r="17.4" fill="none" stroke={`url(#${rim})`} strokeWidth="1.2" />
      <path
        d="M11.5 32.2A14.5 14.5 0 0 0 28.5 32.2"
        fill="none"
        stroke="var(--fw-gloss)"
        strokeOpacity="0.55"
        strokeWidth="1.1"
        strokeLinecap="round"
      />
      <path
        d="M8.6 13.2A13.5 13.5 0 0 1 14.6 7.6"
        fill="none"
        stroke="var(--fw-gloss)"
        strokeWidth="1.5"
        strokeLinecap="round"
      />
    </svg>
  )
}
