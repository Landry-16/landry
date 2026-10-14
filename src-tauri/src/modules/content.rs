//! `/contenu devis 2024`: files whose content contains a text, in the indexed folders and the
//! projects folder. ripgrep when installed (fast, skips hidden and ignored files), grep otherwise.
//! Runs in the background through the cache of the cloud module; results are ordinary file items.

use crate::config::Settings;
use crate::i18n::{t, tf};
use crate::search::Item;
use crate::util;
use std::path::{Path, PathBuf};
use std::time::Duration;

const MAX_FILES: usize = 30;

pub fn query(settings: &Settings, arg: &str) -> Vec<Item> {
    if arg.chars().count() < 2 {
        return vec![Item {
            id: "content:hint".into(),
            module: "files".into(),
            title: "/contenu <texte>".into(),
            subtitle: t("Cherche dans le contenu des fichiers des dossiers indexés et des projets"),
            ..Default::default()
        }];
    }
    crate::modules::cloud::cached(&format!("content\t{arg}"), settings, fetch)
}

fn roots(settings: &Settings) -> Vec<PathBuf> {
    let mut r = crate::modules::files::roots(settings);
    let projects = crate::paths::expand(&settings.projects_root);
    if projects.is_dir() && !r.iter().any(|p| projects.starts_with(p)) {
        r.push(projects);
    }
    r
}

/// Arguments of the search program. Paths come after `--` and are printed followed by a NUL byte.
pub fn command(text: &str, roots: &[PathBuf], ripgrep: bool) -> (String, Vec<String>) {
    let mut a: Vec<String> = if ripgrep {
        ["--null", "--no-heading", "--line-number", "--smart-case", "--fixed-strings", "--max-count", "1", "--max-filesize", "5M", "--max-columns", "300", "--no-messages"]
            .map(String::from)
            .to_vec()
    } else {
        ["-r", "-I", "-n", "-i", "-F", "-Z", "-m", "1", "-s", "--exclude-dir=.git", "--exclude-dir=node_modules", "--exclude-dir=target", "--exclude-dir=.cache"]
            .map(String::from)
            .to_vec()
    };
    a.push("--".into());
    a.push(text.into());
    a.extend(roots.iter().map(|r| r.to_string_lossy().into_owned()));
    ((if ripgrep { "rg" } else { "grep" }).into(), a)
}

/// `path\0line:text` lines -> (path, line, text), one per file.
pub fn parse(out: &str) -> Vec<(PathBuf, u32, String)> {
    let mut seen = std::collections::HashSet::new();
    out.lines()
        .filter_map(|l| {
            let (path, rest) = l.split_once('\0')?;
            let (n, text) = rest.split_once(':')?;
            Some((PathBuf::from(path), n.parse().ok()?, text.trim().to_string()))
        })
        .filter(|(p, _, _)| seen.insert(p.clone()))
        .take(MAX_FILES)
        .collect()
}

fn fetch(settings: &Settings, key: &str) -> Result<Vec<Item>, String> {
    let text = key.split_once('\t').map(|(_, t)| t).unwrap_or("");
    let (program, args) = command(text, &roots(settings), util::has("rg"));
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    // Exit status 1 means "nothing found": not an error.
    let out = match util::output_within(&program, &args, None, Duration::from_secs(10)) {
        Ok(o) => o,
        Err(e) if e == format!("{program} : {}", t("échec")) => String::new(),
        Err(e) => return Err(e),
    };
    let found = parse(&out);
    if found.is_empty() {
        return Ok(vec![Item { id: "content:none".into(), module: "files".into(), title: tf("Aucun fichier ne contient « {t} »", &[("t", &text)]), ..Default::default() }]);
    }
    Ok(found
        .into_iter()
        .map(|(path, line, snippet)| {
            let mtime = std::fs::metadata(&path).and_then(|m| m.modified()).ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs() as i64).unwrap_or(0);
            let mut it = crate::modules::files::item(Path::new(&path), false, mtime, "file");
            let snippet: String = snippet.chars().take(140).collect();
            it.subtitle = tf("ligne {n} · {s}", &[("n", &line), ("s", &snippet)]);
            it
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ':' cannot be in a Windows file name.
    #[cfg(unix)]
    #[test]
    fn finds_text_in_files_with_grep() {
        let dir = std::env::temp_dir().join(format!("landry-content-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sous dossier")).unwrap();
        std::fs::write(dir.join("sous dossier/devis: mars.txt"), "Bonjour\nTotal du Devis 2024 : 1200 €\nDevis encore\n").unwrap();
        std::fs::write(dir.join("autre.txt"), "rien ici").unwrap();
        let (program, args) = command("devis 2024", &[dir.clone()], false);
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let found = parse(&util::output(&program, &args, None).unwrap());
        assert_eq!(found, [(dir.join("sous dossier/devis: mars.txt"), 2, "Total du Devis 2024 : 1200 €".to_string())]);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn options_never_read_the_text_as_an_option() {
        let (_, a) = command("-rf", &[PathBuf::from("/x")], true);
        let dash = a.iter().position(|x| x == "--").unwrap();
        assert_eq!(a[dash + 1], "-rf");
    }
}
