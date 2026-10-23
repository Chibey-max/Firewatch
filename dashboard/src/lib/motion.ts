// Shared motion values: the expo-out curve the whole dashboard moves on.
export const EASE = [0.16, 1, 0.3, 1] as const

/** A gentle ease-in-out for crossfades: no snap at the start, no drag at the end. */
export const SOFT = [0.45, 0, 0.25, 1] as const

/**
 * Content settling in: opacity and a few pixels of lift only. No blur, which
 * is costly to animate over glass and reads as a judder rather than a fade.
 */
export const rise = {
  hidden: { opacity: 0, y: 6 },
  show: { opacity: 1, y: 0, transition: { duration: 0.6, ease: SOFT } },
}

export const spring = { type: 'spring', stiffness: 420, damping: 38, mass: 0.9 } as const
