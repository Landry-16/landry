//! Everything the bar can find and do, behind one lock.

use crate::apps::App;
use crate::config::{Preset, Settings, Store};
use crate::modules::{self, files, Preview};
use crate::search::{Action, Item, Ranker};
use crate::i18n::{t, tf};
use crate::usage;
use serde::Serialize;
use std::collections::HashMap;

/// What the frontend should do after an activation.
#[derive(Serialize, Debug, PartialEq)]
#[serde(tag = "kind", content = "text", rename_all = "lowercase")]
pub enum Outcome {
    Hide,
    Keep,
    Settings,
    /// Keep the bar open and show this text.
    Message(String),
    /// Put the text in the clipboard, then hide.
    Copy(String),
    /// Replace the query with this text (tool catalog).
    Fill(String),
    /// Open a frontend view (e.g. "diff").
    View(String),
    /// Hide, run the screen color picker, then show the picked color.
    Pick,
    /// Put the text in the clipboard, hide, and paste it in the previous app.
    Paste { text: String, back: usize },
    /// Ask these fields, then activate again with their values as a JSON object.
    Ask(Vec<String>),
    /// Hide, take an interactive screenshot through the portal: "save" or "ocr".
    Capture(String),
    /// Run a user script outside the engine lock; its output is shown when it ends.
    Run {
        program: String,
        args: Vec<String>,
        label: String,
        silent: bool,
        /// Killed after this many seconds.
        #[serde(skip)]
        limit: u64,
    },
}

pub struct Engine {
    pub store: Store,
    pub preset: Preset,
    pub settings: Settings,
    /// Shown once in the bar: broken preset or settings file.
    pub warning: Option<String>,
    /// Items kept up to date by background indexers, per module.
    index: HashMap<&'static str, Vec<Item>>,
    /// Last results sent to the UI, so activation and preview get the full item back.
    shown: HashMap<String, Item>,
    db: Option<usage::Db>,
    frecency: HashMap<String, f64>,
    ranker: Ranker,
    /// Set by the `quit` command; the caller exits the process.
    pub quit_requested: bool,
    /// Text selected in another app when the bar opened (primary selection).
    pub selection: Option<String>,
    /// Clipboard text captured just before a snippet is expanded.
    pub clipboard: String,
    pub(crate) timers: Vec<crate::organizer::Timer>,
    pub(crate) next_timer: u32,
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
            index: HashMap::new(),
            shown: HashMap::new(),
            db,
            frecency,
            ranker: Ranker::default(),
            quit_requested: false,
            selection: None,
            clipboard: String::new(),
            timers: Vec::new(),
            next_timer: 0,
        };
        e.reload_config();
        e.reload_snippets();
        e.reload_organizer();
        e.load_timers();
        e
    }

    pub fn reload_config(&mut self) {
        let (settings, w1) = self.store.settings();
        let (preset, w2) = self.store.active();
        crate::i18n::set_locale(&settings.locale);
        self.settings = settings;
        self.preset = preset;
        self.warning = w2.or(w1);
    }

    pub fn set_index(&mut self, module: &'static str, items: Vec<Item>) {
        self.index.insert(module, items);
    }

    pub fn index_items(&self, module: &str) -> Vec<Item> {
        self.index.get(module).cloned().unwrap_or_default()
    }

    pub fn db_ref(&self) -> Option<&usage::Db> {
        self.db.as_ref()
    }

    pub fn set_apps(&mut self, apps: Vec<App>, icons: &HashMap<String, String>) {
        let items = apps
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
                    actions: {
                        let mut v = vec![Action::new("launch", "Lancer"), Action::new("new-window", "Nouvelle fenêtre"), Action::new("reveal", "Afficher le fichier .desktop")];
                        if a.removable {
                            v.push(Action::new("uninstall", "Désinstaller").danger());
                        }
                        v
                    },
                    data: a.path.to_string_lossy().into_owned(),
                }
            })
            .collect();
        self.set_index("apps", items);
    }

    /// Open windows for the search, with the icon of their application.
    pub fn set_windows(&mut self, windows: &[modules::shell::Window]) {
        let mut items: Vec<Item> = windows.iter().map(modules::shell::window_item).collect();
        self.fill_window_icons(&mut items);
        self.set_index("windows", items);
    }

    fn fill_window_icons(&self, items: &mut [Item]) {
        let apps = self.index.get("apps");
        for it in items.iter_mut().filter(|i| i.module == "windows" && i.icon.is_empty()) {
            let app = it.data.split_once('\t').map(|(_, a)| format!("app:{a}")).unwrap_or_default();
            if let Some(a) = apps.and_then(|v| v.iter().find(|a| a.id == app)) {
                it.icon = a.icon.clone();
            }
        }
    }

    fn command_items(&self) -> Vec<Item> {
        let cmd = |id: &str, title: &str, subtitle: &str, kw: &[&str]| Item {
            id: format!("cmd:{id}"),
            module: "commands".into(),
            title: title.into(),
            subtitle: subtitle.into(),
            keywords: kw.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        };
        let mut out = vec![
            cmd("settings", &t("Réglages Landry"), &t("Presets, apparence, alias"), &["settings", "preferences", "config", "reglages"]),
            cmd("reload", &t("Recharger la configuration"), &t("Relit les presets et les réglages"), &["reload"]),
            cmd("quit", &t("Quitter Landry"), &t("Arrête le processus résident"), &["quit", "exit"]),
        ];
        for info in self.store.list().into_iter().filter(|i| i.error.is_none()) {
            let active = info.name == self.preset.name;
            out.push(cmd(
                &format!("preset:{}", info.name),
                &tf("Preset : {name}", &[("name", &info.name)]),
                &if active { t("Preset actif") } else { t("Changer de preset") },
                &["preset", "theme", "apparence"],
            ));
        }
        out
    }

    fn enabled(&self) -> Vec<String> {
        self.preset.module_order().into_iter().filter(|m| m.enabled).map(|m| m.id).collect()
    }

    /// The custom home: an item id or an exact app or command title (with its icon), otherwise a command that
    /// runs like the same text typed and Enter.
    fn home_items(&self) -> Vec<Item> {
        let commands = self.command_items();
        let fold = crate::search::fold;
        self.settings
            .home
            .iter()
            .map(|entry| {
                items_by_id(&self.index, &commands, entry)
                    .or_else(|| commands.iter().chain(self.index.get("apps").into_iter().flatten()).find(|i| fold(&i.title) == fold(entry)).cloned())
                    .unwrap_or_else(|| Item {
                        id: format!("home:{entry}"),
                        module: "commands".into(),
                        title: entry.clone(),
                        // "/spotify ": waits for the rest, so Enter writes it in the bar.
                        actions: if entry.ends_with(' ') {
                            vec![Action::new("fill", "Écrire dans la barre"), Action::new("run", "Exécuter")]
                        } else {
                            vec![Action::new("run", "Exécuter"), Action::new("fill", "Écrire dans la barre")]
                        },
                        ..Default::default()
                    })
            })
            .collect()
    }

    /// An alias whose target is text (not an item id) stands for that text: "pp" -> "/spotify play".
    /// The first word alone also works, followed by arguments: with "sp = /spotify", "sp daft punk"
    /// -> "/spotify daft punk".
    pub fn expand_alias(&self, query: &str) -> Option<String> {
        let q = query.trim();
        let text = |t: &String| items_by_id(&self.index, &self.command_items(), t).is_none().then(|| t.clone());
        if let Some(t) = self.settings.aliases.get(q) {
            return text(t);
        }
        let (word, rest) = q.split_once(' ')?;
        text(self.settings.aliases.get(word)?).map(|t| format!("{t} {}", rest.trim()))
    }

    pub fn search(&mut self, query: &str) -> Vec<Item> {
        match self.expand_alias(query) {
            Some(q) => self.search_query(&q),
            None => self.search_query(query),
        }
    }

    fn search_query(&mut self, query: &str) -> Vec<Item> {
        let slash = self.enabled().iter().any(|m| m == "slash");
        if let Some(rest) = query.trim_start().strip_prefix('/').filter(|_| slash) {
            match modules::slash::query(&self.settings, rest) {
                modules::slash::Slash::Rewrite(q) => return self.search_query(&q),
                modules::slash::Slash::Items(mut out) => {
                    self.fill_window_icons(&mut out);
                    self.shown = out.iter().map(|i| (i.id.clone(), i.clone())).collect();
                    out.iter_mut().for_each(|i| localize_actions(&mut i.actions));
                    return out;
                }
            }
        }
        if query.trim().is_empty() && !self.settings.home.is_empty() {
            let mut out = self.home_items();
            self.shown = out.iter().map(|i| (i.id.clone(), i.clone())).collect();
            out.iter_mut().for_each(|i| localize_actions(&mut i.actions));
            return out;
        }
        // An alias pointing at an item id (text aliases were expanded by `search`).
        let alias = self.settings.aliases.get(query.trim()).cloned();
        let limit = self.preset.max_results as usize;
        let enabled = self.enabled();
        let (text, filter) = files::parse_filters(query);

        let (dynamic, items) = if !filter.is_empty() {
            // Type or date filters only make sense for files.
            let files = if enabled.iter().any(|m| m == "files") { self.index.get("files") } else { None };
            (Vec::new(), files.map(|f| f.iter().filter(|i| filter.matches(i)).cloned().collect()).unwrap_or_default())
        } else {
            let mut dynamic = if query.trim().is_empty() {
                self.status_items(&enabled)
            } else {
                self.organizer_query(&enabled, query.trim())
            };
            dynamic.extend(modules::query(&enabled, &self.settings, self.selection.as_deref(), query.trim()));
            let mut items = Vec::new();
            for m in &enabled {
                match m.as_str() {
                    "commands" => items.extend(self.command_items()),
                    // Thousands of files: only once the query says enough.
                    "files" if text.chars().count() < 2 => {}
                    other => {
                        if let Some(v) = self.index.get(other) {
                            items.extend(v.iter().cloned());
                        }
                    }
                }
            }
            (dynamic, items)
        };

        let mut ranked: Vec<Item> = if filter.is_empty() || !text.is_empty() {
            self.ranker.rank(&items, &text, &self.frecency, limit).into_iter().map(|i| items[i].clone()).collect()
        } else {
            // Filter alone: most recent files first.
            let mut v = items;
            v.sort_by(|a, b| b.data.cmp(&a.data));
            v.truncate(limit);
            v
        };
        // An alias pointing at an item puts it first, whatever the score.
        if let Some(target) = &alias {
            if let Some(it) = items_by_id(&self.index, &self.command_items(), target) {
                ranked.retain(|i| i.id != it.id);
                ranked.insert(0, it);
            }
        }
        // Under a computed answer ("100 ht", "#ff8800"), keep only clear text matches.
        if !dynamic.is_empty() && !query.trim().is_empty() {
            let q = crate::search::fold(query.trim());
            ranked.retain(|i| crate::search::fold(&i.title).contains(&q));
        }
        let mut out = dynamic;
        out.extend(ranked);
        out.truncate(limit);
        self.shown = out.iter().map(|i| (i.id.clone(), i.clone())).collect();
        out.iter_mut().for_each(|i| localize_actions(&mut i.actions));
        out
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

    fn find(&self, id: &str) -> Option<Item> {
        self.shown.get(id).cloned().or_else(|| items_by_id(&self.index, &self.command_items(), id))
    }

    /// Run `action` (empty: the item's first action) with an optional text argument.
    pub fn activate(&mut self, id: &str, action: &str, arg: &str, query: &str) -> Result<Outcome, String> {
        if let Some(text) = id.strip_prefix("home:") {
            // No action given: the first one, "fill" for an entry that ends with a space.
            return if action == "fill" || (action.is_empty() && text.ends_with(' ')) { Ok(Outcome::Fill(format!("{} ", text.trim_end()))) } else { self.run_query(text).map(|(_, o)| o) };
        }
        let outcome = if let Some(name) = id.strip_prefix("cmd:preset:") {
            self.use_preset(name)?;
            Outcome::Keep
        } else if id.starts_with("cmd:") {
            match id {
                "cmd:settings" => Outcome::Settings,
                "cmd:reload" => {
                    self.reload_config();
                    Outcome::Message(t("Configuration rechargée."))
                }
                "cmd:quit" => {
                    self.quit_requested = true;
                    Outcome::Hide
                }
                _ => return Err(format!("commande inconnue : {id}")),
            }
        } else {
            let item = self.find(id).ok_or_else(|| format!("élément introuvable : {id}"))?;
            let action = if action.is_empty() {
                item.actions.first().map(|a| a.id.clone()).unwrap_or_default()
            } else {
                action.to_string()
            };
            if item.module == "snippets" {
                self.run_snippet(&item, &action, arg)?
            } else if matches!(item.module.as_str(), "notes" | "tasks" | "timers" | "tracking") {
                self.organizer_run(&item, &action)?
            } else {
                modules::run(&self.settings, &item, &action, arg)?
            }
        };
        self.record(id, query);
        Ok(outcome)
    }

    pub fn actions(&self, id: &str) -> Vec<Action> {
        let mut a = self.find(id).map(|i| modules::actions(&i)).unwrap_or_default();
        localize_actions(&mut a);
        a
    }

    pub fn preview(&self, id: &str) -> Preview {
        self.find(id).map(|i| modules::preview(&i)).unwrap_or(Preview::None)
    }

    /// Best match for a query, as `landry run <query>` does.
    pub fn run_query(&mut self, query: &str) -> Result<(String, Outcome), String> {
        let first = self.search(query).into_iter().next().ok_or_else(|| tf("aucun résultat pour « {q} »", &[("q", &query)]))?;
        let outcome = self.activate(&first.id, "", "", query)?;
        Ok((first.title, outcome))
    }

    fn run_snippet(&self, item: &Item, action: &str, arg: &str) -> Result<Outcome, String> {
        if action == "edit" {
            return Ok(Outcome::View("settings:snippets".into()));
        }
        let fields = modules::snippets::fields(&item.data);
        if !fields.is_empty() && arg.is_empty() {
            return Ok(Outcome::Ask(fields));
        }
        let values: HashMap<String, String> =
            if arg.is_empty() { HashMap::new() } else { serde_json::from_str(arg).map_err(|e| format!("valeurs invalides : {e}"))? };
        let (text, back) = modules::snippets::expand(&item.data, chrono::Local::now(), &self.clipboard, &values);
        Ok(match action {
            "copy" => Outcome::Copy(text),
            _ => Outcome::Paste { text, back },
        })
    }

    pub fn reload_snippets(&mut self) {
        let items = self.snippets().map(|s| modules::snippets::items(&s)).unwrap_or_default();
        self.set_index("snippets", items);
    }

    pub fn snippets(&self) -> Result<Vec<usage::Snippet>, String> {
        self.db.as_ref().ok_or_else(|| crate::i18n::t("base de données indisponible"))?.snippets().map_err(|e| e.to_string())
    }

    pub fn save_snippet(&mut self, s: &usage::Snippet) -> Result<i64, String> {
        if s.name.trim().is_empty() || s.body.is_empty() {
            return Err(t("un extrait a besoin d'un nom et d'un texte"));
        }
        let id = self.db.as_ref().ok_or_else(|| crate::i18n::t("base de données indisponible"))?.save_snippet(s).map_err(|e| e.to_string())?;
        self.reload_snippets();
        Ok(id)
    }

    pub fn delete_snippet(&mut self, id: i64) -> Result<(), String> {
        self.db.as_ref().ok_or_else(|| crate::i18n::t("base de données indisponible"))?.delete_snippet(id).map_err(|e| e.to_string())?;
        self.reload_snippets();
        Ok(())
    }

    pub fn save_settings(&mut self, s: Settings) -> Result<(), String> {
        self.store.save_settings(&s)?;
        crate::i18n::set_locale(&s.locale);
        self.settings = s;
        Ok(())
    }

    pub fn use_preset(&mut self, name: &str) -> Result<(), String> {
        self.preset = self.store.set_active(name)?;
        self.warning = None;
        Ok(())
    }
}

/// Action titles and prompts are fixed French texts: translated on the way out.
fn localize_actions(actions: &mut [Action]) {
    for a in actions {
        a.title = t(&a.title);
        if let Some(arg) = &a.arg {
            a.arg = Some(t(arg));
        }
    }
}

fn items_by_id(index: &HashMap<&'static str, Vec<Item>>, commands: &[Item], id: &str) -> Option<Item> {
    commands.iter().chain(index.values().flatten()).find(|i| i.id == id).cloned()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

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
            removable: false,
        };
        e.set_apps(
            vec![app("code.desktop", "Visual Studio Code", "code"), app("t.desktop", "Terminal", "gnome-terminal")],
            &HashMap::new(),
        );
        (e, dir)
    }

    #[test]
    fn finds_apps_and_commands() {
        let (mut e, dir) = engine();
        assert_eq!(e.search("vsc")[0].id, "app:code.desktop");
        assert_eq!(e.search("dense")[0].id, "cmd:preset:Dense");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn disabled_module_is_hidden_and_max_results_applies() {
        let (mut e, dir) = engine();
        e.preset.modules.iter_mut().for_each(|m| m.enabled = m.id != "apps");
        assert!(e.search("terminal").iter().all(|i| i.module != "apps"));
        e.preset.max_results = 1;
        assert_eq!(e.search("preset").len(), 1);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn alias_wins_and_preset_switch_persists() {
        let (mut e, dir) = engine();
        e.settings.aliases.insert("t".into(), "app:t.desktop".into());
        assert_eq!(e.search("t")[0].id, "app:t.desktop");
        e.settings.aliases.insert("ed".into(), "visual studio".into());
        assert_eq!(e.search("ed")[0].id, "app:code.desktop");
        assert_eq!(e.activate("cmd:preset:Minimal", "", "", "min").unwrap(), Outcome::Keep);
        assert_eq!(e.preset.name, "Minimal");
        assert_eq!(e.store.settings().0.active_preset, "Minimal");
        assert!(e.history().contains(&"min".to_string()));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn aliases_run_whole_commands_and_take_arguments() {
        let (mut e, dir) = engine();
        e.settings.aliases.insert("pp".into(), "/spotify play".into());
        e.settings.aliases.insert("sp".into(), "/spotify".into());
        e.settings.aliases.insert("min".into(), "minuteur 5 min".into());
        assert_eq!(e.search("pp")[0].title, "Lecture ou pause");
        assert_eq!(e.search("sp daft punk")[0].title, "Chercher « daft punk » dans Spotify");
        assert_eq!(e.search("min")[0].title, "Lancer un minuteur de 5 min");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn custom_home_shows_only_its_entries() {
        let (mut e, dir) = engine();
        e.settings.home = vec!["app:t.desktop".into(), "visual studio code".into(), "/spotify ".into()];
        let home: Vec<String> = e.search("").into_iter().map(|i| i.id).collect();
        assert_eq!(home, ["app:t.desktop", "app:code.desktop", "home:/spotify "]);
        assert_eq!(e.activate("home:/spotify ", "", "", "").unwrap(), Outcome::Fill("/spotify ".into()), "finie par une espace : s'écrit dans la barre");
        assert_eq!(e.search("term")[0].id, "app:t.desktop", "taper retrouve la recherche normale");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn snippet_asks_fields_then_pastes() {
        let (mut e, dir) = engine();
        let id = e.save_snippet(&usage::Snippet { id: 0, name: "Relance".into(), keyword: "rel".into(), body: "Bonjour {champ:Nom}{curseur} !".into() }).unwrap();
        let sid = format!("snip:{id}");
        assert_eq!(e.search("relance")[0].id, sid);
        assert_eq!(e.activate(&sid, "", "", "relance").unwrap(), Outcome::Ask(vec!["Nom".into()]));
        assert_eq!(
            e.activate(&sid, "paste", r#"{"Nom":"Léa"}"#, "relance").unwrap(),
            Outcome::Paste { text: "Bonjour Léa !".into(), back: 2 }
        );
        assert!(e.save_snippet(&usage::Snippet { id: 0, name: " ".into(), keyword: String::new(), body: "x".into() }).is_err());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn slash_lists_commands_and_reuses_keywords() {
        let (mut e, dir) = engine();
        assert!(e.search("/").iter().all(|i| i.title.starts_with('/')));
        assert_eq!(e.search("/minuteur 10 min pâtes")[0].title, "Lancer un minuteur de 10 min");
        let sp = e.search("/spotify nocturne")[0].clone();
        assert_eq!(e.actions(&sp.id).len(), 3, "l'élément affiché reste activable");
        assert_eq!(e.search("/dense")[0].id, "cmd:preset:Dense", "un mot inconnu redevient une recherche");
        e.preset.modules.iter_mut().filter(|m| m.id == "slash").for_each(|m| m.enabled = false);
        assert!(e.search("/").iter().all(|i| i.module != "slash"), "module désactivé : « / » est un texte ordinaire");
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn tool_results_come_first_and_copy() {
        let (mut e, dir) = engine();
        assert_eq!(e.search("12*7").len(), 1, "pas de bruit sous un calcul");
        let r = e.search("b64 bonjour");
        assert_eq!(r[0].title, "Ym9uam91cg==");
        assert_eq!(e.activate(&r[0].id, "", "", "b64 bonjour").unwrap(), Outcome::Copy("Ym9uam91cg==".into()));
        let _ = std::fs::remove_dir_all(dir);
    }
}
