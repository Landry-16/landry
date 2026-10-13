//! Strings produced by the core, in the language chosen in settings. Same dictionary as the
//! interface (src/i18n/en.json): the French text is the key, English the value.

use std::collections::HashMap;
use std::fmt::Display;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::LazyLock;

static EN: LazyLock<HashMap<String, String>> =
    LazyLock::new(|| serde_json::from_str(include_str!("../../src/i18n/en.json")).unwrap_or_default());

static ENGLISH: AtomicBool = AtomicBool::new(false);

pub fn set_locale(locale: &str) {
    ENGLISH.store(locale == "en", Ordering::Relaxed);
}

pub fn english() -> bool {
    ENGLISH.load(Ordering::Relaxed)
}

/// t("Lancer") -> "Launch" in English.
pub fn t(key: &str) -> String {
    if english() {
        if let Some(v) = EN.get(key) {
            return v.clone();
        }
    }
    key.to_string()
}

/// tf("Minuteur lancé : {d}", &[("d", &secs)])
pub fn tf(key: &str, vars: &[(&str, &dyn Display)]) -> String {
    vars.iter().fold(t(key), |s, (k, v)| s.replace(&format!("{{{k}}}"), &v.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_core_key_has_an_english_string() {
        // Literal keys of t("…") and tf("…", …) in the core sources.
        // Also fixed action titles and prompts, translated on the way out by the engine.
        let re = regex::Regex::new(
            r#"(?:\bt[f]?\(|Action::new\(\s*"[^"]*",|\.ask\()\s*"((?:[^"\\]|\\.)*)""#,
        )
        .unwrap();
        let mut missing = Vec::new();
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/src");
        let mut files = vec![std::path::PathBuf::from(dir)];
        while let Some(p) = files.pop() {
            if p.is_dir() {
                files.extend(std::fs::read_dir(&p).unwrap().flatten().map(|e| e.path()));
            } else if p.extension().is_some_and(|e| e == "rs") && !p.ends_with("i18n.rs") {
                let src = std::fs::read_to_string(&p).unwrap();
                for c in re.captures_iter(&src) {
                    let key = c[1].replace("\\n", "\n").replace("\\\"", "\"");
                    if !EN.contains_key(&key) {
                        missing.push(format!("{}: {key}", p.file_name().unwrap().to_string_lossy()));
                    }
                }
            }
        }
        missing.sort();
        missing.dedup();
        assert!(missing.is_empty(), "traductions manquantes :\n{}", missing.join("\n"));
    }

    #[test]
    fn placeholders() {
        assert_eq!(tf("{a} et {b}", &[("a", &1), ("b", &"x")]), "1 et x");
    }
}
