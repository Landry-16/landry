//! Open windows and clipboard history, through Landry's GNOME Shell extension (dev.landry.Shell).
//! Without the extension every call fails fast and Landry behaves as before.

use crate::engine::Outcome;
use crate::i18n::{t, tf};
use crate::search::{fold, Action, Item};
use crate::util;
use base64::Engine as _;
use serde::Deserialize;
use std::time::Duration;

#[cfg_attr(windows, allow(dead_code))]
pub const UUID: &str = "landry@landry-16.github.io";

fn call(method: &str, args: &[&str]) -> Result<String, String> {
    let m = format!("dev.landry.Shell.{method}");
    let mut a = vec!["call", "--session", "--dest", "org.gnome.Shell", "--object-path", "/dev/landry/Shell", "--method", &m];
    a.extend_from_slice(args);
    util::output_within("gdbus", &a, None, Duration::from_secs(2))
}

/// `('eyJ...',)` -> the JSON text the extension encoded in base64.
pub fn decode(reply: &str) -> Result<String, String> {
    let b64: String = reply.chars().skip_while(|c| *c != '\'').skip(1).take_while(|c| *c != '\'').collect();
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64.trim()).map_err(|e| e.to_string())?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Window {
    pub id: u32,
    pub title: String,
    /// Desktop file id of its application, e.g. "firefox_firefox.desktop".
    pub app: String,
    pub name: String,
    pub workspace: i32,
    pub focused: bool,
    pub minimized: bool,
}

#[derive(Deserialize, Debug, Clone, PartialEq)]
pub struct Clip {
    pub id: u32,
    pub text: String,
    pub time: i64,
}

#[derive(Deserialize, Debug, Default)]
pub struct ClipState {
    pub paused: bool,
    pub items: Vec<Clip>,
}

/// Open windows, most recently used first.
pub fn windows() -> Result<Vec<Window>, String> {
    serde_json::from_str(&decode(&call("Windows", &[])?)?).map_err(|e| e.to_string())
}

pub fn activate(id: u32) -> Result<(), String> {
    call("Activate", &[&id.to_string()]).map(|_| ())
}

/// Switch to the most recent window of this application, if it has one.
pub fn switch_to_app(desktop_id: &str) -> bool {
    windows().ok().and_then(|w| w.into_iter().find(|w| w.app == desktop_id)).is_some_and(|w| activate(w.id).is_ok())
}

#[cfg_attr(windows, allow(dead_code))]
pub fn available() -> bool {
    call("Windows", &[]).is_ok()
}

pub fn window_item(w: &Window) -> Item {
    let mut subtitle = if w.name.is_empty() { t("Fenêtre ouverte") } else { tf("{app} · fenêtre ouverte", &[("app", &w.name)]) };
    if w.minimized {
        subtitle.push_str(&format!(" · {}", t("réduite")));
    }
    Item {
        id: format!("win:{}", w.id),
        module: "windows".into(),
        title: if w.title.is_empty() { w.name.clone() } else { w.title.clone() },
        subtitle,
        // "window:12" is the Shell's id for a window without a .desktop file: not a useful keyword.
        keywords: [w.name.clone(), w.app.trim_end_matches(".desktop").to_string()].into_iter().filter(|k| !k.is_empty() && !k.starts_with("window:")).collect(),
        actions: vec![Action::new("activate", "Basculer"), Action::new("close", "Fermer la fenêtre")],
        data: format!("{}\t{}", w.id, w.app),
        ..Default::default()
    }
}

/// `/fenetres [texte]`: every open window, most recent first.
pub fn window_items(filter: &str) -> Vec<Item> {
    match windows() {
        Ok(list) => {
            let f = fold(filter);
            let out: Vec<Item> =
                list.iter().filter(|w| f.is_empty() || fold(&format!("{} {}", w.title, w.name)).contains(&f)).map(window_item).collect();
            if out.is_empty() {
                vec![info("win:none", t("Aucune fenêtre correspondante"), String::new())]
            } else {
                out
            }
        }
        Err(_) => vec![missing()],
    }
}

fn info(id: &str, title: String, subtitle: String) -> Item {
    Item { id: id.into(), module: "clipboard".into(), title, subtitle, ..Default::default() }
}

fn missing() -> Item {
    Item {
        id: "shell:missing".into(),
        module: "clipboard".into(),
        title: t("Extension GNOME de Landry inactive"),
        subtitle: t("Active-la dans /bienvenue, puis reconnecte-toi une fois"),
        actions: vec![Action::new("setup", "Ouvrir l'assistant")],
        ..Default::default()
    }
}

/// "il y a 3 min" from a Unix time.
pub fn ago(time: i64, now: i64) -> String {
    let s = (now - time).max(0);
    match s {
        0..=59 => t("à l'instant"),
        60..=3599 => tf("il y a {n} min", &[("n", &(s / 60))]),
        3600..=86399 => tf("il y a {n} h", &[("n", &(s / 3600))]),
        _ => tf("il y a {n} j", &[("n", &(s / 86400))]),
    }
}

/// `/presse-papiers [texte]`: copied texts, newest first, then pause and clear.
pub fn clipboard_items(filter: &str) -> Vec<Item> {
    let state: ClipState = match call("Clipboard", &[]).and_then(|r| decode(&r)) {
        Ok(json) => serde_json::from_str(&json).unwrap_or_default(),
        Err(_) => return vec![missing()],
    };
    let now = chrono::Local::now().timestamp();
    let f = fold(filter);
    let mut out: Vec<Item> = state
        .items
        .iter()
        .filter(|c| f.is_empty() || fold(&c.text).contains(&f))
        .map(|c| {
            let line: String = c.text.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(120).collect();
            Item {
                id: format!("clip:{}", c.id),
                module: "clipboard".into(),
                title: line,
                subtitle: tf("{ago} · {n} caractères", &[("ago", &ago(c.time, now)), ("n", &c.text.chars().count())]),
                actions: vec![Action::new("paste", "Coller"), Action::new("copy", "Copier"), Action::new("remove", "Retirer de l'historique")],
                data: format!("{}\t{}", c.id, c.text),
                ..Default::default()
            }
        })
        .collect();
    if out.is_empty() {
        out.push(info(
            "clip:empty",
            if state.paused { t("Historique en pause") } else { t("Historique vide") },
            t("Les textes copiés apparaîtront ici. Les secrets des gestionnaires de mots de passe sont ignorés."),
        ));
    }
    if filter.is_empty() {
        let (title, action) = if state.paused { ("Reprendre l'historique", "resume") } else { ("Mettre l'historique en pause", "pause") };
        out.push(Item { id: format!("clip:{action}"), module: "clipboard".into(), title: t(title), subtitle: t("Rien n'est enregistré pendant la pause"), actions: vec![Action::new(action, title)], ..Default::default() });
        out.push(Item {
            id: "clip:clear".into(),
            module: "clipboard".into(),
            title: t("Vider l'historique"),
            subtitle: t("L'historique n'est jamais écrit sur le disque"),
            actions: vec![Action::new("clear", "Vider l'historique").danger()],
            ..Default::default()
        });
    }
    out
}

pub fn run(item: &Item, action: &str) -> Result<Outcome, String> {
    let (id, rest) = item.data.split_once('\t').unwrap_or((&item.data, ""));
    match (item.module.as_str(), action) {
        ("windows", "activate") => {
            activate(id.parse().map_err(|_| t("fenêtre inconnue"))?)?;
            Ok(Outcome::Hide)
        }
        ("windows", "close") => {
            call("Close", &[id])?;
            Ok(Outcome::Message(t("Fenêtre fermée.")))
        }
        ("clipboard", "paste") => Ok(Outcome::Paste { text: rest.to_string(), back: 0 }),
        ("clipboard", "copy") => Ok(Outcome::Copy(rest.to_string())),
        ("clipboard", "remove") => {
            call("ClipboardRemove", &[id])?;
            Ok(Outcome::Message(t("Retiré de l'historique.")))
        }
        ("clipboard", "pause" | "resume") => {
            call("SetPaused", &[if action == "pause" { "true" } else { "false" }])?;
            Ok(Outcome::Message(if action == "pause" { t("Historique en pause.") } else { t("Historique repris.") }))
        }
        ("clipboard", "clear") => {
            call("ClipboardClear", &[])?;
            Ok(Outcome::Message(t("Historique vidé.")))
        }
        ("clipboard", "setup") => Ok(Outcome::View("setup".into())),
        _ => Err(tf("action inconnue : {a}", &[("a", &action)])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_gdbus_replies() {
        let json = r#"[{"id":7,"title":"Landry - GitHub","app":"firefox_firefox.desktop","name":"Firefox","workspace":0,"focused":false,"minimized":true}]"#;
        let reply = format!("('{}',)\n", base64::engine::general_purpose::STANDARD.encode(json));
        let w: Vec<Window> = serde_json::from_str(&decode(&reply).unwrap()).unwrap();
        assert_eq!(w[0].app, "firefox_firefox.desktop");
        let it = window_item(&w[0]);
        assert_eq!(it.title, "Landry - GitHub");
        assert!(it.subtitle.contains("Firefox") && it.subtitle.contains("réduite"));
        assert_eq!(it.data, "7\tfirefox_firefox.desktop");
    }

    #[test]
    fn clipboard_text_keeps_tabs_and_newlines() {
        let it = Item { module: "clipboard".into(), data: "3\tligne 1\n\tligne 2".into(), ..Default::default() };
        assert_eq!(run(&it, "copy").unwrap(), Outcome::Copy("ligne 1\n\tligne 2".into()));
    }

    #[test]
    fn relative_times() {
        assert_eq!(ago(100, 130), "à l'instant");
        assert_eq!(ago(0, 180), "il y a 3 min");
        assert_eq!(ago(0, 7200), "il y a 2 h");
    }
}
