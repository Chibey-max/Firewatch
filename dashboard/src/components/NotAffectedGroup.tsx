import { plural } from '../lib/format'
import type { Finding } from '../types/report'
import { WithCode } from './Code'

/** Advisories checked and ruled out, folded away so the problems stand out. */
export function NotAffectedGroup({ findings }: { findings: Finding[] }) {
  if (findings.length === 0) return null
  return (
    <details className="group mt-4 border-t border-rule pt-3">
      <summary className="flex cursor-pointer list-none items-center gap-2 text-[0.95rem] font-semibold text-safe select-none">
        <span aria-hidden className="inline-block transition-transform group-open:rotate-90">
          ▸
        </span>
        {plural(findings.length, "advisory doesn't apply", "advisories don't apply")}
      </summary>
      <ul className="mt-2 space-y-3">
        {findings.map((f) => (
          <li key={f.advisory_id} className="border-l-4 border-safe pl-4">
            <p className="font-mono text-[0.85rem] text-ink-2">{f.advisory_id}</p>
            <p className="text-[0.98rem] font-semibold">{f.title}</p>
            <p className="text-[0.92rem] text-ink-2">
              <WithCode text={f.reason} />
            </p>
          </li>
        ))}
      </ul>
    </details>
  )
}
