//! Settings (global) and presets (look and layout of the bar).
//! Nothing visual is hardcoded elsewhere: the frontend renders whatever the active preset says.

use crate::i18n::{t, tf};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

pub const SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_NAME: &str = "Défaut";

/// Every module the bar knows, in default display order.
pub const MODULES: &[&str] = &[
    "commands", "timers", "tracking", "apps", "scripts", "snippets", "text", "calc", "tools", "web", "tasks", "notes", "projects", "files",
    "system", "cloud",
];

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ThemeMode {
    System,
    Light,
    Dark,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Anchor {
    Top,
    Center,
    Bottom,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Density {
    Compact,
    Comfortable,
    Spacious,
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Side {
    Left,
    Right,
}

/// How the bar's window is laid out on Wayland, where apps cannot position windows.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Layer {
    /// Transparent layer the size of the screen: anchor and offsets apply. Slower to show.
    Screen,
    /// Window the size of the bar, placed by GNOME: anchor and offsets are ignored. Faster.
    Window,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Layout {
    pub layer: Layer,
    pub anchor: Anchor,
    pub width: u32,
    pub max_height: u32,
    pub offset_x: i32,
    pub offset_y: i32,
    /// Distance to the screen edge for top and bottom anchors.
    pub margin: u32,
    pub radius: u32,
}

impl Default for Layout {
    fn default() -> Self {
        Layout { layer: Layer::Screen, anchor: Anchor::Center, width: 720, max_height: 480, offset_x: 0, offset_y: -80, margin: 96, radius: 16 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Colors {
    pub bg: String,
    pub fg: String,
    pub muted: String,
    pub accent: String,
    pub selection: String,
    pub border: String,
}

impl Colors {
    fn dark() -> Self {
        Colors {
            bg: "#1c1a2e".into(),
            fg: "#f0ecff".into(),
            muted: "#9b95b8".into(),
            accent: "#ffb84c".into(),
            selection: "#2f2b4a".into(),
            border: "#3a3558".into(),
        }
    }
    fn light() -> Self {
        Colors {
            bg: "#fbf9f4".into(),
            fg: "#1f1c2c".into(),
            muted: "#6b6680".into(),
            accent: "#ad4b09".into(),
            selection: "#efe9dc".into(),
            border: "#ddd6c6".into(),
        }
    }
}

impl Default for Colors {
    fn default() -> Self {
        Colors::dark()
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Font {
    pub family: String,
    pub size: u32,
}

impl Default for Font {
    fn default() -> Self {
        Font { family: "Inter, Cantarell, system-ui, sans-serif".into(), size: 15 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Preview {
    pub visible: bool,
    pub side: Side,
    /// Share of the bar width, in percent.
    pub width: u32,
}

impl Default for Preview {
    fn default() -> Self {
        Preview { visible: true, side: Side::Right, width: 40 }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Module {
    pub id: String,
    pub enabled: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Preset {
    pub version: u32,
    pub name: String,
    pub theme: ThemeMode,
    pub light: Colors,
    pub dark: Colors,
    pub font: Font,
    pub density: Density,
    pub opacity: f32,
    /// 0 disables animations.
    pub animation_ms: u32,
    pub max_results: u32,
    pub layout: Layout,
    pub preview: Preview,
    /// Display order and visibility. Modules missing here are appended, enabled.
    pub modules: Vec<Module>,
}

impl Default for Preset {
    fn default() -> Self {
        Preset {
            version: SCHEMA_VERSION,
            name: DEFAULT_NAME.into(),
            theme: ThemeMode::System,
            light: Colors::light(),
            dark: Colors::dark(),
            font: Font::default(),
            density: Density::Comfortable,
            opacity: 0.97,
            animation_ms: 120,
            max_results: 8,
            layout: Layout::default(),
            preview: Preview::default(),
            modules: MODULES.iter().map(|id| Module { id: (*id).into(), enabled: true }).collect(),
        }
    }
}

/// Presets shipped as editable files on first run.
pub fn shipped_presets() -> Vec<Preset> {
    let base = Preset::default();
    let minimal = Preset {
        name: "Minimal".into(),
        max_results: 5,
        animation_ms: 0,
        density: Density::Compact,
        preview: Preview { visible: false, ..Preview::default() },
        layout: Layout { width: 560, layer: Layer::Window, ..Layout::default() },
        ..base.clone()
    };
    let dense = Preset {
        name: "Dense".into(),
        max_results: 14,
        density: Density::Compact,
        font: Font { size: 13, ..Font::default() },
        layout: Layout { width: 860, max_height: 620, radius: 8, ..Layout::default() },
        ..base.clone()
    };
    let dev = Preset {
        name: "Développeur".into(),
        theme: ThemeMode::Dark,
        font: Font { family: "JetBrains Mono, Fira Code, monospace".into(), size: 14 },
        dark: Colors {
            bg: "#0d1117".into(),
            fg: "#e6edf3".into(),
            muted: "#8b949e".into(),
            accent: "#3fb950".into(),
            selection: "#1f2a37".into(),
            border: "#30363d".into(),
        },
        max_results: 10,
        layout: Layout { width: 800, radius: 6, ..Layout::default() },
        ..base
    };
    vec![minimal, dense, dev]
}

fn is_hex_color(s: &str) -> bool {
    let Some(h) = s.strip_prefix('#') else { return false };
    matches!(h.len(), 3 | 4 | 6 | 8) && h.chars().all(|c| c.is_ascii_hexdigit())
}

pub fn validate_name(name: &str) -> Result<(), String> {
    let n = name.trim();
    if n.is_empty() || n.chars().count() > 40 {
        return Err(t("le nom doit faire entre 1 et 40 caractères"));
    }
    if n.chars().any(|c| c.is_control() || c == '/' || c == '\\') {
        return Err(t("le nom contient un caractère interdit"));
    }
    Ok(())
}

impl Preset {
    pub fn validate(&self) -> Result<(), String> {
        let mut errs = Vec::new();
        if self.version > SCHEMA_VERSION {
            errs.push(tf("version {v} plus récente que celle de Landry ({s})", &[("v", &self.version), ("s", &SCHEMA_VERSION)]));
        }
        if let Err(e) = validate_name(&self.name) {
            errs.push(e);
        }
        for (label, c) in [("light", &self.light), ("dark", &self.dark)] {
            for (k, v) in [
                ("bg", &c.bg),
                ("fg", &c.fg),
                ("muted", &c.muted),
                ("accent", &c.accent),
                ("selection", &c.selection),
                ("border", &c.border),
            ] {
                if !is_hex_color(v) {
                    errs.push(tf("{f} : couleur invalide « {v} » (attendu #RRGGBB)", &[("f", &format!("{label}.{k}")), ("v", &v)]));
                }
            }
        }
        if self.font.family.trim().is_empty() || self.font.family.contains([';', '{', '}']) {
            errs.push(t("font.family invalide"));
        }
        let ranges: [(&str, i64, i64, i64); 9] = [
            ("font.size", self.font.size as i64, 8, 48),
            ("max_results", self.max_results as i64, 1, 50),
            ("animation_ms", self.animation_ms as i64, 0, 2000),
            ("layout.width", self.layout.width as i64, 280, 4000),
            ("layout.max_height", self.layout.max_height as i64, 60, 3000),
            ("layout.margin", self.layout.margin as i64, 0, 2000),
            ("layout.radius", self.layout.radius as i64, 0, 64),
            ("preview.width", self.preview.width as i64, 20, 70),
            ("layout.offset", self.layout.offset_x.abs().max(self.layout.offset_y.abs()) as i64, 0, 4000),
        ];
        for (k, v, lo, hi) in ranges {
            if v < lo || v > hi {
                errs.push(tf("{k} = {v} hors limites ({lo} à {hi})", &[("k", &k), ("v", &v), ("lo", &lo), ("hi", &hi)]));
            }
        }
        if !(0.2..=1.0).contains(&self.opacity) {
            errs.push(tf("opacity = {v} hors limites (0.2 à 1)", &[("v", &self.opacity)]));
        }
        if errs.is_empty() {
            Ok(())
        } else {
            Err(errs.join(" ; "))
        }
    }

    /// Module ids in display order with known modules missing from the preset appended.
    pub fn module_order(&self) -> Vec<Module> {
        let mut out: Vec<Module> =
            self.modules.iter().filter(|m| MODULES.contains(&m.id.as_str())).cloned().collect();
        for id in MODULES {
            if !out.iter().any(|m| m.id == *id) {
                out.push(Module { id: (*id).into(), enabled: true });
            }
        }
        out
    }

    pub fn parse(json: &str) -> Result<Preset, String> {
        let mut p: Preset = serde_json::from_str(json).map_err(|e| tf("JSON invalide : {e}", &[("e", &e)]))?;
        p.name = p.name.trim().to_string();
        p.validate()?;
        // Older schema versions would be migrated here.
        p.version = SCHEMA_VERSION;
        p.modules = p.module_order();
        Ok(p)
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct Settings {
    pub version: u32,
    pub active_preset: String,
    pub locale: String,
    /// alias -> item id or query, e.g. "nc" -> "project:nacre".
    pub aliases: BTreeMap<String, String>,
    /// prefix -> URL with {q}, e.g. "gh" -> "https://github.com/search?q={q}".
    pub search_prefixes: BTreeMap<String, String>,
    /// Folders indexed for file search. Empty: the XDG user folders (Documents, Downloads...).
    pub file_roots: Vec<String>,
    /// Where git repositories are looked for.
    pub projects_root: String,
    /// Command templates. Placeholders become single arguments, no shell involved.
    pub editor: String,
    pub terminal: String,
    pub terminal_run: String,
    /// Vercel team id for team projects; empty for a personal account.
    pub vercel_team: String,
    /// VAT rate in percent, for "100 ht" / "120 ttc".
    pub vat_rate: f64,
    /// Micro-entreprise contribution rate in percent ("ca 3000"); none until the user sets it.
    pub micro_rate: Option<f64>,
    /// Optional "versement libératoire" rate in percent.
    pub micro_tax_rate: Option<f64>,
    /// Default target language for "tr <texte>" (DeepL code: EN, FR, DE...).
    pub translate_target: String,
    /// LibreTranslate-compatible URL; empty: DeepL with the key from the keyring.
    pub translate_url: String,
}

impl Default for Settings {
    fn default() -> Self {
        let prefixes = [
            ("g", "https://www.google.com/search?q={q}"),
            ("gh", "https://github.com/search?q={q}"),
            ("mdn", "https://developer.mozilla.org/fr/search?q={q}"),
            ("npm", "https://www.npmjs.com/search?q={q}"),
            ("yt", "https://www.youtube.com/results?search_query={q}"),
            ("wiki", "https://fr.wikipedia.org/w/index.php?search={q}"),
        ];
        Settings {
            version: SCHEMA_VERSION,
            active_preset: DEFAULT_NAME.into(),
            locale: "fr".into(),
            aliases: BTreeMap::new(),
            search_prefixes: prefixes.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect(),
            file_roots: Vec::new(),
            projects_root: "~/dev".into(),
            editor: "code {path}".into(),
            terminal: "gnome-terminal --working-directory={dir}".into(),
            terminal_run: "gnome-terminal --working-directory={dir} -- bash -lc {cmd}".into(),
            vercel_team: String::new(),
            vat_rate: 20.0,
            micro_rate: None,
            micro_tax_rate: None,
            translate_target: "EN".into(),
            translate_url: String::new(),
        }
    }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct PresetInfo {
    pub name: String,
    pub builtin: bool,
    /// Validation error, if the file is broken.
    pub error: Option<String>,
}

/// Atomic write: a crash mid-write never leaves a truncated file.
fn write_atomic(path: &Path, content: &str) -> Result<(), String> {
    let dir = path.parent().ok_or("chemin invalide")?;
    std::fs::create_dir_all(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, content).map_err(|e| format!("{}: {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("{}: {e}", path.display()))
}

fn slug(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c.to_ascii_lowercase() } else { '-' })
        .collect();
    let s = s.trim_matches('-').to_string();
    if s.is_empty() { "preset".into() } else { s }
}

pub struct Store {
    dir: PathBuf,
}

impl Store {
    pub fn new(dir: PathBuf) -> Self {
        Store { dir }
    }

    fn presets_dir(&self) -> PathBuf {
        self.dir.join("presets")
    }

    fn settings_path(&self) -> PathBuf {
        self.dir.join("settings.json")
    }

    /// First run: create the folders and write the shipped presets.
    pub fn seed(&self) -> Result<(), String> {
        if self.presets_dir().exists() {
            return Ok(());
        }
        std::fs::create_dir_all(self.presets_dir()).map_err(|e| e.to_string())?;
        for p in shipped_presets() {
            self.write_preset(&p)?;
        }
        Ok(())
    }

    /// Settings never fail to load: a broken file falls back to defaults with a warning.
    pub fn settings(&self) -> (Settings, Option<String>) {
        match std::fs::read_to_string(self.settings_path()) {
            Err(_) => (Settings::default(), None),
            Ok(s) => match serde_json::from_str::<Settings>(&s) {
                Ok(v) => (v, None),
                Err(e) => (Settings::default(), Some(tf("settings.json invalide, réglages par défaut utilisés : {e}", &[("e", &e)]))),
            },
        }
    }

    /// Keep a copy of settings.json before it gets replaced (import).
    pub fn backup_settings(&self) -> Result<(), String> {
        let p = self.settings_path();
        if p.exists() {
            std::fs::copy(&p, p.with_extension("json.bak")).map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    pub fn save_settings(&self, s: &Settings) -> Result<(), String> {
        let json = serde_json::to_string_pretty(s).map_err(|e| e.to_string())?;
        write_atomic(&self.settings_path(), &json)
    }

    /// (file path, parsed result) for every preset file.
    fn files(&self) -> Vec<(PathBuf, Result<Preset, String>)> {
        let Ok(rd) = std::fs::read_dir(self.presets_dir()) else { return vec![] };
        let mut out: Vec<_> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|x| x == "json"))
            .map(|p| {
                let r = std::fs::read_to_string(&p).map_err(|e| e.to_string()).and_then(|s| Preset::parse(&s));
                (p, r)
            })
            .collect();
        out.sort_by(|a, b| a.0.cmp(&b.0));
        out
    }

    fn find_file(&self, name: &str) -> Option<PathBuf> {
        self.files().into_iter().find(|(_, r)| r.as_ref().is_ok_and(|p| p.name == name)).map(|(p, _)| p)
    }

    pub fn list(&self) -> Vec<PresetInfo> {
        let mut out = vec![PresetInfo { name: DEFAULT_NAME.into(), builtin: true, error: None }];
        for (path, r) in self.files() {
            out.push(match r {
                Ok(p) => PresetInfo { name: p.name, builtin: false, error: None },
                Err(e) => PresetInfo {
                    name: path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default(),
                    builtin: false,
                    error: Some(e),
                },
            });
        }
        out
    }

    pub fn get(&self, name: &str) -> Result<Preset, String> {
        if name == DEFAULT_NAME {
            return Ok(Preset::default());
        }
        let path = self.find_file(name).ok_or_else(|| tf("preset « {n} » introuvable", &[("n", &name)]))?;
        let s = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
        Preset::parse(&s)
    }

    /// The active preset, or Défaut with a message if it cannot be used.
    pub fn active(&self) -> (Preset, Option<String>) {
        let (settings, warn) = self.settings();
        match self.get(&settings.active_preset) {
            Ok(p) => (p, warn),
            Err(e) => (
                Preset::default(),
                Some(tf("Preset « {n} » inutilisable ({e}). Retour au preset {d}.", &[("n", &settings.active_preset), ("e", &e), ("d", &DEFAULT_NAME)])),
            ),
        }
    }

    pub fn set_active(&self, name: &str) -> Result<Preset, String> {
        let p = self.get(name)?;
        let (mut s, _) = self.settings();
        s.active_preset = p.name.clone();
        self.save_settings(&s)?;
        Ok(p)
    }

    fn write_preset(&self, p: &Preset) -> Result<(), String> {
        let json = serde_json::to_string_pretty(p).map_err(|e| e.to_string())?;
        let path = self.find_file(&p.name).unwrap_or_else(|| {
            let base = slug(&p.name);
            let mut path = self.presets_dir().join(format!("{base}.json"));
            let mut i = 2;
            while path.exists() {
                path = self.presets_dir().join(format!("{base}-{i}.json"));
                i += 1;
            }
            path
        });
        write_atomic(&path, &json)
    }

    /// Create or update. `previous` is the old name when renaming.
    pub fn save(&self, p: &Preset, previous: Option<&str>) -> Result<(), String> {
        let mut p = p.clone();
        p.name = p.name.trim().to_string();
        p.version = SCHEMA_VERSION;
        p.validate()?;
        if p.name == DEFAULT_NAME || previous == Some(DEFAULT_NAME) {
            return Err(tf("le preset {d} est intégré : duplique-le pour le modifier", &[("d", &DEFAULT_NAME)]));
        }
        let renaming = previous.is_some_and(|old| old != p.name);
        if (renaming || previous.is_none()) && self.find_file(&p.name).is_some() {
            return Err(tf("un preset « {n} » existe déjà", &[("n", &p.name)]));
        }
        let old_path = previous.and_then(|old| self.find_file(old));
        if renaming {
            self.write_preset(&p)?;
            if let Some(old) = old_path {
                std::fs::remove_file(old).map_err(|e| e.to_string())?;
            }
            let (mut s, _) = self.settings();
            if Some(s.active_preset.as_str()) == previous {
                s.active_preset = p.name.clone();
                self.save_settings(&s)?;
            }
            Ok(())
        } else {
            self.write_preset(&p)
        }
    }

    pub fn duplicate(&self, name: &str, new_name: &str) -> Result<Preset, String> {
        let mut p = self.get(name)?;
        p.name = new_name.trim().to_string();
        self.save(&p, None)?;
        Ok(p)
    }

    pub fn delete(&self, name: &str) -> Result<(), String> {
        if name == DEFAULT_NAME {
            return Err(tf("le preset {d} ne peut pas être supprimé", &[("d", &DEFAULT_NAME)]));
        }
        let path = self.find_file(name).ok_or_else(|| tf("preset « {n} » introuvable", &[("n", &name)]))?;
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
        let (mut s, _) = self.settings();
        if s.active_preset == name {
            s.active_preset = DEFAULT_NAME.into();
            self.save_settings(&s)?;
        }
        Ok(())
    }

    /// Import a preset file. A name clash gets a numbered suffix instead of overwriting.
    pub fn import(&self, src: &Path) -> Result<String, String> {
        let s = std::fs::read_to_string(src).map_err(|e| format!("{}: {e}", src.display()))?;
        let mut p = Preset::parse(&s)?;
        let base = p.name.clone();
        let mut i = 2;
        while p.name == DEFAULT_NAME || self.find_file(&p.name).is_some() {
            p.name = format!("{base} ({i})");
            i += 1;
        }
        self.save(&p, None)?;
        Ok(p.name)
    }

    pub fn export(&self, name: &str, dest: &Path) -> Result<(), String> {
        let p = self.get(name)?;
        let json = serde_json::to_string_pretty(&p).map_err(|e| e.to_string())?;
        write_atomic(dest, &json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (Store, PathBuf) {
        let dir = std::env::temp_dir().join(format!(
            "landry-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        let s = Store::new(dir.clone());
        s.seed().unwrap();
        (s, dir)
    }

    #[test]
    fn defaults_and_shipped_are_valid() {
        Preset::default().validate().unwrap();
        for p in shipped_presets() {
            p.validate().unwrap();
        }
    }

    #[test]
    fn seed_lists_builtin_and_shipped() {
        let (s, dir) = store();
        let names: Vec<_> = s.list().into_iter().map(|i| i.name).collect();
        assert_eq!(names[0], DEFAULT_NAME);
        for n in ["Minimal", "Dense", "Développeur"] {
            assert!(names.contains(&n.to_string()), "{n} manquant dans {names:?}");
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn invalid_values_are_rejected() {
        let mut p = Preset::default();
        p.dark.bg = "red; background: url(x)".into();
        p.layout.width = 10;
        p.opacity = 3.0;
        let e = p.validate().unwrap_err();
        assert!(e.contains("dark.bg") && e.contains("layout.width") && e.contains("opacity"), "{e}");
        assert!(Preset::parse(r#"{"inconnu": 1}"#).is_err());
    }

    #[test]
    fn partial_json_fills_defaults() {
        let p = Preset::parse(r#"{"name": "Perso", "max_results": 3}"#).unwrap();
        assert_eq!(p.max_results, 3);
        assert_eq!(p.layout, Layout::default());
        assert_eq!(p.modules.len(), MODULES.len());
    }

    #[test]
    fn broken_active_preset_falls_back_to_default() {
        let (s, dir) = store();
        s.set_active("Dense").unwrap();
        assert_eq!(s.active().0.name, "Dense");
        let path = s.find_file("Dense").unwrap();
        std::fs::write(&path, "{ cassé").unwrap();
        let (p, warn) = s.active();
        assert_eq!(p.name, DEFAULT_NAME);
        assert!(warn.unwrap().contains("Dense"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn crud_rename_duplicate_delete() {
        let (s, dir) = store();
        assert!(s.delete(DEFAULT_NAME).is_err());
        assert!(s.save(&Preset::default(), None).is_err());

        let d = s.duplicate(DEFAULT_NAME, "Mon preset").unwrap();
        assert!(s.duplicate("Minimal", "Mon preset").is_err(), "nom en double");
        s.set_active("Mon preset").unwrap();

        let renamed = Preset { name: "Renommé".into(), ..d };
        s.save(&renamed, Some("Mon preset")).unwrap();
        assert!(s.get("Mon preset").is_err());
        assert_eq!(s.settings().0.active_preset, "Renommé");

        s.delete("Renommé").unwrap();
        assert_eq!(s.settings().0.active_preset, DEFAULT_NAME);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn import_export_roundtrip_without_overwrite() {
        let (s, dir) = store();
        let out = dir.join("export.json");
        s.export("Minimal", &out).unwrap();
        let name = s.import(&out).unwrap();
        assert_eq!(name, "Minimal (2)");
        assert_eq!(s.get("Minimal (2)").unwrap().max_results, 5);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
