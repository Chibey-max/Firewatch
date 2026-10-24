import type { Reading } from './format'

/** Tailwind classes for each verdict's colour. */
export const tone: Record<Reading, { text: string; bg: string; dot: string; bar: string }> = {
  affected: { text: 'text-red', bg: 'bg-red-soft', dot: 'bg-red', bar: 'bg-red' },
  cant_tell: { text: 'text-amber', bg: 'bg-amber-soft', dot: 'bg-amber', bar: 'bg-amber' },
  not_affected: { text: 'text-green', bg: 'bg-green-soft', dot: 'bg-green', bar: 'bg-green' },
  none: { text: 'text-neutral', bg: 'bg-neutral-soft', dot: 'bg-neutral', bar: 'bg-neutral' },
}
