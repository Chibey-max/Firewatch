import { useState } from 'react'
import { WithCode } from './Code'

// Ticks are kept in this browser only, by node, version and advisory: a new
// version or a reworded check starts unticked, because it has to be redone.
const KEY = 'fw-checks'
type Ticks = Record<string, number>

function loadAll(): Record<string, Ticks> {
  try {
    return JSON.parse(localStorage.getItem(KEY) ?? '{}') as Record<string, Ticks>
  } catch {
    return {}
  }
}

function saveTicks(scope: string, ticks: Ticks) {
  try {
    const all = loadAll()
    if (Object.keys(ticks).length) all[scope] = ticks
    else delete all[scope]
    localStorage.setItem(KEY, JSON.stringify(all))
  } catch {
    // storage blocked: ticks last for this visit only
  }
}

const nowSecs = () => Math.floor(Date.now() / 1000)

const day = (secs: number) =>
  new Date(secs * 1000).toLocaleDateString(undefined, { day: 'numeric', month: 'short' })

/**
 * The hand checks for an amber finding, as a checklist. Ticking is a note for
 * the operator only: it's remembered in this browser, never sent anywhere,
 * and never turns the finding green, because Firewatch still can't confirm it.
 * `scope` names the node, its version and the advisory.
 */
export function ManualChecklist({
  id,
  scope,
  checks,
}: {
  id: string
  scope: string
  checks: string[]
}) {
  const [state, setState] = useState(() => ({ scope, ticks: loadAll()[scope] ?? {} }))
  // The node was upgraded while open: the old ticks don't carry over.
  if (state.scope !== scope) setState({ scope, ticks: loadAll()[scope] ?? {} })
  const ticks = state.ticks

  const toggle = (check: string, on: boolean) => {
    const next = { ...ticks }
    if (on) next[check] = nowSecs()
    else delete next[check]
    setState({ scope, ticks: next })
    saveTicks(scope, next)
  }
  const allDone = checks.length > 0 && checks.every((c) => c in ticks)
  const last = allDone ? Math.max(...checks.map((c) => ticks[c])) : 0

  return (
    <>
      <ul className="space-y-2">
        {checks.map((check) => (
          <li key={check}>
            <label className="flex cursor-pointer gap-3 text-[0.98rem] leading-relaxed">
              <input
                type="checkbox"
                checked={check in ticks}
                onChange={(e) => toggle(check, e.target.checked)}
                aria-describedby={`${id}-why`}
                className="mt-1 size-[1.1rem] shrink-0 accent-amber"
              />
              <span>
                <span className={check in ticks ? 'text-ink-3 line-through decoration-amber' : ''}>
                  <WithCode text={check} />
                </span>
                {check in ticks && (
                  <span className="ml-2 text-[0.8rem] text-ink-3">ticked {day(ticks[check])}</span>
                )}
              </span>
            </label>
          </li>
        ))}
      </ul>
      {allDone && (
        <p className="mt-2 text-[0.9rem] text-ink-3">
          Checked by hand on {day(last)}. This stays amber here, because Firewatch can’t confirm it
          itself.
        </p>
      )}
    </>
  )
}
