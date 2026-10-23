import { AnimatePresence, motion } from 'motion/react'
import { implementationName, nodeSummary, readingOf, splitVersion } from '../lib/format'
import { EASE, SOFT, spring } from '../lib/motion'
import { tone } from '../lib/tone'
import type { NodeReport } from '../types/report'
import { StatusPill } from './StatusPill'

function NodeRow({
  node,
  index,
  selected,
  onSelect,
}: {
  node: NodeReport
  index: number
  selected: boolean
  onSelect: () => void
}) {
  const reading = readingOf(node)
  const version = node.version ? splitVersion(node.version).version : null

  return (
    // `layout` slides the row to its new place when a verdict reorders the list.
    <motion.li
      layout="position"
      transition={spring}
      initial={{ opacity: 0, y: 14 }}
      animate={{
        opacity: 1,
        y: 0,
        transition: { duration: 0.6, ease: EASE, delay: 0.15 + index * 0.06 },
      }}
    >
      <motion.button
        type="button"
        onClick={onSelect}
        aria-current={selected ? 'true' : undefined}
        whileHover={selected ? undefined : { y: -2 }}
        whileTap={{ scale: 0.985 }}
        transition={{ duration: 0.25, ease: EASE }}
        className="relative w-full rounded-[18px] text-left"
      >
        {/* Each row has its own highlight: choosing another node fades this
            one out and the new one in, so it never travels across other rows.
            A chrome rim turns round the selected row. */}
        <AnimatePresence initial={false}>
          {selected && (
            <motion.span
              key="highlight"
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              exit={{ opacity: 0 }}
              transition={{ duration: 0.5, ease: SOFT }}
              className="absolute -inset-px rounded-[19px] bg-surface-solid shadow-raised"
            >
              <span className="fw-chrome-rim" />
            </motion.span>
          )}
        </AnimatePresence>
        <span
          className={`relative grid grid-cols-[auto_1fr_auto] items-center gap-x-3.5 rounded-[18px] px-4 py-3.5 transition-colors ${
            selected ? '' : 'fw-glass hover:bg-surface-hover'
          }`}
        >
          <span
            aria-hidden
            className={`h-9 w-1 rounded-full transition-colors duration-500 ${tone[reading].bar}`}
          />
          <span className="min-w-0">
            <span className="block truncate text-[1.02rem] font-semibold">{node.name}</span>
            <span className="mt-0.5 block truncate text-[0.84rem] text-ink-2">
              {implementationName[node.implementation]}
              {version && <span className="font-mono text-[0.8rem]"> {version}</span>}
            </span>
          </span>
          <StatusPill reading={reading} />
          <span className="col-span-2 col-start-2 mt-1 truncate text-[0.82rem] text-ink-3">
            {nodeSummary(node)}
          </span>
        </span>
      </motion.button>
    </motion.li>
  )
}

export function NodeList({
  nodes,
  selected,
  onSelect,
}: {
  nodes: NodeReport[]
  selected: string | null
  onSelect: (name: string) => void
}) {
  return (
    <nav aria-label="Nodes">
      <p className="mb-3 px-1 font-mono text-[0.72rem] tracking-[0.14em] text-ink-3 uppercase">
        Nodes · worst first
      </p>
      <ul className="space-y-2.5">
        {nodes.map((n, i) => (
          <NodeRow
            key={n.name}
            node={n}
            index={i}
            selected={n.name === selected}
            onSelect={() => onSelect(n.name)}
          />
        ))}
      </ul>
    </nav>
  )
}
