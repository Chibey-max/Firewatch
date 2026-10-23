import { AnimatePresence, motion } from 'motion/react'
import { useRef, useState } from 'react'
import { flushSync } from 'react-dom'
import { EASE } from '../lib/motion'

type Theme = 'light' | 'dark'

function current(): Theme {
  return document.documentElement.dataset.theme === 'dark' ? 'dark' : 'light'
}

const REVEAL_MS = 1100

/**
 * Light by day, dark for a dim room; remembered per browser. Switching opens
 * the new theme from the toggle in an expanding circle (View Transitions):
 * the new landscape, glass and colours arrive together. Without View
 * Transitions, or with reduced motion, the photos crossfade and every colour
 * eases on its own (see the registered colours in index.css).
 */
export function ThemeToggle() {
  const [theme, setTheme] = useState<Theme>(current)
  const button = useRef<HTMLButtonElement>(null)
  const next: Theme = theme === 'light' ? 'dark' : 'light'

  const apply = () => {
    const root = document.documentElement
    if (next === 'dark') root.setAttribute('data-theme', 'dark')
    else root.removeAttribute('data-theme')
    try {
      localStorage.setItem('fw-theme', next)
    } catch {
      // storage blocked: the theme still applies for this visit
    }
    flushSync(() => setTheme(next))
  }

  const toggle = () => {
    const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches
    if (!document.startViewTransition || reduce || !button.current) {
      apply()
      return
    }
    const rect = button.current.getBoundingClientRect()
    const x = rect.left + rect.width / 2
    const y = rect.top + rect.height / 2
    const radius = Math.hypot(Math.max(x, innerWidth - x), Math.max(y, innerHeight - y))
    const root = document.documentElement
    // Nothing else transitions while the snapshot is taken, so the new
    // snapshot is the finished theme.
    root.classList.add('fw-theme-switching')
    const transition = document.startViewTransition(apply)
    transition.ready
      .then(() =>
        root.animate(
          {
            clipPath: [`circle(0px at ${x}px ${y}px)`, `circle(${radius}px at ${x}px ${y}px)`],
          },
          {
            duration: REVEAL_MS,
            easing: 'cubic-bezier(0.65, 0, 0.35, 1)',
            pseudoElement: '::view-transition-new(root)',
          },
        ),
      )
      .catch(() => {})
    transition.finished.finally(() => root.classList.remove('fw-theme-switching'))
  }

  return (
    <motion.button
      ref={button}
      type="button"
      onClick={toggle}
      whileTap={{ scale: 0.92 }}
      aria-label={`Switch to ${next} theme`}
      title={`Switch to ${next} theme`}
      className="fw-glass grid size-10 place-items-center overflow-hidden rounded-full text-ink-2 hover:text-ink"
    >
      <AnimatePresence mode="popLayout" initial={false}>
        <motion.svg
          key={theme}
          viewBox="0 0 20 20"
          width="16"
          height="16"
          aria-hidden
          initial={{ rotate: -90, opacity: 0, scale: 0.6 }}
          animate={{ rotate: 0, opacity: 1, scale: 1 }}
          exit={{ rotate: 90, opacity: 0, scale: 0.6 }}
          transition={{ duration: 0.4, ease: EASE }}
        >
          {theme === 'light' ? (
            <path d="M16.5 12.5A7 7 0 0 1 7.5 3.5a7 7 0 1 0 9 9Z" fill="currentColor" />
          ) : (
            <>
              <circle cx="10" cy="10" r="4" fill="currentColor" />
              <g stroke="currentColor" strokeWidth="1.6" strokeLinecap="round">
                <path d="M10 1.5v2M10 16.5v2M1.5 10h2M16.5 10h2M4 4l1.4 1.4M14.6 14.6 16 16M4 16l1.4-1.4M14.6 5.4 16 4" />
              </g>
            </>
          )}
        </motion.svg>
      </AnimatePresence>
    </motion.button>
  )
}
