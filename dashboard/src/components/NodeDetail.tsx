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
import type { Finding, NodeReport } from '../types/report'
import { FindingItem } from './FindingItem'
import { StatusPill } from './StatusPill'

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
    <section className="mt-7">
      <h3 className="mb-3 text-[0.75rem] font-semibold tracking-[0.08em] text-fg-3 uppercase">
        {title} <span className="font-mono">{count}</span>
      </h3>
      <ul className="space-y-3">{children}</ul>
    </section>
  )
}

function items(findings: Finding[]) {
  return findings.map((f) => <FindingItem key={f.advisory_id} finding={f} />)
}

/** The one thing to do about this node, ahead of the detail. */
function Action({ node }: { node: NodeReport }) {
  if (node.error) {
    const plain = describeError(node.error)
    return (
      <div className="rounded-lg bg-amber-soft px-5 py-4">
        <p className="text-[1.05rem] font-semibold">{plain ?? node.error}</p>
        <p className="mt-1 text-[0.9rem] text-fg-2">
          Firewatch won’t guess, so this node stays amber until it can be read.
        </p>
        {plain && (
          <details className="mt-2 text-[0.8rem] text-fg-3">
            <summary className="cursor-pointer select-none">Technical detail</summary>
            <p className="mt-1 font-mono break-words">{node.error}</p>
          </details>
        )}
      </div>
    )
  }
  const build = node.build_check
  if (build?.status === 'mismatch') {
    return (
      <div className="rounded-lg bg-amber-soft px-5 py-4">
        <p className="text-[1.05rem] font-semibold">
          This node says it runs {build.release}, but its build isn’t that release.
        </p>
        <p className="mt-1 text-[0.9rem] text-fg-2">
          Commit <span className="font-mono">{shortHash(build.actual)}</span>, official release{' '}
          <span className="font-mono">{shortHash(build.expected)}</span>. Firewatch can’t trust the
          version number, so every advisory below is amber.
        </p>
      </div>
    )
  }
  const reading = readingOf(node)
  if (reading === 'affected') {
    const target = fixedTarget(node)
    return (
      <div className="rounded-lg bg-red-soft px-5 py-4">
        <p className="text-[1.05rem] font-semibold">
          {target ? (
            <>
              Upgrade: fixed in <span className="font-mono">{target}</span> or later
            </>
          ) : (
            'Upgrade: each advisory below names its fix'
          )}
        </p>
        <p className="mt-1 text-[0.9rem] text-fg-2">
          {target
            ? 'The highest fix among the advisories below. Their own notes are linked.'
            : 'The fixes are on different release lines.'}
        </p>
      </div>
    )
  }
  if (reading === 'cant_tell') {
    return (
      <div className="rounded-lg bg-amber-soft px-5 py-4">
        <p className="text-[1.05rem] font-semibold">Check by hand before trusting this node</p>
        <p className="mt-1 text-[0.9rem] text-fg-2">
          Something an advisory depends on can’t be read over RPC.
        </p>
      </div>
    )
  }
  return null
}

export function NodeDetail({ node }: { node: NodeReport }) {
  const reading = readingOf(node)
  const version = node.version ? splitVersion(node.version) : null
  const affected = node.findings.filter((f) => f.status === 'affected')
  const toCheck = node.findings.filter((f) => f.status === 'cant_tell')
  const ruledOut = node.findings.filter(ruledOutByCondition)
  const clear = node.findings.filter((f) => f.status === 'not_affected' && !ruledOutByCondition(f))

  return (
    <article
      aria-labelledby="detail-title"
      className="rounded-xl border border-line bg-surface-2 p-6 sm:p-7"
    >
      <header className="flex flex-wrap items-start justify-between gap-4">
        <div className="min-w-0">
          <h2 id="detail-title" className="truncate text-[1.6rem] leading-tight font-bold">
            {node.name}
          </h2>
          <p className="mt-1 text-[0.95rem] text-fg-2">
            {implementationName[node.implementation]}
            {version && <span className="font-mono text-fg"> {version.version}</span>}
            {node.network && <span className="text-fg-3"> · {node.network}</span>}
          </p>
          {version?.build && (
            <p className="mt-0.5 font-mono text-[0.78rem] text-fg-3">build {version.build}</p>
          )}
          {node.build_check?.status === 'unknown_release' && (
            <p className="mt-1 text-[0.8rem] text-fg-3">
              Release not in Firewatch’s table, so its build wasn’t verified.
            </p>
          )}
        </div>
        <StatusPill reading={reading} large />
      </header>

      <p className="mt-3 text-[0.95rem] text-fg-2">{nodeSummary(node)}</p>

      <div className="mt-5">
        <Action node={node} />
      </div>

      {reading === 'none' && (
        <p className="mt-5 text-[0.95rem] text-fg-2">
          No advisory in Firewatch’s list covers {implementationName[node.implementation]}{' '}
          {version?.version}. That isn’t proof it’s safe, only that nothing published applies.
        </p>
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
        <details className="group mt-7">
          <summary className="flex cursor-pointer list-none items-center gap-2 text-[0.85rem] text-fg-3 select-none hover:text-fg-2">
            <span aria-hidden className="transition-transform group-open:rotate-90">
              ▸
            </span>
            {plural(clear.length, 'advisory doesn’t apply', 'advisories don’t apply')} to this
            version
          </summary>
          <ul className="mt-3 space-y-3">{items(clear)}</ul>
        </details>
      )}
    </article>
  )
}
