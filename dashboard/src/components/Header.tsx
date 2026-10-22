import type { ReactNode } from 'react'
import { plural, readingOf } from '../lib/format'
import type { Report } from '../types/report'

function Tally({ report }: { report: Report }) {
  const readings = report.nodes.map(readingOf)
  const n = (r: string) => readings.filter((x) => x === r).length
  const items = [
    { count: n('affected'), label: 'affected', className: 'text-affected' },
    { count: n('cant_tell'), label: "can't tell", className: 'text-canttell-ink' },
    { count: n('not_affected'), label: 'not affected', className: 'text-safe' },
    { count: n('none'), label: 'no known advisories', className: 'text-none' },
  ].filter((i) => i.count > 0)
  return (
    <p className="text-[0.98rem] font-semibold">
      {items.map((i, idx) => (
        <span key={i.label} className={i.className}>
          {idx > 0 && <span className="text-rule"> · </span>}
          {i.count} {i.label}
        </span>
      ))}
    </p>
  )
}

export function Header({ report, children }: { report: Report | null; children?: ReactNode }) {
  return (
    <header className="flex flex-wrap items-end justify-between gap-x-8 gap-y-3 border-b-2 border-ink pb-4">
      <div>
        <h1 className="font-display text-[2.4rem] leading-none font-extrabold tracking-[0.06em] uppercase">
          Firewatch
        </h1>
        <p className="mt-1 text-[0.98rem] text-ink-2">
          {report && report.nodes.length > 0
            ? `Watching ${plural(report.nodes.length, 'Lightning node', 'Lightning nodes')} for published security flaws`
            : 'Security advisories for Lightning nodes'}
        </p>
      </div>
      <div className="flex flex-wrap items-center gap-x-6 gap-y-2">
        {report && report.nodes.length > 0 && <Tally report={report} />}
        {children}
      </div>
    </header>
  )
}
