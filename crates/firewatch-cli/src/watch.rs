//! `firewatch watch`: re-check every N seconds, alert on changes, and serve
//! the latest report to the dashboard.
//!
//! API (JSON):
//! - `GET  /api/report`     latest report (503 until the first check finishes)
//! - `POST /api/check`      check now and return the new report
//! - `GET  /api/advisories` the advisories being checked
//! - `GET  /api/health`     liveness and alert status

use std::collections::{HashMap, HashSet};
use std::net::IpAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::extract::{Request, State};
use axum::http::{header, Method, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use firewatch_alerts::{message, NostrAlerter};
use firewatch_connectors::NodeConfig;
use firewatch_core::{changes, Advisory, Change, NodeReport, ReleaseTable, Report};
use serde_json::json;
use tokio::sync::{Mutex, RwLock};
use tower_http::cors::{AllowOrigin, CorsLayer};
use tower_http::services::ServeDir;

use crate::config::Config;
use crate::{check_live, log};

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
}

#[derive(Default)]
struct Tracker {
    /// Each node's last successful read, so findings are compared across a
    /// blip and "fixed" is still reported when a restart hides the change.
    known: HashMap<String, NodeReport>,
    down_streak: HashMap<String, u32>,
    down_alerted: HashSet<String>,
}

impl Tracker {
    /// The alert texts for this check, updating what is known.
    fn alerts(&mut self, report: &Report) -> Vec<String> {
        let baseline = Report {
            checked_at: 0,
            nodes: report
                .nodes
                .iter()
                .filter_map(|n| self.known.get(&n.name).cloned())
                .collect(),
        };
        let mut texts: Vec<String> = changes(Some(&baseline), report)
            .iter()
            .filter(|c| matches!(c, Change::Raised { .. } | Change::Resolved { .. }))
            .map(message)
            .collect();

        for node in &report.nodes {
            if node.error.is_some() {
                let streak = self.down_streak.entry(node.name.clone()).or_default();
                *streak += 1;
                if *streak == UNREACHABLE_AFTER {
                    texts.push(message(&Change::Unreachable { node }));
                    self.down_alerted.insert(node.name.clone());
                }
            } else {
                self.down_streak.remove(&node.name);
                if self.down_alerted.remove(&node.name) {
                    texts.push(message(&Change::Reachable { node }));
                }
                self.known.insert(node.name.clone(), node.clone());
            }
        }
        texts
    }
}

impl App {
    async fn run_check(&self) -> Report {
        let mut tracker = self.tracker.lock().await;
        let report = check_live(&self.nodes, &self.advisories, self.releases.as_ref()).await;

        for text in tracker.alerts(&report) {
            log(&text.lines().take(2).collect::<Vec<_>>().join(" — "));
            if let Some(alerter) = &self.alerter {
                match alerter.send(&text).await {
                    Ok(relays) => log(&format!("  alert sent ({relays} relays)")),
                    Err(e) => log(&format!("  ALERT FAILED: {e}")),
                }
            }
        }

        *self.latest.write().await = Some(report.clone());
        report
    }
}

pub async fn run(
    config: Config,
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

    let app = Arc::new(App {
        nodes: config.nodes,
        advisories,
        releases,
        alerter,
        latest: RwLock::new(None),
        tracker: Mutex::new(Tracker::default()),
    });

    let interval = Duration::from_secs(config.watch.interval_secs.max(1));
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
    Json(json!({
        "ok": true,
        "nodes": app.nodes.len(),
        "advisories": app.advisories.len(),
        "alerts": if app.alerter.is_some() { "nostr" } else { "off" },
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

    fn app() -> Arc<App> {
        Arc::new(App {
            nodes: vec![],
            advisories: vec![],
            releases: None,
            alerter: None,
            latest: RwLock::new(None),
            tracker: Mutex::new(Tracker::default()),
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
