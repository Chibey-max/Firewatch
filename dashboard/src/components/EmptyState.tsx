export function EmptyState() {
  return (
    <div className="mx-auto mt-16 max-w-md text-center">
      <p className="font-display text-[2rem] font-bold">No nodes to watch</p>
      <p className="mt-2 text-[1.02rem] leading-relaxed text-ink-2">
        Add a node to <code className="font-mono text-ink">firewatch.toml</code>, then restart{' '}
        <code className="font-mono text-ink">firewatch watch</code>.
      </p>
    </div>
  )
}
