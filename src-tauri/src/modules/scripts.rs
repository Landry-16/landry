//! User scripts: any file in ~/.config/landry/scripts becomes a command.
//!
//! Header lines (comments, anywhere in the first 30 lines):
//!   # @landry title: Vider le cache npm
//!   # @landry subtitle: npm cache clean
//!   # @landry argument: Nom du paquet      (asked before running, passed as $1)
//!   # @landry mode: silent                 (default: show the output in the bar)

use crate::engine::Outcome;
use crate::i18n::{t, tf};
use crate::search::{Action, Item};
use std::path::{Path, PathBuf};

pub fn dir() -> PathBuf {
    crate::paths::config_dir().join("scripts")
}

#[derive(Debug, Default, PartialEq)]
pub struct Header {
    pub title: Option<String>,
    pub subtitle: Option<String>,
    pub argument: Option<String>,
    pub silent: bool,
}

pub fn parse_header(src: &str) -> Header {
    let mut h = Header::default();
    for line in src.lines().take(30) {
        let Some((_, rest)) = line.split_once("@landry ") else { continue };
        let Some((key, value)) = rest.split_once(':') else { continue };
        let value = value.trim().to_string();
        match key.trim() {
            "title" => h.title = Some(value),
            "subtitle" => h.subtitle = Some(value),
            "argument" => h.argument = Some(value),
            "mode" => h.silent = value == "silent",
            _ => {}
        }
    }
    h
}

/// Interpreter for a script: its shebang if executable, else from the extension.
pub fn command(path: &Path) -> Option<(String, Vec<String>)> {
    use std::os::unix::fs::PermissionsExt;
    let p = path.to_string_lossy().into_owned();
    let executable = std::fs::metadata(path).map(|m| m.permissions().mode() & 0o111 != 0).unwrap_or(false);
    if executable {
        return Some((p, vec![]));
    }
    let interp = match path.extension()?.to_str()? {
        "sh" | "bash" => "bash",
        "py" => "python3",
        "js" | "mjs" | "cjs" => "node",
        _ => return None,
    };
    Some((interp.to_string(), vec![p]))
}

pub fn scan() -> Vec<Item> {
    let Ok(rd) = std::fs::read_dir(dir()) else { return vec![] };
    let mut out: Vec<Item> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && !p.file_name().is_some_and(|n| n.to_string_lossy().starts_with('.')))
        .filter(|p| command(p).is_some())
        .map(|p| {
            let src = std::fs::read_to_string(&p).unwrap_or_default();
            let h = parse_header(&src);
            let name = p.file_stem().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
            let mut run = Action::new("run", "Lancer");
            if let Some(a) = &h.argument {
                run = run.ask(a, "");
            }
            Item {
                id: format!("script:{}", p.display()),
                module: "scripts".into(),
                title: h.title.clone().unwrap_or(name.clone()),
                subtitle: h.subtitle.clone().unwrap_or_else(|| tf("Script · {f}", &[("f", &p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default())])),
                keywords: vec![name, "script".into()],
                actions: vec![run, Action::new("edit", "Modifier le script"), Action::new("reveal", "Ouvrir le dossier des scripts")],
                data: format!("{}\t{}", if h.silent { "silent" } else { "output" }, p.display()),
                ..Default::default()
            }
        })
        .collect();
    out.sort_by(|a, b| a.title.to_lowercase().cmp(&b.title.to_lowercase()));
    out
}

pub fn run(settings: &crate::config::Settings, item: &Item, action: &str, arg: &str) -> Result<Outcome, String> {
    let (mode, path) = item.data.split_once('\t').ok_or_else(|| t("script invalide"))?;
    let path = PathBuf::from(path);
    match action {
        "run" => {
            let (program, mut args) = command(&path).ok_or_else(|| t("type de script non pris en charge"))?;
            if !arg.is_empty() {
                args.push(arg.to_string());
            }
            Ok(Outcome::Run { program, args, label: item.title.clone(), silent: mode == "silent", limit: 60 })
        }
        "edit" => {
            crate::util::run_template(&settings.editor, &[("path", &path.to_string_lossy())], None)?;
            Ok(Outcome::Hide)
        }
        "reveal" => {
            crate::util::open(&dir().to_string_lossy())?;
            Ok(Outcome::Hide)
        }
        a => Err(tf("action inconnue : {a}", &[("a", &a)])),
    }
}

/// Example script written on first run, so the folder explains itself.
pub fn seed() {
    let d = dir();
    if d.exists() {
        return;
    }
    let _ = std::fs::create_dir_all(&d);
    let example = "#!/usr/bin/env bash\n# @landry title: Bonjour depuis un script\n# @landry subtitle: Exemple : modifie ou supprime ce fichier\n# @landry argument: Ton prénom\n\necho \"Bonjour ${1:-toi}, il est $(date +%H:%M).\"\n";
    let path = d.join("bonjour.sh");
    if std::fs::write(&path, example).is_ok() {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn header() {
        let src = "#!/usr/bin/env python3\n# @landry title: Nettoyer\n# @landry argument: Dossier\n// @landry mode: silent\nprint('x')";
        assert_eq!(
            parse_header(src),
            Header { title: Some("Nettoyer".into()), subtitle: None, argument: Some("Dossier".into()), silent: true }
        );
        assert_eq!(parse_header("echo hi"), Header::default());
    }

    #[test]
    fn interpreters() {
        let d = std::env::temp_dir().join(format!("landry-scripts-{}", std::process::id()));
        std::fs::create_dir_all(&d).unwrap();
        let py = d.join("a.py");
        std::fs::write(&py, "print(1)").unwrap();
        assert_eq!(command(&py).unwrap().0, "python3");
        let txt = d.join("notes.txt");
        std::fs::write(&txt, "x").unwrap();
        assert!(command(&txt).is_none());
        use std::os::unix::fs::PermissionsExt;
        let exe = d.join("tool");
        std::fs::write(&exe, "#!/bin/sh\necho ok").unwrap();
        std::fs::set_permissions(&exe, std::fs::Permissions::from_mode(0o755)).unwrap();
        assert_eq!(command(&exe).unwrap(), (exe.to_string_lossy().into_owned(), vec![]));
        std::fs::remove_dir_all(d).unwrap();
    }
}
