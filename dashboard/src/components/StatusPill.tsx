import { readingWord, type Reading } from '../lib/format'
import { tone } from '../lib/tone'

/** The verdict as a word with a dot: colour never carries it alone. */
export function StatusPill({ reading, large = false }: { reading: Reading; large?: boolean }) {
  const t = tone[reading]
  return (
    <span
      className={`inline-flex shrink-0 items-center gap-2 rounded-full font-semibold whitespace-nowrap ${t.bg} ${t.text} ${
        large ? 'px-3.5 py-1.5 text-[0.95rem]' : 'px-2.5 py-0.5 text-[0.8rem]'
      }`}
    >
      <span aria-hidden className={`rounded-full ${t.dot} ${large ? 'size-2.5' : 'size-2'}`} />
      {readingWord[reading]}
    </span>
  )
}
