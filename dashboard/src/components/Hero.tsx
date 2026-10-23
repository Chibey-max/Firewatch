import { AnimatePresence, motion } from 'motion/react'
import { needsAction, plural, priorityLine, readingCounts, type Reading } from '../lib/format'
import { EASE } from '../lib/motion'
import type { Phase } from '../lib/useReport'
import type { Report } from '../types/report'

interface Line {
  before: string
  accent: string
  after: string
  tone: Reading
}

/** The one sentence a glance should get, with its key word set in italic. */
function headline(report: Report | null, phase: Phase): Line {
  if (!report) {
    return phase === 'offline'
      ? { before: 'Can’t reach the', accent: 'daemon', after: '.', tone: 'cant_tell' }
      : { before: 'Connecting to', accent: 'Firewatch', after: '…', tone: 'none' }
  }
  const total = report.nodes.length
  if (total === 0) return { before: 'No nodes to', accent: 'watch', after: ' yet.', tone: 'none' }
  const n = needsAction(report.nodes)
  if (n === 0) {
    const anyChecked = report.nodes.some((node) => node.findings.length > 0)
    return anyChecked
      ? { before: 'Every node is', accent: 'clear', after: '.', tone: 'not_affected' }
      : { before: 'Nothing published', accent: 'applies', after: '.', tone: 'none' }
  }
  const counts = readingCounts(report.nodes)
  return {
    before: `${n} of ${total} nodes ${n === 1 ? 'needs' : 'need'}`,
    accent: 'attention',
    after: '.',
    tone: counts.affected ? 'affected' : 'cant_tell',
  }
}

const accentColor: Record<Reading, string> = {
  affected: 'text-red',
  cant_tell: 'text-amber',
  not_affected: 'text-green',
  none: 'text-ink-2',
}
const underline: Record<Reading, string> = {
  affected: 'bg-red',
  cant_tell: 'bg-amber',
  not_affected: 'bg-green',
  none: 'bg-ink-3',
}

const word = {
  hidden: { opacity: 0, y: '0.35em', filter: 'blur(8px)' },
  show: { opacity: 1, y: 0, filter: 'blur(0px)', transition: { duration: 0.8, ease: EASE } },
  exit: { opacity: 0, y: '-0.25em', filter: 'blur(6px)', transition: { duration: 0.3 } },
}

function Headline({ line }: { line: Line }) {
  const words = line.before.split(' ')
  const text = `${line.before} ${line.accent}${line.after}`
  return (
    <h1 className="font-serif text-[clamp(2.3rem,4.6vw,3.9rem)] leading-[1.04] font-medium tracking-[-0.025em]">
      <span className="sr-only">{text}</span>
      <AnimatePresence mode="wait" initial={true}>
        <motion.span
          key={text}
          aria-hidden
          className="block"
          initial="hidden"
          animate="show"
          exit="exit"
          transition={{ staggerChildren: 0.06 }}
        >
          {words.map((w, i) => (
            <motion.span key={i} variants={word} className="inline-block whitespace-pre">
              {w}{' '}
            </motion.span>
          ))}
          <motion.span
            variants={word}
            className={`relative inline-block italic ${accentColor[line.tone]}`}
          >
            {line.accent}
            <motion.span
              className={`absolute right-0 -bottom-1 left-0 h-[3px] origin-left rounded-full opacity-60 ${underline[line.tone]}`}
              variants={{
                hidden: { scaleX: 0 },
                show: { scaleX: 1, transition: { duration: 1.1, ease: EASE, delay: 0.35 } },
                exit: { opacity: 0 },
              }}
            />
          </motion.span>
          <motion.span variants={word} className="inline-block">
            {line.after}
          </motion.span>
        </motion.span>
      </AnimatePresence>
    </h1>
  )
}

const bar: Record<Reading, string> = {
  affected: 'bg-red',
  cant_tell: 'bg-amber',
  not_affected: 'bg-green',
  none: 'bg-neutral',
}
const legend: { key: Reading; label: string }[] = [
  { key: 'affected', label: 'affected' },
  { key: 'cant_tell', label: 'can’t tell' },
  { key: 'not_affected', label: 'not affected' },
  { key: 'none', label: 'no known advisories' },
]

/** The fleet in one bar: segments resize as verdicts change. */
function FleetBar({ report }: { report: Report }) {
  const counts = readingCounts(report.nodes)
  const total = report.nodes.length
  return (
    <div className="w-full max-w-xl">
      <div className="flex h-2 gap-1 overflow-hidden rounded-full" aria-hidden>
        {legend.map(({ key }) =>
          counts[key] ? (
            <motion.span
              key={key}
              layout
              className={`h-full rounded-full ${bar[key]}`}
              initial={{ flexGrow: 0 }}
              animate={{ flexGrow: counts[key] }}
              transition={{ duration: 0.9, ease: EASE }}
              style={{ flexBasis: 0 }}
            />
          ) : null,
        )}
      </div>
      <p className="mt-3 flex flex-wrap gap-x-5 gap-y-1 text-[0.85rem] text-ink-2">
        {legend
          .filter(({ key }) => counts[key])
          .map(({ key, label }) => (
            <span key={key} className="flex items-center gap-2">
              <span aria-hidden className={`size-2 rounded-full ${bar[key]}`} />
              <span className="font-semibold text-ink tabular-nums">{counts[key]}</span> {label}
            </span>
          ))}
        <span className="sr-only">of {plural(total, 'node', 'nodes')}</span>
      </p>
    </div>
  )
}

export function Hero({ report, phase }: { report: Report | null; phase: Phase }) {
  const line = headline(report, phase)
  return (
    <section className="flex flex-col gap-5 pt-10 pb-8 sm:pt-14">
      <p className="font-mono text-[0.75rem] tracking-[0.14em] text-ink-3 uppercase">
        {report
          ? `Watching ${plural(report.nodes.length, 'Lightning node', 'Lightning nodes')} · read-only`
          : 'Security advisories for Lightning nodes'}
      </p>
      <Headline line={line} />
      {report && report.nodes.length > 0 && (
        <AnimatePresence mode="wait">
          <motion.p
            key={priorityLine(report.nodes)}
            initial={{ opacity: 0, y: 6 }}
            animate={{ opacity: 1, y: 0, transition: { duration: 0.6, ease: EASE, delay: 0.25 } }}
            exit={{ opacity: 0, transition: { duration: 0.2 } }}
            className="max-w-2xl text-[1.1rem] leading-relaxed text-ink-2"
          >
            {priorityLine(report.nodes)}
          </motion.p>
        </AnimatePresence>
      )}
      {report && report.nodes.length > 0 && <FleetBar report={report} />}
    </section>
  )
}
