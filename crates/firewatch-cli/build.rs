//! Builds the advisories, the LND release table and the dashboard into the
//! binary, so an installed `firewatch` needs no files from this repo.
//!
//! Inside the workspace the data comes from the repo root (`advisories/`,
//! `releases/lnd.toml`, `dashboard/dist`). A packaged crate has no repo
//! around it, so it carries a copy in `data/` (see scripts/package-data.sh). The dashboard is optional: build
//! it first (`cd dashboard && npm run build`) or the binary serves the API only.

use std::fmt::Write as _;
use std::path::{Path, PathBuf};

fn main() {
    let manifest = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let root = manifest.join("../..");
    let packaged = manifest.join("data");
    let (advisories, releases, dashboard) = if !root.join("advisories").is_dir() {
        (
            packaged.join("advisories"),
            packaged.join("releases/lnd.toml"),
            packaged.join("dashboard"),
        )
    } else {
        (
            root.join("advisories"),
            root.join("releases/lnd.toml"),
            root.join("dashboard/dist"),
        )
    };
    for path in [&advisories, &releases, &dashboard] {
        println!("cargo:rerun-if-changed={}", path.display());
    }

    let mut out = String::new();

    let mut files = list(&advisories);
    files.retain(|p| p.extension().is_some_and(|e| e == "toml"));
    assert!(
        !files.is_empty(),
        "no advisories in {}",
        advisories.display()
    );
    out.push_str("pub static ADVISORIES: &[(&str, &str)] = &[\n");
    for file in &files {
        let name = file.file_name().unwrap().to_string_lossy();
        writeln!(out, "    ({name:?}, include_str!({:?})),", abs(file)).unwrap();
    }
    out.push_str("];\n\n");

    assert!(releases.is_file(), "missing {}", releases.display());
    writeln!(
        out,
        "pub static LND_RELEASES: &str = include_str!({:?});\n",
        abs(&releases)
    )
    .unwrap();

    out.push_str("pub static DASHBOARD: &[(&str, &[u8])] = &[\n");
    if dashboard.join("index.html").is_file() {
        for file in list(&dashboard) {
            let rel = file
                .strip_prefix(&dashboard)
                .unwrap()
                .to_string_lossy()
                .replace('\\', "/");
            writeln!(out, "    ({rel:?}, include_bytes!({:?})),", abs(&file)).unwrap();
        }
    } else {
        println!(
            "cargo:warning=no dashboard at {}: the binary will serve the API only (run `npm run build` in dashboard/)",
            dashboard.display()
        );
    }
    out.push_str("];\n");

    let dest = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("embedded.rs");
    std::fs::write(dest, out).unwrap();
}

/// Every file under `dir`, sorted, recursively.
fn list(dir: &Path) -> Vec<PathBuf> {
    let mut found = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return found;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            found.extend(list(&path));
        } else {
            found.push(path);
        }
    }
    found.sort();
    found
}

fn abs(path: &Path) -> String {
    path.canonicalize().unwrap().to_string_lossy().into_owned()
}
