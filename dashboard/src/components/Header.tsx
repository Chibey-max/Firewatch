import type { ReactNode } from 'react'
import { needsAction, plural } from '../lib/format'
import type { Report } from '../types/report'
import { Logo } from './Logo'

/** The one sentence a glance should get: how many nodes need the operator. */
function headline(report: Report | null): string {
  if (!report) return 'Security advisories for Lightning nodes'
  const total = report.nodes.length
  if (total === 0) return 'No nodes configured'
  const n = needsAction(report.nodes)
  if (n === 0) return `All ${plural(total, 'node', 'nodes')} clear of known advisories`
  return `${n} of ${plural(total, 'node needs', 'nodes need')} attention`
}

export function Header({ report, children }: { report: Report | null; children?: ReactNode }) {
  const attention = report ? needsAction(report.nodes) : 0
  return (
    <header className="flex flex-wrap items-center justify-between gap-x-8 gap-y-3 border-b border-line pb-5">
      <div className="flex items-center gap-4">
        <div className="flex items-center gap-2.5">
          <Logo />
          <span className="text-[1.05rem] font-bold tracking-[0.02em]">Firewatch</span>
        </div>
        <span aria-hidden className="h-5 w-px bg-line" />
        <h1 className={`text-[1.05rem] font-semibold ${attention ? 'text-fg' : 'text-fg-2'}`}>
          {headline(report)}
        </h1>
      </div>
      <div className="flex flex-wrap items-center gap-3">{children}</div>
    </header>
  )
}
