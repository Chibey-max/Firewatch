import {
  countBy,
  describeError,
  implementationName,
  plural,
  readingOf,
  readingWord,
  ruledOutByCondition,
  splitVersion,
  type Reading,
} from '../lib/format'
import type { NodeReport } from '../types/report'
import { Dial } from './Dial'
import { FindingItem } from './FindingItem'
import { NotAffectedGroup } from './NotAffectedGroup'

const tone: Record<Reading, { bar: string; word: string }> = {
  affected: { bar: 'bg-affected', word: 'text-affected text-[2.6rem]' },
  cant_tell: { bar: 'bg-canttell', word: 'text-canttell-ink text-[2.6rem]' },
  not_affected: { bar: 'bg-safe', word: 'text-safe text-[2.6rem]' },
  // Longer and deliberately quieter: no advisory applies, which isn't a verdict.
  none: { bar: 'bg-none', word: 'text-none text-[1.9rem] py-1.5' },
}

/** One sentence under the verdict word, saying what the dial is based on. */
function summary(node: NodeReport, reading: Reading): string {
  if (node.error) return "Couldn't read this node"
  const counts = countBy(node.findings)
  const exploited = node.findings.filter(
    (f) => f.status === 'affected' && f.exploited_in_wild,
  ).length
  const conditional = node.findings.filter(ruledOutByCondition).length
  const ruledOutNote = conditional ? ` · ${conditional} ruled out by a condition` : ''
  switch (reading) {
    case 'affected':
      return (
        `${plural(counts.affected, 'advisory applies', 'advisories apply')}` +
        (exploited ? ` · ${exploited} exploited in the wild` : '') +
        ruledOutNote
      )
    case 'cant_tell':
      return `${plural(counts.cant_tell, 'advisory', 'advisories')} can't be confirmed over RPC`
    case 'not_affected':
      return `Checked ${plural(node.findings.length, 'advisory', 'advisories')}, none apply${ruledOutNote}`
    case 'none':
      return "Nothing in Firewatch's list applies. That isn't proof it's safe."
  }
}

export function NodeCard({ node }: { node: NodeReport }) {
  const reading = readingOf(node)
  const version = node.version ? splitVersion(node.version) : null
  // Shown: anything not green, plus green findings a condition ruled out.
  const shown = node.findings.filter((f) => f.status !== 'not_affected' || ruledOutByCondition(f))
  const folded = node.findings.filter((f) => !shown.includes(f))

  return (
    <article
      className="overflow-hidden rounded-md bg-panel shadow-[0_1px_0_var(--color-rule),0_8px_24px_-16px_rgb(20_36_32/.35)]"
      aria-labelledby={`node-${node.name}`}
    >
      <div className={`h-1.5 ${tone[reading].bar}`} />
      <div className="px-6 pt-5 pb-6">
        {/* Fixed height so every dial sits on the same line across cards. */}
        <header className="flex min-h-[5.25rem] items-start justify-between gap-3">
          <div className="min-w-0">
            <h2
              id={`node-${node.name}`}
              className="truncate font-display text-[1.9rem] leading-none font-bold"
            >
              {node.name}
            </h2>
            <p className="mt-1.5 text-[0.95rem] text-ink-2">
              {implementationName[node.implementation]}
              {version && (
                <>
                  {' · '}
                  <span className="font-mono text-ink">{version.version}</span>
                </>
              )}
            </p>
            {version?.build && (
              <p className="mt-0.5 font-mono text-[0.78rem] text-ink-2">build {version.build}</p>
            )}
          </div>
          {node.network && (
            <span className="shrink-0 rounded-sm border border-rule px-1.5 py-px font-mono text-[0.75rem] text-ink-2">
              {node.network}
            </span>
          )}
        </header>

        <div className="mt-4 flex flex-col items-center text-center">
          <Dial reading={reading} />
          <p
            className={`mt-1 font-display leading-none font-extrabold tracking-wide uppercase ${tone[reading].word}`}
          >
            {readingWord[reading]}
          </p>
          <p className="mt-2 max-w-[34ch] text-[0.98rem] text-ink-2">{summary(node, reading)}</p>
        </div>

        {node.error && (
          <div className="mt-5 rounded-sm bg-canttell-tint px-4 py-3">
            {describeError(node.error) && (
              <p className="text-[1.02rem] font-semibold text-ink">{describeError(node.error)}</p>
            )}
            <p className="mt-1 text-[0.95rem] text-canttell-ink">
              Firewatch won't guess, so this node stays amber until it can be read.
            </p>
            <p className="mt-1.5 font-mono text-[0.8rem] break-words text-ink-2">{node.error}</p>
          </div>
        )}

        {shown.length > 0 && (
          <ul className="mt-5 space-y-5 border-t border-rule pt-5">
            {shown.map((f) => (
              <FindingItem key={f.advisory_id} finding={f} />
            ))}
          </ul>
        )}

        <NotAffectedGroup findings={folded} />
      </div>
    </article>
  )
}
