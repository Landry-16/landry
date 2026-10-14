//! Installed applications from .desktop files (XDG, Flatpak, Snap), or Start Menu shortcuts on Windows.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub struct App {
    /// Desktop file id, e.g. "org.gnome.Nautilus.desktop".
    pub id: String,
    pub path: PathBuf,
    pub name: String,
    pub generic: String,
    pub comment: String,
    pub keywords: Vec<String>,
    /// Program name from Exec, e.g. "code".
    pub exec_name: String,
    pub icon: String,
    /// Installed by Flatpak or Snap: Landry can uninstall it.
    pub removable: bool,
}

/// Value of a localized key: `Key[fr_FR]`, then `Key[fr]`, then `Key`.
#[cfg_attr(windows, allow(dead_code))]
fn localized<'a>(kv: &'a HashMap<String, String>, key: &str, lang: &str) -> Option<&'a String> {
    let short = lang.split(['_', '.', '@']).next().unwrap_or("");
    let full = lang.split(['.', '@']).next().unwrap_or("");
    kv.get(&format!("{key}[{full}]"))
        .or_else(|| kv.get(&format!("{key}[{short}]")))
        .or_else(|| kv.get(key))
}

/// Parse the [Desktop Entry] group. Returns None for entries that must not be shown.
#[cfg_attr(windows, allow(dead_code))]
pub fn parse(id: &str, path: &Path, content: &str, lang: &str, desktop: &str) -> Option<App> {
    let mut kv = HashMap::new();
    let mut in_entry = false;
    for line in content.lines() {
        let line = line.trim();
        if line.starts_with('[') {
            in_entry = line == "[Desktop Entry]";
            continue;
        }
        if !in_entry || line.starts_with('#') {
            continue;
        }
        if let Some((k, v)) = line.split_once('=') {
            kv.entry(k.trim().to_string()).or_insert_with(|| v.trim().to_string());
        }
    }
    let flag = |k: &str| kv.get(k).is_some_and(|v| v == "true");
    if kv.get("Type").map(String::as_str) != Some("Application") || flag("NoDisplay") || flag("Hidden") {
        return None;
    }
    let list = |k: &str| -> Vec<String> {
        kv.get(k).map(|v| v.split(';').filter(|s| !s.is_empty()).map(String::from).collect()).unwrap_or_default()
    };
    let desktops: Vec<&str> = desktop.split(':').filter(|s| !s.is_empty()).collect();
    let only = list("OnlyShowIn");
    if !only.is_empty() && !only.iter().any(|d| desktops.contains(&d.as_str())) {
        return None;
    }
    if list("NotShowIn").iter().any(|d| desktops.contains(&d.as_str())) {
        return None;
    }
    let exec = kv.get("Exec")?;
    let exec_name = exec
        .split_whitespace()
        .map(|w| w.trim_matches('"'))
        // Skip `env` and VAR=value prefixes to reach the real program.
        .find(|w| !(w.contains('=') && !w.starts_with('/')) && !matches!(*w, "env" | "/usr/bin/env"))
        .and_then(|w| w.rsplit('/').next())
        .unwrap_or("")
        .to_string();
    let keywords = localized(&kv, "Keywords", lang)
        .map(|v| v.split(';').filter(|s| !s.is_empty()).map(String::from).collect())
        .unwrap_or_default();
    Some(App {
        id: id.to_string(),
        path: path.to_path_buf(),
        name: localized(&kv, "Name", lang)?.clone(),
        generic: localized(&kv, "GenericName", lang).cloned().unwrap_or_default(),
        comment: localized(&kv, "Comment", lang).cloned().unwrap_or_default(),
        keywords,
        exec_name,
        icon: kv.get("Icon").cloned().unwrap_or_default(),
        removable: kv.contains_key("X-Flatpak") || kv.contains_key("X-SnapInstanceName"),
    })
}

/// Entry files: .desktop, or .lnk shortcuts in the Windows Start Menu.
const ENTRY: &str = if cfg!(windows) { "lnk" } else { "desktop" };

fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(base, &p, out);
        } else if p.extension().is_some_and(|x| x.eq_ignore_ascii_case(ENTRY)) {
            if let Ok(rel) = p.strip_prefix(base) {
                // Desktop file id: subdirectories joined with '-'.
                out.push((rel.to_string_lossy().replace(['/', '\\'], "-"), p));
            }
        }
    }
}

/// A Start Menu shortcut: its file name is the app name. Uninstallers are left out.
#[cfg(windows)]
fn shortcut(id: &str, path: &Path) -> Option<App> {
    let name = path.file_stem()?.to_string_lossy().into_owned();
    let lower = name.to_lowercase();
    if lower.starts_with("uninstall") || lower.starts_with("désinstaller") {
        return None;
    }
    Some(App {
        id: id.to_string(),
        path: path.to_path_buf(),
        name,
        generic: String::new(),
        comment: String::new(),
        keywords: vec![],
        exec_name: String::new(),
        icon: String::new(),
        removable: false,
    })
}

/// Scan every applications dir. The first dir defining an id wins (user before system).
pub fn scan(app_dirs: &[PathBuf]) -> Vec<App> {
    #[cfg(not(windows))]
    let lang = std::env::var("LC_MESSAGES")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_else(|_| "C".into());
    #[cfg(not(windows))]
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let mut seen = std::collections::HashSet::new();
    let mut apps = Vec::new();
    for base in app_dirs {
        let mut files = Vec::new();
        walk(base, base, &mut files);
        files.sort();
        for (id, path) in files {
            if !seen.insert(id.clone()) {
                continue;
            }
            #[cfg(windows)]
            let app = shortcut(&id, &path);
            #[cfg(not(windows))]
            let Ok(content) = std::fs::read_to_string(&path) else { continue };
            #[cfg(not(windows))]
            let app = parse(&id, &path, &content, &lang, &desktop);
            apps.extend(app);
        }
    }
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

/// Command removing the package behind a .desktop file, for Flatpak and Snap only (apt
/// packages are left to the Software app). Polkit asks for the password when needed.
pub fn uninstaller(path: &Path, content: &str, home: &Path) -> Option<(String, Vec<String>)> {
    let key = |k: &str| content.lines().find_map(|l| l.trim().strip_prefix(k)?.strip_prefix('=').map(str::trim)).filter(|v| !v.is_empty() && !v.starts_with('-'));
    if let Some(id) = key("X-Flatpak") {
        let scope = if path.starts_with(home) { "--user" } else { "--system" };
        return Some(("flatpak".into(), ["uninstall", "-y", "--noninteractive", scope, id].map(String::from).to_vec()));
    }
    key("X-SnapInstanceName").map(|name| ("pkexec".into(), ["snap", "remove", name].map(String::from).to_vec()))
}

/// Calls `on_change` once the applications folders have been quiet for 2 s after a change
/// (install, removal, update). Only folders existing at startup are watched.
pub fn watch(app_dirs: &[PathBuf], mut on_change: impl FnMut() + Send + 'static) {
    use notify::{RecursiveMode, Watcher};
    let (tx, rx) = std::sync::mpsc::channel();
    let Ok(mut watcher) = notify::recommended_watcher(move |res: notify::Result<notify::Event>| {
        if res.is_ok_and(|e| !e.kind.is_access()) {
            let _ = tx.send(());
        }
    }) else {
        return;
    };
    for d in app_dirs.iter().filter(|d| d.is_dir()) {
        let _ = watcher.watch(d, RecursiveMode::Recursive);
    }
    while rx.recv().is_ok() {
        while rx.recv_timeout(std::time::Duration::from_secs(2)).is_ok() {}
        on_change();
    }
}

/// Launch through gio: it handles field codes, Terminal=true and D-Bus activation.
/// On Windows, Explorer opens the shortcut.
pub fn launch(path: &Path) -> Result<(), String> {
    let mut c = std::process::Command::new(if cfg!(windows) { "explorer" } else { "gio" });
    if !cfg!(windows) {
        c.arg("launch");
    }
    crate::util::detach(c.arg(path).stdin(std::process::Stdio::null()).stdout(std::process::Stdio::null()).stderr(std::process::Stdio::null()))
        .spawn()
        .map(|_| ())
        .map_err(|e| crate::i18n::tf("lancement impossible : {e}", &[("e", &e)]))
}

#[cfg(test)]
mod tests {
    use super::*;

    const CODE: &str = "[Desktop Entry]\nName=Visual Studio Code\nComment=Code Editing. Redefined.\nGenericName=Text Editor\nExec=/usr/share/code/code %F\nIcon=vscode\nType=Application\nKeywords=vscode;\n\n[Desktop Action new-empty-window]\nName=New Empty Window\nExec=/usr/share/code/code --new-window %F\n";

    #[test]
    fn parses_entry_and_ignores_actions() {
        let a = parse("code.desktop", Path::new("/x/code.desktop"), CODE, "fr_FR.UTF-8", "ubuntu:GNOME").unwrap();
        assert_eq!(a.name, "Visual Studio Code");
        assert_eq!(a.exec_name, "code");
        assert_eq!(a.keywords, vec!["vscode"]);
        assert_eq!(a.icon, "vscode");
    }

    #[test]
    fn prefers_localized_name() {
        let s = "[Desktop Entry]\nType=Application\nName=Files\nName[fr]=Fichiers\nExec=nautilus --new-window %U\n";
        let a = parse("n.desktop", Path::new("/n"), s, "fr_FR.UTF-8", "GNOME").unwrap();
        assert_eq!(a.name, "Fichiers");
        assert_eq!(a.exec_name, "nautilus");
    }

    #[test]
    fn env_prefix_in_exec_is_skipped() {
        let s = "[Desktop Entry]\nType=Application\nName=X\nExec=env FOO=1 /opt/x/bin/xapp --flag\n";
        assert_eq!(parse("x.desktop", Path::new("/x"), s, "C", "").unwrap().exec_name, "xapp");
        let s = "[Desktop Entry]\nType=Application\nName=X\nExec=FOO=1 /opt/x/bin/xapp\n";
        assert_eq!(parse("x.desktop", Path::new("/x"), s, "C", "").unwrap().exec_name, "xapp");
    }

    #[test]
    fn hidden_and_desktop_filters() {
        let base = "[Desktop Entry]\nType=Application\nName=X\nExec=x\n";
        assert!(parse("x", Path::new("/x"), &format!("{base}NoDisplay=true\n"), "C", "GNOME").is_none());
        assert!(parse("x", Path::new("/x"), &format!("{base}OnlyShowIn=KDE;\n"), "C", "ubuntu:GNOME").is_none());
        assert!(parse("x", Path::new("/x"), &format!("{base}OnlyShowIn=GNOME;\n"), "C", "ubuntu:GNOME").is_some());
        assert!(parse("x", Path::new("/x"), &format!("{base}NotShowIn=GNOME;\n"), "C", "ubuntu:GNOME").is_none());
        assert!(parse("x", Path::new("/x"), "[Desktop Entry]\nType=Link\nName=X\nURL=y\n", "C", "").is_none());
    }

    #[test]
    fn uninstall_only_flatpak_and_snap() {
        let home = Path::new("/home/u");
        let base = "[Desktop Entry]\nType=Application\nName=X\nExec=x\n";
        let fp = format!("{base}X-Flatpak=org.gimp.GIMP\n");
        assert!(parse("g", Path::new("/g"), &fp, "C", "").unwrap().removable);
        let user = uninstaller(Path::new("/home/u/.local/share/flatpak/exports/share/applications/g.desktop"), &fp, home).unwrap();
        assert_eq!(user, ("flatpak".into(), vec!["uninstall".into(), "-y".into(), "--noninteractive".into(), "--user".into(), "org.gimp.GIMP".into()]));
        assert_eq!(uninstaller(Path::new("/var/lib/flatpak/x.desktop"), &fp, home).unwrap().1[3], "--system");
        let snap = format!("{base}X-SnapInstanceName=firefox\n");
        assert_eq!(uninstaller(Path::new("/var/lib/snapd/desktop/applications/firefox_firefox.desktop"), &snap, home).unwrap(), ("pkexec".into(), vec!["snap".into(), "remove".into(), "firefox".into()]));
        assert!(uninstaller(Path::new("/usr/share/applications/x.desktop"), base, home).is_none());
        assert!(!parse("x", Path::new("/x"), base, "C", "").unwrap().removable);
        assert!(uninstaller(Path::new("/x"), &format!("{base}X-Flatpak=--help\n"), home).is_none(), "jamais une option");
    }

    #[test]
    fn scans_this_machine() {
        // Smoke test on the real system: Ubuntu always ships some applications.
        let apps = scan(&crate::paths::app_dirs());
        assert!(!apps.is_empty());
    }
}
