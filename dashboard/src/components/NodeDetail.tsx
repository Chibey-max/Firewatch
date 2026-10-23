import { AnimatePresence, motion } from 'motion/react'
import type { ReactNode } from 'react'
import {
  describeError,
  fixedTarget,
  implementationName,
  nodeSummary,
  plural,
  readingOf,
  ruledOutByCondition,
  shortHash,
  splitVersion,
} from '../lib/format'
import { EASE, rise } from '../lib/motion'
import type { Finding, NodeReport } from '../types/report'
import { FindingItem } from './FindingItem'
import { StatusPill } from './StatusPill'

const list = { hidden: {}, show: { transition: { staggerChildren: 0.06, delayChildren: 0.12 } } }

function Section({
  title,
  count,
  children,
}: {
  title: string
  count: number
  children: ReactNode
}) {
  if (count === 0) return null
  return (
    <section className="mt-8">
      <h3 className="mb-3 flex items-center gap-2 font-mono text-[0.72rem] tracking-[0.14em] text-ink-3 uppercase">
        {title}
        <span className="rounded-full border border-line px-1.5 py-px">{count}</span>
      </h3>
      <motion.ul variants={list} initial="hidden" animate="show" className="space-y-3">
        {children}
      </motion.ul>
    </section>
  )
}

function items(findings: Finding[]) {
  return findings.map((f) => <FindingItem key={f.advisory_id} finding={f} />)
}

function Callout({
  tone,
  icon,
  title,
  children,
}: {
  tone: 'red' | 'amber'
  icon: ReactNode
  title: ReactNode
  children?: ReactNode
}) {
  const t =
    tone === 'red'
      ? 'border-red/25 bg-red-soft text-red'
      : 'border-amber/25 bg-amber-soft text-amber'
  return (
    <motion.div variants={rise} className={`flex gap-4 rounded-2xl border px-5 py-4.5 ${t}`}>
      {/* A droplet of liquid metal holds the icon: wobbling, rimmed in turning chrome. */}
      <span className="fw-droplet mt-0.5 grid size-10 shrink-0 place-items-center">
        <span className="fw-chrome-rim" />
        <span className="relative">{icon}</span>
      </span>
      <div className="min-w-0 text-ink">
        <p className="text-[1.08rem] leading-snug font-semibold">{title}</p>
        {children && (
          <div className="mt-1 text-[0.92rem] leading-relaxed text-ink-2">{children}</div>
        )}
      </div>
    </motion.div>
  )
}

const ArrowUp = (
  <svg viewBox="0 0 16 16" className="size-4" aria-hidden>
    <path
      d="M8 13V3M3.5 7.5 8 3l4.5 4.5"
      fill="none"
      stroke="currentColor"
      strokeWidth="1.8"
      strokeLinecap="round"
      strokeLinejoin="round"
    />
  </svg>
)
const Pause = (
  <svg viewBox="0 0 16 16" className="size-4" aria-hidden>
    <path
      d="M8 4.5v4.5M8 11.6v.1"
      fill="none"
      stroke="currentColor"
      strokeWidth="2"
      strokeLinecap="round"
    />
  </svg>
)

/** The one thing to do about this node, ahead of the detail. */
function Action({ node }: { node: NodeReport }) {
  if (node.error) {
    const plain = describeError(node.error)
    return (
      <Callout tone="amber" icon={Pause} title={plain ?? node.error}>
        <p>Firewatch won’t guess, so this node stays amber until it can be read.</p>
        {plain && (
          <details className="mt-1.5 text-[0.8rem] text-ink-3">
            <summary className="cursor-pointer select-none hover:text-ink-2">
              Technical detail
            </summary>
            <p className="mt-1 font-mono break-words">{node.error}</p>
          </details>
        )}
      </Callout>
    )
  }
  const build = node.build_check
  if (build?.status === 'mismatch') {
    return (
      <Callout
        tone="amber"
        icon={Pause}
        title={<>This node says it runs {build.release}, but its build isn’t that release</>}
      >
        Commit <span className="font-mono">{shortHash(build.actual)}</span>; the official release is{' '}
        <span className="font-mono">{shortHash(build.expected)}</span>. Firewatch can’t trust the
        version number, so every advisory below is amber.
      </Callout>
    )
  }
  const reading = readingOf(node)
  if (reading === 'affected') {
    const target = fixedTarget(node)
    return (
      <Callout
        tone="red"
        icon={ArrowUp}
        title={
          target ? (
            <>
              Upgrade: fixed in <span className="font-mono">{target}</span> or later
            </>
          ) : (
            'Upgrade: each advisory below names its fix'
          )
        }
      >
        {target
          ? 'The highest fix among the advisories below. Their own notes are linked.'
          : 'The fixes are on different release lines.'}
      </Callout>
    )
  }
  if (reading === 'cant_tell') {
    return (
      <Callout tone="amber" icon={Pause} title="Check by hand before trusting this node">
        Something an advisory depends on can’t be read over RPC.
      </Callout>
    )
  }
  return null
}

function Detail({ node }: { node: NodeReport }) {
  const reading = readingOf(node)
  const version = node.version ? splitVersion(node.version) : null
  const affected = node.findings.filter((f) => f.status === 'affected')
  const toCheck = node.findings.filter((f) => f.status === 'cant_tell')
  const ruledOut = node.findings.filter(ruledOutByCondition)
  const clear = node.findings.filter((f) => f.status === 'not_affected' && !ruledOutByCondition(f))

  return (
    <motion.div initial="hidden" animate="show" variants={list}>
      <motion.header variants={rise} className="flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0">
          <h2 id="detail-title" className="fw-display truncate text-[2.1rem] leading-tight">
            {node.name}
          </h2>
          <p className="mt-1 text-[0.95rem] text-ink-2">
            {implementationName[node.implementation]}
            {version && <span className="font-mono text-ink"> {version.version}</span>}
            {node.network && <span className="text-ink-3"> · {node.network}</span>}
          </p>
          {version?.build && (
            <p className="mt-0.5 font-mono text-[0.76rem] text-ink-3">build {version.build}</p>
          )}
          {node.build_check?.status === 'match' && (
            <p className="mt-1 text-[0.8rem] text-green">
              Official build: its commit matches the release
            </p>
          )}
          {node.build_check?.status === 'unknown_release' && (
            <p className="mt-1 text-[0.8rem] text-ink-3">
              Release not in Firewatch’s table, so its build wasn’t verified
            </p>
          )}
        </div>
        <StatusPill reading={reading} large />
      </motion.header>

      <motion.p variants={rise} className="mt-3 text-[0.95rem] text-ink-2">
        {nodeSummary(node)}
      </motion.p>

      <div className="mt-5">
        <Action node={node} />
      </div>

      {reading === 'none' && (
        <motion.p variants={rise} className="mt-5 text-[0.95rem] text-ink-2">
          No advisory in Firewatch’s list covers {implementationName[node.implementation]}{' '}
          {version?.version}. That isn’t proof it’s safe, only that nothing published applies.
        </motion.p>
      )}

      <Section title="Affected" count={affected.length}>
        {items(affected)}
      </Section>
      <Section title="Check by hand" count={toCheck.length}>
        {items(toCheck)}
      </Section>
      <Section title="Ruled out by a condition" count={ruledOut.length}>
        {items(ruledOut)}
      </Section>

      {clear.length > 0 && (
        <motion.details variants={rise} className="group mt-8">
          <summary className="flex cursor-pointer list-none items-center gap-2 text-[0.86rem] text-ink-3 select-none hover:text-ink-2">
            <span
              aria-hidden
              className="inline-block transition-transform duration-300 group-open:rotate-90"
            >
              ▸
            </span>
            {plural(clear.length, 'advisory doesn’t apply', 'advisories don’t apply')} to this
            version
          </summary>
          <ul className="mt-3 space-y-3">{items(clear)}</ul>
        </motion.details>
      )}
    </motion.div>
  )
}

/** The selected node; switching nodes crossfades the panel. */
export function NodeDetail({ node }: { node: NodeReport }) {
  return (
    <article
      aria-labelledby="detail-title"
      data-surface
      className="fw-glass-strong relative min-h-full overflow-hidden rounded-[28px] p-6 sm:p-9"
    >
      <AnimatePresence mode="wait" initial={false}>
        <motion.div
          key={node.name}
          initial={{ opacity: 0, y: 10, filter: 'blur(6px)' }}
          animate={{ opacity: 1, y: 0, filter: 'blur(0px)' }}
          exit={{ opacity: 0, y: -6, filter: 'blur(4px)' }}
          transition={{ duration: 0.4, ease: EASE }}
        >
          <Detail node={node} />
        </motion.div>
      </AnimatePresence>
    </article>
  )
}
