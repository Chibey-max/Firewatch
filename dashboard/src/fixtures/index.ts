import type { Report } from '../types/report'
import demoAfter from './report.demo-after.json'
import demo from './report.demo.json'
import empty from './report.empty.json'
import error from './report.error.json'
import green from './report.green.json'
import mixed from './report.mixed.json'

// Generated from the real CLI; regenerate with the commands in dashboard/README.md.
// Open the app with ?fixture=<name> to show one instead of the live API.
export const fixtures = {
  // The docs/DEMO.md network, before and after old-cln's upgrade.
  demo: demo as Report,
  'demo-after': demoAfter as Report,
  mixed: mixed as Report,
  green: green as Report,
  error: error as Report,
  empty: empty as Report,
}

export type FixtureName = keyof typeof fixtures

export function isFixtureName(name: string | null): name is FixtureName {
  return name !== null && name in fixtures
}
