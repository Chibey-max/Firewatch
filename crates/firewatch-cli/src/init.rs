//! `firewatch init`: find the operator's nodes, set up a read-only
//! connection to each, and write the config.
//!
//! LND needs nothing from the operator: its `readonly.macaroon` already
//! exists. Core Lightning needs one restricted rune, and creating a rune is a
//! change to the node, which Firewatch never makes, so `init` prints the
//! command and asks for the result.

use std::io::Write;
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use firewatch_connectors::cln::RUNE_RESTRICTIONS;
use firewatch_connectors::NodeConfig;

use crate::config;

const RELAYS: [&str; 3] = [
    "wss://relay.damus.io",
    "wss://nos.lol",
    "wss://relay.primal.net",
];

pub async fn run(path: Option<PathBuf>, force: bool) -> Result<()> {
    let path = path.unwrap_or_else(config::default_path);
    if path.exists() && !force {
        bail!(
            "{} already exists. Pass --force to replace it, or --config to write somewhere else",
            path.display()
        );
    }
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .context("HOME isn't set")?;
    let mut ask = Prompt::stdin();

    println!("Firewatch setup.");
    println!("Firewatch only reads your node. It never needs a credential that can spend or change settings.\n");

    let lnd_dir = std::env::var_os("LND_DIR").map(PathBuf::from);
    let found = detect(&home, lnd_dir, &mut ask);
    if found.is_empty() {
        println!("No node found in the usual places (~/.lnd, ~/.lightning, Umbrel, Polar).");
    } else {
        println!("Found {} node(s):", found.len());
        for c in &found {
            println!("  - {}", c.label);
        }
        println!();
    }

    let http = firewatch_connectors::http_client();
    let mut nodes: Vec<String> = Vec::new();
    let mut names: Vec<String> = Vec::new();
    for candidate in found {
        if !ask.yes(&format!("Watch {}?", candidate.label), true) {
            continue;
        }
        let Some(toml) = candidate.into_toml(&mut ask) else {
            println!("  Skipped.\n");
            continue;
        };
        if keep_after_test(&toml, &http, &mut ask).await {
            names.push(node_name(&toml));
            nodes.push(toml);
        }
    }
    add_by_hand(&mut ask, &http, &mut nodes, &mut names).await;
    if nodes.is_empty() {
        bail!("no nodes to watch, so no config was written");
    }

    let dir = path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let npub = loop {
        let answer = ask.line(
            "\nYour Nostr npub, for encrypted alerts on your phone (Enter to skip). \
             On iPhone use Nostur; on Android, Amethyst or 0xchat:\n> ",
        );
        if answer.is_empty() {
            break None;
        }
        match firewatch_alerts::check_recipient(&answer) {
            Ok(()) => break Some(answer),
            Err(e) => println!("  {e}. Try again, or press Enter to skip."),
        }
    };

    let text = render(&nodes, npub.as_deref(), &dir.join("firewatch.nsec"));
    std::fs::create_dir_all(&dir).with_context(|| format!("creating {}", dir.display()))?;
    write_private(&path, &text)?;

    println!("\nWrote {} ({} node(s)).\n", path.display(), nodes.len());
    println!("Next:");
    println!("  firewatch check        # check your nodes once");
    if npub.is_some() {
        println!("  firewatch test-alert   # make sure alerts reach your phone; accept the first message request");
    }
    println!("  firewatch watch        # keep checking, alert on changes, dashboard at http://127.0.0.1:7070");
    Ok(())
}

/// Asks for nodes by hand until the operator says no. Stops at end of input:
/// "yes" is the default while there are no nodes, so it would ask forever.
async fn add_by_hand(
    ask: &mut Prompt,
    http: &reqwest::Client,
    nodes: &mut Vec<String>,
    names: &mut Vec<String>,
) {
    while ask.yes("Add a node by hand?", nodes.is_empty()) && !ask.ended() {
        let Some(toml) = manual(ask, names) else {
            continue;
        };
        if keep_after_test(&toml, http, ask).await {
            names.push(node_name(&toml));
            nodes.push(toml);
        }
    }
}

/// Writes the config so only this user can read it: it holds runes.
/// `mode` only applies when the file is created, so a file that `--force`
/// replaces is set to 0600 too, before anything is written to it.
fn write_private(path: &Path, text: &str) -> Result<()> {
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .open(path)
        .with_context(|| format!("writing {}", path.display()))?;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))
        .with_context(|| format!("making {} private", path.display()))?;
    file.write_all(text.as_bytes())
        .with_context(|| format!("writing {}", path.display()))
}

/// A node found on this machine, before the operator has confirmed it.
struct Candidate {
    label: String,
    name: String,
    kind: Kind,
}

enum Kind {
    Lnd {
        url: String,
        macaroon: PathBuf,
        tls_cert: PathBuf,
        /// Whether the REST address is known (Polar) or a default to confirm.
        url_known: bool,
    },
    Cln {
        url: String,
        url_known: bool,
        tls_cert: Option<PathBuf>,
        /// How to create the restricted rune on this node.
        rune_command: String,
    },
}

impl Candidate {
    /// The node's `[[nodes]]` block, after asking for whatever detection
    /// couldn't know. `None` if the operator gave up on it.
    fn into_toml(self, ask: &mut Prompt) -> Option<String> {
        match self.kind {
            Kind::Lnd {
                url,
                macaroon,
                tls_cert,
                url_known,
            } => {
                let url = if url_known {
                    url
                } else {
                    ask.default("  LND REST address", &url)
                };
                Some(lnd_toml(&self.name, &url, &macaroon, Some(&tls_cert)))
            }
            Kind::Cln {
                url,
                url_known,
                tls_cert,
                rune_command,
            } => {
                let url = if url_known {
                    url
                } else {
                    ask.default("  clnrest address", &url)
                };
                println!("  Firewatch needs a rune that can only read. Create one by running:\n");
                println!("    {rune_command}\n");
                let rune = ask.secret(
                    "  Paste the \"rune\" value it prints (Enter to skip this node):\n  > ",
                );
                let rune = rune.trim_matches('"').to_string();
                (!rune.is_empty()).then(|| cln_toml(&self.name, &url, &rune, tls_cert.as_deref()))
            }
        }
    }
}

/// Everything Firewatch can find on this machine.
fn detect(home: &Path, lnd_dir: Option<PathBuf>, ask: &mut Prompt) -> Vec<Candidate> {
    let mut found = Vec::new();

    // LND in its usual data folders. The REST port is the default unless the
    // operator says otherwise.
    let lnd_dirs = [
        lnd_dir,
        Some(home.join(".lnd")),
        Some(home.join("umbrel/app-data/lightning/data/lnd")),
    ];
    let mut seen = Vec::new();
    for dir in lnd_dirs.into_iter().flatten() {
        // LND_DIR is often ~/.lnd itself, or a link to it.
        let real = dir.canonicalize().unwrap_or_else(|_| dir.clone());
        if seen.contains(&real) || !dir.join("tls.cert").is_file() {
            continue;
        }
        seen.push(real);
        for network in ["mainnet", "testnet", "testnet4", "signet", "regtest"] {
            let macaroon = dir.join(format!("data/chain/bitcoin/{network}/readonly.macaroon"));
            if macaroon.is_file() {
                found.push(Candidate {
                    label: format!("LND ({network}) in {}", dir.display()),
                    name: format!("lnd-{network}"),
                    kind: Kind::Lnd {
                        url: "https://127.0.0.1:8080".into(),
                        macaroon,
                        tls_cert: dir.join("tls.cert"),
                        url_known: false,
                    },
                });
            }
        }
    }

    // Core Lightning in ~/.lightning: clnrest has no default port, so ask.
    for network in ["bitcoin", "testnet", "testnet4", "signet", "regtest"] {
        let dir = home.join(".lightning").join(network);
        if !dir.join("lightning-rpc").exists() {
            continue;
        }
        let server_pem = dir.join("server.pem");
        let https = server_pem.is_file();
        found.push(Candidate {
            label: format!("Core Lightning ({network}) in {}", dir.display()),
            name: format!("cln-{network}"),
            kind: Kind::Cln {
                url: format!("{}://127.0.0.1:3010", if https { "https" } else { "http" }),
                url_known: false,
                tls_cert: https.then_some(server_pem),
                rune_command: format!(
                    "lightning-cli --network={network} createrune restrictions='{RUNE_RESTRICTIONS}'"
                ),
            },
        });
    }

    found.extend(polar(home, ask));
    unique_names(&mut found);
    found
}

/// Gives each candidate its own name: two LND folders on one network would
/// both be `lnd-mainnet`, and `watch` keeps each node's state by name.
fn unique_names(found: &mut [Candidate]) {
    let mut used: Vec<String> = Vec::new();
    for c in found.iter_mut() {
        let base = c.name.clone();
        let mut n = 2;
        while used.contains(&c.name) {
            c.name = format!("{base}-{n}");
            n += 1;
        }
        used.push(c.name.clone());
    }
}

/// Nodes in a Polar network, with their real ports. Polar serves clnrest over
/// plain HTTP on localhost.
fn polar(home: &Path, ask: &mut Prompt) -> Vec<Candidate> {
    let polar = home.join(".polar/networks");
    let Ok(text) = std::fs::read_to_string(polar.join("networks.json")) else {
        return vec![];
    };
    let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
        return vec![];
    };
    let networks: Vec<&serde_json::Value> = json["networks"]
        .as_array()
        .map(|n| {
            n.iter()
                .filter(|n| {
                    n["nodes"]["lightning"]
                        .as_array()
                        .is_some_and(|l| !l.is_empty())
                })
                .collect()
        })
        .unwrap_or_default();
    let network = match networks.as_slice() {
        [] => return vec![],
        [only] => *only,
        many => {
            println!("Polar networks with Lightning nodes:");
            for (i, n) in many.iter().enumerate() {
                println!("  {}. {}", i + 1, n["name"].as_str().unwrap_or("?").trim());
            }
            let pick = ask.default("Which one? (number, or 0 for none)", "1");
            match pick.parse::<usize>() {
                Ok(i) if (1..=many.len()).contains(&i) => many[i - 1],
                _ => return vec![],
            }
        }
    };
    let id = network["id"].as_u64().unwrap_or_default();
    let net_name = network["name"].as_str().unwrap_or("Polar").trim();
    let volumes = polar.join(id.to_string()).join("volumes");

    let mut found = Vec::new();
    for node in network["nodes"]["lightning"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let (Some(name), Some(rest)) = (node["name"].as_str(), node["ports"]["rest"].as_u64())
        else {
            continue;
        };
        let label = format!("{name} (Polar \"{net_name}\", port {rest})");
        match node["implementation"].as_str() {
            Some("LND") => {
                let dir = volumes.join("lnd").join(name);
                found.push(Candidate {
                    label,
                    name: name.into(),
                    kind: Kind::Lnd {
                        url: format!("https://127.0.0.1:{rest}"),
                        macaroon: dir.join("data/chain/bitcoin/regtest/readonly.macaroon"),
                        tls_cert: dir.join("tls.cert"),
                        url_known: true,
                    },
                });
            }
            Some("c-lightning") => found.push(Candidate {
                label,
                name: name.into(),
                kind: Kind::Cln {
                    url: format!("http://127.0.0.1:{rest}"),
                    url_known: true,
                    tls_cert: None,
                    rune_command: format!(
                        "docker exec polar-n{id}-{name} lightning-cli --lightning-dir=/home/clightning/.lightning --network=regtest createrune restrictions='{RUNE_RESTRICTIONS}'"
                    ),
                },
            }),
            _ => {}
        }
    }
    found
}

/// A node the operator describes by hand.
fn manual(ask: &mut Prompt, taken: &[String]) -> Option<String> {
    let kind = ask
        .default("  Implementation: lnd or cln", "lnd")
        .to_lowercase();
    let mut name = ask.default("  A name for it", &format!("my-{kind}"));
    while name.is_empty() || taken.contains(&name) {
        if ask.ended() {
            return None;
        }
        name = if name.is_empty() {
            ask.line("  The node needs a name: ")
        } else {
            ask.line(&format!("  \"{name}\" is taken. Another name: "))
        };
    }
    match kind.as_str() {
        "lnd" => {
            let url = ask.default("  REST address", "https://127.0.0.1:8080");
            let macaroon = ask.line("  Path to readonly.macaroon: ");
            let cert = ask.line("  Path to tls.cert: ");
            (!macaroon.is_empty()).then(|| {
                lnd_toml(
                    &name,
                    &url,
                    &expand(&macaroon),
                    (!cert.is_empty()).then(|| expand(&cert)).as_deref(),
                )
            })
        }
        "cln" => {
            let url = ask.default("  clnrest address", "http://127.0.0.1:3010");
            let cert = if url.starts_with("https") {
                let c = ask.line(
                    "  Path to clnrest's server.pem (Enter if it has a trusted certificate): ",
                );
                (!c.is_empty()).then(|| expand(&c))
            } else {
                None
            };
            println!("  Create a read-only rune on the node with:\n\n    lightning-cli createrune restrictions='{RUNE_RESTRICTIONS}'\n");
            let rune = ask.secret("  Paste the rune: ");
            let rune = rune.trim_matches('"').to_string();
            (!rune.is_empty()).then(|| cln_toml(&name, &url, &rune, cert.as_deref()))
        }
        _ => {
            println!("  Firewatch reads lnd and cln nodes.");
            None
        }
    }
}

/// Tries the node once. A node that fails can still be kept, e.g. one that
/// is down right now.
async fn keep_after_test(toml: &str, http: &reqwest::Client, ask: &mut Prompt) -> bool {
    let node = match parse_node(toml) {
        Ok(node) => node,
        Err(e) => {
            println!("  {e}");
            return false;
        }
    };
    match firewatch_connectors::fetch(&node, http).await {
        Ok(info) => {
            println!(
                "  Connected: {} {}, read-only.",
                node.implementation(),
                info.version
            );
            for warning in &info.warnings {
                println!("  Warning: {warning}");
            }
            println!();
            true
        }
        Err(e) => {
            println!("  Couldn't read it: {e}");
            ask.yes("  Keep it anyway?", false)
        }
    }
}

fn parse_node(block: &str) -> Result<NodeConfig> {
    #[derive(serde::Deserialize)]
    struct One {
        nodes: Vec<NodeConfig>,
    }
    let one: One = toml::from_str(block).context("building the node's config")?;
    one.nodes.into_iter().next().context("no node in the block")
}

fn node_name(block: &str) -> String {
    parse_node(block)
        .map(|n| n.name().to_string())
        .unwrap_or_default()
}

fn lnd_toml(name: &str, url: &str, macaroon: &Path, tls_cert: Option<&Path>) -> String {
    let mut s = format!(
        "[[nodes]]\nkind = \"lnd\"\nname = {}\nurl = {}\nmacaroon_file = {}\n",
        quote(name),
        quote(url),
        quote(&macaroon.to_string_lossy())
    );
    if let Some(cert) = tls_cert {
        s.push_str(&format!("tls_cert = {}\n", quote(&cert.to_string_lossy())));
    }
    s
}

fn cln_toml(name: &str, url: &str, rune: &str, tls_cert: Option<&Path>) -> String {
    let mut s = format!(
        "[[nodes]]\nkind = \"cln\"\nname = {}\nurl = {}\nrune = {}\n",
        quote(name),
        quote(url),
        quote(rune)
    );
    if let Some(cert) = tls_cert {
        s.push_str(&format!("tls_cert = {}\n", quote(&cert.to_string_lossy())));
    }
    s
}

/// The whole config file.
fn render(nodes: &[String], npub: Option<&str>, secret_key_file: &Path) -> String {
    let mut s = String::from(
        "# Written by `firewatch init`. Edit freely: see firewatch.example.toml in\n\
         # the repo for every option. Holds runes, so keep it private.\n\n\
         [watch]\ninterval_secs = 30\nlisten = \"127.0.0.1:7070\"\n",
    );
    if let Some(npub) = npub {
        s.push_str(&format!(
            "\n[alerts.nostr]\nrecipient = {}\nsecret_key_file = {}\nrelays = [{}]\n",
            quote(npub.trim()),
            quote(&secret_key_file.to_string_lossy()),
            RELAYS.map(quote).join(", ")
        ));
    }
    for node in nodes {
        s.push('\n');
        s.push_str(node);
    }
    s
}

fn quote(s: &str) -> String {
    toml::Value::String(s.to_string()).to_string()
}

/// `~/x` to an absolute path, since the config can't expand `~`.
fn expand(path: &str) -> PathBuf {
    match (path.strip_prefix("~/"), std::env::var_os("HOME")) {
        (Some(rest), Some(home)) => PathBuf::from(home).join(rest),
        _ => PathBuf::from(path),
    }
}

/// Reads answers one line at a time, as they're typed. At end of input every
/// question takes its default, and loops check `ended` so `init` can't hang
/// when stdin isn't a terminal.
struct Prompt {
    lines: Box<dyn Iterator<Item = String>>,
    ended: bool,
}

impl Prompt {
    fn stdin() -> Self {
        // Lazy: collecting the lines first would wait for Ctrl-D before the
        // first question is shown.
        Prompt::new(std::io::stdin().lines().map_while(Result::ok))
    }

    fn new(lines: impl Iterator<Item = String> + 'static) -> Self {
        Prompt {
            lines: Box::new(lines),
            ended: false,
        }
    }

    /// Whether input has run out.
    fn ended(&self) -> bool {
        self.ended
    }

    fn line(&mut self, prompt: &str) -> String {
        self.read(prompt, true)
    }

    /// Like `line`, but never repeats the answer back: for runes.
    fn secret(&mut self, prompt: &str) -> String {
        self.read(prompt, false)
    }

    fn read(&mut self, prompt: &str, echo: bool) -> String {
        print!("{prompt}");
        std::io::stdout().flush().ok();
        let answer = match self.lines.next() {
            Some(line) => line.trim().to_string(),
            None => {
                self.ended = true;
                String::new()
            }
        };
        // A terminal already shows what was typed; piped answers don't.
        if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
            println!("{}", if echo { answer.as_str() } else { "(hidden)" });
        }
        answer
    }

    fn default(&mut self, prompt: &str, default: &str) -> String {
        let answer = self.line(&format!("{prompt} [{default}]: "));
        if answer.is_empty() {
            default.to_string()
        } else {
            answer
        }
    }

    fn yes(&mut self, prompt: &str, default: bool) -> bool {
        let hint = if default { "Y/n" } else { "y/N" };
        match self
            .line(&format!("{prompt} [{hint}] "))
            .to_lowercase()
            .as_str()
        {
            "" => default,
            a => a.starts_with('y'),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scripted(answers: &[&str]) -> Prompt {
        let answers: Vec<String> = answers.iter().map(|s| s.to_string()).collect();
        Prompt::new(answers.into_iter())
    }

    fn temp_home(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("fw-init-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn finds_lnd_in_its_default_folder() {
        let home = temp_home("lnd");
        let lnd = home.join(".lnd");
        std::fs::create_dir_all(lnd.join("data/chain/bitcoin/mainnet")).unwrap();
        std::fs::write(lnd.join("tls.cert"), "x").unwrap();
        std::fs::write(
            lnd.join("data/chain/bitcoin/mainnet/readonly.macaroon"),
            "x",
        )
        .unwrap();
        let found = detect(&home, None, &mut scripted(&[]));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].name, "lnd-mainnet");
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn finds_polar_nodes_with_their_ports() {
        let home = temp_home("polar");
        std::fs::create_dir_all(home.join(".polar/networks")).unwrap();
        std::fs::write(
            home.join(".polar/networks/networks.json"),
            r#"{"networks":[{"id":2,"name":"Demo","nodes":{"lightning":[
                {"name":"alice","implementation":"LND","ports":{"rest":8081}},
                {"name":"old-cln","implementation":"c-lightning","ports":{"rest":8182}}]}}]}"#,
        )
        .unwrap();
        let found = detect(&home, None, &mut scripted(&[]));
        assert_eq!(found.len(), 2);
        assert!(matches!(&found[0].kind, Kind::Lnd { url, .. } if url == "https://127.0.0.1:8081"));
        assert!(matches!(&found[1].kind, Kind::Cln { rune_command, .. }
            if rune_command.starts_with("docker exec polar-n2-old-cln lightning-cli")));
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn rendered_config_loads() {
        let nodes = [
            lnd_toml(
                "a",
                "https://127.0.0.1:8080",
                Path::new("/m"),
                Some(Path::new("/c")),
            ),
            cln_toml("b \"quoted\"", "http://127.0.0.1:3010", "abc=", None),
        ];
        let text = render(&nodes, Some("npub1x"), Path::new("/k/firewatch.nsec"));
        let config: config::Config = toml::from_str(&text).unwrap();
        assert_eq!(config.nodes.len(), 2);
        assert_eq!(config.nodes[1].name(), "b \"quoted\"");
        assert_eq!(config.alerts.nostr.unwrap().relays.len(), 3);
    }

    #[test]
    fn a_skipped_rune_skips_the_node() {
        let c = Candidate {
            label: "x".into(),
            name: "x".into(),
            kind: Kind::Cln {
                url: "http://127.0.0.1:1".into(),
                url_known: true,
                tls_cert: None,
                rune_command: "lightning-cli createrune".into(),
            },
        };
        assert!(c.into_toml(&mut scripted(&[""])).is_none());
    }

    #[tokio::test]
    async fn end_of_input_stops_adding_nodes() {
        let (mut nodes, mut names) = (Vec::new(), Vec::new());
        let http = firewatch_connectors::http_client();
        let done = tokio::time::timeout(
            std::time::Duration::from_secs(5),
            add_by_hand(&mut scripted(&[]), &http, &mut nodes, &mut names),
        )
        .await;
        assert!(done.is_ok(), "asked forever at end of input");
        assert!(nodes.is_empty());
    }

    #[test]
    fn a_taken_or_empty_name_is_asked_again() {
        let mut ask = scripted(&["lnd", "", "", "b", "", "/m", ""]);
        let toml = manual(&mut ask, &["my-lnd".into()]).unwrap();
        assert_eq!(node_name(&toml), "b");
        // At end of input the node is dropped, not given an empty name.
        assert!(manual(&mut scripted(&["lnd", ""]), &["my-lnd".into()]).is_none());
    }

    #[test]
    fn two_lnd_folders_get_their_own_names() {
        let home = temp_home("two-lnd");
        let other = home.join("other");
        for dir in [home.join(".lnd"), other.clone()] {
            std::fs::create_dir_all(dir.join("data/chain/bitcoin/mainnet")).unwrap();
            std::fs::write(dir.join("tls.cert"), "x").unwrap();
            std::fs::write(
                dir.join("data/chain/bitcoin/mainnet/readonly.macaroon"),
                "x",
            )
            .unwrap();
        }
        let found = detect(&home, Some(other), &mut scripted(&[]));
        let names: Vec<&str> = found.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["lnd-mainnet", "lnd-mainnet-2"]);
        // LND_DIR pointing at ~/.lnd through a link is the same folder.
        let link = home.join("link");
        std::os::unix::fs::symlink(home.join(".lnd"), &link).unwrap();
        assert_eq!(detect(&home, Some(link), &mut scripted(&[])).len(), 1);
        std::fs::remove_dir_all(home).unwrap();
    }

    #[test]
    fn a_replaced_config_is_private() {
        let home = temp_home("force");
        let path = home.join("firewatch.toml");
        std::fs::write(&path, "old").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
        write_private(&path, "new").unwrap();
        let mode = std::fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "new");
        std::fs::remove_dir_all(home).unwrap();
    }
}
