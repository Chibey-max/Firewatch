import { useState } from 'react'

type Theme = 'dark' | 'light'

function current(): Theme {
  return document.documentElement.dataset.theme === 'light' ? 'light' : 'dark'
}

/** Dark by default; light is for projectors that wash dark screens out. Remembered per browser. */
export function ThemeToggle() {
  const [theme, setTheme] = useState<Theme>(current)
  const next: Theme = theme === 'dark' ? 'light' : 'dark'

  const toggle = () => {
    if (next === 'light') document.documentElement.dataset.theme = 'light'
    else delete document.documentElement.dataset.theme
    try {
      localStorage.setItem('fw-theme', next)
    } catch {
      // storage blocked: the theme still applies for this visit
    }
    setTheme(next)
  }

  return (
    <button
      type="button"
      onClick={toggle}
      aria-label={`Switch to ${next} theme`}
      title={`Switch to ${next} theme`}
      className="grid size-9 place-items-center rounded-md border border-line text-fg-2 hover:bg-hover hover:text-fg"
    >
      {theme === 'dark' ? (
        <svg viewBox="0 0 20 20" width="16" height="16" aria-hidden>
          <circle cx="10" cy="10" r="4" fill="currentColor" />
          <g stroke="currentColor" strokeWidth="1.6" strokeLinecap="round">
            <path d="M10 1.5v2M10 16.5v2M1.5 10h2M16.5 10h2M4 4l1.4 1.4M14.6 14.6 16 16M4 16l1.4-1.4M14.6 5.4 16 4" />
          </g>
        </svg>
      ) : (
        <svg viewBox="0 0 20 20" width="16" height="16" aria-hidden>
          <path d="M16.5 12.5A7 7 0 0 1 7.5 3.5a7 7 0 1 0 9 9Z" fill="currentColor" />
        </svg>
      )}
    </button>
  )
}
