import { severityName, sourceLabel } from '../lib/format'
import type { Finding } from '../types/report'
import { WithCode } from './Code'
import { ManualChecklist } from './ManualChecklist'

export function SourceLink({ url }: { url: string }) {
  return (
    <a
      href={url}
      target="_blank"
      rel="noopener noreferrer"
      className="whitespace-nowrap text-fg-2 underline decoration-line underline-offset-4 hover:text-fg hover:decoration-fg-2"
    >
      {sourceLabel(url)} ↗
    </a>
  )
}

function Meta({ f }: { f: Finding }) {
  return (
    <p className="flex flex-wrap items-center gap-x-2 gap-y-1 text-[0.8rem] text-fg-3">
      <span className="font-mono">{f.advisory_id}</span>
      <span aria-hidden>·</span>
      <span>{severityName[f.severity]}</span>
      {f.exploited_in_wild && (
        <>
          <span aria-hidden>·</span>
          <span className="font-semibold text-red">Exploited in the wild</span>
        </>
      )}
      {!f.verified && (
        <>
          <span aria-hidden>·</span>
          <span className="rounded bg-neutral-soft px-1.5 text-neutral">Not yet verified</span>
        </>
      )}
    </p>
  )
}

/** One advisory on the selected node. What it shows depends on its verdict. */
export function FindingItem({ finding: f }: { finding: Finding }) {
  return (
    <li className="rounded-lg border border-line bg-surface px-5 py-4">
      <Meta f={f} />
      <h4 className="mt-1.5 text-[1.02rem] leading-snug font-semibold">{f.title}</h4>

      {f.status === 'affected' && (
        <figure className="mt-3">
          <blockquote className="border-l-2 border-red pl-3.5 text-[0.98rem] leading-relaxed">
            “<WithCode text={f.steps} />”
          </blockquote>
          <figcaption className="mt-1.5 pl-4 text-[0.8rem] text-fg-3">
            The developers’ words
          </figcaption>
        </figure>
      )}

      {f.status === 'cant_tell' && (
        <>
          <p className="mt-2 text-[0.9rem] leading-relaxed text-fg-2">
            <WithCode text={f.reason} />
          </p>
          {f.manual_checks.length > 0 && (
            <div className="mt-3 rounded-md bg-amber-soft px-3.5 py-3">
              <p
                id={`${f.advisory_id}-why`}
                className="mb-2 text-[0.85rem] font-semibold text-amber"
              >
                Check by hand. Firewatch won’t guess, so this stays amber.
              </p>
              <ManualChecklist id={f.advisory_id} checks={f.manual_checks} />
            </div>
          )}
        </>
      )}

      {f.status === 'not_affected' && (
        <p className="mt-2 text-[0.9rem] leading-relaxed text-fg-2">
          <WithCode text={f.reason} />
        </p>
      )}

      <p className="mt-3 flex flex-wrap items-baseline gap-x-4 gap-y-1 text-[0.85rem] text-fg-3">
        <span>{f.fixed ? `Fixed in ${f.fixed}` : 'No fix released yet'}</span>
        <SourceLink url={f.source} />
      </p>
    </li>
  )
}
