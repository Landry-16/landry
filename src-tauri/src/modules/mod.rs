//! Modules: what each kind of item can do.

pub mod cloud;
pub mod color;
pub mod dates;
pub mod money;
pub mod organize;
pub mod calc;
pub mod files;
pub mod projects;
pub mod scripts;
pub mod slash;
pub mod snippets;
pub mod system;
pub mod text;
pub mod tools;
pub mod web;

use crate::config::Settings;
use crate::engine::Outcome;
use crate::i18n::tf;
use crate::search::{Action, Item};
use crate::util;
use serde::Serialize;

#[derive(Serialize, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Preview {
    None,
    Text { text: String },
    Image { src: String },
    Props { rows: Vec<(String, String)> },
}

/// Results computed from the query itself, shown before indexed matches.
pub fn query(enabled: &[String], settings: &Settings, selection: Option<&str>, q: &str) -> Vec<Item> {
    let mut out = Vec::new();
    if q.is_empty() {
        return out;
    }
    if enabled.iter().any(|m| m == "calc") {
        let mut found = dates::query(q);
        found.extend(money::query(settings, q));
        // The calculator would read "15h" as 15 hours: only when nothing else understood the query.
        if found.is_empty() {
            found = calc::query(q);
        }
        out.extend(found);
    }
    if enabled.iter().any(|m| m == "text") {
        out.extend(text::query(q, selection));
    }
    if enabled.iter().any(|m| m == "tools") {
        out.extend(tools::query(q));
    }
    if enabled.iter().any(|m| m == "system") {
        out.extend(system::query(settings, q));
    }
    if enabled.iter().any(|m| m == "cloud") {
        out.extend(cloud::query(settings, q));
    }
    if enabled.iter().any(|m| m == "web") {
        out.extend(web::query(settings, q));
        let (word, rest) = q.split_once(' ').map(|(a, b)| (a, b.trim())).unwrap_or((q, ""));
        if matches!(word, "tr" | "traduire" | "traduction") && !rest.is_empty() {
            let (lang, text) = web::translation_request(settings, rest);
            out.extend(cloud::cached(&format!("translate\t{lang}\t{text}"), settings, web::translate));
        }
    }
    out
}

/// Every action of an item, including the ones only computed on demand.
pub fn actions(item: &Item) -> Vec<Action> {
    let mut a = item.actions.clone();
    if item.module == "files" {
        a.extend(files::extra_actions(item));
    }
    a
}

pub fn run(settings: &Settings, item: &Item, action: &str, arg: &str) -> Result<Outcome, String> {
    match item.module.as_str() {
        "apps" => match action {
            "launch" => {
                crate::apps::launch(std::path::Path::new(&item.data))?;
                Ok(Outcome::Hide)
            }
            "reveal" => {
                util::open(std::path::Path::new(&item.data).parent().map(|p| p.to_string_lossy().into_owned()).unwrap_or_default().as_str())?;
                Ok(Outcome::Hide)
            }
            _ => Err(tf("action inconnue : {a}", &[("a", &action)])),
        },
        "files" => files::run(settings, item, action, arg),
        "projects" => projects::run(settings, item, action, arg),
        "system" => system::run(settings, item, action, arg),
        "cloud" => cloud::run(settings, item, action),
        "web" => web::run(item, action),
        "scripts" => scripts::run(settings, item, action, arg),
        "slash" => slash::run(item, action),
        "text" => match action {
            "paste" => Ok(Outcome::Paste { text: item.data.clone(), back: 0 }),
            "copy" => Ok(Outcome::Copy(item.data.clone())),
            _ => Err(tf("action inconnue : {a}", &[("a", &action)])),
        },
        "calc" => match action {
            "copy" => Ok(Outcome::Copy(item.data.clone())),
            "settings" => Ok(Outcome::Settings),
            _ => Err(item.subtitle.clone()),
        },
        "tools" => match action {
            "copy" => Ok(Outcome::Copy(item.data.clone())),
            "fill" => Ok(Outcome::Fill(item.data.clone())),
            "view" => Ok(Outcome::View(item.data.clone())),
            "pick" => Ok(Outcome::Pick),
            "capture" => Ok(Outcome::Capture("save".into())),
            "ocr" => Ok(Outcome::Capture("ocr".into())),
            "qr-save" => {
                let svg = tools::qr_svg(&item.data)?;
                let dir = util::output("xdg-user-dir", &["DOWNLOAD"], None).map(|s| s.trim().to_string()).unwrap_or_default();
                let dir = if dir.is_empty() { crate::paths::expand("~") } else { std::path::PathBuf::from(dir) };
                let path = dir.join(format!("qr-{}.svg", chrono::Local::now().format("%Y%m%d-%H%M%S")));
                std::fs::write(&path, svg).map_err(|e| format!("{}: {e}", path.display()))?;
                Ok(Outcome::Message(tf("QR code enregistré : {p}", &[("p", &path.display())])))
            }
            "" => Err(item.subtitle.clone()),
            _ => Err(tf("action inconnue : {a}", &[("a", &action)])),
        },
        m => Err(tf("module inconnu : {m}", &[("m", &m)])),
    }
}

pub fn preview(item: &Item) -> Preview {
    match item.module.as_str() {
        "files" => files::preview(item),
        "projects" => projects::preview(item),
        "tools" if item.id == "tool:qr" => match tools::qr_svg(&item.data) {
            Ok(svg) => {
                use base64::Engine as _;
                Preview::Image { src: format!("data:image/svg+xml;base64,{}", base64::engine::general_purpose::STANDARD.encode(svg)) }
            }
            Err(e) => Preview::Text { text: e },
        },
        "tools" if !item.data.is_empty() && item.actions.iter().any(|a| a.id == "copy") => Preview::Text { text: item.data.clone() },
        _ => Preview::None,
    }
}
