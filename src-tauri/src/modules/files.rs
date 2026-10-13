//! File search: index of chosen folders, recent files, GTK bookmarks, filters and actions.

use super::Preview;
use crate::config::Settings;
use crate::engine::Outcome;
use crate::i18n::{t, tf};
use crate::search::{Action, Item};
use crate::util;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const MAX_ENTRIES: usize = 200_000;
const MAX_DEPTH: usize = 12;
const SKIP_DIRS: &[&str] = &["node_modules", "target", "__pycache__", "venv", "site-packages"];

const KINDS: &[(&str, &[&str])] = &[
    ("img", &["png", "jpg", "jpeg", "gif", "webp", "svg", "bmp", "heic", "avif", "tif", "tiff"]),
    ("pdf", &["pdf"]),
    ("doc", &["odt", "doc", "docx", "rtf", "txt", "md", "ods", "xls", "xlsx", "csv", "odp", "ppt", "pptx"]),
    ("vid", &["mp4", "mkv", "webm", "mov", "avi"]),
    ("audio", &["mp3", "flac", "ogg", "opus", "wav", "m4a"]),
    ("code", &["rs", "js", "ts", "svelte", "py", "go", "c", "h", "cpp", "java", "kt", "html", "css", "json", "toml", "yaml", "yml", "sh"]),
    ("zip", &["zip", "gz", "xz", "zst", "bz2", "7z", "rar", "tar"]),
];

/// Type and date filters typed in the query: `img:`, `pdf:`, `dir:`, `<7j`, `<2s`, `<3m`, `<1a`.
#[derive(Debug, Default, PartialEq)]
pub struct Filter {
    exts: Vec<&'static str>,
    dirs_only: bool,
    max_age: Option<i64>,
}

impl Filter {
    pub fn is_empty(&self) -> bool {
        self.exts.is_empty() && !self.dirs_only && self.max_age.is_none()
    }

    pub fn matches(&self, item: &Item) -> bool {
        let Some((mtime, is_dir, path)) = parts(item) else { return false };
        if self.dirs_only && !is_dir {
            return false;
        }
        if !self.exts.is_empty() {
            let ext = Path::new(path).extension().and_then(|e| e.to_str()).map(str::to_lowercase).unwrap_or_default();
            if is_dir || !self.exts.contains(&ext.as_str()) {
                return false;
            }
        }
        self.max_age.is_none_or(|age| now() - mtime <= age)
    }
}

fn age(token: &str) -> Option<i64> {
    let t = token.strip_prefix('<')?;
    let unit = t.chars().last()?;
    let n: i64 = t[..t.len() - unit.len_utf8()].parse().ok()?;
    let day = 86_400;
    Some(n * match unit {
        'h' => 3600,
        'j' | 'd' => day,
        's' | 'w' => 7 * day,
        'm' => 30 * day,
        'a' | 'y' => 365 * day,
        _ => return None,
    })
}

/// Split filter tokens from the text to search.
pub fn parse_filters(query: &str) -> (String, Filter) {
    let mut f = Filter::default();
    let mut text = Vec::new();
    for tok in query.split_whitespace() {
        if let Some(a) = age(tok) {
            f.max_age = Some(a);
            continue;
        }
        if let Some((kind, rest)) = tok.split_once(':') {
            if kind == "dir" {
                f.dirs_only = true;
            } else if let Some((_, exts)) = KINDS.iter().find(|(k, _)| *k == kind) {
                f.exts.extend_from_slice(exts);
            } else {
                text.push(tok);
                continue;
            }
            if !rest.is_empty() {
                text.push(rest);
            }
            continue;
        }
        text.push(tok);
    }
    (text.join(" "), f)
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Item payload: "mtime(12 digits)\t(d|f)\tpath". Zero-padded so string order is date order.
fn data(mtime: i64, is_dir: bool, path: &Path) -> String {
    format!("{:012}\t{}\t{}", mtime.max(0), if is_dir { 'd' } else { 'f' }, path.display())
}

pub fn parts(item: &Item) -> Option<(i64, bool, &str)> {
    let mut it = item.data.splitn(3, '\t');
    let m = it.next()?.parse().ok()?;
    let d = it.next()? == "d";
    Some((m, d, it.next()?))
}

fn home() -> PathBuf {
    crate::paths::expand("~")
}

fn pretty(path: &Path) -> String {
    match path.strip_prefix(home()) {
        Ok(rel) => format!("~/{}", rel.display()),
        Err(_) => path.display().to_string(),
    }
}

fn actions(is_dir: bool, name: &str) -> Vec<Action> {
    let mut a = vec![Action::new("open", "Ouvrir"), Action::new("reveal", "Afficher dans le dossier")];
    if is_dir {
        a.push(Action::new("terminal", "Ouvrir un terminal ici"));
    }
    a.extend([
        Action::new("copy-path", "Copier le chemin"),
        Action::new("rename", "Renommer").ask("Nouveau nom", name),
        Action::new("move", "Déplacer vers…").ask("Dossier de destination", "~/"),
        Action::new("compress", "Compresser (.zip)"),
        Action::new("trash", "Mettre à la corbeille").danger(),
    ]);
    a
}

pub fn item(path: &Path, is_dir: bool, mtime: i64, tag: &str) -> Item {
    let name = path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| path.display().to_string());
    let parent = path.parent().map(pretty).unwrap_or_default();
    Item {
        id: format!("{tag}:{}", path.display()),
        module: "files".into(),
        subtitle: if tag == "file" { parent } else { format!("{} · {parent}", if tag == "recent" { t("Récent") } else { t("Favori") }) },
        keywords: match tag {
            "recent" => vec!["recent".into(), "récent".into()],
            "bookmark" => vec!["favori".into(), "signet".into()],
            _ => vec![],
        },
        actions: actions(is_dir, &name),
        title: name,
        data: data(mtime, is_dir, path),
        ..Default::default()
    }
}

fn mtime_of(meta: &std::fs::Metadata) -> i64 {
    meta.modified().ok().and_then(|t| t.duration_since(UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0)
}

/// Folders to index: settings, or the XDG user folders.
pub fn roots(settings: &Settings) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = if settings.file_roots.is_empty() {
        ["DOCUMENTS", "DOWNLOAD", "DESKTOP", "PICTURES", "MUSIC", "VIDEOS"]
            .iter()
            .filter_map(|k| util::output("xdg-user-dir", &[k], None).ok())
            .map(|s| PathBuf::from(s.trim()))
            .collect()
    } else {
        settings.file_roots.iter().map(|r| crate::paths::expand(r)).collect()
    };
    let home = home();
    out.retain(|p| p.is_dir() && *p != home);
    out.sort();
    out.dedup();
    out
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<Item>) {
    if depth > MAX_DEPTH || out.len() >= MAX_ENTRIES {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let name = e.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        let Ok(meta) = e.metadata() else { continue };
        let path = e.path();
        // Symlinks are listed but never followed (loops).
        let is_dir = meta.is_dir() && !e.file_type().is_ok_and(|t| t.is_symlink());
        out.push(item(&path, is_dir, mtime_of(&meta), "file"));
        if is_dir && !SKIP_DIRS.contains(&name.as_ref()) {
            walk(&path, depth + 1, out);
        }
        if out.len() >= MAX_ENTRIES {
            return;
        }
    }
}

fn file_url_to_path(href: &str) -> Option<PathBuf> {
    url::Url::parse(href).ok()?.to_file_path().ok()
}

/// GTK recent files (~/.local/share/recently-used.xbel), most recent first.
fn recent() -> Vec<Item> {
    let Ok(xml) = std::fs::read_to_string(crate::paths::data_dir().with_file_name("recently-used.xbel")) else {
        return vec![];
    };
    let re = regex::Regex::new(r#"<bookmark href="(file://[^"]+)"[^>]*?modified="([^"]+)""#).expect("regex valide");
    let mut v: Vec<(i64, PathBuf)> = re
        .captures_iter(&xml)
        .filter_map(|c| {
            let p = file_url_to_path(&c[1])?;
            let t = chrono::DateTime::parse_from_rfc3339(&c[2]).ok()?.timestamp();
            p.exists().then_some((t, p))
        })
        .collect();
    v.sort_by(|a, b| b.0.cmp(&a.0));
    v.into_iter().take(100).map(|(t, p)| item(&p, p.is_dir(), t, "recent")).collect()
}

/// GTK bookmarks (Files sidebar favorites).
fn bookmarks() -> Vec<Item> {
    let path = home().join(".config/gtk-3.0/bookmarks");
    let Ok(s) = std::fs::read_to_string(path) else { return vec![] };
    s.lines()
        .filter_map(|l| {
            let (href, label) = l.split_once(' ').unwrap_or((l, ""));
            let p = file_url_to_path(href)?;
            let mut it = item(&p, true, 0, "bookmark");
            if !label.is_empty() {
                it.title = label.to_string();
            }
            Some(it)
        })
        .collect()
}

pub fn scan(settings: &Settings) -> Vec<Item> {
    let mut out = bookmarks();
    out.extend(recent());
    for r in roots(settings) {
        walk(&r, 0, &mut out);
    }
    out
}

/// Rescan when a watched folder changes, a few seconds after the last event.
/// shortcut: full rescan instead of incremental updates; fine up to ~100k files, make it incremental beyond.
pub fn watch(settings: Settings, mut on_change: impl FnMut(Vec<Item>) + Send + 'static) {
    use notify::{RecursiveMode, Watcher};
    let (tx, rx) = std::sync::mpsc::channel();
    let mut watcher = match notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if let Ok(e) = res {
            if !e.kind.is_access() {
                let _ = tx.send(());
            }
        }
    }) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("landry: surveillance des fichiers impossible : {e}");
            return;
        }
    };
    for r in roots(&settings) {
        if let Err(e) = watcher.watch(&r, RecursiveMode::Recursive) {
            eprintln!("landry: surveillance de {} impossible : {e}", r.display());
        }
    }
    while rx.recv().is_ok() {
        // Debounce: wait for a quiet period.
        while rx.recv_timeout(Duration::from_secs(2)).is_ok() {}
        on_change(scan(&settings));
    }
}

fn reveal(path: &str) -> Result<(), String> {
    let uri = url::Url::from_file_path(path).map_err(|_| t("chemin invalide"))?;
    util::output(
        "gdbus",
        &[
            "call", "--session", "--dest", "org.freedesktop.FileManager1",
            "--object-path", "/org/freedesktop/FileManager1",
            "--method", "org.freedesktop.FileManager1.ShowItems",
            &format!("['{uri}']"), "",
        ],
        None,
    )
    .map(|_| ())
}

fn rename(path: &Path, new_name: &str) -> Result<PathBuf, String> {
    let n = new_name.trim();
    if n.is_empty() || n.contains('/') || n == "." || n == ".." {
        return Err(t("nom invalide"));
    }
    let dest = path.with_file_name(n);
    if dest.exists() {
        return Err(tf("« {n} » existe déjà", &[("n", &n)]));
    }
    std::fs::rename(path, &dest).map_err(|e| e.to_string())?;
    Ok(dest)
}

fn move_to(path: &Path, dir: &str) -> Result<PathBuf, String> {
    let dir = crate::paths::expand(dir.trim());
    if !dir.is_dir() {
        return Err(tf("{p} n'est pas un dossier", &[("p", &dir.display())]));
    }
    let dest = dir.join(path.file_name().ok_or_else(|| t("chemin invalide"))?);
    if dest.exists() {
        return Err(tf("{p} existe déjà", &[("p", &pretty(&dest))]));
    }
    // rename fails across file systems: copy-then-delete is left to the file manager.
    std::fs::rename(path, &dest).map_err(|e| tf("déplacement impossible : {e}", &[("e", &e)]))?;
    Ok(dest)
}

fn compress(path: &Path) -> Result<PathBuf, String> {
    let parent = path.parent().ok_or_else(|| t("chemin invalide"))?;
    let name = path.file_name().ok_or_else(|| t("chemin invalide"))?.to_string_lossy().into_owned();
    let mut dest = parent.join(format!("{name}.zip"));
    let mut i = 2;
    while dest.exists() {
        dest = parent.join(format!("{name} ({i}).zip"));
        i += 1;
    }
    let d = dest.file_name().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
    util::output("zip", &["-r", "-q", &d, &name], Some(parent))?;
    Ok(dest)
}

pub fn run(settings: &Settings, item: &Item, action: &str, arg: &str) -> Result<Outcome, String> {
    let (_, is_dir, path) = parts(item).ok_or_else(|| t("élément de fichier invalide"))?;
    let p = Path::new(path);
    if !p.exists() {
        return Err(tf("{p} n'existe plus", &[("p", &pretty(p))]));
    }
    Ok(match action {
        "open" => {
            util::open(path)?;
            Outcome::Hide
        }
        "reveal" => {
            reveal(path)?;
            Outcome::Hide
        }
        "terminal" if is_dir => {
            util::run_template(&settings.terminal, &[("dir", path)], Some(p))?;
            Outcome::Hide
        }
        "copy-path" => Outcome::Copy(path.to_string()),
        "rename" => Outcome::Message(tf("Renommé en {p}", &[("p", &pretty(&rename(p, arg)?))])),
        "move" => Outcome::Message(tf("Déplacé vers {p}", &[("p", &pretty(&move_to(p, arg)?))])),
        "compress" => Outcome::Message(tf("Archive créée : {p}", &[("p", &pretty(&compress(p)?))])),
        "trash" => {
            util::output("gio", &["trash", path], None)?;
            Outcome::Message(tf("{p} mis à la corbeille", &[("p", &pretty(p))]))
        }
        other => {
            if let Some(app_id) = other.strip_prefix("open-with:") {
                open_with(app_id, path)?;
                return Ok(Outcome::Hide);
            }
            return Err(tf("action inconnue : {a}", &[("a", &other)]));
        }
    })
}

/// "Open with" actions for the file's type, computed when the action panel opens.
pub fn extra_actions(item: &Item) -> Vec<Action> {
    use gtk::gio::prelude::AppInfoExt;
    let Some((_, false, path)) = parts(item) else { return vec![] };
    let (ctype, _) = gtk::gio::content_type_guess(Some(path), &[]);
    gtk::gio::AppInfo::recommended_for_type(&ctype)
        .into_iter()
        .filter_map(|a| {
            let id = a.id()?;
            Some(Action::new(&format!("open-with:{id}"), &tf("Ouvrir avec {app}", &[("app", &a.display_name())])))
        })
        .collect()
}

fn open_with(app_id: &str, path: &str) -> Result<(), String> {
    use gtk::gio::prelude::AppInfoExt;
    let app = gtk::gio::DesktopAppInfo::new(app_id).ok_or_else(|| tf("application introuvable : {a}", &[("a", &app_id)]))?;
    let file = gtk::gio::File::for_path(path);
    app.launch(&[file], gtk::gio::AppLaunchContext::NONE).map_err(|e| e.to_string())
}

fn human_size(n: u64) -> String {
    let units = ["o", "Ko", "Mo", "Go", "To"];
    let mut v = n as f64;
    let mut i = 0;
    while v >= 1024.0 && i < units.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    if i == 0 { format!("{n} o") } else { format!("{v:.1} {}", units[i]) }
}

pub fn preview(item: &Item) -> Preview {
    let Some((mtime, is_dir, path)) = parts(item) else { return Preview::None };
    let p = Path::new(path);
    if is_dir {
        let mut names: Vec<String> = std::fs::read_dir(p)
            .map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).filter(|n| !n.starts_with('.')).collect())
            .unwrap_or_default();
        names.sort();
        let total = names.len();
        names.truncate(40);
        let mut text = names.join("\n");
        if total > 40 {
            text.push_str(&format!("\n{}", tf("… et {n} autres", &[("n", &(total - 40))])));
        }
        return Preview::Text { text };
    }
    let Ok(meta) = std::fs::metadata(p) else { return Preview::None };
    let ext = p.extension().and_then(|e| e.to_str()).map(str::to_lowercase).unwrap_or_default();
    let mime = match ext.as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        "svg" => Some("image/svg+xml"),
        _ => None,
    };
    if let Some(mime) = mime {
        if meta.len() <= 8 * 1024 * 1024 {
            if let Ok(bytes) = std::fs::read(p) {
                use base64::Engine as _;
                let src = format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes));
                return Preview::Image { src };
            }
        }
    }
    let modified = chrono::DateTime::from_timestamp(mtime, 0)
        .map(|d| d.with_timezone(&chrono::Local).format("%d/%m/%Y %H:%M").to_string())
        .unwrap_or_default();
    let mut rows = vec![(t("Taille"), human_size(meta.len())), (t("Modifié"), modified)];
    // Text files: show the beginning.
    if meta.len() <= 1024 * 1024 {
        if let Ok(bytes) = std::fs::read(p) {
            let head = &bytes[..bytes.len().min(4096)];
            if let Ok(text) = std::str::from_utf8(head) {
                if !text.contains('\0') {
                    return Preview::Text { text: text.to_string() };
                }
            }
        }
    }
    rows.push((t("Chemin"), pretty(p)));
    Preview::Props { rows }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmpdir(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("landry-files-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn filters_are_parsed_out_of_the_text() {
        let (t, f) = parse_filters("img: vacances <7j");
        assert_eq!(t, "vacances");
        assert!(f.exts.contains(&"jpg"));
        assert_eq!(f.max_age, Some(7 * 86_400));
        let (t, f) = parse_filters("pdf:facture");
        assert_eq!(t, "facture");
        assert_eq!(f.exts, vec!["pdf"]);
        let (t, f) = parse_filters("http://x.fr a:b");
        assert_eq!(t, "http://x.fr a:b");
        assert!(f.is_empty());
        assert_eq!(parse_filters("dir: <2s").1, Filter { dirs_only: true, max_age: Some(14 * 86_400), ..Default::default() });
    }

    #[test]
    fn filter_matching() {
        let n = now();
        let pdf = item(Path::new("/x/Facture.PDF"), false, n - 3600, "file");
        let old = item(Path::new("/x/old.pdf"), false, n - 90 * 86_400, "file");
        let dir = item(Path::new("/x/Photos"), true, n, "file");
        let (_, f) = parse_filters("pdf: <1m");
        assert!(f.matches(&pdf));
        assert!(!f.matches(&old));
        assert!(!f.matches(&dir));
        assert!(parse_filters("dir:").1.matches(&dir));
    }

    #[test]
    fn walk_skips_hidden_and_heavy_dirs() {
        let d = tmpdir("walk");
        std::fs::create_dir_all(d.join("projet/node_modules/pkg")).unwrap();
        std::fs::create_dir_all(d.join(".cache")).unwrap();
        std::fs::write(d.join("projet/notes.md"), "salut").unwrap();
        std::fs::write(d.join("projet/node_modules/pkg/index.js"), "").unwrap();
        let mut out = Vec::new();
        walk(&d, 0, &mut out);
        let names: Vec<_> = out.iter().map(|i| i.title.clone()).collect();
        assert!(names.contains(&"notes.md".to_string()));
        assert!(names.contains(&"node_modules".to_string()), "listé mais pas parcouru");
        assert!(!names.contains(&"index.js".to_string()));
        assert!(!names.contains(&".cache".to_string()));
        std::fs::remove_dir_all(d).unwrap();
    }

    #[test]
    fn rename_move_compress() {
        let d = tmpdir("ops");
        let f = d.join("a.txt");
        std::fs::write(&f, "x").unwrap();
        assert!(rename(&f, "../evil").is_err());
        let r = rename(&f, "b.txt").unwrap();
        assert!(r.exists() && !f.exists());
        std::fs::create_dir(d.join("sub")).unwrap();
        let m = move_to(&r, d.join("sub").to_str().unwrap()).unwrap();
        assert_eq!(m, d.join("sub/b.txt"));
        assert!(move_to(&m, "/nonexistent-dir").is_err());
        if which("zip") {
            let z = compress(&d.join("sub")).unwrap();
            assert_eq!(z, d.join("sub.zip"));
            assert_eq!(compress(&d.join("sub")).unwrap(), d.join("sub (2).zip"));
        }
        std::fs::remove_dir_all(d).unwrap();
    }

    fn which(p: &str) -> bool {
        std::process::Command::new("which").arg(p).output().is_ok_and(|o| o.status.success())
    }

    #[test]
    fn preview_text_and_dir() {
        let d = tmpdir("prev");
        std::fs::write(d.join("n.md"), "# Titre").unwrap();
        let it = item(&d.join("n.md"), false, now(), "file");
        assert_eq!(preview(&it), Preview::Text { text: "# Titre".into() });
        let dir = item(&d, true, now(), "file");
        assert_eq!(preview(&dir), Preview::Text { text: "n.md".into() });
        std::fs::remove_dir_all(d).unwrap();
    }
}
