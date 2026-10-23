//! `firewatch watch`: re-check every N seconds, alert on changes, and serve
//! the latest report to the dashboard.
//!
//! API (JSON):
//! - `GET  /api/report`     latest report (503 until the first check finishes)
//! - `POST /api/check`      check now and return the new report
//! - `GET  /api/advisories` the advisories being checked
//! - `GET  /api/health`     liveness and alert status

use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::routing::{get, post};
use axum::{Json, Router};
use firewatch_alerts::{message, NostrAlerter};
use firewatch_connectors::NodeConfig;
use firewatch_core::{changes, Advisory, Change, NodeReport, Report};
use serde_json::json;
use tokio::sync::{Mutex, RwLock};
use tower_http::cors::CorsLayer;
use tower_http::services::ServeDir;

use crate::config::Config;
use crate::{check_live, log};

/// Failed reads in a row before "can't read this node" is sent. A node
/// restarting for an upgrade is down for a few seconds; that is not news.
const UNREACHABLE_AFTER: u32 = 3;

struct App {
    nodes: Vec<NodeConfig>,
    advisories: Vec<Advisory>,
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
        let report = check_live(&self.nodes, &self.advisories).await;

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

pub async fn run(config: Config, advisories: Vec<Advisory>, allow_remote: bool) -> Result<()> {
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

    let mut router = Router::new()
        .route("/api/report", get(get_report))
        .route("/api/check", post(post_check))
        .route("/api/advisories", get(get_advisories))
        .route("/api/health", get(get_health))
        .layer(CorsLayer::permissive())
        .with_state(app);
    if let Some(dir) = &config.watch.dashboard_dir {
        router = router.fallback_service(ServeDir::new(dir));
    }

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
