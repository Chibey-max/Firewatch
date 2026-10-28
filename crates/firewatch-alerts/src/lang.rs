//! The fixed words of an alert, per language (task B6).
//!
//! Only the short phrases Firewatch writes itself are here. The developers'
//! fixes, advisory titles and the engine's reason sentences stay in English,
//! word for word: that's the promise, and they aren't ours to translate.
//!
//! A language other than English is **not offered** until a native speaker
//! has reviewed every phrase (`reviewed: true`, with their name in
//! `reviewer`). Until then it can only be tried as a draft, and every alert
//! says so. Placeholders such as `{node}` are filled in by [`fill`] and must
//! survive translation unchanged; a test checks that.

use serde::Deserialize;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    #[default]
    En,
    Ha,
    Yo,
    Ig,
    Pcm,
}

impl Language {
    pub const ALL: [Language; 5] = [
        Language::En,
        Language::Ha,
        Language::Yo,
        Language::Ig,
        Language::Pcm,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Language::En => "en",
            Language::Ha => "ha",
            Language::Yo => "yo",
            Language::Ig => "ig",
            Language::Pcm => "pcm",
        }
    }

    pub fn parse(code: &str) -> Option<Language> {
        Language::ALL
            .into_iter()
            .find(|l| l.code().eq_ignore_ascii_case(code.trim()))
    }

    pub fn strings(self) -> &'static Strings {
        match self {
            Language::En => &EN,
            Language::Ha => &HA,
            Language::Yo => &YO,
            Language::Ig => &IG,
            Language::Pcm => &PCM,
        }
    }
}

/// Every fixed phrase in an alert. Adding a field forces every language to
/// fill it in, so no language can silently fall behind.
#[derive(Debug)]
pub struct Strings {
    /// The language's name in itself, for menus.
    pub name: &'static str,
    /// The language's name in English.
    pub english_name: &'static str,
    /// True once a native speaker has reviewed every phrase below.
    pub reviewed: bool,
    /// Who reviewed it, credited in the README. Empty until then.
    pub reviewer: &'static str,

    /// `{n}` findings are red.
    pub affected_count: &'static str,
    /// `{n}` findings are amber.
    pub cant_tell_count: &'static str,
    /// The label in front of a red finding.
    pub affected_label: &'static str,
    /// The label in front of an amber finding.
    pub cant_tell_label: &'static str,
    /// First line of a one-finding alert: `{node}` and its `{label}`.
    pub node_status: &'static str,
    /// First line when every change on `{node}` is a fix: `{ids}` are the
    /// advisories that no longer apply.
    pub no_longer_affected: &'static str,
    /// A line added to a red alert when other advisories were fixed.
    pub also_no_longer: &'static str,
    /// `{node}` couldn't be read for several checks in a row.
    pub cant_read: &'static str,
    /// `{node}` can be read again.
    pub readable_again: &'static str,
    /// Before the developers' own words, which stay in English.
    pub developers_say: &'static str,
    /// Before a check the operator has to do themselves.
    pub check_by_hand: &'static str,
    /// Before the source link.
    pub source: &'static str,
    /// Before a list of advisories that share one reason.
    pub advisories: &'static str,
    /// A finding's advisory is being used in attacks.
    pub exploited: &'static str,
    /// Severity words: low, medium, high, critical.
    pub severity: [&'static str; 4],
    /// The whole `firewatch test-alert` message.
    pub test_alert: &'static str,
    /// First line of a message sent after a restart (A4): `{since}` is when
    /// Firewatch last checked before it stopped.
    pub catch_up: &'static str,
    /// Last line of a message that waited for a connection (A2).
    pub delayed: &'static str,
}

pub static EN: Strings = Strings {
    name: "English",
    english_name: "English",
    reviewed: true,
    reviewer: "",
    affected_count: "{n} affected",
    cant_tell_count: "{n} can't tell",
    affected_label: "AFFECTED",
    cant_tell_label: "CAN'T TELL",
    node_status: "{node} is {label}",
    no_longer_affected: "{node} is no longer affected by {ids}",
    also_no_longer: "No longer affected by {ids}",
    cant_read: "can't read {node}, so it can't be checked.",
    readable_again: "{node} can be read again.",
    developers_say: "Developers say",
    check_by_hand: "Check by hand",
    source: "Source",
    advisories: "Advisories",
    exploited: "exploited in the wild",
    severity: ["low", "medium", "high", "critical"],
    test_alert: "Firewatch test alert: if you can read this, alerts reach you.",
    catch_up: "While Firewatch was off (since {since}):",
    delayed: "Detected {detected}, sent {sent} (no connection until then).",
};

// The four drafts below are first drafts by Dave, waiting for a native
// speaker. Don't set `reviewed: true` without one.

pub static HA: Strings = Strings {
    name: "Hausa",
    english_name: "Hausa",
    reviewed: false,
    reviewer: "",
    affected_count: "{n} sun shafa",
    cant_tell_count: "{n} ba a tabbatar ba",
    affected_label: "AN SHAFA",
    cant_tell_label: "BA A TABBATAR BA",
    node_status: "{node}: {label}",
    no_longer_affected: "{ids} ba ya ƙara shafar {node}",
    also_no_longer: "Ba ya ƙara shafa ba: {ids}",
    cant_read: "ba a iya karanta {node} ba, don haka ba za a iya duba shi ba.",
    readable_again: "an sake iya karanta {node}.",
    developers_say: "Masu haɓakawa sun ce",
    check_by_hand: "Duba da kanka",
    source: "Tushe",
    advisories: "Sanarwar tsaro",
    exploited: "ana amfani da shi wajen kai hari",
    severity: ["ƙarami", "matsakaici", "babba", "mai tsanani"],
    test_alert: "Gwajin Firewatch: idan kana iya karanta wannan, saƙonni suna isowa gare ka.",
    catch_up: "Lokacin da Firewatch ke kashe (tun {since}):",
    delayed: "An gano a {detected}, an aika a {sent} (babu haɗi har sai lokacin).",
};

pub static YO: Strings = Strings {
    name: "Yorùbá",
    english_name: "Yoruba",
    reviewed: false,
    reviewer: "",
    affected_count: "{n} ló kàn án",
    cant_tell_count: "{n} a kò lè sọ",
    affected_label: "Ó KÀN ÁN",
    cant_tell_label: "A KÒ LÈ SỌ",
    node_status: "{node}: {label}",
    no_longer_affected: "{ids} kò kan {node} mọ́",
    also_no_longer: "Kò kàn án mọ́: {ids}",
    cant_read: "a kò lè ka {node}, nítorí náà a kò lè ṣàyẹ̀wò rẹ̀.",
    readable_again: "a tún lè ka {node} báyìí.",
    developers_say: "Àwọn olùgbéejáde sọ pé",
    check_by_hand: "Ṣàyẹ̀wò fúnra rẹ",
    source: "Orísun",
    advisories: "Àwọn ìkìlọ̀ ààbò",
    exploited: "wọ́n ń lò ó láti kọlu àwọn node",
    severity: ["kékeré", "àárín", "gíga", "tó le gan-an"],
    test_alert: "Ìdánwò Firewatch: tí o bá lè ka èyí, àwọn ìkìlọ̀ ń dé ọ̀dọ̀ rẹ.",
    catch_up: "Nígbà tí Firewatch wà ní pípa (láti {since}):",
    delayed: "A rí i ní {detected}, a fi ránṣẹ́ ní {sent} (kò sí ìsopọ̀ títí di ìgbà náà).",
};

pub static IG: Strings = Strings {
    name: "Igbo",
    english_name: "Igbo",
    reviewed: false,
    reviewer: "",
    affected_count: "{n} metụtara ya",
    cant_tell_count: "{n} enweghị ike ịma",
    affected_label: "Ọ METỤTARA YA",
    cant_tell_label: "ENWEGHỊ IKE ỊMA",
    node_status: "{node}: {label}",
    no_longer_affected: "{ids} anaghịzi emetụta {node}",
    also_no_longer: "Anaghịzi emetụta ya: {ids}",
    cant_read: "enweghị ike ịgụ {node}, ya mere enweghị ike inyocha ya.",
    readable_again: "a nwere ike ịgụ {node} ọzọ.",
    developers_say: "Ndị mmepe kwuru",
    check_by_hand: "Nyochaa n'onwe gị",
    source: "Isi mmalite",
    advisories: "Ndụmọdụ nchekwa",
    exploited: "a na-eji ya awakpo",
    severity: ["obere", "etiti", "elu", "dị oke njọ"],
    test_alert: "Nnwale Firewatch: ọ bụrụ na ị nwere ike ịgụ nke a, ọkwa na-eru gị aka.",
    catch_up: "Mgbe Firewatch gbanyụrụ (kemgbe {since}):",
    delayed: "Achọpụtara ya na {detected}, ezitere ya na {sent} (enweghị njikọ ruo mgbe ahụ).",
};

pub static PCM: Strings = Strings {
    name: "Naijá",
    english_name: "Nigerian Pidgin",
    reviewed: false,
    reviewer: "",
    affected_count: "{n} dey affect am",
    cant_tell_count: "{n} we no fit talk",
    affected_label: "E AFFECT AM",
    cant_tell_label: "WE NO FIT TALK",
    node_status: "{node}: {label}",
    no_longer_affected: "{ids} no dey affect {node} again",
    also_no_longer: "E no dey affect am again: {ids}",
    cant_read: "we no fit read {node}, so we no fit check am.",
    readable_again: "we fit read {node} again.",
    developers_say: "Di developers talk say",
    check_by_hand: "Check am by yourself",
    source: "Where e come from",
    advisories: "Security warnings",
    exploited: "people don dey use am attack",
    severity: ["small", "medium", "high", "serious well well"],
    test_alert: "Firewatch test: if you fit read dis one, alerts dey reach you.",
    catch_up: "When Firewatch off (since {since}):",
    delayed: "We see am {detected}, we send am {sent} (network no dey until then).",
};

/// Replaces each `{key}` in `template` with its value.
pub fn fill(template: &str, values: &[(&str, &str)]) -> String {
    let mut out = template.to_string();
    for (key, value) in values {
        out = out.replace(&format!("{{{key}}}"), value);
    }
    out
}

impl Strings {
    /// Every phrase with its key, in a fixed order, for the strings sheet
    /// (B1) and the placeholder test.
    pub fn phrases(&self) -> Vec<(&'static str, String)> {
        let mut out = vec![
            ("affected_count", self.affected_count.to_string()),
            ("cant_tell_count", self.cant_tell_count.to_string()),
            ("affected_label", self.affected_label.to_string()),
            ("cant_tell_label", self.cant_tell_label.to_string()),
            ("node_status", self.node_status.to_string()),
            ("no_longer_affected", self.no_longer_affected.to_string()),
            ("also_no_longer", self.also_no_longer.to_string()),
            ("cant_read", self.cant_read.to_string()),
            ("readable_again", self.readable_again.to_string()),
            ("developers_say", self.developers_say.to_string()),
            ("check_by_hand", self.check_by_hand.to_string()),
            ("source", self.source.to_string()),
            ("advisories", self.advisories.to_string()),
            ("exploited", self.exploited.to_string()),
        ];
        for (key, word) in [
            "severity_low",
            "severity_medium",
            "severity_high",
            "severity_critical",
        ]
        .into_iter()
        .zip(self.severity)
        {
            out.push((key, word.to_string()));
        }
        out.extend([
            ("test_alert", self.test_alert.to_string()),
            ("catch_up", self.catch_up.to_string()),
            ("delayed", self.delayed.to_string()),
        ]);
        out
    }
}

/// The `{placeholders}` in a phrase, sorted.
pub fn placeholders(phrase: &str) -> Vec<String> {
    let mut found: Vec<String> = phrase
        .split('{')
        .skip(1)
        .filter_map(|rest| rest.split_once('}').map(|(name, _)| name.to_string()))
        .collect();
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_keeps_every_placeholder() {
        let english = EN.phrases();
        for lang in Language::ALL {
            let phrases = lang.strings().phrases();
            assert_eq!(phrases.len(), english.len());
            for ((key, en), (other_key, other)) in english.iter().zip(&phrases) {
                assert_eq!(key, other_key);
                assert_eq!(
                    placeholders(en),
                    placeholders(other),
                    "{} {key}: {other:?}",
                    lang.code()
                );
                assert!(!other.trim().is_empty(), "{} {key} is empty", lang.code());
            }
        }
    }

    #[test]
    fn a_reviewed_language_names_its_reviewer() {
        for lang in Language::ALL.into_iter().filter(|l| *l != Language::En) {
            let s = lang.strings();
            assert_eq!(s.reviewed, !s.reviewer.is_empty(), "{}", lang.code());
        }
    }

    #[test]
    fn parses_codes_and_fills_placeholders() {
        assert_eq!(Language::parse("HA"), Some(Language::Ha));
        assert_eq!(Language::parse("pcm"), Some(Language::Pcm));
        assert_eq!(Language::parse("fr"), None);
        assert_eq!(
            fill(
                EN.no_longer_affected,
                &[("node", "old-cln"), ("ids", "FW-1")]
            ),
            "old-cln is no longer affected by FW-1"
        );
    }
}
