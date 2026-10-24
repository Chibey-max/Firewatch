/** The lookout dial, kept as the mark: green, amber, red, needle on red. */
export function Logo() {
  return (
    <svg viewBox="0 0 32 22" width="30" height="21" aria-hidden className="shrink-0">
      <path d="M4 19 A12 12 0 0 1 9.2 9.3" fill="none" stroke="var(--fw-green)" strokeWidth="4" />
      <path
        d="M10.7 8.2 A12 12 0 0 1 21.3 8.2"
        fill="none"
        stroke="var(--fw-amber)"
        strokeWidth="4"
      />
      <path d="M22.8 9.3 A12 12 0 0 1 28 19" fill="none" stroke="var(--fw-red)" strokeWidth="4" />
      <line
        x1="16"
        y1="19"
        x2="23"
        y2="12.5"
        stroke="var(--fw-ink)"
        strokeWidth="2.4"
        strokeLinecap="round"
      />
      <circle cx="16" cy="19" r="2.4" fill="var(--fw-ink)" />
    </svg>
  )
}
