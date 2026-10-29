import { AnimatePresence, motion } from 'motion/react'
import { useState, type ReactNode } from 'react'
import { copyText } from '../lib/clipboard'
import { EASE, SOFT } from '../lib/motion'
import type { GuideTab } from '../lib/useGuide'
import { StatusPill } from './StatusPill'

// Everything the guide tells people to run, in one place. It mirrors the
// README's "Getting started": when a release or a command changes there,
// change it here too.
const RELEASE = 'v0.1.1'
const CMD = {
  install: `curl --proto '=https' --tlsv1.2 -LsSf https://github.com/Chibey-max/Firewatch/releases/download/${RELEASE}/firewatch-installer.sh | sh`,
  cargo: `cargo install firewatch --version ${RELEASE.slice(1)} --locked`,
  version: 'firewatch --version',
  clnPort: 'clnrest-port=3010',
  init: 'firewatch init',
  rune: `lightning-cli createrune restrictions='[["method=getinfo","method=listconfigs"],["method/listconfigs","pnameconfig=clnrest-host","pnameconfig=clnrest-port","pnameconfig=grpc-host","pnameconfig=grpc-port"]]'`,
  check: 'firewatch check',
  testAlert: 'firewatch test-alert',
  watch: 'firewatch watch',
  service: `mkdir -p ~/.config/systemd/user
cat > ~/.config/systemd/user/firewatch.service <<'UNIT'
[Unit]
Description=Firewatch: security alerts for Lightning nodes
After=network-online.target

[Service]
ExecStart=%h/.local/bin/firewatch watch
Restart=on-failure

[Install]
WantedBy=default.target
UNIT
systemctl --user enable --now firewatch
loginctl enable-linger "$USER"`,
  tunnel: 'ssh -L 7070:127.0.0.1:7070 you@your-node',
}

/** A command to copy: monospace, scrolls sideways when long, one tap to copy. */
function Cmd({ children }: { children: string }) {
  const [done, setDone] = useState(false)
  const copy = async () => {
    if (await copyText(children)) {
      setDone(true)
      setTimeout(() => setDone(false), 1400)
    }
  }
  return (
    <div className="mt-2.5 flex items-start rounded-xl border border-line bg-ink/5">
      <pre className="min-w-0 flex-1 overflow-x-auto py-3 pl-4 font-mono text-[0.8rem] leading-relaxed text-ink">
        <code>{children}</code>
      </pre>
      <button
        type="button"
        onClick={copy}
        aria-label={done ? 'Copied' : 'Copy command'}
        title={done ? 'Copied' : 'Copy'}
        className="m-1.5 grid size-8 shrink-0 place-items-center rounded-full border border-line bg-surface-solid text-ink-2 hover:text-ink"
      >
        <svg
          viewBox="0 0 16 16"
          className="size-3.5"
          fill="none"
          stroke="currentColor"
          strokeWidth="1.6"
          strokeLinecap="round"
          strokeLinejoin="round"
          aria-hidden
        >
          {done ? (
            <path d="M3 8.5 6.5 12 13 4.5" />
          ) : (
            <>
              <rect x="5.5" y="5.5" width="8" height="8" rx="1.6" />
              <path d="M10.5 3.2A1.6 1.6 0 0 0 9 2.5H4.1a1.6 1.6 0 0 0-1.6 1.6V9a1.6 1.6 0 0 0 .7 1.3" />
            </>
          )}
        </svg>
      </button>
    </div>
  )
}

const c = 'rounded bg-ink/8 px-1 font-mono text-[0.88em] text-ink'

function Step({ n, title, children }: { n: number; title: string; children: ReactNode }) {
  return (
    <li className="grid grid-cols-[auto_1fr] gap-x-3.5">
      <span className="fw-glass mt-0.5 grid size-7 place-items-center rounded-full font-mono text-[0.78rem] font-semibold text-ink-2">
        {n}
      </span>
      <div className="min-w-0 pb-6">
        <h4 className="text-[1.02rem] font-semibold">{title}</h4>
        <div className="mt-1 text-[0.92rem] leading-relaxed text-ink-2">{children}</div>
      </div>
    </li>
  )
}

function Note({ children }: { children: ReactNode }) {
  return (
    <div className="mt-1 mb-6 rounded-xl border border-line bg-ink/5 px-4 py-3 text-[0.88rem] leading-relaxed text-ink-2">
      {children}
    </div>
  )
}

function SetUp() {
  return (
    <>
      <p className="text-[0.95rem] leading-relaxed text-ink-2">
        About ten minutes, on Linux or macOS (a Raspberry Pi works). Firewatch only ever reads your
        node: nothing here gives it a credential that can spend or change settings. On Umbrel or
        Start9, SSH into the box first and run everything there.
      </p>
      <Note>
        <b className="text-ink">Core Lightning:</b> its REST API is off until it has a port. Add{' '}
        <code className={c}>{CMD.clnPort}</code> to the node's config (usually{' '}
        <code className={c}>~/.lightning/config</code>) and restart it. LND's is on already.
      </Note>
      <ol>
        <Step n={1} title="Install">
          One command, no Rust or Node.js needed. It puts a single{' '}
          <code className={c}>firewatch</code> binary in <code className={c}>~/.local/bin</code>,
          with the advisories and this dashboard built in.
          <Cmd>{CMD.install}</Cmd>
          <p className="mt-2.5">
            If the shell says <code className={c}>command not found</code>, open a new terminal.
            With Rust installed you can build it instead:
          </p>
          <Cmd>{CMD.cargo}</Cmd>
        </Step>
        <Step n={2} title="Connect it to your node">
          <code className={c}>init</code> finds LND in <code className={c}>~/.lnd</code> (or
          Umbrel's folder), Core Lightning in <code className={c}>~/.lightning</code>, and Polar
          networks, tests each connection, and asks for your npub for phone alerts (see the Phone
          alerts tab; press Enter to skip).
          <Cmd>{CMD.init}</Cmd>
          <p className="mt-2.5">
            <b className="text-ink">LND</b> needs nothing else: it uses the{' '}
            <code className={c}>readonly.macaroon</code> LND already has.{' '}
            <b className="text-ink">Core Lightning</b> needs a rune that can only read. Making one
            changes the node, which Firewatch never does, so run this yourself and paste the{' '}
            <code className={c}>rune</code> it prints:
          </p>
          <Cmd>{CMD.rune}</Cmd>
        </Step>
        <Step n={3} title="Check your nodes">
          Each node gets <b className="text-ink">Affected</b>,{' '}
          <b className="text-ink">Can’t tell</b> (with what to check by hand) or{' '}
          <b className="text-ink">Not affected</b> (with the reason).
          <Cmd>{CMD.check}</Cmd>
        </Step>
        <Step n={4} title="Keep watching, and open this dashboard">
          It checks every 30 seconds and sends one alert per node when something turns red or amber,
          and another when it's fixed. The dashboard is at{' '}
          <code className={c}>http://127.0.0.1:7070</code>.<Cmd>{CMD.watch}</Cmd>
          <p className="mt-2.5">
            Alerts only come while <code className={c}>watch</code> runs. On Linux, run it as a
            service so it survives logouts and reboots:
          </p>
          <Cmd>{CMD.service}</Cmd>
          <p className="mt-2.5">
            The dashboard only answers on the node's machine, because it says which nodes are
            vulnerable. To see it from your laptop, tunnel the port, then open the same address:
          </p>
          <Cmd>{CMD.tunnel}</Cmd>
        </Step>
        <Step n={5} title="Update for new advisories">
          Advisories are built into each release, so run the install command again to get new ones.
          Your config stays.
        </Step>
      </ol>
      <h4 className="mb-2 text-[1.02rem] font-semibold">If something's wrong</h4>
      <ul className="space-y-2 text-[0.9rem] leading-relaxed text-ink-2">
        <li>
          <b className="text-ink">Can't connect … is the node running?</b> Start the node; for Core
          Lightning, check <code className={c}>clnrest-port</code> is set and matches the config.
        </li>
        <li>
          <b className="text-ink">TLS certificate not trusted:</b> point{' '}
          <code className={c}>tls_cert</code> at the node's own certificate (LND's{' '}
          <code className={c}>tls.cert</code>, or Core Lightning's{' '}
          <code className={c}>server.pem</code>).
        </li>
        <li>
          <b className="text-ink">Credential rejected</b> or{' '}
          <b className="text-ink">not read-only:</b> make a new read-only rune or use LND's{' '}
          <code className={c}>readonly.macaroon</code>, then run{' '}
          <code className={c}>firewatch init --force</code>.
        </li>
      </ul>
    </>
  )
}

type Phone = 'ios' | 'android'

function Alerts() {
  const [phone, setPhone] = useState<Phone>('ios')
  const app = phone === 'ios' ? 'Nostur' : 'Amethyst'
  return (
    <>
      <p className="text-[0.95rem] leading-relaxed text-ink-2">
        Alerts arrive as encrypted Nostr direct messages (NIP-17). Nostr has no accounts or
        passwords: your phone app makes a pair of keys for you, and only your phone can read the
        alerts.
      </p>

      <div
        role="radiogroup"
        aria-label="Your phone"
        className="fw-glass mt-5 inline-flex rounded-full p-1"
      >
        {(
          [
            ['ios', 'iPhone'],
            ['android', 'Android'],
          ] as const
        ).map(([key, label]) => (
          <button
            key={key}
            type="button"
            role="radio"
            aria-checked={phone === key}
            onClick={() => setPhone(key)}
            className={`rounded-full px-4 py-1.5 text-[0.9rem] font-semibold transition-colors duration-300 ${
              phone === key
                ? 'bg-surface-solid text-ink shadow-raised'
                : 'text-ink-2 hover:text-ink'
            }`}
          >
            {label}
          </button>
        ))}
      </div>

      <AnimatePresence mode="wait" initial={false}>
        <motion.div
          key={phone}
          initial={{ opacity: 0 }}
          animate={{ opacity: 1, transition: { duration: 0.3, ease: SOFT } }}
          exit={{ opacity: 0, transition: { duration: 0.15 } }}
          className="mt-5"
        >
          <Note>
            {phone === 'ios' ? (
              <>
                Use <b className="text-ink">Nostur</b> from the App Store: it's tested with
                Firewatch.
              </>
            ) : (
              <>
                Use <b className="text-ink">Amethyst</b> from the Play Store: it's tested with
                Firewatch.
              </>
            )}{' '}
            Not Damus: it doesn't show these messages. The right app talks about Nostr and never
            asks for a phone number or password.
          </Note>
          <ol>
            <Step n={1} title={`Install ${app} and create an account`}>
              Choose <i>create a new account</i>. It makes your keys on the phone. An account just
              for alerts keeps them apart from anything social.
            </Step>
            <Step n={2} title="Back up your private key">
              Your <code className={c}>nsec1…</code> key opens your messages. Save it in a password
              manager, and <b className="text-ink">never share it</b>: not with Firewatch, a
              teammate or a website.
            </Step>
            <Step n={3} title="Copy your public key">
              {phone === 'ios' ? (
                <>
                  Open your profile in Nostur and copy your <code className={c}>npub1…</code>.
                </>
              ) : (
                <>
                  Open your profile in Amethyst, tap <b className="text-ink">share</b> and copy the
                  link (<code className={c}>https://njump.to/nprofile1…</code>). It names the relays
                  where Amethyst keeps the list Firewatch delivers to.
                </>
              )}{' '}
              It's your address, and it's safe to share.
            </Step>
            <Step n={4} title="Give it to Firewatch">
              Paste it when <code className={c}>firewatch init</code> asks. To change it later, edit{' '}
              <code className={c}>recipient</code> in{' '}
              <code className={c}>~/.config/firewatch/firewatch.toml</code>.
            </Step>
            <Step n={5} title="Send a test alert">
              <Cmd>{CMD.testAlert}</Cmd>
            </Step>
            <Step n={6} title="Accept the first message">
              It lands in {app}'s <b className="text-ink">message requests</b>, because the app
              doesn't know the sender yet. Tap <i>Accept message request</i>, follow the sender
              (Firewatch's own key; <code className={c}>test-alert</code> prints it) and rename it
              “Firewatch”. Later alerts go straight to your inbox.
            </Step>
          </ol>
          <Note>
            <b className="text-ink">Nothing arrived?</b> Look in message requests, and check this
            machine has internet: <code className={c}>test-alert</code> prints which relays took the
            message. If it did, add <code className={c}>wss://relay.damus.io</code> and{' '}
            <code className={c}>wss://nos.lol</code> in {app}'s relay settings and reopen your
            messages.
          </Note>
        </motion.div>
      </AnimatePresence>
    </>
  )
}

function Feature({ title, children }: { title: ReactNode; children: ReactNode }) {
  return (
    <li className="border-t border-line py-4 first:border-t-0 first:pt-0">
      <h4 className="flex flex-wrap items-center gap-2 text-[1rem] font-semibold">{title}</h4>
      <p className="mt-1 text-[0.92rem] leading-relaxed text-ink-2">{children}</p>
    </li>
  )
}

function Dashboard() {
  return (
    <ul>
      <Feature title="Three answers, never a guess">
        <span className="mb-2 flex flex-wrap gap-2">
          <StatusPill reading="affected" />
          <StatusPill reading="cant_tell" />
          <StatusPill reading="not_affected" />
        </span>
        <b className="text-ink">Affected:</b> the version is in range and every condition is
        confirmed; the developers' fix is quoted with their link.{' '}
        <b className="text-ink">Can't tell:</b> something can't be read, so it lists what to check
        by hand. <b className="text-ink">Not affected:</b> with the reason. A node no advisory
        covers reads <i>No known advisories</i>, which isn't proof it's safe.
      </Feature>
      <Feature title="Re-check now">
        Checks every node straight away instead of waiting for the next round. The status beside the
        logo says when the last check ran; the logo's needle points at the worst verdict.
      </Feature>
      <Feature title="Why amber?">
        On a node that can't be told, says why: an unreadable node, an unofficial or changed build,
        or something only you can check.
      </Feature>
      <Feature title="Since you last looked">
        Opens with what changed since you last pressed <i>Got it</i>: a verdict that moved, an
        advisory that newly applies, a new node. Changed nodes are tagged until then.
      </Feature>
      <Feature title="Copy for help">
        Copies a node's verdict and advisories as plain text, with the developers' links, to paste
        where you ask for help. It leaves out the node's name, addresses and credentials.
      </Feature>
      <Feature title="Hand checks">
        Tick the checks you've done; they're remembered in this browser with the day, and start
        again when the node's version changes. They never turn a finding green.
      </Feature>
      <Feature title="API exposed">
        A separate note when the node's API can be reached from other machines, where a leaked
        macaroon or rune would work. It never changes the verdict.
      </Feature>
      <Feature title="Links and sample data">
        Each node has its own link (<code className={c}>#node=old-cln</code>).{' '}
        <code className={c}>?fixture=demo</code> shows sample data without a node, and{' '}
        <code className={c}>?theme=dark</code> opens the dark theme. On a phone, tap a node to open
        it; Back, the backdrop or a swipe down closes it.
      </Feature>
    </ul>
  )
}

const tabs: { key: GuideTab; label: string }[] = [
  { key: 'setup', label: 'Set up' },
  { key: 'alerts', label: 'Phone alerts' },
  { key: 'dashboard', label: 'Using the dashboard' },
]

/** The how-to: running Firewatch, phone alerts, and what the dashboard does. */
export function Guide({ tab, onTab }: { tab: GuideTab; onTab: (tab: GuideTab) => void }) {
  return (
    <div className="px-5 pt-1 pb-10 sm:px-8">
      <p className="font-mono text-[0.72rem] tracking-[0.14em] text-ink-3 uppercase">Guide</p>
      <h2 id="guide-title" className="fw-display mt-1 text-[2rem] leading-tight">
        How to use Firewatch
      </h2>
      <div
        role="tablist"
        aria-label="Guide sections"
        className="fw-glass mt-5 flex w-full max-w-full overflow-x-auto rounded-full p-1"
      >
        {tabs.map((t) => (
          <button
            key={t.key}
            type="button"
            role="tab"
            id={`guide-tab-${t.key}`}
            aria-selected={tab === t.key}
            aria-controls="guide-panel"
            onClick={() => onTab(t.key)}
            className={`flex-1 rounded-full px-3.5 py-2 text-[0.9rem] font-semibold whitespace-nowrap transition-colors duration-300 ${
              tab === t.key
                ? 'bg-surface-solid text-ink shadow-raised'
                : 'text-ink-2 hover:text-ink'
            }`}
          >
            {t.label}
          </button>
        ))}
      </div>
      <AnimatePresence mode="wait" initial={false}>
        <motion.div
          key={tab}
          id="guide-panel"
          role="tabpanel"
          aria-labelledby={`guide-tab-${tab}`}
          className="mt-6"
          initial={{ opacity: 0, y: 6 }}
          animate={{ opacity: 1, y: 0, transition: { duration: 0.4, ease: EASE } }}
          exit={{ opacity: 0, transition: { duration: 0.15 } }}
        >
          {tab === 'setup' ? <SetUp /> : tab === 'alerts' ? <Alerts /> : <Dashboard />}
        </motion.div>
      </AnimatePresence>
    </div>
  )
}

/** The guide's button: a question mark in a glass circle, beside the theme toggle. */
export function GuideButton({ onOpen }: { onOpen: () => void }) {
  return (
    <motion.button
      type="button"
      onClick={onOpen}
      whileTap={{ scale: 0.92 }}
      aria-label="How to use Firewatch"
      title="How to use Firewatch"
      className="fw-glass grid size-10 place-items-center rounded-full text-ink-2 hover:text-ink"
    >
      <svg
        viewBox="0 0 20 20"
        className="size-[17px]"
        fill="none"
        stroke="currentColor"
        strokeWidth="1.7"
        strokeLinecap="round"
        aria-hidden
      >
        <path d="M7.3 7.4a2.8 2.8 0 1 1 3.9 2.6c-.8.4-1.2 1-1.2 1.8v.4" />
        <circle cx="10" cy="15.2" r=".9" fill="currentColor" stroke="none" />
      </svg>
    </motion.button>
  )
}
