import type { NodeReport } from '../types/report'
import { NodeDetail } from './NodeDetail'
import { Sheet } from './Sheet'

/**
 * Below the side-by-side layout, a node's detail opens over the list instead
 * of sitting below it (see Sheet for how it opens and closes). Closing hands
 * focus back to the node's row.
 */
export function DetailSheet({
  node,
  onClose,
  beside = false,
}: {
  node: NodeReport | null
  onClose: () => void
  beside?: boolean
}) {
  return (
    <Sheet open={node !== null} onClose={onClose} labelledBy="detail-title" beside={beside}>
      {node && <NodeDetail node={node} bare />}
    </Sheet>
  )
}
