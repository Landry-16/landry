//! Everything the bar can find and do, behind one lock.

use crate::apps::App;
use crate::config::{Preset, Settings, Store};
use crate::search::{Item, Ranker};
use crate::usage;
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;

/// What the frontend should do after an activation.
#[derive(Serialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Outcome {
    Hide,
    Keep,
    Settings,
}

pub struct Engine {
    pub store: Store,
    pub preset: Preset,
    pub settings: Settings,
    /// Shown once in the bar: broken preset or settings file.
    pub warning: Option<String>,
    apps: Vec<Item>,
    app_paths: HashMap<String, PathBuf>,
    db: Option<usage::Db>,
    frecency: HashMap<String, f64>,
    ranker: Ranker,
    /// Set by the `quit` command; the caller exits the process.
    pub quit_requested: bool,
}

impl Engine {
    pub fn new(store: Store, db: Option<usage::Db>) -> Engine {
        if let Err(e) = store.seed() {
            eprintln!("landry: création des presets impossible : {e}");
        }
        let frecency = db.as_ref().and_then(|d| d.frecency(usage::now()).ok()).unwrap_or_default();
        let mut e = Engine {
            store,
            preset: Preset::default(),
            settings: Settings::default(),
            warning: None,
            apps: Vec::new(),
            app_paths: HashMap::new(),
            db,
            frecency,
            ranker: Ranker::default(),
            quit_requested: false,
        };
        e.reload_config();
        e
    }

    pub fn reload_config(&mut self) {
        let (settings, w1) = self.store.settings();
        let (preset, w2) = self.store.active();
        self.settings = settings;
        self.preset = preset;
        self.warning = w2.or(w1);
    }

    pub fn set_apps(&mut self, apps: Vec<App>, icons: &HashMap<String, String>) {
        self.app_paths = apps.iter().map(|a| (format!("app:{}", a.id), a.path.clone())).collect();
        self.apps = apps
            .into_iter()
            .map(|a| {
                let mut keywords = a.keywords;
                keywords.push(a.exec_name);
                if !a.generic.is_empty() {
                    keywords.push(a.generic.clone());
                }
                Item {
                    id: format!("app:{}", a.id),
                    module: "apps".into(),
                    subtitle: if a.comment.is_empty() { a.generic } else { a.comment },
                    title: a.name,
                    icon: icons.get(&a.icon).cloned().unwrap_or_default(),
                    keywords,
                }
            })
            .collect();
    }

    fn command_items(&self) -> Vec<Item> {
        let cmd = |id: &str, title: &str, subtitle: &str, kw: &[&str]| Item {
            id: format!("cmd:{id}"),
            module: "commands".into(),
            title: title.into(),
            subtitle: subtitle.into(),
            icon: String::new(),
            keywords: kw.iter().map(|s| s.to_string()).collect(),
        };
        let mut out = vec![
            cmd("settings", "Réglages Landry", "Presets, apparence, alias", &["settings", "preferences", "config"]),
            cmd("reload", "Recharger la configuration", "Relit les presets et les réglages", &["reload"]),
            cmd("quit", "Quitter Landry", "Arrête le processus résident", &["quit", "exit"]),
        ];
        for info in self.store.list().into_iter().filter(|i| i.error.is_none()) {
            let active = info.name == self.preset.name;
            out.push(cmd(
                &format!("preset:{}", info.name),
                &format!("Preset : {}", info.name),
                if active { "Preset actif" } else { "Changer de preset" },
                &["preset", "theme", "apparence"],
            ));
        }
        out
    }

    /// Candidate items in module display order; disabled modules are left out.
    fn candidates(&self) -> Vec<Item> {
        let mut out = Vec::new();
        for m in self.preset.module_order().into_iter().filter(|m| m.enabled) {
            match m.id.as_str() {
                "commands" => out.extend(self.command_items()),
                "apps" => out.extend(self.apps.iter().cloned()),
                _ => {}
            }
        }
        out
    }

    pub fn search(&mut self, query: &str) -> Vec<Item> {
        let items = self.candidates();
        let limit = self.preset.max_results as usize;
        let mut ranked: Vec<Item> =
            self.ranker.rank(&items, query, &self.frecency, limit).into_iter().map(|i| items[i].clone()).collect();
        // An alias puts its target first, whatever the score.
        if let Some(target) = self.settings.aliases.get(query.trim()) {
            if let Some(it) = items.iter().find(|i| &i.id == target) {
                ranked.retain(|i| i.id != it.id);
                ranked.insert(0, it.clone());
                ranked.truncate(limit);
            }
        }
        ranked
    }

    pub fn history(&self) -> Vec<String> {
        self.db.as_ref().and_then(|d| d.recent_queries(50).ok()).unwrap_or_default()
    }

    fn record(&mut self, id: &str, query: &str) {
        if let Some(db) = &self.db {
            let now = usage::now();
            let _ = db.record_use(id, now);
            let _ = db.push_query(query, now);
            *self.frecency.entry(id.to_string()).or_insert(0.0) += usage::weight(now, now);
        }
    }

    pub fn activate(&mut self, id: &str, query: &str) -> Result<Outcome, String> {
        let outcome = if let Some(path) = self.app_paths.get(id) {
            crate::apps::launch(path)?;
            Outcome::Hide
        } else if let Some(name) = id.strip_prefix("cmd:preset:") {
            self.use_preset(name)?;
            Outcome::Keep
        } else {
            match id {
                "cmd:settings" => Outcome::Settings,
                "cmd:reload" => {
                    self.reload_config();
                    Outcome::Keep
                }
                "cmd:quit" => {
                    self.quit_requested = true;
                    Outcome::Hide
                }
                _ => return Err(format!("élément inconnu : {id}")),
            }
        };
        self.record(id, query);
        Ok(outcome)
    }

    /// Best match for a query, as `landry run <query>` does.
    pub fn run_query(&mut self, query: &str) -> Result<(String, Outcome), String> {
        let first = self.search(query).into_iter().next().ok_or_else(|| format!("aucun résultat pour « {query} »"))?;
        let outcome = self.activate(&first.id, query)?;
        Ok((first.title, outcome))
    }

    pub fn use_preset(&mut self, name: &str) -> Result<(), String> {
        self.preset = self.store.set_active(name)?;
        self.warning = None;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn engine() -> (Engine, PathBuf) {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("landry-engine-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let db = usage::Db::open(&dir.join("landry.db")).ok();
        let mut e = Engine::new(Store::new(dir.join("config")), db);
        let app = |id: &str, name: &str, exec: &str| App {
            id: id.into(),
            path: PathBuf::from(format!("/nonexistent/{id}")),
            name: name.into(),
            generic: String::new(),
            comment: String::new(),
            keywords: vec![],
            exec_name: exec.into(),
            icon: String::new(),
        };
        e.set_apps(vec![app("code.desktop", "Visual Studio Code", "code"), app("t.desktop", "Terminal", "gnome-terminal")], &HashMap::new());
        (e, dir)
    }

    #[test]
    fn finds_apps_and_commands() {
        let (mut e, dir) = engine();
        assert_eq!(e.search("vsc")[0].id, "app:code.desktop");
        assert_eq!(e.search("dense")[0].id, "cmd:preset:Dense");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn disabled_module_is_hidden_and_max_results_applies() {
        let (mut e, dir) = engine();
        e.preset.modules.iter_mut().for_each(|m| m.enabled = m.id != "apps");
        assert!(e.search("terminal").iter().all(|i| i.module != "apps"));
        e.preset.max_results = 1;
        assert_eq!(e.search("preset").len(), 1);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn alias_wins_and_preset_switch_persists() {
        let (mut e, dir) = engine();
        e.settings.aliases.insert("t".into(), "app:t.desktop".into());
        assert_eq!(e.search("t")[0].id, "app:t.desktop");
        assert_eq!(e.activate("cmd:preset:Minimal", "min").unwrap(), Outcome::Keep);
        assert_eq!(e.preset.name, "Minimal");
        assert_eq!(e.store.settings().0.active_preset, "Minimal");
        assert!(e.history().contains(&"min".to_string()));
        std::fs::remove_dir_all(dir).unwrap();
    }
}
