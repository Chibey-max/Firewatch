import { useSyncExternalStore } from 'react'

export type ToastTone = 'good' | 'warn' | 'bad'

export interface Toast {
  id: number
  tone: ToastTone
  title: string
  body?: string
}

// A tiny store: toasts are raised from event handlers and fetch callbacks,
// never from effects, and the stack renders them.
let toasts: Toast[] = []
let nextId = 1
const listeners = new Set<() => void>()
const emit = () => listeners.forEach((l) => l())

export function dismiss(id: number) {
  toasts = toasts.filter((t) => t.id !== id)
  emit()
}

/** Shows a toast for `ms` (5 s by default). A toast with the same title replaces the old one. */
export function toast(tone: ToastTone, title: string, body?: string, ms = 5000) {
  const id = nextId++
  toasts = [...toasts.filter((t) => t.title !== title), { id, tone, title, body }].slice(-3)
  emit()
  setTimeout(() => dismiss(id), ms)
}

export function useToasts(): Toast[] {
  return useSyncExternalStore(
    (l) => {
      listeners.add(l)
      return () => listeners.delete(l)
    },
    () => toasts,
  )
}
