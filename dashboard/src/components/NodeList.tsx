import { useEffect, useRef } from 'react'
import {
  implementationName,
  nodeSummary,
  readingOf,
  splitVersion,
  type Reading,
} from '../lib/format'
import type { NodeReport } from '../types/report'
import { tone } from '../lib/tone'
import { StatusPill } from './StatusPill'

const flash: Record<Reading, string> = {
  affected: 'var(--fw-red-soft)',
  cant_tell: 'var(--fw-amber-soft)',
  not_affected: 'var(--fw-green-soft)',
  none: 'var(--fw-neutral-soft)',
}

function NodeRow({
  node,
  selected,
  onSelect,
}: {
  node: NodeReport
  selected: boolean
  onSelect: () => void
}) {
  const reading = readingOf(node)
  const version = node.version ? splitVersion(node.version).version : null

  // Glow once when this node's verdict changes (e.g. after an upgrade).
  const el = useRef<HTMLButtonElement>(null)
  const prev = useRef(reading)
  useEffect(() => {
    if (prev.current === reading || !el.current) return
    prev.current = reading
    el.current.classList.remove('fw-changed')
    void el.current.offsetWidth // restart the animation
    el.current.classList.add('fw-changed')
  }, [reading])

  return (
    <li>
      <button
        ref={el}
        type="button"
        onClick={onSelect}
        aria-current={selected ? 'true' : undefined}
        style={{ ['--flash' as string]: flash[reading] }}
        className={`relative grid w-full grid-cols-[1fr_auto] items-center gap-x-4 gap-y-1 rounded-lg px-4 py-3.5 text-left transition-colors ${
          selected ? 'bg-surface-2' : 'hover:bg-hover'
        }`}
      >
        <span
          aria-hidden
          className={`absolute top-3 bottom-3 left-0 w-[3px] rounded-full ${
            selected ? tone[reading].bar : 'bg-transparent'
          }`}
        />
        <span className="min-w-0">
          <span className="block truncate text-[1.02rem] font-semibold">{node.name}</span>
          <span className="mt-0.5 block truncate text-[0.85rem] text-fg-2">
            {implementationName[node.implementation]}
            {version && <span className="font-mono"> {version}</span>}
          </span>
        </span>
        <StatusPill reading={reading} />
        <span className="col-span-2 truncate text-[0.85rem] text-fg-3">{nodeSummary(node)}</span>
      </button>
    </li>
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
      <p className="mb-2 px-4 text-[0.75rem] font-semibold tracking-[0.08em] text-fg-3 uppercase">
        Nodes · worst first
      </p>
      <ul className="space-y-1">
        {nodes.map((n) => (
          <NodeRow
            key={n.name}
            node={n}
            selected={n.name === selected}
            onSelect={() => onSelect(n.name)}
          />
        ))}
      </ul>
    </nav>
  )
}
