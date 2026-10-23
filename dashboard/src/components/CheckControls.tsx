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
    <>
      {checkError && (
        <span role="alert" className="text-[0.85rem] text-red">
          {checkError}
        </span>
      )}
      {checkedAt !== null && (
        <span className="text-[0.85rem] text-fg-3 tabular-nums" aria-live="polite">
          {checkedAgo(checkedAt, now)}
        </span>
      )}
      {onCheck && (
        <button
          type="button"
          onClick={onCheck}
          disabled={checking}
          className="rounded-md bg-button px-3.5 py-2 text-[0.88rem] font-semibold text-button-fg hover:opacity-90 disabled:cursor-wait disabled:opacity-60"
        >
          {checking ? 'Checking…' : 'Re-check now'}
        </button>
      )}
    </>
  )
}
