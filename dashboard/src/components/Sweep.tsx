/**
 * A specular sweep across the glass. Keyed by the report's time by its parent,
 * so it plays once each time a fresh report lands.
 */
export function Sweep() {
  return (
    <span
      aria-hidden
      className="pointer-events-none absolute inset-0 overflow-hidden rounded-[inherit]"
    >
      <span className="fw-sweep" />
    </span>
  )
}
