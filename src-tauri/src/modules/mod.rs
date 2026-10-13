//! Modules: what each kind of item can do.

pub mod cloud;
pub mod color;
pub mod dates;
pub mod money;
pub mod calc;
pub mod files;
pub mod projects;
pub mod snippets;
pub mod system;
pub mod text;
pub mod tools;

use crate::config::Settings;
use crate::engine::Outcome;
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
            _ => Err(format!("action inconnue : {action}")),
        },
        "files" => files::run(settings, item, action, arg),
        "projects" => projects::run(settings, item, action, arg),
        "system" => system::run(settings, item, action, arg),
        "cloud" => cloud::run(settings, item, action),
        "text" => match action {
            "paste" => Ok(Outcome::Paste { text: item.data.clone(), back: 0 }),
            "copy" => Ok(Outcome::Copy(item.data.clone())),
            _ => Err(format!("action inconnue : {action}")),
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
            "" => Err(item.subtitle.clone()),
            _ => Err(format!("action inconnue : {action}")),
        },
        m => Err(format!("module inconnu : {m}")),
    }
}

pub fn preview(item: &Item) -> Preview {
    match item.module.as_str() {
        "files" => files::preview(item),
        "projects" => projects::preview(item),
        "tools" if !item.data.is_empty() && item.actions.iter().any(|a| a.id == "copy") => Preview::Text { text: item.data.clone() },
        _ => Preview::None,
    }
}
