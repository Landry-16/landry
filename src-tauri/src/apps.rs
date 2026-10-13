//! Installed applications from .desktop files (XDG, Flatpak, Snap).

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
}

/// Value of a localized key: `Key[fr_FR]`, then `Key[fr]`, then `Key`.
fn localized<'a>(kv: &'a HashMap<String, String>, key: &str, lang: &str) -> Option<&'a String> {
    let short = lang.split(['_', '.', '@']).next().unwrap_or("");
    let full = lang.split(['.', '@']).next().unwrap_or("");
    kv.get(&format!("{key}[{full}]"))
        .or_else(|| kv.get(&format!("{key}[{short}]")))
        .or_else(|| kv.get(key))
}

/// Parse the [Desktop Entry] group. Returns None for entries that must not be shown.
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
    })
}

fn walk(base: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(base, &p, out);
        } else if p.extension().is_some_and(|x| x == "desktop") {
            if let Ok(rel) = p.strip_prefix(base) {
                // Desktop file id: subdirectories joined with '-'.
                out.push((rel.to_string_lossy().replace('/', "-"), p));
            }
        }
    }
}

/// Scan every applications dir. The first dir defining an id wins (user before system).
pub fn scan(data_dirs: &[PathBuf]) -> Vec<App> {
    let lang = std::env::var("LC_MESSAGES")
        .or_else(|_| std::env::var("LANG"))
        .unwrap_or_else(|_| "C".into());
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let mut seen = std::collections::HashSet::new();
    let mut apps = Vec::new();
    for d in data_dirs {
        // Snap exports .desktop files directly in /var/lib/snapd/desktop/applications.
        let base = d.join("applications");
        let mut files = Vec::new();
        walk(&base, &base, &mut files);
        files.sort();
        for (id, path) in files {
            if !seen.insert(id.clone()) {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&path) else { continue };
            if let Some(app) = parse(&id, &path, &content, &lang, &desktop) {
                apps.push(app);
            }
        }
    }
    apps.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    apps
}

/// Launch through gio: it handles field codes, Terminal=true and D-Bus activation.
pub fn launch(path: &Path) -> Result<(), String> {
    use std::os::unix::process::CommandExt;
    std::process::Command::new("gio")
        .arg("launch")
        .arg(path)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .process_group(0)
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
    fn scans_this_machine() {
        // Smoke test on the real system: Ubuntu always ships some applications.
        let apps = scan(&crate::paths::data_dirs());
        assert!(!apps.is_empty());
    }
}
