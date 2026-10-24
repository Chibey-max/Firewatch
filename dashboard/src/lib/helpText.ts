import type { Finding, NodeReport } from '../types/report'
import {
  implementationName,
  readingOf,
  readingWord,
  ruledOutByCondition,
  severityName,
  splitVersion,
} from './format'

/**
 * Takes out what would point at the operator's machine: URLs and IP
 * addresses (with their ports). Advisory links are added separately and kept.
 */
export function scrub(text: string): string {
  return text
    .replace(/\b[a-z][a-z0-9+.-]*:\/\/[^\s;,)]+/gi, '<node url>')
    .replace(/\[[0-9a-f:]+\](:\d+)?|\b\d{1,3}(\.\d{1,3}){3}(:\d+)?\b/gi, '<address>')
}

function stamp(at: number): string {
  return new Date(at * 1000).toISOString().slice(0, 16).replace('T', ' ') + ' UTC'
}

function head(f: Finding): string {
  const tags = [severityName[f.severity].toLowerCase()]
  if (f.exploited_in_wild) tags.push('exploited in the wild')
  if (!f.verified) tags.push('not yet verified')
  return `- ${f.advisory_id} (${tags.join(', ')}): ${f.title}`
}

const fixLine = (f: Finding) =>
  `  ${f.fixed ? `Fixed in ${f.fixed}` : 'No fix released yet'} · ${f.source}`

/**
 * A plain-text summary of one node to paste where you ask for help: what it
 * runs, its verdict, and each advisory with the developers' own words and
 * link. It leaves out the node's name, URLs, addresses, credentials and
 * where its APIs listen.
 */
export function helpText(node: NodeReport, checkedAt: number): string {
  const lines: string[] = []
  const version = node.version ? splitVersion(node.version).version : 'version unknown'
  lines.push(`Firewatch check, ${stamp(checkedAt)} (read-only)`)
  lines.push(
    `${implementationName[node.implementation]} ${version}${node.network ? ` on ${node.network}` : ''}: ${readingWord[readingOf(node)]}`,
  )

  if (node.error) lines.push(`Couldn't read the node: ${scrub(node.error)}`)
  const build = node.build_check
  if (build?.status === 'mismatch') {
    lines.push(
      build.actual === build.expected
        ? `Build: the ${build.release} commit, built with local changes (${build.build ?? 'dirty'})`
        : `Build: not the official ${build.release} (commit ${build.actual.slice(0, 7)}, official ${build.expected.slice(0, 7)})`,
    )
  } else if (build?.status === 'unreleased') {
    lines.push(`Build: ${build.build}, a commit after ${build.release}, not the release`)
  } else if (build?.status === 'modified') {
    lines.push(`Build: ${build.release} with local changes (${build.build})`)
  } else if (build?.status === 'match') {
    lines.push('Build: the official release')
  }

  const affected = node.findings.filter((f) => f.status === 'affected')
  const toCheck = node.findings.filter((f) => f.status === 'cant_tell')
  const ruledOut = node.findings.filter(ruledOutByCondition)
  const clear = node.findings.filter((f) => f.status === 'not_affected' && !ruledOutByCondition(f))

  if (affected.length) {
    lines.push('', 'Affected')
    for (const f of affected) {
      lines.push(head(f), `  The developers: "${f.steps}"`, fixLine(f))
    }
  }
  if (toCheck.length) {
    lines.push('', 'Check by hand')
    for (const f of toCheck) {
      lines.push(head(f), `  ${scrub(f.reason)}`)
      for (const c of f.manual_checks) lines.push(`  To check: ${scrub(c)}`)
      lines.push(fixLine(f))
    }
  }
  if (ruledOut.length) {
    lines.push('', 'Ruled out by a condition')
    for (const f of ruledOut) lines.push(head(f), `  ${scrub(f.reason)}`)
  }
  if (clear.length) {
    lines.push('', `Don't apply to this version: ${clear.map((f) => f.advisory_id).join(', ')}`)
  }

  lines.push(
    '',
    "Copied from Firewatch. It leaves out the node's name, URLs, addresses and credentials.",
  )
  return lines.join('\n')
}
