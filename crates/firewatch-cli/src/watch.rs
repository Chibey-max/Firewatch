//! `firewatch watch`: re-check every N seconds, alert on changes, and serve
//! the latest report to the dashboard.
//!
//! API (JSON):
//! - `GET  /api/report`     latest report (503 until the first check finishes)
//! - `POST /api/check`      check now and return the new report
//! - `GET  /api/advisories` the advisories being checked
//! - `GET  /api/health`     liveness, and whether alerts are getting through

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::extract::{Request, State};
use axum::http::{header, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use firewatch_alerts::{fill, message_in, node_message_in, NostrAlerter, Strings, EN};
use firewatch_connectors::NodeConfig;
use firewatch_core::{changes, Advisory, Change, NodeReport, ReleaseTable, Report, Status};
use serde::Serialize;
use serde_json::json;
use tokio::sync::{Mutex, RwLock};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;

use crate::config::Config;
use crate::outbox::{Entry, Kind, Outbox};
use crate::state::{self, State as SavedState};
use crate::{check_live, local_time, log, now};

/// Failed reads in a row before "can't read this node" is sent. A node
/// restarting for an upgrade is down for a few seconds; that is not news.
const UNREACHABLE_AFTER: u32 = 3;

struct App {
    nodes: Vec<NodeConfig>,
    advisories: Vec<Advisory>,
    releases: Option<ReleaseTable>,
    alerter: Option<NostrAlerter>,
    latest: RwLock<Option<Report>>,
    /// Behind a lock, so two checks never race on "what changed".
    tracker: Mutex<Tracker>,
    /// Where the tracker is saved after every check (A4).
    state_path: PathBuf,
    /// A message that waited longer than this says when it happened (A2).
    interval_secs: u64,
    /// Outside the tracker's lock, so `/api/health` never waits on a send.
    delivery: RwLock<Delivery>,
}

struct Tracker {
    /// Each node's last successful read, so findings are compared across a
    /// blip and "fixed" is still reported when a restart hides the change.
    known: HashMap<String, NodeReport>,
    down_streak: HashMap<String, u32>,
    down_alerted: HashSet<String>,
    /// Connector warnings already logged, so each shows once, not every check.
    warned: HashSet<String>,
    /// The words alerts are written in.
    strings: &'static Strings,
    /// Set after a restart from saved state: the first check's messages say
    /// they happened while Firewatch was off, since this time.
    catch_up_since: Option<u64>,
    /// Whether saving the state has failed and been logged already.
    save_failed: bool,
    /// Alerts not sent yet (A1).
    outbox: Outbox,
    /// When sending started failing, while it still fails.
    offline_since: Option<u64>,
    last_sent_at: Option<u64>,
}

impl Default for Tracker {
    fn default() -> Self {
        Tracker::new(&EN)
    }
}

impl Tracker {
    fn new(strings: &'static Strings) -> Self {
        Tracker {
            known: HashMap::new(),
            down_streak: HashMap::new(),
            down_alerted: HashSet::new(),
            warned: HashSet::new(),
            strings,
            catch_up_since: None,
            save_failed: false,
            outbox: Outbox::default(),
            offline_since: None,
            last_sent_at: None,
        }
    }

    /// A tracker that remembers what was known before a restart. Its first
    /// check reports only what changed since, as catch-up messages.
    fn restored(state: SavedState, strings: &'static Strings) -> Self {
        Tracker {
            known: state.known,
            down_alerted: state.down_alerted,
            catch_up_since: Some(state.saved_at),
            outbox: Outbox::from_entries(state.outbox),
            ..Tracker::new(strings)
        }
    }

    fn state(&self, saved_at: u64) -> SavedState {
        SavedState {
            known: self.known.clone(),
            down_alerted: self.down_alerted.clone(),
            outbox: self.outbox.entries().to_vec(),
            ..SavedState::new(saved_at)
        }
    }

    /// Queues the alerts for this check, updating what is known. `now` is
    /// when the changes were seen.
    fn queue(&mut self, report: &Report, now: u64) {
        let catch_up_since = self.catch_up_since.take();
        let entry = |node: &NodeReport, kind, text| Entry {
            node: node.name.clone(),
            kind,
            text,
            detected_at: now,
            waiting_since: now,
            catch_up_since,
            heard: None,
            affected_since: None,
        };

        // Findings first, one message per node, in report order.
        for node in report.nodes.iter().filter(|n| n.error.is_none()) {
            let known = self.known.get(&node.name);
            if node_changes(known, report, &node.name).is_empty() {
                continue;
            }
            // Told against what the operator last heard, which is older than
            // `known` if a message for this node is still waiting.
            let waiting = self.outbox.waiting(&node.name, Kind::Findings);
            let heard = match waiting {
                Some(w) => w.heard.clone(),
                None => known.cloned(),
            };
            let affected_since = waiting
                .and_then(|w| w.affected_since)
                .or((node.status == Status::Affected).then_some(now));
            let told = node_changes(heard.as_ref(), report, &node.name);
            let text = if !told.is_empty() {
                node_message_in(node, &told.iter().collect::<Vec<_>>(), self.strings)
            } else if let (Some(from), false) = (affected_since, node.status == Status::Affected) {
                format!(
                    "✅ Firewatch: {}",
                    fill(
                        self.strings.fixed_while_waiting,
                        &[
                            ("node", &node.name),
                            ("from", &local_time(from)),
                            ("to", &local_time(now)),
                        ]
                    )
                )
            } else {
                // Back where the operator last heard it: nothing to say.
                self.outbox.cancel(&node.name, Kind::Findings);
                continue;
            };
            self.outbox.put(Entry {
                heard,
                affected_since,
                ..entry(node, Kind::Findings, text)
            });
        }

        for node in &report.nodes {
            if node.error.is_some() {
                let streak = self.down_streak.entry(node.name.clone()).or_default();
                *streak += 1;
                if *streak == UNREACHABLE_AFTER {
                    // A waiting "can be read again" means the operator last
                    // heard "can't read": they needn't hear it twice.
                    let told_down = self
                        .outbox
                        .waiting(&node.name, Kind::Reachable)
                        .is_some_and(|w| w.kind == Kind::Reachable);
                    if told_down {
                        self.outbox.cancel(&node.name, Kind::Reachable);
                    } else {
                        let text = message_in(&Change::Unreachable { node }, self.strings);
                        self.outbox.put(entry(node, Kind::Unreachable, text));
                    }
                    self.down_alerted.insert(node.name.clone());
                }
            } else {
                self.down_streak.remove(&node.name);
                if self.down_alerted.remove(&node.name) {
                    let never_told = self
                        .outbox
                        .waiting(&node.name, Kind::Unreachable)
                        .is_some_and(|w| w.kind == Kind::Unreachable);
                    if never_told {
                        self.outbox.cancel(&node.name, Kind::Unreachable);
                    } else {
                        let text = message_in(&Change::Reachable { node }, self.strings);
                        self.outbox.put(entry(node, Kind::Reachable, text));
                    }
                }
                self.known.insert(node.name.clone(), node.clone());
            }
        }
    }

    /// A waiting message as it is sent at `sent_at`: marked as a draft
    /// translation or a catch-up if it is one, and saying when it happened
    /// if it waited longer than one check (A2).
    fn render(&self, entry: &Entry, sent_at: u64, interval_secs: u64) -> String {
        let mut text = entry.text.clone();
        if let Some(since) = entry.catch_up_since {
            let header = fill(self.strings.catch_up, &[("since", &local_time(since))]);
            text = format!("{header}\n{text}");
        }
        if !self.strings.reviewed {
            text = format!(
                "[Draft {} translation, not reviewed yet]\n{text}",
                self.strings.english_name
            );
        }
        if sent_at.saturating_sub(entry.detected_at) > interval_secs {
            let line = fill(
                self.strings.delayed,
                &[
                    ("detected", &local_time(entry.detected_at)),
                    ("sent", &local_time(sent_at)),
                ],
            );
            text = format!("{text}\n\n{line}");
        }
        text
    }

    /// The alerts for this check, as if every one was sent at once.
    #[cfg(test)]
    fn alerts(&mut self, report: &Report) -> Vec<String> {
        self.queue(report, report.checked_at);
        let mut texts = vec![];
        while let Some(entry) = self.outbox.first() {
            texts.push(self.render(entry, report.checked_at, 60));
            self.outbox.sent();
        }
        texts
    }

    fn delivery(&self) -> Delivery {
        Delivery {
            pending: self.outbox.len(),
            oldest_pending_since: self.outbox.oldest(),
            last_sent_at: self.last_sent_at,
        }
    }
}

/// What changed in the findings of `report`'s node `name` since `before`.
fn node_changes<'a>(
    before: Option<&NodeReport>,
    report: &'a Report,
    name: &str,
) -> Vec<Change<'a>> {
    let before = Report {
        checked_at: 0,
        nodes: before.cloned().into_iter().collect(),
    };
    changes(Some(&before), report)
        .into_iter()
        .filter(|c| match c {
            Change::Raised { node, .. } | Change::Resolved { node, .. } => node.name == name,
            _ => false,
        })
        .collect()
}

/// Delivery status for `/api/health` (A3).
#[derive(Debug, Clone, Default, Serialize)]
struct Delivery {
    pending: usize,
    oldest_pending_since: Option<u64>,
    last_sent_at: Option<u64>,
}

impl App {
    async fn run_check(&self) -> Report {
        let mut tracker = self.tracker.lock().await;
        let (report, warnings) =
            check_live(&self.nodes, &self.advisories, self.releases.as_ref()).await;
        for warning in warnings {
            if tracker.warned.insert(warning.clone()) {
                log(&format!("warning: {warning}"));
            }
        }

        let before = tracker.outbox.entries().to_vec();
        tracker.queue(&report, now());
        for entry in tracker.outbox.entries() {
            if !before.contains(entry) {
                log(&entry
                    .text
                    .lines()
                    .filter(|l| !l.is_empty())
                    .take(2)
                    .collect::<Vec<_>>()
                    .join(" — "));
            }
        }

        // Oldest first, stopping at the first failure: with no connection,
        // one try per check is enough, and the rest wait their turn.
        while let Some(entry) = tracker.outbox.first() {
            let Some(alerter) = &self.alerter else {
                tracker.outbox.sent();
                continue;
            };
            let sent_at = now();
            let text = tracker.render(entry, sent_at, self.interval_secs);
            match alerter.send(&text).await {
                Ok(relays) => {
                    tracker.outbox.sent();
                    tracker.last_sent_at = Some(sent_at);
                    if let Some(since) = tracker.offline_since.take() {
                        log(&format!(
                            "  connection back (failing since {}): sending waiting alerts",
                            local_time(since)
                        ));
                    }
                    log(&format!("  alert sent ({relays} relays)"));
                }
                Err(e) => {
                    if tracker.offline_since.is_none() {
                        tracker.offline_since = Some(sent_at);
                        log(&format!(
                            "  ALERT FAILED: {e}; {} waiting, retrying every check",
                            tracker.outbox.len()
                        ));
                    }
                    break;
                }
            }
        }
        *self.delivery.write().await = tracker.delivery();

        match state::save(&self.state_path, &tracker.state(report.checked_at)) {
            Ok(()) => tracker.save_failed = false,
            Err(e) if !tracker.save_failed => {
                tracker.save_failed = true;
                log(&format!(
                    "warning: can't save {} ({e:#}); a restart will re-send current alerts",
                    self.state_path.display()
                ));
            }
            Err(_) => {}
        }

        *self.latest.write().await = Some(report.clone());
        report
    }
}

pub async fn run(
    config: Config,
    config_path: &Path,
    advisories: Vec<Advisory>,
    releases: Option<ReleaseTable>,
    allow_remote: bool,
) -> Result<()> {
    if config.nodes.is_empty() {
        anyhow::bail!("no [[nodes]] in the config");
    }
    ensure_local(&config.watch.listen, allow_remote).await?;
    let alerter = match &config.alerts.nostr {
        Some(nostr) => {
            let alerter = NostrAlerter::connect(nostr)
                .await
                .context("setting up Nostr alerts")?;
            log(&format!(
                "Nostr alerts on: sending as {} via {} relays",
                alerter.sender(),
                alerter.relays().len()
            ));
            Some(alerter)
        }
        None => {
            log("Nostr alerts off: add [alerts.nostr] to the config to enable them");
            None
        }
    };

    let strings = config.alerts.strings()?;
    let state_path = state::path_for(config_path);
    let tracker = match state::load(&state_path) {
        Ok(Some(saved)) => {
            log(&format!(
                "remembering {} node(s) from {} (last check {}): changes since then are sent as catch-up alerts",
                saved.known.len(),
                state_path.display(),
                local_time(saved.saved_at)
            ));
            Tracker::restored(saved, strings)
        }
        Ok(None) => Tracker::new(strings),
        Err(why) => {
            log(&format!("warning: {why}; starting fresh"));
            Tracker::new(strings)
        }
    };

    let interval = Duration::from_secs(config.watch.interval_secs.max(1));
    let delivery = RwLock::new(tracker.delivery());
    let app = Arc::new(App {
        nodes: config.nodes,
        advisories,
        releases,
        alerter,
        latest: RwLock::new(None),
        tracker: Mutex::new(tracker),
        state_path,
        interval_secs: interval.as_secs(),
        delivery,
    });

    let looping = app.clone();
    tokio::spawn(async move {
        loop {
            let report = looping.run_check().await;
            log(&format!(
                "checked {} nodes, worst: {:?}",
                report.nodes.len(),
                report.worst()
            ));
            tokio::time::sleep(interval).await;
        }
    });

    let router = router(app, config.watch.dashboard_dir.as_deref(), allow_remote);
    let listener = tokio::net::TcpListener::bind(&config.watch.listen)
        .await
        .with_context(|| format!("listening on {}", config.watch.listen))?;
    log(&format!(
        "dashboard API on http://{} (checking every {}s)",
        config.watch.listen,
        interval.as_secs()
    ));
    axum::serve(listener, router).await?;
    Ok(())
}

/// The API, and the dashboard when `dashboard_dir` is set. Every request,
/// static files included, passes `local_only` first: the fallback is added
/// before the layers because axum only layers what is already routed.
fn router(app: Arc<App>, dashboard_dir: Option<&Path>, allow_remote: bool) -> Router {
    let mut router = Router::new()
        .route("/api/report", get(get_report))
        .route("/api/check", post(post_check))
        .route("/api/advisories", get(get_advisories))
        .route("/api/health", get(get_health));
    if let Some(dir) = dashboard_dir {
        router = router.fallback_service(ServeDir::new(dir));
    } else if crate::embedded::has_dashboard() {
        router = router.fallback(crate::embedded::dashboard);
    }
    // The dashboard is same-origin (served here, or through Vite's proxy), so
    // CORS is only for a dev page on another localhost port (VITE_API_BASE).
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::predicate(|origin, _| {
            origin.to_str().is_ok_and(is_local_origin)
        }))
        .allow_methods([Method::GET, Method::POST]);
    router
        .layer(cors)
        .layer(middleware::from_fn_with_state(allow_remote, local_only))
        .with_state(app)
}

/// Turns away requests a hostile web page can make from the operator's own
/// browser. The report says which nodes are vulnerable, and `listen` being
/// 127.0.0.1 doesn't help when the request comes from a browser on this
/// machine:
/// - DNS rebinding: a site whose name resolves to 127.0.0.1 is same-origin
///   with itself, so CORS never applies. Its requests still carry its own
///   name in `Host`, so only localhost names are served. With
///   `--allow-remote`, IP addresses are served too: a rebinding site needs a
///   domain name, and a LAN browser can use the address.
/// - A cross-site `POST /api/check` runs a check without reading the answer,
///   and no preflight is sent for it. Browsers do send `Origin`, which must
///   be this server or localhost.
///
/// A request without `Host` didn't come from a browser, so it isn't refused.
async fn local_only(State(allow_remote): State<bool>, request: Request, next: Next) -> Response {
    let host = request
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .or_else(|| request.uri().authority().map(|a| a.as_str()))
        .map(str::to_owned);
    if let Some(h) = host.as_deref() {
        let allowed = is_local_host(h) || (allow_remote && is_ip_host(h));
        if !allowed {
            return refuse(if allow_remote {
                "this server only answers to localhost or an IP address (a guard against DNS rebinding); open it by its IP address"
            } else {
                "this server only answers to localhost, 127.0.0.1 or [::1] (a guard against DNS rebinding); open it by one of those names"
            });
        }
    }
    if let Some(origin) = request.headers().get(header::ORIGIN) {
        let origin = origin.to_str().unwrap_or_default();
        let same_origin = host
            .as_deref()
            .is_some_and(|h| origin == format!("http://{h}") || origin == format!("https://{h}"));
        if !same_origin && !is_local_origin(origin) {
            return refuse(
                "cross-site request refused: only pages served by this machine may use this API",
            );
        }
    }
    next.run(request).await
}

fn refuse(why: &str) -> Response {
    log(&format!("refused a request: {why}"));
    (StatusCode::FORBIDDEN, Json(json!({ "error": why }))).into_response()
}

/// The name in a `Host` value, without its port: `localhost:5173` gives
/// `localhost`, `[::1]:7070` gives `::1`. `None` if it's malformed.
fn host_name(host: &str) -> Option<&str> {
    match host.strip_prefix('[') {
        Some(rest) => match rest.split_once(']') {
            Some((ip, port)) if port.is_empty() || port.starts_with(':') => Some(ip),
            _ => None,
        },
        None => match host.rsplit_once(':') {
            Some((name, port)) if port.bytes().all(|b| b.is_ascii_digit()) => Some(name),
            Some(_) => None,
            None => Some(host),
        },
    }
}

/// `localhost` or a loopback address, with or without a port:
/// `localhost:5173`, `127.0.0.1`, `[::1]:7070`.
fn is_local_host(host: &str) -> bool {
    host_name(host).is_some_and(|name| {
        name.eq_ignore_ascii_case("localhost")
            || name.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
    })
}

/// An IP address rather than a name, e.g. `192.168.1.20:7070`.
fn is_ip_host(host: &str) -> bool {
    host_name(host).is_some_and(|name| name.parse::<IpAddr>().is_ok())
}

/// An `Origin` header naming a page served from this machine.
fn is_local_origin(origin: &str) -> bool {
    origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
        .is_some_and(|host| !host.contains('/') && is_local_host(host))
}

/// Refuses a non-loopback `listen` address unless `allow_remote` is set: the
/// report says which nodes are vulnerable, so by default only this machine
/// may read it.
async fn ensure_local(listen: &str, allow_remote: bool) -> Result<()> {
    if allow_remote {
        return Ok(());
    }
    let addrs: Vec<_> = tokio::net::lookup_host(listen)
        .await
        .with_context(|| format!("resolving listen address {listen}"))?
        .collect();
    if let Some(addr) = addrs.iter().find(|a| !a.ip().is_loopback()) {
        anyhow::bail!(
            "refusing to serve the report on {addr}: it is reachable from other machines and \
             says which nodes are vulnerable. Use 127.0.0.1, or pass --allow-remote if you mean it"
        );
    }
    Ok(())
}

async fn get_report(State(app): State<Arc<App>>) -> Response {
    match app.latest.read().await.clone() {
        Some(report) => Json(report).into_response(),
        None => (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "the first check is still running" })),
        )
            .into_response(),
    }
}

async fn post_check(State(app): State<Arc<App>>) -> Json<Report> {
    Json(app.run_check().await)
}

async fn get_advisories(State(app): State<Arc<App>>) -> Json<Vec<Advisory>> {
    Json(app.advisories.clone())
}

async fn get_health(State(app): State<Arc<App>>) -> Json<serde_json::Value> {
    let delivery = app.delivery.read().await.clone();
    Json(json!({
        "ok": true,
        "nodes": app.nodes.len(),
        "advisories": app.advisories.len(),
        "alerts": if app.alerter.is_some() { "nostr" } else { "off" },
        "alerts_pending": delivery.pending,
        "oldest_pending_since": delivery.oldest_pending_since,
        "last_sent_at": delivery.last_sent_at,
    }))
}

#[cfg(test)]
mod tests {
    use firewatch_core::{Action, Finding, Implementation, Severity, Status};

    use super::*;

    fn up(status: Status) -> Report {
        Report {
            checked_at: 0,
            nodes: vec![NodeReport {
                name: "old-cln".into(),
                implementation: Implementation::Cln,
                version: Some("v26.06.7".into()),
                network: None,
                status,
                error: None,
                build_check: Default::default(),
                exposure: vec![],
                findings: vec![Finding {
                    advisory_id: "FW-2026-003".into(),
                    title: "t".into(),
                    severity: Severity::Critical,
                    exploited_in_wild: true,
                    verified: true,
                    fixed: Some("26.06.8".into()),
                    source: "https://example.com".into(),
                    status,
                    reason: "r".into(),
                    manual_checks: vec![],
                    in_range: true,
                    action: Action::Upgrade,
                    steps: "s".into(),
                }],
            }],
        }
    }

    fn down() -> Report {
        Report {
            checked_at: 0,
            nodes: vec![NodeReport::unreachable(
                "old-cln",
                Implementation::Cln,
                "restarting".into(),
            )],
        }
    }

    fn first_words(texts: &[String]) -> Vec<String> {
        texts
            .iter()
            .map(|t| t.split(" is ").next().unwrap_or_default().to_string())
            .collect()
    }

    #[tokio::test]
    async fn listen_must_be_loopback_unless_allowed() {
        for ok in ["127.0.0.1:7070", "[::1]:7070", "localhost:7070"] {
            assert!(ensure_local(ok, false).await.is_ok(), "{ok}");
        }
        for remote in ["0.0.0.0:7070", "[::]:7070", "192.168.1.10:7070"] {
            assert!(ensure_local(remote, false).await.is_err(), "{remote}");
            assert!(ensure_local(remote, true).await.is_ok(), "{remote}");
        }
    }

    const T: u64 = 1_800_000_000;

    /// Sends everything waiting at `at`, as `run_check` does once the
    /// connection is back.
    fn deliver(tracker: &mut Tracker, at: u64) -> Vec<String> {
        let mut texts = vec![];
        while let Some(entry) = tracker.outbox.first() {
            texts.push(tracker.render(entry, at, 10));
            tracker.outbox.sent();
        }
        texts
    }

    #[test]
    fn a_queued_alert_is_sent_once_and_says_when_it_happened() {
        let mut tracker = Tracker::default();
        tracker.queue(&up(Status::Affected), T);
        // Still offline: later checks see no change and queue nothing more.
        tracker.queue(&up(Status::Affected), T + 10);
        tracker.queue(&up(Status::Affected), T + 20);
        assert_eq!(tracker.delivery().pending, 1);
        assert_eq!(tracker.delivery().oldest_pending_since, Some(T));

        let sent = deliver(&mut tracker, T + 2400);
        assert_eq!(sent.len(), 1);
        assert!(sent[0].contains("old-cln: 1 affected"), "{}", sent[0]);
        let last = sent[0].lines().last().unwrap();
        assert!(
            last.starts_with("Detected ") && last.ends_with("(no connection until then)."),
            "{last}"
        );
        assert!(deliver(&mut tracker, T + 2410).is_empty());
        assert!(tracker.alerts(&up(Status::Affected)).is_empty());
    }

    #[test]
    fn an_alert_sent_on_time_does_not_say_when() {
        let mut tracker = Tracker::default();
        tracker.queue(&up(Status::Affected), T);
        let sent = deliver(&mut tracker, T + 3);
        assert!(!sent[0].contains("Detected"), "{}", sent[0]);
    }

    #[test]
    fn red_then_fixed_before_anything_got_through_is_one_message() {
        let mut tracker = Tracker::default();
        tracker.alerts(&up(Status::NotAffected));
        tracker.queue(&up(Status::Affected), T);
        tracker.queue(&up(Status::NotAffected), T + 1080);
        let sent = deliver(&mut tracker, T + 1200);
        assert_eq!(sent.len(), 1, "{sent:?}");
        assert!(
            sent[0].starts_with("✅ Firewatch: old-cln was affected from ")
                && sent[0].contains("; it's fixed now."),
            "{}",
            sent[0]
        );
    }

    #[test]
    fn a_newer_state_replaces_the_waiting_alert() {
        let mut tracker = Tracker::default();
        tracker.alerts(&up(Status::Affected));
        // Amber while offline, then fixed: the operator, who last heard
        // "red", gets only "no longer affected", never the stale amber.
        tracker.queue(&up(Status::CantTell), T);
        tracker.queue(&up(Status::NotAffected), T + 60);
        let sent = deliver(&mut tracker, T + 120);
        assert_eq!(sent.len(), 1, "{sent:?}");
        assert!(
            sent[0].starts_with("✅ Firewatch: old-cln is no longer affected by FW-2026-003"),
            "{}",
            sent[0]
        );
    }

    #[test]
    fn back_where_the_operator_last_heard_it_sends_nothing() {
        let mut tracker = Tracker::default();
        tracker.alerts(&up(Status::Affected));
        tracker.queue(&up(Status::CantTell), T);
        tracker.queue(&up(Status::Affected), T + 60);
        assert!(deliver(&mut tracker, T + 120).is_empty());
    }

    #[test]
    fn down_and_back_up_while_waiting_sends_nothing() {
        let mut tracker = Tracker::default();
        tracker.alerts(&up(Status::Affected));
        for i in 0..UNREACHABLE_AFTER {
            tracker.queue(&down(), T + u64::from(i) * 10);
        }
        assert_eq!(tracker.delivery().pending, 1);
        tracker.queue(&up(Status::Affected), T + 100);
        assert!(deliver(&mut tracker, T + 110).is_empty());
    }

    #[test]
    fn waiting_alerts_survive_a_restart() {
        let mut before = Tracker::default();
        before.alerts(&up(Status::NotAffected));
        before.queue(&up(Status::Affected), T);
        let json = serde_json::to_string(&before.state(T)).unwrap();
        let mut after = Tracker::restored(serde_json::from_str(&json).unwrap(), &EN);
        assert_eq!(after.delivery().pending, 1);
        // The first check after the restart sees no change from `known`.
        after.queue(&up(Status::Affected), T + 600);
        let sent = deliver(&mut after, T + 600);
        assert_eq!(sent.len(), 1, "{sent:?}");
        assert!(sent[0].contains("old-cln: 1 affected"), "{}", sent[0]);
        assert!(sent[0].contains("Detected "), "{}", sent[0]);
    }

    /// What `watch` saves after a check, and starts from after a restart.
    fn restart_after(report: &Report) -> Tracker {
        let mut before = Tracker::default();
        before.alerts(report);
        let saved = before.state(1_800_000_000);
        let json = serde_json::to_string(&saved).unwrap();
        Tracker::restored(serde_json::from_str(&json).unwrap(), &EN)
    }

    #[test]
    fn a_restart_with_nothing_changed_sends_nothing() {
        let mut t = restart_after(&up(Status::Affected));
        assert!(t.alerts(&up(Status::Affected)).is_empty());
    }

    #[test]
    fn a_fix_while_off_is_one_catch_up_message() {
        let mut t = restart_after(&up(Status::Affected));
        let texts = t.alerts(&up(Status::NotAffected));
        assert_eq!(texts.len(), 1, "{texts:?}");
        let mut lines = texts[0].lines();
        assert!(
            lines
                .next()
                .unwrap()
                .starts_with("While Firewatch was off (since "),
            "{texts:?}"
        );
        assert!(lines
            .next()
            .unwrap()
            .starts_with("✅ Firewatch: old-cln is no longer affected by FW-2026-003"));
        // Only the first check after the restart is a catch-up.
        assert!(t.alerts(&up(Status::NotAffected)).is_empty());
        let again = t.alerts(&up(Status::Affected));
        assert!(
            !again[0].starts_with("While Firewatch was off"),
            "{again:?}"
        );
    }

    #[test]
    fn a_node_down_across_a_restart_is_not_re_alerted() {
        let mut before = Tracker::default();
        before.alerts(&up(Status::Affected));
        for _ in 0..UNREACHABLE_AFTER {
            before.alerts(&down());
        }
        let saved = before.state(1_800_000_000);
        let mut t = Tracker::restored(saved, &EN);
        let back = t.alerts(&up(Status::Affected));
        assert_eq!(back.len(), 1, "{back:?}");
        assert!(back[0].contains("old-cln can be read again"), "{back:?}");
    }

    #[test]
    fn alerts_use_the_configured_language_and_mark_drafts() {
        let mut t = Tracker::new(firewatch_alerts::Language::Ha.strings());
        let texts = t.alerts(&up(Status::Affected));
        assert!(
            texts[0].starts_with(
                "[Draft Hausa translation, not reviewed yet]\n🔴 Firewatch: old-cln: 1 sun shafa"
            ),
            "{texts:?}"
        );
        // The developers' words and the reason stay in English.
        assert!(
            texts[0].contains("Masu haɓakawa sun ce: \"s\""),
            "{texts:?}"
        );
    }

    #[test]
    fn restart_blip_is_silent_and_fix_is_still_reported() {
        let mut t = Tracker::default();
        assert_eq!(t.alerts(&up(Status::Affected)).len(), 1);
        assert!(t.alerts(&down()).is_empty(), "one failed read is not news");
        let texts = t.alerts(&up(Status::NotAffected));
        assert_eq!(texts.len(), 1);
        assert!(texts[0].starts_with("✅"), "{texts:?}");
    }

    #[test]
    fn long_outage_alerts_once_and_recovery_is_reported() {
        let mut t = Tracker::default();
        t.alerts(&up(Status::Affected));
        assert!(t.alerts(&down()).is_empty());
        assert!(t.alerts(&down()).is_empty());
        let texts = t.alerts(&down());
        assert!(texts[0].starts_with("⚠️"), "{texts:?}");
        assert!(t.alerts(&down()).is_empty(), "no repeat while still down");
        let back = t.alerts(&up(Status::Affected));
        assert_eq!(
            first_words(&back),
            ["Firewatch: old-cln can be read again."]
        );
    }
}

/// The guard against hostile web pages (B4), through the real router.
#[cfg(test)]
mod guard_tests {
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn health_says_whether_alerts_are_getting_through() {
        let app = app();
        *app.delivery.write().await = Delivery {
            pending: 2,
            oldest_pending_since: Some(1_800_000_000),
            last_sent_at: Some(1_799_999_940),
        };
        let health = get_health(State(app)).await.0;
        assert_eq!(health["alerts_pending"], 2);
        assert_eq!(health["oldest_pending_since"], 1_800_000_000u64);
        assert_eq!(health["last_sent_at"], 1_799_999_940u64);
    }

    fn app() -> Arc<App> {
        Arc::new(App {
            nodes: vec![],
            advisories: vec![],
            releases: None,
            alerter: None,
            latest: RwLock::new(None),
            tracker: Mutex::new(Tracker::default()),
            state_path: std::env::temp_dir().join("fw-watch-test-state.json"),
            interval_secs: 10,
            delivery: RwLock::new(Delivery::default()),
        })
    }

    async fn send(
        router: &Router,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
    ) -> axum::response::Response {
        let mut request = Request::builder().method(method).uri(path);
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        router
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    #[test]
    fn local_hosts() {
        for h in [
            "localhost",
            "localhost:5173",
            "LOCALHOST:7070",
            "127.0.0.1:7070",
            "127.9.9.9",
            "[::1]:7070",
            "[::1]",
        ] {
            assert!(is_local_host(h), "{h}");
        }
        for h in [
            "evil.example:7070",
            "127.0.0.1.evil.example",
            "localhost.evil.example",
            "0.0.0.0:7070",
            "192.168.1.20:7070",
            "[::]:7070",
            "localhost:abc",
            "[::1]x",
            "",
        ] {
            assert!(!is_local_host(h), "{h}");
        }
    }

    #[test]
    fn ip_hosts_for_remote_mode() {
        assert!(is_ip_host("192.168.1.20:7070"));
        assert!(is_ip_host("[fe80::1]:7070"));
        assert!(!is_ip_host("mynode.local:7070"));
        assert!(!is_ip_host("evil.example"));
    }

    #[test]
    fn local_origins() {
        assert!(is_local_origin("http://localhost:5173"));
        assert!(is_local_origin("https://127.0.0.1:7070"));
        assert!(is_local_origin("http://[::1]:7070"));
        for o in [
            "https://evil.example",
            "http://localhost.evil.example",
            "null",
            "file://",
            "http://localhost:5173/path",
        ] {
            assert!(!is_local_origin(o), "{o}");
        }
    }

    #[tokio::test]
    async fn localhost_and_the_vite_proxy_are_served() {
        let r = router(app(), None, false);
        for host in [
            "127.0.0.1:7070",
            "localhost:7070",
            "localhost:5173",
            "[::1]:7070",
        ] {
            let res = send(&r, "GET", "/api/health", &[("host", host)]).await;
            assert_eq!(res.status(), StatusCode::OK, "{host}");
        }
        // Vite's proxy keeps the page's Host and Origin; a POST carries Origin.
        let res = send(
            &r,
            "POST",
            "/api/check",
            &[
                ("host", "localhost:5173"),
                ("origin", "http://localhost:5173"),
            ],
        )
        .await;
        assert_eq!(res.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn dns_rebinding_is_refused() {
        let r = router(app(), None, false);
        for host in ["evil.example:7070", "0.0.0.0:7070", "192.168.1.20:7070"] {
            let res = send(&r, "GET", "/api/report", &[("host", host)]).await;
            assert_eq!(res.status(), StatusCode::FORBIDDEN, "{host}");
        }
    }

    #[tokio::test]
    async fn another_site_can_neither_read_nor_trigger_a_check() {
        let r = router(app(), None, false);
        let evil = [
            ("host", "127.0.0.1:7070"),
            ("origin", "https://evil.example"),
        ];
        let res = send(&r, "GET", "/api/report", &evil).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        assert!(res.headers().get("access-control-allow-origin").is_none());

        let res = send(&r, "POST", "/api/check", &evil).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
        // The refused POST ran no check: the first report is still pending.
        let res = send(&r, "GET", "/api/report", &[("host", "127.0.0.1:7070")]).await;
        assert_eq!(res.status(), StatusCode::SERVICE_UNAVAILABLE);

        let preflight = [
            ("host", "127.0.0.1:7070"),
            ("origin", "https://evil.example"),
            ("access-control-request-method", "POST"),
        ];
        let res = send(&r, "OPTIONS", "/api/check", &preflight).await;
        assert_eq!(res.status(), StatusCode::FORBIDDEN);
    }

    #[tokio::test]
    async fn a_dev_page_on_another_localhost_port_gets_cors() {
        // VITE_API_BASE=http://127.0.0.1:7070 with the page on :5173.
        let r = router(app(), None, false);
        let res = send(
            &r,
            "GET",
            "/api/health",
            &[
                ("host", "127.0.0.1:7070"),
                ("origin", "http://localhost:5173"),
            ],
        )
        .await;
        assert_eq!(res.status(), StatusCode::OK);
        assert_eq!(
            res.headers()["access-control-allow-origin"],
            "http://localhost:5173"
        );
    }

    #[tokio::test]
    async fn remote_mode_serves_ip_addresses_but_not_names() {
        let r = router(app(), None, true);
        let lan = [("host", "192.168.1.20:7070")];
        assert_eq!(
            send(&r, "GET", "/api/health", &lan).await.status(),
            StatusCode::OK
        );
        let same_origin = [
            ("host", "192.168.1.20:7070"),
            ("origin", "http://192.168.1.20:7070"),
        ];
        assert_eq!(
            send(&r, "POST", "/api/check", &same_origin).await.status(),
            StatusCode::OK
        );
        for headers in [
            &[("host", "evil.example:7070")][..],
            &[
                ("host", "192.168.1.20:7070"),
                ("origin", "https://evil.example"),
            ][..],
        ] {
            assert_eq!(
                send(&r, "GET", "/api/health", headers).await.status(),
                StatusCode::FORBIDDEN
            );
        }
    }

    #[tokio::test]
    async fn the_served_dashboard_is_guarded_too() {
        let dir = std::env::temp_dir().join(format!("firewatch-guard-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.html"), "<p>dashboard</p>").unwrap();
        let r = router(app(), Some(&dir), false);
        let ok = send(&r, "GET", "/index.html", &[("host", "127.0.0.1:7070")]).await;
        let evil = send(&r, "GET", "/index.html", &[("host", "evil.example:7070")]).await;
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(ok.status(), StatusCode::OK);
        assert_eq!(evil.status(), StatusCode::FORBIDDEN);
    }
}
