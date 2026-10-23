import { motion } from 'motion/react'
import { severityName, sourceLabel } from '../lib/format'
import { rise } from '../lib/motion'
import type { Finding } from '../types/report'
import { WithCode } from './Code'
import { ManualChecklist } from './ManualChecklist'

export function SourceLink({ url }: { url: string }) {
  return (
    <a
      href={url}
      target="_blank"
      rel="noopener noreferrer"
      className="group inline-flex items-center gap-1 whitespace-nowrap text-ink-2 hover:text-ink"
    >
      <span className="underline decoration-line-strong underline-offset-4 group-hover:decoration-ink">
        {sourceLabel(url)}
      </span>
      <span
        aria-hidden
        className="transition-transform duration-300 group-hover:translate-x-0.5 group-hover:-translate-y-0.5"
      >
        ↗
      </span>
    </a>
  )
}

function Meta({ f }: { f: Finding }) {
  return (
    <p className="flex flex-wrap items-center gap-x-2 gap-y-1 font-mono text-[0.74rem] tracking-[0.04em] text-ink-3 uppercase">
      <span>{f.advisory_id}</span>
      <span aria-hidden>·</span>
      <span>{severityName[f.severity]}</span>
      {f.exploited_in_wild && (
        <span className="ml-1 rounded-full bg-red px-2 py-[1px] font-sans text-[0.72rem] font-semibold tracking-normal text-white normal-case">
          Exploited in the wild
        </span>
      )}
      {!f.verified && (
        <span className="ml-1 rounded-full bg-neutral-soft px-2 py-[1px] font-sans text-[0.72rem] tracking-normal text-neutral normal-case">
          Not yet verified
        </span>
      )}
    </p>
  )
}

/** One advisory on the selected node. What it shows depends on its verdict. */
export function FindingItem({ finding: f }: { finding: Finding }) {
  return (
    <motion.li variants={rise} data-surface className="fw-glass rounded-2xl px-5 py-4.5">
      <Meta f={f} />
      <h4 className="mt-2 text-[1.05rem] leading-snug font-semibold">{f.title}</h4>

      {f.status === 'affected' && (
        <figure className="mt-3.5">
          <blockquote className="relative pl-6 font-serif text-[1.12rem] leading-relaxed italic">
            <span
              aria-hidden
              className="absolute top-[-0.4rem] left-0 font-serif text-[2.2rem] leading-none text-red"
            >
              “
            </span>
            <WithCode text={f.steps} />
          </blockquote>
          <figcaption className="mt-1.5 pl-6 text-[0.8rem] text-ink-3">
            The developers’ words
          </figcaption>
        </figure>
      )}

      {f.status !== 'affected' && (
        <p className="mt-2 text-[0.92rem] leading-relaxed text-ink-2">
          <WithCode text={f.reason} />
        </p>
      )}

      {f.status === 'cant_tell' && f.manual_checks.length > 0 && (
        <div className="mt-3.5 rounded-xl border border-amber/25 bg-amber-soft px-4 py-3.5">
          <p id={`${f.advisory_id}-why`} className="mb-2.5 text-[0.86rem] font-semibold text-amber">
            Check by hand. Firewatch won’t guess, so this stays amber.
          </p>
          <ManualChecklist id={f.advisory_id} checks={f.manual_checks} />
        </div>
      )}

      <p className="mt-3.5 flex flex-wrap items-baseline gap-x-4 gap-y-1 text-[0.85rem] text-ink-3">
        <span>{f.fixed ? `Fixed in ${f.fixed}` : 'No fix released yet'}</span>
        <SourceLink url={f.source} />
      </p>
    </motion.li>
  )
}
