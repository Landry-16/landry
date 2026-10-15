//! Notes, tasks, timers and time tracking, on the engine side (storage and running state).

use crate::engine::{Engine, Outcome};
use crate::i18n::{t, tf};
use crate::modules::organize::{clock, human, parse_duration, parse_task, totals, tracking_csv, when};
use crate::search::{fold, Action, Item};
use crate::usage;
use chrono::Local;

const FOCUS: i64 = 25 * 60;
const BREAK: i64 = 5 * 60;

#[derive(Clone, Debug, PartialEq)]
pub enum TimerKind {
    Countdown,
    Stopwatch,
    /// `focus`: working phase (else break); `round`: completed focus phases.
    Pomodoro { focus: bool, round: u32 },
}

#[derive(Clone, Debug)]
pub struct Timer {
    pub id: u32,
    pub kind: TimerKind,
    pub label: String,
    /// Start of the current phase (the timer itself for countdowns and stopwatches).
    pub start: i64,
    /// Phase length; 0 for a stopwatch.
    pub length: i64,
}

impl Timer {
    fn stored(&self) -> usage::StoredTimer {
        let (kind, focus, round) = match &self.kind {
            TimerKind::Countdown => ("countdown", false, 0),
            TimerKind::Stopwatch => ("stopwatch", false, 0),
            TimerKind::Pomodoro { focus, round } => ("pomodoro", *focus, *round),
        };
        usage::StoredTimer { id: self.id, kind: kind.into(), label: self.label.clone(), start: self.start, length: self.length, focus, round }
    }

    fn from_stored(s: usage::StoredTimer) -> Option<Timer> {
        let kind = match s.kind.as_str() {
            "countdown" => TimerKind::Countdown,
            "stopwatch" => TimerKind::Stopwatch,
            "pomodoro" => TimerKind::Pomodoro { focus: s.focus, round: s.round },
            _ => return None,
        };
        Some(Timer { id: s.id, kind, label: s.label, start: s.start, length: s.length })
    }

    fn title(&self, now: i64) -> String {
        let elapsed = now - self.start;
        match &self.kind {
            TimerKind::Countdown => tf("Minuteur {l} · reste {r}", &[("l", &self.label), ("r", &clock(self.length - elapsed))]),
            TimerKind::Stopwatch => tf("Chronomètre · {e}", &[("e", &clock(elapsed))]),
            TimerKind::Pomodoro { focus, round } => tf(
                "Pomodoro {n} · {phase} · reste {r}",
                &[("n", &(round + 1)), ("phase", &t(if *focus { "travail" } else { "pause" })), ("r", &clock(self.length - elapsed))],
            ),
        }
    }
}

fn item(module: &str, id: String, title: String, subtitle: String, actions: Vec<Action>, data: String) -> Item {
    // Fixed texts are French keys; user texts pass through unchanged.
    Item { id, module: module.into(), title: t(&title), subtitle: t(&subtitle), actions, data, ..Default::default() }
}

fn first_line(body: &str) -> String {
    body.lines().find(|l| !l.trim().is_empty()).unwrap_or("Note vide").trim_start_matches('#').trim().to_string()
}

fn downloads() -> std::path::PathBuf {
    crate::util::output("xdg-user-dir", &["DOWNLOAD"], None)
        .ok()
        .map(|s| std::path::PathBuf::from(s.trim()))
        .filter(|p| p.is_dir())
        .unwrap_or_else(|| crate::paths::expand("~"))
}

impl Engine {
    fn db(&self) -> Result<&usage::Db, String> {
        self.db_ref().ok_or_else(|| t("base de données indisponible"))
    }

    /// Notes and open tasks are indexed so they show up in any search.
    pub fn reload_organizer(&mut self) {
        let now = Local::now();
        let tasks = self
            .db_ref()
            .and_then(|d| d.open_tasks().ok())
            .unwrap_or_default()
            .into_iter()
            .map(|t| {
                item(
                    "tasks",
                    format!("task:{}", t.id),
                    t.text.clone(),
                    t.due.map(|d| tf("Rappel {w}", &[("w", &when(d, now))])).unwrap_or_else(|| "Sans rappel".into()),
                    vec![Action::new("task-done", "Marquer comme faite"), Action::new("task-delete", "Supprimer").danger()],
                    t.id.to_string(),
                )
            })
            .collect();
        self.set_index("tasks", tasks);
        let notes = self
            .db_ref()
            .and_then(|d| d.notes().ok())
            .unwrap_or_default()
            .into_iter()
            .map(|n| {
                let mut it = item(
                    "notes",
                    format!("note:{}", n.id),
                    first_line(&n.body),
                    tf("Note · {w}", &[("w", &when(n.updated, now))]),
                    vec![Action::new("note-open", "Ouvrir")],
                    n.id.to_string(),
                );
                // The whole body is searchable, not only the first line.
                it.keywords = vec![n.body.chars().take(2000).collect()];
                it
            })
            .collect();
        self.set_index("notes", notes);
    }

    /// Running timers and tracking, shown when the bar opens with an empty query.
    pub fn status_items(&self, enabled: &[String]) -> Vec<Item> {
        let now = usage::now();
        let mut out = Vec::new();
        if enabled.iter().any(|m| m == "timers") {
            out.extend(self.timers.iter().map(|t| {
                item("timers", format!("timer:{}", t.id), t.title(now), t.label.clone(), vec![Action::new("timer-stop", "Arrêter")], t.id.to_string())
            }));
        }
        if enabled.iter().any(|m| m == "tracking") {
            if let Some(e) = self.db_ref().and_then(|d| d.running_entry().ok().flatten()) {
                out.push(item(
                    "tracking",
                    "track:running".into(),
                    tf("Suivi : {l} · {e}", &[("l", &e.label), ("e", &clock(now - e.start))]),
                    "En cours".into(),
                    vec![Action::new("track-stop", "Arrêter le suivi")],
                    e.label,
                ));
            }
        }
        out
    }

    pub fn organizer_query(&self, enabled: &[String], q: &str) -> Vec<Item> {
        let (word, rest) = q.split_once(' ').map(|(a, b)| (a, b.trim())).unwrap_or((q, ""));
        let on = |m: &str| enabled.iter().any(|e| e == m);
        let now = Local::now();
        match fold(word).as_str() {
            "todo" | "tache" | "rappel" if on("tasks") && !rest.is_empty() => {
                let (text, due) = parse_task(rest, now);
                let ts = due.map(|d| d.timestamp());
                vec![item(
                    "tasks",
                    "task:new".into(),
                    tf("Ajouter : {t}", &[("t", &text)]),
                    ts.map(|ts| tf("Rappel {w}", &[("w", &when(ts, now))])).unwrap_or_else(|| "Sans rappel".into()),
                    vec![Action::new("task-add", "Ajouter la tâche")],
                    format!("{}\t{text}", ts.map(|t| t.to_string()).unwrap_or_default()),
                )]
            }
            "todo" | "taches" | "todos" | "tache" if on("tasks") => self.index_items("tasks"),
            "minuteur" | "timer" if on("timers") => {
                let mut out = Vec::new();
                // "10 min pâtes": the longest leading part that is a duration, the rest is the label.
                let words: Vec<&str> = rest.split_whitespace().collect();
                let split = (1..=words.len().min(3)).rev().find(|n| parse_duration(&words[..*n].join(" ")).is_some());
                match split {
                    Some(n) => {
                        let secs = parse_duration(&words[..n].join(" ")).unwrap_or(0);
                        let label = words[n..].join(" ");
                        out.push(item(
                            "timers",
                            "timer:new".into(),
                            tf("Lancer un minuteur de {d}", &[("d", &human(secs))]),
                            if label.is_empty() { "Notification à la fin".into() } else { label.clone() },
                            vec![Action::new("timer-start", "Lancer")],
                            format!("{secs}\t{label}"),
                        ));
                    }
                    None => out.push(item("timers", "timer:hint".into(), "minuteur <durée> [libellé]".into(), "Exemples : minuteur 10 min, minuteur 1h30 four".into(), vec![], String::new())),
                }
                out.extend(self.status_items(enabled).into_iter().filter(|i| i.module == "timers"));
                out
            }
            "chrono" | "chronometre" if on("timers") => {
                let mut out = vec![item("timers", "timer:stopwatch".into(), "Démarrer un chronomètre".into(), String::new(), vec![Action::new("stopwatch-start", "Démarrer")], String::new())];
                out.extend(self.status_items(enabled).into_iter().filter(|i| i.module == "timers"));
                out
            }
            "pomodoro" if on("timers") => {
                let mut out = vec![item("timers", "timer:pomodoro".into(), "Démarrer un Pomodoro".into(), "25 min de travail, 5 min de pause, en boucle".into(), vec![Action::new("pomodoro-start", "Démarrer")], rest.to_string())];
                out.extend(self.status_items(enabled).into_iter().filter(|i| i.module == "timers"));
                out
            }
            "suivi" | "track" if on("tracking") => {
                let mut out = Vec::new();
                if !rest.is_empty() && fold(rest) != "stop" {
                    out.push(item("tracking", "track:new".into(), tf("Démarrer le suivi : {l}", &[("l", &rest)]), "Arrête le suivi en cours s'il y en a un".into(), vec![Action::new("track-start", "Démarrer")], rest.to_string()));
                }
                out.extend(self.status_items(enabled).into_iter().filter(|i| i.module == "tracking"));
                let midnight = now.date_naive().and_hms_opt(0, 0, 0).and_then(|d| d.and_local_timezone(Local).earliest()).map(|d| d.timestamp()).unwrap_or(0);
                let today = self.db_ref().and_then(|d| d.entries_since(midnight).ok()).unwrap_or_default();
                for (label, secs) in totals(&today, midnight, now.timestamp()) {
                    out.push(item("tracking", format!("track:today:{label}"), tf("{l} : {d} aujourd'hui", &[("l", &label), ("d", &human(secs))]), "Total du jour".into(), vec![Action::new("track-start", "Reprendre")], label));
                }
                out.push(item("tracking", "track:export".into(), "Exporter le suivi du temps (CSV)".into(), "Dans le dossier Téléchargements".into(), vec![Action::new("track-export", "Exporter")], String::new()));
                out
            }
            "note" if on("notes") && !rest.is_empty() => {
                vec![item("notes", "note:new".into(), tf("Nouvelle note : {t}", &[("t", &rest)]), "Crée la note et l'ouvre".into(), vec![Action::new("note-new", "Créer")], rest.to_string())]
            }
            "notes" | "note" if on("notes") => {
                let mut out = vec![item("notes", "note:view".into(), "Ouvrir les notes".into(), "Toutes les notes, avec recherche".into(), vec![Action::new("notes-view", "Ouvrir")], String::new())];
                out.extend(self.index_items("notes"));
                out
            }
            _ => vec![],
        }
    }

    pub fn organizer_run(&mut self, item: &Item, action: &str) -> Result<Outcome, String> {
        let now = usage::now();
        let msg = |s: String| Ok(Outcome::Message(t(&s)));
        match action {
            "timer-start" => {
                let (secs, label) = item.data.split_once('\t').unwrap_or((&item.data, ""));
                let secs: i64 = secs.parse().map_err(|_| t("durée invalide"))?;
                self.add_timer(TimerKind::Countdown, label, now, secs);
                msg(tf("Minuteur lancé : {d}", &[("d", &human(secs))]))
            }
            "stopwatch-start" => {
                self.add_timer(TimerKind::Stopwatch, "", now, 0);
                msg("Chronomètre démarré.".into())
            }
            "pomodoro-start" => {
                self.add_timer(TimerKind::Pomodoro { focus: true, round: 0 }, &item.data, now, FOCUS);
                msg("Pomodoro démarré : 25 minutes de travail.".into())
            }
            "timer-stop" => {
                let id: u32 = item.data.parse().map_err(|_| t("minuteur inconnu"))?;
                let t = self.timers.iter().find(|t| t.id == id).cloned().ok_or_else(|| t("ce minuteur est déjà terminé"))?;
                self.timers.retain(|t| t.id != id);
                self.persist_timers();
                msg(match t.kind {
                    TimerKind::Stopwatch => tf("Chronomètre arrêté à {e}", &[("e", &clock(now - t.start))]),
                    _ => "Minuteur arrêté.".into(),
                })
            }
            "task-add" => {
                let (due, text) = item.data.split_once('\t').ok_or_else(|| t("tâche invalide"))?;
                if text.trim().is_empty() {
                    return Err(t("la tâche est vide"));
                }
                let due: Option<i64> = due.parse().ok();
                self.db()?.add_task(text, due).map_err(|e| e.to_string())?;
                self.reload_organizer();
                msg(match due {
                    Some(d) => tf("Tâche ajoutée, rappel {w}", &[("w", &when(d, Local::now()))]),
                    None => "Tâche ajoutée.".into(),
                })
            }
            "task-done" | "task-delete" => {
                let id: i64 = item.data.parse().map_err(|_| t("tâche inconnue"))?;
                if action == "task-done" {
                    self.db()?.finish_task(id).map_err(|e| e.to_string())?;
                } else {
                    self.db()?.delete_task(id).map_err(|e| e.to_string())?;
                }
                self.reload_organizer();
                msg(if action == "task-done" { tf("Faite : {t}", &[("t", &item.title)]) } else { "Tâche supprimée.".into() })
            }
            "track-start" => {
                self.db()?.start_tracking(&item.data, now).map_err(|e| e.to_string())?;
                msg(tf("Suivi démarré : {l}", &[("l", &item.data)]))
            }
            "track-stop" => match self.db()?.stop_tracking(now).map_err(|e| e.to_string())? {
                Some(e) => msg(tf("Suivi arrêté : {l}, {d}", &[("l", &e.label), ("d", &human(now - e.start))])),
                None => msg("Aucun suivi en cours.".into()),
            },
            "track-export" => {
                let entries = self.db()?.entries_since(0).map_err(|e| e.to_string())?;
                let path = downloads().join(format!("landry-suivi-{}.csv", Local::now().format("%Y-%m-%d")));
                std::fs::write(&path, tracking_csv(&entries, now)).map_err(|e| format!("{}: {e}", path.display()))?;
                msg(tf("Exporté : {p}", &[("p", &path.display())]))
            }
            "note-new" => {
                let id = self.db()?.save_note(0, &item.data).map_err(|e| e.to_string())?;
                self.reload_organizer();
                Ok(Outcome::View(format!("notes:{id}")))
            }
            "note-open" => Ok(Outcome::View(format!("notes:{}", item.data))),
            "notes-view" => Ok(Outcome::View("notes".into())),
            "" => Err(item.subtitle.clone()),
            a => Err(tf("action inconnue : {a}", &[("a", &a)])),
        }
    }

    fn add_timer(&mut self, kind: TimerKind, label: &str, start: i64, length: i64) {
        self.next_timer += 1;
        self.timers.push(Timer { id: self.next_timer, kind, label: label.to_string(), start, length });
        self.persist_timers();
    }

    /// Timers survive a restart; one that ended meanwhile is reported at the next tick.
    pub fn load_timers(&mut self) {
        let stored = self.db_ref().and_then(|d| d.load_timers().ok()).unwrap_or_default();
        self.timers = stored.into_iter().filter_map(Timer::from_stored).collect();
        self.next_timer = self.timers.iter().map(|t| t.id).max().unwrap_or(0);
    }

    fn persist_timers(&self) {
        if let Some(db) = self.db_ref() {
            let rows: Vec<_> = self.timers.iter().map(Timer::stored).collect();
            if let Err(e) = db.save_timers(&rows) {
                eprintln!("landry: minuteurs non enregistrés : {e}");
            }
        }
    }

    pub fn has_timers(&self) -> bool {
        !self.timers.is_empty()
    }

    /// Advance timers and collect due reminders. Returns notifications (title, body).
    pub fn tick(&mut self, now: i64) -> Vec<(String, String)> {
        let mut out: Vec<(String, String)> = Vec::new();
        let mut keep = Vec::new();
        let mut changed = false;
        for mut tm in std::mem::take(&mut self.timers) {
            let done = tm.length > 0 && now - tm.start >= tm.length;
            match (&tm.kind, done) {
                (TimerKind::Countdown, true) => {
                    changed = true;
                    out.push(("Minuteur terminé".into(), if tm.label.is_empty() { human(tm.length) } else { tm.label.clone() }));
                    continue;
                }
                (TimerKind::Pomodoro { focus, round }, true) => {
                    changed = true;
                    let (focus, round) = (!*focus, if *focus { round + 1 } else { *round });
                    out.push(if focus {
                        ("Pomodoro : au travail".into(), tf("Session {n} : 25 minutes", &[("n", &(round + 1))]))
                    } else {
                        ("Pomodoro : pause".into(), if round > 1 { tf("{n} sessions terminées, 5 minutes de pause", &[("n", &round)]) } else { t("1 session terminée, 5 minutes de pause") })
                    });
                    tm.kind = TimerKind::Pomodoro { focus, round };
                    tm.start = now;
                    tm.length = if focus { FOCUS } else { BREAK };
                }
                _ => {}
            }
            keep.push(tm);
        }
        self.timers = keep;
        if changed {
            self.persist_timers();
        }
        let mut out: Vec<(String, String)> = out.into_iter().map(|(a, b)| (t(&a), b)).collect();
        if let Some(db) = self.db_ref() {
            for task in db.take_due_reminders(now).unwrap_or_default() {
                out.push((t("Rappel"), task.text));
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Store;
    use std::path::PathBuf;

    fn engine() -> (Engine, PathBuf) {
        static N: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = N.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("landry-org-{}-{n}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let db = usage::Db::open(&dir.join("landry.db")).ok();
        (Engine::new(Store::new(dir.join("config")), db), dir)
    }

    fn all() -> Vec<String> {
        crate::config::MODULES.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn countdown_notifies_once() {
        let (mut e, dir) = engine();
        let it = &e.organizer_query(&all(), "minuteur 10 min pâtes")[0];
        assert_eq!(it.title, "Lancer un minuteur de 10 min");
        assert_eq!(it.data, "600\tpâtes");
        e.organizer_run(&it.clone(), "timer-start").unwrap();
        let t0 = e.timers[0].start;
        assert!(e.tick(t0 + 599).is_empty());
        assert_eq!(e.tick(t0 + 600), vec![("Minuteur terminé".to_string(), "pâtes".to_string())]);
        assert!(!e.has_timers());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn timers_survive_a_restart() {
        let (mut e, dir) = engine();
        let it = e.organizer_query(&all(), "minuteur 1h four")[0].clone();
        e.organizer_run(&it, "timer-start").unwrap();
        let start = e.timers[0].start;
        let db = usage::Db::open(&dir.join("landry.db")).ok();
        let mut again = Engine::new(Store::new(dir.join("config")), db);
        assert_eq!(again.timers.len(), 1);
        assert_eq!(again.timers[0].label, "four");
        assert_eq!(again.tick(start + 3600)[0].0, "Minuteur terminé");
        assert!(usage::Db::open(&dir.join("landry.db")).unwrap().load_timers().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn pomodoro_cycles() {
        let (mut e, dir) = engine();
        let it = e.organizer_query(&all(), "pomodoro")[0].clone();
        e.organizer_run(&it, "pomodoro-start").unwrap();
        let t0 = e.timers[0].start;
        assert_eq!(e.tick(t0 + FOCUS)[0].0, "Pomodoro : pause");
        assert_eq!(e.tick(t0 + FOCUS + BREAK)[0].0, "Pomodoro : au travail");
        assert_eq!(e.timers[0].kind, TimerKind::Pomodoro { focus: true, round: 1 });
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn tasks_are_indexed_and_remind() {
        let (mut e, dir) = engine();
        let it = e.organizer_query(&all(), "todo appeler la boulangerie dans 1 min")[0].clone();
        assert_eq!(it.title, "Ajouter : appeler la boulangerie");
        e.organizer_run(&it, "task-add").unwrap();
        assert_eq!(e.search("boulangerie")[0].module, "tasks");
        let notes = e.tick(usage::now() + 120);
        assert_eq!(notes, vec![("Rappel".to_string(), "appeler la boulangerie".to_string())]);
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn tracking_and_notes() {
        let (mut e, dir) = engine();
        let it = e.organizer_query(&all(), "suivi Nacre")[0].clone();
        e.organizer_run(&it, "track-start").unwrap();
        assert!(e.status_items(&all()).iter().any(|i| i.title.starts_with("Suivi : Nacre")));
        let new = e.organizer_query(&all(), "note Idées de presets")[0].clone();
        let out = e.organizer_run(&new, "note-new").unwrap();
        assert!(matches!(out, Outcome::View(ref v) if v.starts_with("notes:")));
        assert_eq!(e.search("idées presets")[0].module, "notes");
        let _ = std::fs::remove_dir_all(dir);
    }
}
