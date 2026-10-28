//! Writes the alert strings sheet (task B1): one CSV per language, for its
//! native-speaker reviewer, straight from the code so the two never drift.
//!
//!   cargo run -p firewatch-alerts --example strings_sheet -- docs/strings

use std::path::PathBuf;

use firewatch_alerts::lang::{placeholders, Language, EN};

/// Where each phrase shows up, so the reviewer sees it in context.
fn context(key: &str) -> &'static str {
    match key {
        "affected_count" | "cant_tell_count" => "First line of a node's alert, after the node name: \"🔴 Firewatch: old-cln: 2 affected\"",
        "affected_label" | "cant_tell_label" => "In front of each finding, in capitals",
        "node_status" => "First line of a one-finding alert",
        "no_longer_affected" => "First line when a node is fixed: \"✅ Firewatch: …\"",
        "also_no_longer" => "Last line of a red alert when other advisories were fixed",
        "cant_read" => "After \"⚠️ Firewatch:\" when a node can't be read",
        "readable_again" => "After \"Firewatch:\" when the node can be read again",
        "developers_say" => "Before the developers' fix, which stays in English in quotes",
        "check_by_hand" => "Before a check the operator does themselves (the check stays in English)",
        "source" => "Before the link to the developers' notes",
        "advisories" => "Before a list of advisories",
        "exploited" => "Next to the severity: attackers are using this bug",
        k if k.starts_with("severity_") => "Severity word next to each finding",
        "test_alert" => "The whole message `firewatch test-alert` sends",
        "catch_up" => "First line after a restart, e.g. a power cut: {since} is a time like 14:02",
        "delayed" => "Last line of an alert that waited for a connection: times like 14:02",
        _ => "",
    }
}

fn csv(cell: &str) -> String {
    format!("\"{}\"", cell.replace('"', "\"\""))
}

fn main() {
    let dir = PathBuf::from(
        std::env::args()
            .nth(1)
            .unwrap_or_else(|| "docs/strings".into()),
    );
    std::fs::create_dir_all(&dir).unwrap();
    for lang in Language::ALL.into_iter().filter(|l| *l != Language::En) {
        let strings = lang.strings();
        let mut out = String::from(
            "key,english,where it appears,keep these placeholders,draft,reviewed (write the final text here),notes\n",
        );
        for ((key, english), (_, draft)) in EN.phrases().into_iter().zip(strings.phrases()) {
            let keep = placeholders(&english).join(" ");
            let reviewed = if strings.reviewed { draft.as_str() } else { "" };
            out.push_str(&format!(
                "{},{},{},{},{},{},\n",
                csv(key),
                csv(&english),
                csv(context(key)),
                csv(&keep
                    .split(' ')
                    .filter(|p| !p.is_empty())
                    .map(|p| format!("{{{p}}}"))
                    .collect::<Vec<_>>()
                    .join(" ")),
                csv(&draft),
                csv(reviewed),
            ));
        }
        let path = dir.join(format!("alerts-{}.csv", lang.code()));
        std::fs::write(&path, out).unwrap();
        println!("wrote {} ({})", path.display(), strings.english_name);
    }
}
