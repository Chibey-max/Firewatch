import { motion } from 'motion/react'
import { useId } from 'react'
import { isExposed } from '../lib/format'
import { rise } from '../lib/motion'
import type { Exposure } from '../types/report'
import { WithCode } from './Code'
import { SourceLink } from './FindingItem'

/**
 * Who can reach the node's APIs: a leaked macaroon or rune only works where
 * its API can be reached. Kept apart from the findings and in no verdict
 * colour, because it never changes the node's status: a fully patched node
 * can still be exposed. Nothing shows when every API is local.
 */
export function ExposureNote({ exposure }: { exposure: Exposure[] | undefined }) {
  const id = useId()
  const open = (exposure ?? []).filter((e) => e.status !== 'local')
  if (open.length === 0) return null
  const exposed = isExposed(open)
  const checks = [...new Set(open.map((e) => e.manual_check).filter((c): c is string => !!c))]
  const source = open.find((e) => e.source)?.source

  return (
    <motion.section
      variants={rise}
      aria-labelledby={id}
      className="fw-glass mt-8 rounded-2xl px-5 py-4.5"
    >
      <p className="font-mono text-[0.72rem] tracking-[0.14em] text-ink-3 uppercase">
        API access · separate from the verdict
      </p>
      <h3 id={id} className="mt-2 flex items-center gap-2.5 text-[1.05rem] font-semibold">
        <svg
          viewBox="0 0 20 20"
          className="size-[18px] shrink-0 text-ink-2"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.5"
          aria-hidden
        >
          <circle cx="10" cy="10" r="7.5" />
          <path d="M2.5 10h15M10 2.5c2.2 2.1 3.2 4.6 3.2 7.5s-1 5.4-3.2 7.5c-2.2-2.1-3.2-4.6-3.2-7.5s1-5.4 3.2-7.5Z" />
        </svg>
        {exposed ? 'Reachable from other machines' : 'Couldn’t tell who can reach its APIs'}
      </h3>
      <ul className="mt-2 space-y-1.5 text-[0.92rem] leading-relaxed text-ink-2">
        {open.map((e) => (
          <li key={e.api}>
            <WithCode text={e.reason} />
          </li>
        ))}
      </ul>
      {checks.length > 0 && (
        <div className="mt-3.5 rounded-xl border border-line bg-ink/5 px-4 py-3">
          <p className="mb-1.5 text-[0.86rem] font-semibold">To check</p>
          <ul className="list-disc space-y-1 pl-5 text-[0.92rem] leading-relaxed text-ink-2">
            {checks.map((c) => (
              <li key={c}>
                <WithCode text={c} />
              </li>
            ))}
          </ul>
        </div>
      )}
      <p className="mt-3.5 flex flex-wrap items-baseline gap-x-4 gap-y-1 text-[0.85rem] text-ink-3">
        <span>A leaked macaroon or rune only works where its API can be reached.</span>
        {source && <SourceLink url={source} />}
      </p>
    </motion.section>
  )
}
