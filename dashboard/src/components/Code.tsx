// Release notes use `backticks` for code. Render those spans as code, and
// leave every other character exactly as written: quotes are never reworded.
export function WithCode({ text }: { text: string }) {
  const parts = text.split('`')
  if (parts.length < 3) return <>{text}</>
  return (
    <>
      {parts.map((part, i) =>
        i % 2 === 1 ? (
          <code key={i} className="rounded bg-fg/10 px-1 font-mono text-[0.88em]">
            {part}
          </code>
        ) : (
          part
        ),
      )}
    </>
  )
}
