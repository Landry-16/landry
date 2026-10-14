//! Git repositories under the projects root: branch, state, editor, terminal, npm scripts.

use super::Preview;
use crate::config::Settings;
use crate::engine::Outcome;
use crate::i18n::{t, tf};
use crate::search::{Action, Item};
use crate::util;
use std::path::{Path, PathBuf};

const MAX_DEPTH: usize = 4;

fn find_repos(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if dir.join(".git").exists() {
        out.push(dir.to_path_buf());
        return;
    }
    if depth >= MAX_DEPTH {
        return;
    }
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let name = e.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') || name == "node_modules" || name == "target" {
            continue;
        }
        if e.file_type().is_ok_and(|t| t.is_dir()) {
            find_repos(&e.path(), depth + 1, out);
        }
    }
}

/// The git dir, also for worktrees and submodules where `.git` is a file.
fn git_dir(repo: &Path) -> PathBuf {
    let dot = repo.join(".git");
    if let Ok(s) = std::fs::read_to_string(&dot) {
        if let Some(p) = s.trim().strip_prefix("gitdir:") {
            let p = PathBuf::from(p.trim());
            return if p.is_absolute() { p } else { repo.join(p) };
        }
    }
    dot
}

pub fn branch(repo: &Path) -> String {
    let head = std::fs::read_to_string(git_dir(repo).join("HEAD")).unwrap_or_default();
    match head.trim().strip_prefix("ref: refs/heads/") {
        Some(b) => b.to_string(),
        None => head.trim().chars().take(7).collect(),
    }
}

/// URL of `origin` read from .git/config.
pub fn origin(repo: &Path) -> Option<String> {
    let cfg = std::fs::read_to_string(git_dir(repo).join("config")).ok()?;
    let mut in_origin = false;
    for line in cfg.lines().map(str::trim) {
        if line.starts_with('[') {
            in_origin = line == r#"[remote "origin"]"#;
        } else if in_origin {
            if let Some(v) = line.strip_prefix("url").map(str::trim).and_then(|v| v.strip_prefix('=')) {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

/// git@github.com:me/repo.git -> https://github.com/me/repo
pub fn web_url(remote: &str) -> Option<String> {
    let r = remote.trim().trim_end_matches('/').trim_end_matches(".git");
    if let Some(rest) = r.strip_prefix("git@") {
        let (host, path) = rest.split_once(':')?;
        return Some(format!("https://{host}/{path}"));
    }
    if let Some(rest) = r.strip_prefix("ssh://") {
        let rest = rest.split_once('@').map_or(rest, |(_, h)| h);
        let (host, path) = rest.split_once('/')?;
        let host = host.split(':').next()?;
        return Some(format!("https://{host}/{path}"));
    }
    if r.starts_with("https://") || r.starts_with("http://") {
        // Drop credentials from https://user:token@host/...
        return Some(match url::Url::parse(r) {
            Ok(mut u) => {
                let _ = u.set_username("");
                let _ = u.set_password(None);
                u.to_string()
            }
            Err(_) => r.to_string(),
        });
    }
    None
}

#[derive(Debug, Default, PartialEq)]
pub struct Status {
    pub changed: usize,
    pub ahead: u32,
    pub behind: u32,
}

/// Parse `git status --porcelain=v2 --branch`.
pub fn parse_status(out: &str) -> Status {
    let mut s = Status::default();
    for l in out.lines() {
        if let Some(ab) = l.strip_prefix("# branch.ab ") {
            let mut it = ab.split_whitespace();
            s.ahead = it.next().and_then(|v| v.trim_start_matches('+').parse().ok()).unwrap_or(0);
            s.behind = it.next().and_then(|v| v.trim_start_matches('-').parse().ok()).unwrap_or(0);
        } else if !l.starts_with('#') && !l.is_empty() {
            s.changed += 1;
        }
    }
    s
}

fn status(repo: &Path) -> Option<Status> {
    util::output("git", &["status", "--porcelain=v2", "--branch"], Some(repo)).ok().map(|o| parse_status(&o))
}

fn describe(st: &Status) -> String {
    let mut parts = Vec::new();
    if st.changed > 0 {
        parts.push(if st.changed > 1 { tf("{n} modifiés", &[("n", &st.changed)]) } else { t("1 modifié") });
    }
    if st.behind > 0 {
        parts.push(tf("{n} en retard", &[("n", &st.behind)]));
    }
    if st.ahead > 0 {
        parts.push(tf("{n} en avance", &[("n", &st.ahead)]));
    }
    if parts.is_empty() { t("propre") } else { parts.join(", ") }
}

pub fn npm_scripts(repo: &Path) -> Vec<String> {
    let Ok(s) = std::fs::read_to_string(repo.join("package.json")) else { return vec![] };
    serde_json::from_str::<serde_json::Value>(&s)
        .ok()
        .and_then(|v| v.get("scripts")?.as_object().map(|o| o.keys().cloned().collect()))
        .unwrap_or_default()
}

fn item(repo: &Path, with_status: bool) -> Item {
    let name = repo.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let br = branch(repo);
    let state = if with_status { status(repo).map(|s| format!(" · {}", describe(&s))).unwrap_or_default() } else { String::new() };
    let mut actions = vec![
        Action::new("editor", "Ouvrir dans l'éditeur"),
        Action::new("terminal", "Ouvrir un terminal"),
        Action::new("pull", "git pull"),
    ];
    if origin(repo).as_deref().and_then(web_url).is_some() {
        actions.push(Action::new("web", "Ouvrir le dépôt en ligne"));
        actions.push(Action::new("pr", "Ouvrir une pull request"));
    }
    actions.push(Action::new("reveal", "Ouvrir le dossier"));
    actions.push(Action::new("copy-path", "Copier le chemin"));
    for s in npm_scripts(repo) {
        actions.push(Action::new(&format!("npm:{s}"), &format!("npm run {s}")));
    }
    let rel = repo.strip_prefix(crate::paths::expand("~")).map(|r| format!("~/{}", r.display())).unwrap_or_default();
    Item {
        id: format!("proj:{}", repo.display()),
        module: "projects".into(),
        title: name,
        subtitle: format!("{br}{state} · {rel}"),
        keywords: vec![rel, "projet".into(), "git".into()],
        actions,
        data: repo.display().to_string(),
        ..Default::default()
    }
}

/// Fast pass (no git process) then the caller can run `scan(.., true)` for states.
pub fn scan(settings: &Settings, with_status: bool) -> Vec<Item> {
    let mut repos = Vec::new();
    find_repos(&crate::paths::expand(&settings.projects_root), 0, &mut repos);
    repos.sort();
    repos.iter().map(|r| item(r, with_status)).collect()
}

pub fn run(settings: &Settings, item: &Item, action: &str, _arg: &str) -> Result<Outcome, String> {
    let repo = Path::new(&item.data);
    let dir = item.data.as_str();
    Ok(match action {
        "editor" => {
            util::run_template(&settings.editor, &[("path", dir)], Some(repo))?;
            Outcome::Hide
        }
        "terminal" => {
            util::run_template(&settings.terminal, &[("dir", dir)], Some(repo))?;
            Outcome::Hide
        }
        "pull" => {
            let out = util::output("git", &["pull", "--ff-only"], Some(repo))?;
            Outcome::Message(format!("git pull : {}", out.lines().last().map(str::to_string).unwrap_or_else(|| t("à jour")).trim()))
        }
        "web" => {
            let url = origin(repo).as_deref().and_then(web_url).ok_or_else(|| t("pas de dépôt distant origin"))?;
            util::open(&url)?;
            Outcome::Hide
        }
        "pr" => {
            let url = origin(repo).as_deref().and_then(web_url).ok_or_else(|| t("pas de dépôt distant origin"))?;
            util::open(&format!("{url}/compare/{}?expand=1", branch(repo)))?;
            Outcome::Hide
        }
        "reveal" => {
            util::open(dir)?;
            Outcome::Hide
        }
        "copy-path" => Outcome::Copy(dir.to_string()),
        a => {
            let script = a.strip_prefix("npm:").ok_or_else(|| tf("action inconnue : {a}", &[("a", &a)]))?;
            let cmd = format!("npm run {script}; exec bash");
            util::run_template(&settings.terminal_run, &[("dir", dir), ("cmd", &cmd)], Some(repo))?;
            Outcome::Hide
        }
    })
}

pub fn preview(item: &Item) -> Preview {
    let repo = Path::new(&item.data);
    let mut rows = vec![(t("Branche"), branch(repo))];
    if let Some(st) = status(repo) {
        rows.push((t("État"), describe(&st)));
    }
    if let Some(u) = origin(repo).as_deref().and_then(web_url) {
        rows.push((t("Dépôt"), u));
    }
    let scripts = npm_scripts(repo);
    if !scripts.is_empty() {
        rows.push((t("Scripts npm"), scripts.join(", ")));
    }
    if let Ok(log) = util::output("git", &["log", "-1", "--format=%s (%cr)"], Some(repo)) {
        rows.push((t("Dernier commit"), log.trim().to_string()));
    }
    Preview::Props { rows }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remote_urls() {
        assert_eq!(web_url("git@github.com:Landry-16/landry.git").unwrap(), "https://github.com/Landry-16/landry");
        assert_eq!(web_url("https://github.com/a/b.git").unwrap(), "https://github.com/a/b");
        assert_eq!(web_url("https://me:secret@gitlab.com/a/b.git").unwrap(), "https://gitlab.com/a/b");
        assert_eq!(web_url("ssh://git@host.fr:2222/a/b.git").unwrap(), "https://host.fr/a/b");
        assert!(web_url("/local/path").is_none());
    }

    #[test]
    fn status_parsing() {
        let out = "# branch.oid abc\n# branch.head main\n# branch.upstream origin/main\n# branch.ab +1 -3\n1 .M N... 100644 100644 100644 a b src/x.rs\n? new.txt\n";
        assert_eq!(parse_status(out), Status { changed: 2, ahead: 1, behind: 3 });
        assert_eq!(describe(&parse_status(out)), "2 modifiés, 3 en retard, 1 en avance");
        assert_eq!(describe(&Status::default()), "propre");
    }

    #[test]
    fn finds_this_repo() {
        // The Landry repo itself: its .git and package.json sit two levels above this crate.
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let mut repos = Vec::new();
        find_repos(root.parent().unwrap(), 0, &mut repos);
        assert!(repos.iter().any(|r| r == root), "{repos:?}");
        assert!(npm_scripts(root).contains(&"build".to_string()));
        assert!(!branch(root).is_empty());
        assert_eq!(origin(root).as_deref().and_then(web_url).unwrap(), "https://github.com/Landry-16/landry");
    }
}
