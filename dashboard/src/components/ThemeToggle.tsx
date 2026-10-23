import { AnimatePresence, motion } from 'motion/react'
import { useState } from 'react'
import { EASE } from '../lib/motion'

type Theme = 'light' | 'dark'

function current(): Theme {
  return document.documentElement.dataset.theme === 'dark' ? 'dark' : 'light'
}

/** Light ("lookout stone") by default; dark for a dim room. Remembered per browser. */
export function ThemeToggle() {
  const [theme, setTheme] = useState<Theme>(current)
  const next: Theme = theme === 'light' ? 'dark' : 'light'

  const toggle = () => {
    if (next === 'dark') document.documentElement.dataset.theme = 'dark'
    else delete document.documentElement.dataset.theme
    try {
      localStorage.setItem('fw-theme', next)
    } catch {
      // storage blocked: the theme still applies for this visit
    }
    setTheme(next)
  }

  return (
    <motion.button
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
