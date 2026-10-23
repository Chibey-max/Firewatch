// Records the backup demo video: the dashboard during the upgrade beat, from
// the real daemon against dashboard/dev/sim.sh's simulated nodes. A caption bar
// says so on every frame, so the video can't pass for the Polar demo.
//
// Needs: dashboard/dev/sim.sh and `npm run dev` running, Google Chrome, ffmpeg.
//   npm run record:backup                  # -> firewatch-backup-demo.mp4
//   npm run record:backup -- out/demo.mp4
import { execFileSync } from 'node:child_process'
import { rmSync } from 'node:fs'
import { dirname, resolve } from 'node:path'
import { fileURLToPath } from 'node:url'
import puppeteer from 'puppeteer-core'

const here = dirname(fileURLToPath(import.meta.url))
const out = resolve(process.argv[2] ?? 'firewatch-backup-demo.mp4')
const webm = out.replace(/\.mp4$/, '') + '.webm'
const chrome = process.env.CHROME ?? '/usr/bin/google-chrome'
const sleep = (ms) => new Promise((r) => setTimeout(r, ms))
const sim = (cmd) => execFileSync(`${here}/sim.sh`, [cmd], { stdio: 'inherit' })

for (const [name, url] of [
  ['dashboard dev server (npm run dev)', 'http://127.0.0.1:5173/'],
  ['firewatch watch (dashboard/dev/sim.sh)', 'http://127.0.0.1:7070/api/health'],
]) {
  await fetch(url).catch(() => {
    console.error(`Not running: ${name}`)
    process.exit(1)
  })
}

sim('reset')
await sleep(6000) // one 5 s check, so the daemon reports v26.06.7 again

const browser = await puppeteer.launch({
  executablePath: chrome,
  headless: true,
  defaultViewport: { width: 1920, height: 1080 },
  args: ['--hide-scrollbars', '--window-size=1920,1080'],
})
const page = await browser.newPage()
await page.goto('http://127.0.0.1:5173/#node=old-cln', { waitUntil: 'networkidle0' })

// The caption bar is the recorder's, not part of the app.
await page.evaluate(() => {
  const bar = document.createElement('div')
  Object.assign(bar.style, {
    position: 'fixed',
    left: '0',
    right: '0',
    bottom: '0',
    zIndex: '9999',
    background: 'rgba(20,36,32,.94)',
    color: '#fafbf8',
    padding: '22px 48px',
    font: '600 30px "Public Sans", sans-serif',
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    gap: '32px',
  })
  bar.innerHTML =
    '<span id="rec-text"></span><span style="font:400 20px \'JetBrains Mono\',monospace;opacity:.75;white-space:nowrap">Recorded with simulated Core Lightning nodes · the live demo uses Polar</span>'
  document.body.appendChild(bar)
})
const caption = (text) =>
  page.evaluate((t) => (document.getElementById('rec-text').textContent = t), text)

const recorder = await page.screencast({ path: webm, fps: 30 })
await caption('Firewatch checks each node against the developers’ own advisories.')
await sleep(5000)
await caption(
  'old-cln runs Core Lightning 26.06.7: affected, and one advisory is exploited in the wild.',
)
await sleep(5000)
await caption('Every red finding quotes the developers’ fix and links their release notes.')
await sleep(5000)
await page.click('::-p-text(stopped-cln)')
await caption('stopped-cln can’t be read, so it stays amber. Firewatch never guesses green.')
await sleep(5000)
await page.click('::-p-text(old-cln)')
await caption(
  'The operator upgrades old-cln to 26.06.9, by hand. Firewatch never touches the node.',
)
await sleep(3000)
sim('upgrade')
await sleep(1500)
await caption('Re-check now: old-cln turns green, and says why.')
await page.click('::-p-text(Re-check now)')
await sleep(6000)
await caption('Fixes ship in days. Firewatch makes sure you hear in minutes.')
await sleep(4500)
await recorder.stop()
await browser.close()
sim('reset')

execFileSync(
  'ffmpeg',
  [
    '-v',
    'error',
    '-y',
    '-i',
    webm,
    '-c:v',
    'libx264',
    '-pix_fmt',
    'yuv420p',
    '-crf',
    '20',
    '-movflags',
    '+faststart',
    '-r',
    '30',
    out,
  ],
  { stdio: 'inherit' },
)
rmSync(webm)
console.log(`Saved ${out}`)
