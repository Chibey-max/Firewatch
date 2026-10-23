import { motion } from 'motion/react'
import { useState } from 'react'
import { EASE } from '../lib/motion'

/** The type sets in index.css, with what each one is known for. */
const SETS = [
  { id: 'satoshi', label: 'Satoshi', note: 'crypto and Bitcoin startup sites' },
  { id: 'space', label: 'Space Grotesk', note: 'with Space Mono: the crypto staple' },
  { id: 'unbounded', label: 'Unbounded', note: 'with Manrope: made for a crypto fund' },
  { id: 'inter', label: 'Inter', note: 'with IBM Plex Mono: the wallet look' },
  { id: 'sora', label: 'Sora', note: 'made for a blockchain project' },
  { id: 'editorial', label: 'Fraunces', note: 'the editorial serif from before' },
] as const

function current(): string {
  return document.documentElement.dataset.font ?? 'satoshi'
}

/**
 * A temporary picker for trying type sets on the real page. Shown with ?fonts;
 * the choice is remembered per browser.
 */
export function FontPicker() {
  const [font, setFont] = useState(current)
  const choose = (id: string) => {
    document.documentElement.setAttribute('data-font', id)
    try {
      localStorage.setItem('fw-font', id)
    } catch {
      // storage blocked: the set still applies for this visit
    }
    setFont(id)
  }
  const active = SETS.find((s) => s.id === font) ?? SETS[0]
  return (
    <div className="fw-glass fixed bottom-4 left-1/2 z-40 flex -translate-x-1/2 flex-col items-center gap-1.5 rounded-2xl px-2.5 py-2 shadow-raised">
      <div role="radiogroup" aria-label="Typeface" className="flex flex-wrap justify-center gap-1">
        {SETS.map((s) => (
          <button
            key={s.id}
            type="button"
            role="radio"
            aria-checked={s.id === font}
            onClick={() => choose(s.id)}
            className="relative rounded-full px-3 py-1.5 text-[0.82rem] font-semibold text-ink-2 hover:text-ink"
          >
            {s.id === font && (
              <motion.span
                layoutId="font-pick"
                transition={{ duration: 0.5, ease: EASE }}
                className="absolute inset-0 rounded-full bg-surface-solid shadow-card"
              />
            )}
            <span className="relative" style={{ fontFamily: `var(--fw-font-display)` }}>
              {s.label}
            </span>
          </button>
        ))}
      </div>
      <p className="text-[0.74rem] text-ink-3">
        Type: {active.label}, {active.note}
      </p>
    </div>
  )
}
