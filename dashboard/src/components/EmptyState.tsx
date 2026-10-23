import type { ReactNode } from 'react'

export function EmptyState({ title, children }: { title: string; children: ReactNode }) {
  return (
    <div className="mx-auto mt-20 max-w-md text-center">
      <p className="text-[1.3rem] font-semibold">{title}</p>
      <p className="mt-2 text-[0.98rem] leading-relaxed text-fg-2">{children}</p>
    </div>
  )
}
