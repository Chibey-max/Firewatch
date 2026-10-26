//! The data built into the binary by `build.rs`: the advisories, the LND
//! release table and the dashboard. Flags can still point at files instead.

use std::path::Path;

use axum::http::{header, StatusCode, Uri};
use axum::response::{IntoResponse, Response};
use firewatch_core::{load_texts, Advisory, LoadError, ReleaseTable};

include!(concat!(env!("OUT_DIR"), "/embedded.rs"));

pub fn advisories() -> Result<Vec<Advisory>, LoadError> {
    load_texts(ADVISORIES.iter().copied())
}

pub fn releases() -> Result<ReleaseTable, LoadError> {
    ReleaseTable::from_toml(Path::new("releases/lnd.toml (built in)"), LND_RELEASES)
}

pub fn has_dashboard() -> bool {
    !DASHBOARD.is_empty()
}

/// Serves the built-in dashboard. Unknown paths without a file extension get
/// `index.html`, so the app's own links keep working on reload.
pub async fn dashboard(uri: Uri) -> Response {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };
    let found = DASHBOARD
        .iter()
        .find(|(name, _)| *name == path)
        .or_else(|| {
            (!path.rsplit('/').next().unwrap_or_default().contains('.'))
                .then(|| DASHBOARD.iter().find(|(name, _)| *name == "index.html"))
                .flatten()
        });
    match found {
        Some((name, bytes)) => {
            // Vite fingerprints everything under assets/, so it never changes.
            let cache = if name.starts_with("assets/") {
                "public, max-age=31536000, immutable"
            } else {
                "no-cache"
            };
            (
                [
                    (header::CONTENT_TYPE, content_type(name)),
                    (header::CACHE_CONTROL, cache),
                ],
                *bytes,
            )
                .into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

fn content_type(name: &str) -> &'static str {
    match name.rsplit('.').next().unwrap_or_default() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "json" | "map" => "application/json",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "webp" => "image/webp",
        "ico" => "image/x-icon",
        "woff2" => "font/woff2",
        "woff" => "font/woff",
        "txt" => "text/plain; charset=utf-8",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn built_in_advisories_and_releases_load() {
        assert!(advisories().unwrap().len() >= 10);
        assert!(releases().unwrap().releases.len() > 50);
    }

    #[test]
    fn content_types() {
        assert_eq!(content_type("index.html"), "text/html; charset=utf-8");
        assert_eq!(content_type("assets/x-abc.woff2"), "font/woff2");
        assert_eq!(content_type("README"), "application/octet-stream");
    }
}
