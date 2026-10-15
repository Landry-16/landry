//! One JSON file with settings, presets, snippets and notes. Import never deletes anything.

use crate::config::{Preset, Settings, DEFAULT_NAME};
use crate::engine::Engine;
use crate::i18n::{t, tf};
use crate::usage::{Note, Snippet};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const FORMAT: &str = "landry-sauvegarde";

#[derive(Serialize, Deserialize)]
pub struct Backup {
    pub format: String,
    pub version: u32,
    pub created: String,
    pub settings: Settings,
    pub presets: Vec<Preset>,
    pub snippets: Vec<Snippet>,
    pub notes: Vec<Note>,
}

impl Engine {
    pub fn backup(&self) -> Result<Backup, String> {
        let presets = self
            .store
            .list()
            .into_iter()
            .filter(|i| !i.builtin && i.error.is_none())
            .filter_map(|i| self.store.get(&i.name).ok())
            .collect();
        let db = self.db_ref().ok_or_else(|| t("base de données indisponible"))?;
        Ok(Backup {
            format: FORMAT.into(),
            version: 1,
            created: chrono::Local::now().to_rfc3339(),
            settings: self.settings.clone(),
            presets,
            snippets: db.snippets().map_err(|e| e.to_string())?,
            notes: db.notes().map_err(|e| e.to_string())?,
        })
    }

    pub fn export_backup(&self, dir: &Path) -> Result<PathBuf, String> {
        let json = serde_json::to_string_pretty(&self.backup()?).map_err(|e| e.to_string())?;
        let path = dir.join(format!("landry-sauvegarde-{}.json", chrono::Local::now().format("%Y-%m-%d")));
        std::fs::write(&path, json).map_err(|e| format!("{}: {e}", path.display()))?;
        Ok(path)
    }

    /// Merge a backup: settings replaced (the old file kept as settings.json.bak), presets added
    /// under a new name on conflict, snippets and notes added unless already present.
    pub fn import_backup(&mut self, path: &Path) -> Result<String, String> {
        let s = std::fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let b: Backup = serde_json::from_str(&s).map_err(|e| tf("fichier de sauvegarde invalide : {e}", &[("e", &e)]))?;
        if b.format != FORMAT {
            return Err(t("ce fichier n'est pas une sauvegarde Landry"));
        }
        self.store.backup_settings()?;
        let mut settings = b.settings;
        // The active preset must exist here; checked again below once presets are in.
        let wanted = settings.active_preset.clone();
        settings.active_preset = self.settings.active_preset.clone();
        self.save_settings(settings)?;

        let mut presets = 0;
        let same_content = |a: &Preset, b: &Preset| Preset { name: String::new(), ..a.clone() } == Preset { name: String::new(), ..b.clone() };
        let have: Vec<Preset> = self.store.list().into_iter().filter_map(|i| self.store.get(&i.name).ok()).collect();
        for mut p in b.presets {
            // Already here, maybe under another name after an earlier import.
            if have.iter().any(|h| same_content(h, &p)) {
                continue;
            }
            let base = p.name.clone();
            let mut i = 2;
            while p.name == DEFAULT_NAME || self.store.get(&p.name).is_ok() {
                p.name = format!("{base} ({i})");
                i += 1;
            }
            self.store.save(&p, None)?;
            presets += 1;
        }
        if self.store.get(&wanted).is_ok() {
            self.use_preset(&wanted)?;
        }

        let existing = self.snippets()?;
        let mut snippets = 0;
        for s in b.snippets {
            if !existing.iter().any(|e| e.name == s.name && e.body == s.body) {
                self.save_snippet(&Snippet { id: 0, ..s })?;
                snippets += 1;
            }
        }
        let db = self.db_ref().ok_or_else(|| t("base de données indisponible"))?;
        let notes_now = db.notes().map_err(|e| e.to_string())?;
        let mut notes = 0;
        for n in b.notes {
            if !notes_now.iter().any(|e| e.body == n.body) {
                db.save_note(0, &n.body).map_err(|e| e.to_string())?;
                notes += 1;
            }
        }
        self.reload_organizer();
        Ok(tf(
            "Importé : réglages, {p} preset(s), {s} extrait(s), {n} note(s).",
            &[("p", &presets), ("s", &snippets), ("n", &notes)],
        ))
    }
}

#[cfg(test)]
mod tests {
    use crate::config::Store;
    use crate::engine::Engine;
    use crate::usage::{self, Snippet};

    fn engine(name: &str) -> (Engine, std::path::PathBuf) {
        let dir = std::env::temp_dir().join(format!("landry-backup-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db = usage::Db::open(&dir.join("landry.db")).ok();
        (Engine::new(Store::new(dir.join("config")), db), dir)
    }

    #[test]
    fn export_then_import_merges_without_duplicates() {
        let (mut a, da) = engine("a");
        a.save_snippet(&Snippet { id: 0, name: "Sig".into(), keyword: "sig".into(), body: "Landry".into() }).unwrap();
        a.db_ref().unwrap().save_note(0, "# Note A").unwrap();
        let mut perso = a.store.duplicate("Dense", "Perso").unwrap();
        perso.max_results = 3;
        a.store.save(&perso, Some("Perso")).unwrap();
        let file = a.export_backup(&da).unwrap();

        let (mut b, db) = engine("b");
        b.store.duplicate("Minimal", "Perso").unwrap();
        let msg = b.import_backup(&file).unwrap();
        assert!(msg.contains("1 extrait(s), 1 note(s)"), "{msg}");
        assert!(b.store.get("Perso (2)").is_ok(), "conflit de nom : suffixe");
        let again = b.import_backup(&file).unwrap();
        assert!(again.contains("0 preset(s), 0 extrait(s), 0 note(s)"), "{again}");
        let _ = std::fs::remove_dir_all(da);
        let _ = std::fs::remove_dir_all(db);
    }
}
