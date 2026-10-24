import { useId } from 'react'
import type { Reading } from '../lib/format'

// Where the needle points, in degrees from straight up: the middle of each
// arc. With nothing to judge it stands upright.
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
        <radialGradient id={gloss} cx="0.42" cy="0.18" r="0.62">
          <stop offset="0" style={{ stopColor: 'var(--fw-gloss)', stopOpacity: 0.9 }} />
          <stop offset="1" style={{ stopColor: 'var(--fw-gloss)', stopOpacity: 0 }} />
        </radialGradient>
      </defs>

      {/* the lens */}
      <circle cx="20" cy="20" r="18" fill={`url(#${body})`} />

      {/* the dial: green, amber, red over the top */}
      <g fill="none" strokeWidth="3.6">
        <path d="M9.17 23.09A11 11 0 0 1 14.84 15.29" stroke="var(--fw-green)" />
        <path d="M15.88 14.8A11 11 0 0 1 24.12 14.8" stroke="var(--fw-amber)" />
        <path d="M25.16 15.29A11 11 0 0 1 30.83 23.09" stroke="var(--fw-red)" />
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
      <circle cx="20" cy="25" r="2.3" fill="var(--fw-ink)" />

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
