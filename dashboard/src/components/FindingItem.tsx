import { severityName, sourceLabel } from '../lib/format'
import type { Finding, Status } from '../types/report'
import { WithCode } from './Code'

const rule: Record<Status, string> = {
  affected: 'border-affected',
  cant_tell: 'border-canttell',
  not_affected: 'border-safe',
}

function Chip({ children, tone }: { children: string; tone: 'red' | 'grey' | 'plain' }) {
  const tones = {
    red: 'bg-affected text-white',
    grey: 'bg-none/15 text-ink-2',
    plain: 'border border-rule text-ink-2',
  }
  return (
    <span
      className={`rounded-sm px-1.5 py-px text-[0.78rem] font-semibold uppercase tracking-wide ${tones[tone]}`}
    >
      {children}
    </span>
  )
}

export function SourceLink({ url }: { url: string }) {
  return (
    <a
      href={url}
      target="_blank"
      rel="noopener noreferrer"
      className="font-mono text-[0.85rem] text-ink-2 underline decoration-rule underline-offset-4 hover:text-ink hover:decoration-ink"
    >
      {sourceLabel(url)} ↗
    </a>
  )
}

function Label({ children }: { children: string }) {
  return (
    <p className="mt-3 mb-1 text-[0.75rem] font-bold uppercase tracking-[0.12em] text-ink-2">
      {children}
    </p>
  )
}

/** One advisory that applies, or might apply, to this node. */
export function FindingItem({ finding: f }: { finding: Finding }) {
  return (
    <li className={`border-l-4 py-1 pl-4 ${rule[f.status]}`}>
      <div className="flex flex-wrap items-center gap-x-2 gap-y-1">
        <span className="font-mono text-[0.85rem] text-ink-2">{f.advisory_id}</span>
        <Chip tone="plain">{severityName[f.severity]}</Chip>
        {f.exploited_in_wild && <Chip tone="red">Exploited in the wild</Chip>}
        {!f.verified && <Chip tone="grey">Not yet verified</Chip>}
      </div>
      <h3 className="mt-1 text-[1.05rem] leading-snug font-semibold">{f.title}</h3>
      <p className="mt-1 text-[0.95rem] leading-relaxed text-ink-2">
        <WithCode text={f.reason} />
      </p>

      {f.status === 'affected' && (
        <>
          <Label>Developers say</Label>
          <blockquote className="relative pl-5 text-[1.05rem] leading-relaxed">
            <span
              aria-hidden
              className="absolute top-[-0.35rem] left-0 font-display text-[2rem] leading-none font-extrabold text-affected"
            >
              “
            </span>
            <WithCode text={f.steps} />
          </blockquote>
        </>
      )}

      {f.status === 'cant_tell' && f.manual_checks.length > 0 && (
        <>
          <Label>Check by hand</Label>
          <ul className="list-disc space-y-1 pl-5 text-[0.98rem] leading-relaxed marker:text-canttell">
            {f.manual_checks.map((check) => (
              <li key={check}>
                <WithCode text={check} />
              </li>
            ))}
          </ul>
        </>
      )}

      <p className="mt-3 flex flex-wrap items-baseline gap-x-3 gap-y-1 text-[0.9rem] text-ink-2">
        <span>{f.fixed ? `Fixed in ${f.fixed}` : 'No fix released yet'}</span>
        <SourceLink url={f.source} />
      </p>
    </li>
  )
}
