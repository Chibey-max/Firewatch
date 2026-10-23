import type { CSSProperties } from 'react'
import type { Reading } from '../lib/format'

const glow: Record<Reading, string> = {
  affected: 'color-mix(in srgb, var(--fw-red) 22%, transparent)',
  cant_tell: 'color-mix(in srgb, var(--fw-amber) 22%, transparent)',
  not_affected: 'color-mix(in srgb, var(--fw-green) 18%, transparent)',
  none: 'color-mix(in srgb, var(--fw-neutral) 14%, transparent)',
}

/** The light behind the page takes the fleet's worst verdict and eases between them. */
export function Ambient({ reading }: { reading: Reading }) {
  return (
    <div
      aria-hidden
      className="fw-ambient"
      style={{ ['--fw-glow' as string]: glow[reading] } as CSSProperties}
    />
  )
}
