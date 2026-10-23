import { useState } from 'react'
import { WithCode } from './Code'

/**
 * The hand checks for an amber finding, as a checklist. Ticking is a note for
 * the operator only: it's never sent anywhere and never turns the finding green,
 * because Firewatch still can't confirm it.
 */
export function ManualChecklist({ id, checks }: { id: string; checks: string[] }) {
  const [done, setDone] = useState<boolean[]>(() => checks.map(() => false))
  const allDone = done.length > 0 && done.every(Boolean)

  return (
    <>
      <ul className="space-y-2">
        {checks.map((check, i) => (
          <li key={check}>
            <label className="flex cursor-pointer gap-3 text-[0.98rem] leading-relaxed">
              <input
                type="checkbox"
                checked={done[i] ?? false}
                onChange={(e) => setDone((d) => d.map((v, j) => (j === i ? e.target.checked : v)))}
                aria-describedby={`${id}-why`}
                className="mt-1 size-[1.1rem] shrink-0 accent-amber"
              />
              <span className={done[i] ? 'text-ink-3 line-through decoration-amber' : ''}>
                <WithCode text={check} />
              </span>
            </label>
          </li>
        ))}
      </ul>
      {allDone && (
        <p className="mt-2 text-[0.9rem] text-ink-3">
          Checked by hand. This stays amber here, because Firewatch can't confirm it itself.
        </p>
      )}
    </>
  )
}
