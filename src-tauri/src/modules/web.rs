//! Web searches by prefix ("gh landry"), browser bookmarks (Firefox, Chrome, Chromium, Brave)
//! and translation through a configurable API.

use crate::config::Settings;
use crate::engine::Outcome;
use crate::i18n::{t, tf};
use crate::search::{Action, Item};
use crate::util;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::path::{Path, PathBuf};

pub fn search_url(template: &str, text: &str) -> String {
    template.replace("{q}", &utf8_percent_encode(text, NON_ALPHANUMERIC).to_string())
}

fn host(url: &str) -> String {
    url::Url::parse(url).ok().and_then(|u| u.host_str().map(|h| h.trim_start_matches("www.").to_string())).unwrap_or_default()
}

fn open_item(id: String, title: String, subtitle: String, url: String) -> Item {
    Item {
        id,
        module: "web".into(),
        title,
        subtitle,
        actions: vec![Action::new("open", "Ouvrir"), Action::new("copy", "Copier le lien")],
        data: url,
        ..Default::default()
    }
}

pub fn query(settings: &Settings, q: &str) -> Vec<Item> {
    let (word, rest) = q.split_once(' ').map(|(a, b)| (a, b.trim())).unwrap_or((q, ""));
    if let Some(tpl) = settings.search_prefixes.get(word) {
        if rest.is_empty() {
            return vec![Item {
                id: format!("web:hint:{word}"),
                module: "web".into(),
                title: format!("{word} <texte>"),
                subtitle: tf("Recherche sur {h}", &[("h", &host(tpl))]),
                ..Default::default()
            }];
        }
        let url = search_url(tpl, rest);
        return vec![open_item(format!("web:search:{word}"), tf("Chercher « {q} »", &[("q", &rest)]), tf("sur {h}", &[("h", &host(tpl))]), url)];
    }
    // A typed address opens directly.
    if !q.contains(' ') && (q.starts_with("http://") || q.starts_with("https://")) && url::Url::parse(q).is_ok() {
        return vec![open_item("web:url".into(), tf("Ouvrir {u}", &[("u", &q)]), t("Dans le navigateur"), q.to_string())];
    }
    vec![]
}

// ---------- Bookmarks ----------

fn firefox_places() -> Vec<PathBuf> {
    let home = crate::paths::expand("~");
    let mut out = Vec::new();
    for root in [home.join(".mozilla/firefox"), home.join("snap/firefox/common/.mozilla/firefox")] {
        if let Ok(rd) = std::fs::read_dir(root) {
            out.extend(rd.flatten().map(|e| e.path().join("places.sqlite")).filter(|p| p.exists()));
        }
    }
    out
}

/// Firefox keeps places.sqlite locked while running: read a temporary copy, never the original.
fn firefox_bookmarks(places: &Path) -> Vec<(String, String)> {
    let tmp = std::env::temp_dir().join(format!("landry-places-{}.sqlite", std::process::id()));
    if std::fs::copy(places, &tmp).is_err() {
        return vec![];
    }
    let read = || -> rusqlite::Result<Vec<(String, String)>> {
        let conn = rusqlite::Connection::open_with_flags(&tmp, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        let mut stmt = conn.prepare(
            "SELECT b.title, p.url FROM moz_bookmarks b JOIN moz_places p ON b.fk = p.id
             WHERE b.type = 1 AND b.title IS NOT NULL AND b.title != '' AND p.url LIKE 'http%'",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        rows.collect()
    };
    let out = read().unwrap_or_default();
    let _ = std::fs::remove_file(&tmp);
    out
}

fn chromium_files() -> Vec<PathBuf> {
    let home = crate::paths::expand("~");
    let roots = [
        home.join(".config/google-chrome"),
        home.join(".config/chromium"),
        home.join("snap/chromium/common/chromium"),
        home.join(".config/BraveSoftware/Brave-Browser"),
    ];
    let mut out = Vec::new();
    for root in roots {
        if let Ok(rd) = std::fs::read_dir(root) {
            out.extend(rd.flatten().map(|e| e.path().join("Bookmarks")).filter(|p| p.exists()));
        }
    }
    out
}

pub fn chromium_bookmarks(json: &str) -> Vec<(String, String)> {
    fn walk(v: &serde_json::Value, out: &mut Vec<(String, String)>) {
        if v.get("type").and_then(|t| t.as_str()) == Some("url") {
            if let (Some(n), Some(u)) = (v.get("name").and_then(|n| n.as_str()), v.get("url").and_then(|u| u.as_str())) {
                out.push((n.to_string(), u.to_string()));
            }
        }
        if let Some(children) = v.get("children").and_then(|c| c.as_array()) {
            children.iter().for_each(|c| walk(c, out));
        }
        if let Some(roots) = v.get("roots").and_then(|r| r.as_object()) {
            roots.values().for_each(|r| walk(r, out));
        }
    }
    let mut out = Vec::new();
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json) {
        walk(&v, &mut out);
    }
    out
}

pub fn bookmarks() -> Vec<Item> {
    let mut all: Vec<(String, String, &str)> = Vec::new();
    for p in firefox_places() {
        all.extend(firefox_bookmarks(&p).into_iter().map(|(t, u)| (t, u, "Firefox")));
    }
    for p in chromium_files() {
        let json = std::fs::read_to_string(&p).unwrap_or_default();
        all.extend(chromium_bookmarks(&json).into_iter().map(|(t, u)| (t, u, "Chrome")));
    }
    all.sort_by(|a, b| a.1.cmp(&b.1));
    all.dedup_by(|a, b| a.1 == b.1);
    all.into_iter()
        .map(|(title, url, browser)| {
            let mut it = open_item(format!("bm:{url}"), title, tf("Favori {b} · {h}", &[("b", &browser), ("h", &host(&url))]), url.clone());
            it.keywords = vec![host(&url), "favori".into()];
            it
        })
        .collect()
}

// ---------- Translation ----------

const LANGS: &[&str] = &["fr", "en", "de", "es", "it", "pt", "nl", "pl", "ru", "ja", "zh", "ko", "ar", "uk", "sv", "tr"];

/// "tr en bonjour" -> ("EN", "bonjour"); without a code, the default target.
pub fn translation_request<'a>(settings: &Settings, rest: &'a str) -> (String, &'a str) {
    match rest.split_once(' ') {
        Some((code, text)) if LANGS.contains(&code.to_lowercase().as_str()) && !text.trim().is_empty() => (code.to_uppercase(), text.trim()),
        _ => (settings.translate_target.to_uppercase(), rest),
    }
}

/// Runs in the background (see `cloud::cached`); key is "translate\tLANG\ttext".
pub fn translate(settings: &Settings, key: &str) -> Result<Vec<Item>, String> {
    let mut parts = key.splitn(3, '\t');
    let (_, lang, text) = (parts.next(), parts.next().unwrap_or("EN"), parts.next().unwrap_or(""));
    let client = reqwest::blocking::Client::builder()
        .user_agent("landry")
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())?;
    let translated = if !settings.translate_url.trim().is_empty() {
        // LibreTranslate-compatible server.
        let mut body = serde_json::json!({ "q": text, "source": "auto", "target": lang.to_lowercase(), "format": "text" });
        if let Some(k) = super::cloud::token("translate") {
            body["api_key"] = k.into();
        }
        let v: serde_json::Value = client.post(settings.translate_url.trim()).json(&body).send().map_err(|e| e.to_string())?.json().map_err(|e| e.to_string())?;
        v.get("translatedText").and_then(|t| t.as_str()).map(String::from).ok_or_else(|| v.get("error").and_then(|e| e.as_str()).map(str::to_string).unwrap_or_else(|| t("réponse inattendue")))?
    } else {
        let Some(k) = super::cloud::token("translate") else {
            return Ok(vec![Item {
                id: "tr:token".into(),
                module: "web".into(),
                title: t("Clé de traduction manquante"),
                subtitle: t("Réglages Landry, Comptes : clé DeepL (ou une URL LibreTranslate dans Général)"),
                actions: vec![Action::new("settings", "Ouvrir les réglages")],
                ..Default::default()
            }]);
        };
        // Free DeepL keys end with ":fx" and use another host.
        let host = if k.ends_with(":fx") { "https://api-free.deepl.com" } else { "https://api.deepl.com" };
        let r = client
            .post(format!("{host}/v2/translate"))
            .header("Authorization", format!("DeepL-Auth-Key {k}"))
            .json(&serde_json::json!({ "text": [text], "target_lang": lang }))
            .send()
            .map_err(|e| e.to_string())?;
        let status = r.status();
        let v: serde_json::Value = r.json().map_err(|e| e.to_string())?;
        if !status.is_success() {
            return Err(format!("DeepL {status} {}", v.get("message").and_then(|m| m.as_str()).unwrap_or("")));
        }
        v.pointer("/translations/0/text").and_then(|t| t.as_str()).map(String::from).ok_or_else(|| t("réponse DeepL inattendue"))?
    };
    Ok(vec![Item {
        id: format!("tr:{lang}"),
        module: "web".into(),
        title: translated.clone(),
        subtitle: tf("Traduction ({lang}) · Entrée pour copier", &[("lang", &lang)]),
        actions: vec![Action::new("copy", "Copier")],
        data: translated,
        ..Default::default()
    }])
}

pub fn run(item: &Item, action: &str) -> Result<Outcome, String> {
    match action {
        "open" => {
            util::open(&item.data)?;
            Ok(Outcome::Hide)
        }
        "copy" => Ok(Outcome::Copy(item.data.clone())),
        "settings" => Ok(Outcome::Settings),
        _ => Err(item.subtitle.clone()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prefixes() {
        let s = Settings::default();
        let r = query(&s, "gh landry tauri");
        assert_eq!(r[0].data, "https://github.com/search?q=landry%20tauri");
        assert_eq!(r[0].subtitle, "sur github.com");
        assert_eq!(query(&s, "wiki")[0].title, "wiki <texte>");
        assert_eq!(query(&s, "https://landry.dev")[0].data, "https://landry.dev");
        assert!(query(&s, "firefox").is_empty());
    }

    #[test]
    fn chromium_json() {
        let json = r#"{"roots": {"bookmark_bar": {"children": [
            {"type": "url", "name": "Tauri", "url": "https://tauri.app"},
            {"type": "folder", "name": "Dev", "children": [{"type": "url", "name": "MDN", "url": "https://developer.mozilla.org"}]}
        ]}}}"#;
        let b = chromium_bookmarks(json);
        assert_eq!(b, vec![("Tauri".into(), "https://tauri.app".into()), ("MDN".into(), "https://developer.mozilla.org".into())]);
        assert!(chromium_bookmarks("pas du json").is_empty());
    }

    #[test]
    fn translation_target() {
        let s = Settings::default();
        assert_eq!(translation_request(&s, "en bonjour le monde"), ("EN".into(), "bonjour le monde"));
        assert_eq!(translation_request(&s, "bonjour"), (s.translate_target.to_uppercase(), "bonjour"));
        assert_eq!(translation_request(&s, "es"), (s.translate_target.to_uppercase(), "es"));
    }
}
