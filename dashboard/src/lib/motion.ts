// Shared motion values: the expo-out curve the whole dashboard moves on.
export const EASE = [0.16, 1, 0.3, 1] as const

export const rise = {
  hidden: { opacity: 0, y: 12, filter: 'blur(6px)' },
  show: { opacity: 1, y: 0, filter: 'blur(0px)', transition: { duration: 0.7, ease: EASE } },
}

export const spring = { type: 'spring', stiffness: 420, damping: 38, mass: 0.9 } as const
