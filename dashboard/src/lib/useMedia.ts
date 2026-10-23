import { useSyncExternalStore } from 'react'

/** Tracks a media query, so layout that CSS can't express can follow it. */
export function useMedia(query: string): boolean {
  return useSyncExternalStore(
    (onChange) => {
      const list = window.matchMedia(query)
      list.addEventListener('change', onChange)
      return () => list.removeEventListener('change', onChange)
    },
    () => window.matchMedia(query).matches,
  )
}

/** Side-by-side panes from Tailwind's `lg` up; one column with a sheet below. */
export const useWide = () => useMedia('(min-width: 1024px)')
