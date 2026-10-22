import { checkedAgo } from '../lib/format'

interface Props {
  checkedAt: number | null
  now: number
  checking: boolean
  checkError: string | null
  onCheck: (() => void) | null
}

/** "Checked 4 s ago" and the Re-check button. `onCheck` is null in fixture mode. */
export function CheckControls({ checkedAt, now, checking, checkError, onCheck }: Props) {
  return (
    <div className="flex flex-wrap items-center gap-x-4 gap-y-1">
      {checkedAt !== null && (
        <span className="text-[0.95rem] text-ink-2 tabular-nums" aria-live="polite">
          {checkedAgo(checkedAt, now)}
        </span>
      )}
      {onCheck && (
        <button
          type="button"
          onClick={onCheck}
          disabled={checking}
          className="rounded-sm bg-ink px-4 py-2 text-[0.95rem] font-semibold text-panel transition-colors hover:bg-ink/85 disabled:cursor-wait disabled:bg-ink-2"
        >
          {checking ? 'Checking…' : 'Re-check now'}
        </button>
      )}
      {checkError && (
        <span role="alert" className="basis-full text-right text-[0.9rem] text-affected">
          {checkError}
        </span>
      )}
    </div>
  )
}
