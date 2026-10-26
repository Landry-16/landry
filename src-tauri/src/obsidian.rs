//! Obsidian bridge: notes and tasks captured into the vault's inbox, and Landry's notes mirrored
//! as Markdown files in the vault. The mirror goes both ways: a note edited in Obsidian comes back
//! to Landry at the next sync. Files are only ever written or moved to the vault's `.trash`.

use crate::config::Settings;
use crate::i18n::t;
use crate::usage::Note;
use chrono::{DateTime, Local};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

const MARK: &str = "%% Mirrored from Landry: edits made here come back to Landry. Delete the note in Landry. %%";
/// Tasks land in one file of the inbox, under this heading.
pub const TASK_FILE: &str = "Inbox.md";
const TASK_HEADING: &str = "## Tasks";

/// The configured vault, if it exists.
pub fn vault(settings: &Settings) -> Option<PathBuf> {
    let v = settings.obsidian_vault.trim();
    if v.is_empty() {
        return None;
    }
    let p = crate::paths::expand(v);
    p.is_dir().then_some(p)
}

/// `obsidian://open?path=...`: Obsidian finds the vault that holds the file (or the vault folder).
pub fn open_uri(path: &Path) -> String {
    format!("obsidian://open?path={}", utf8_percent_encode(&path.to_string_lossy(), NON_ALPHANUMERIC))
}

/// A file name that works in Obsidian and on every system: no link characters, at most 60
/// characters cut on a word.
pub fn safe_name(s: &str) -> String {
    let clean: String = s.chars().map(|c| if "\\/:*?\"<>|#^[]".contains(c) || c.is_control() { ' ' } else { c }).collect();
    let mut out = String::new();
    for w in clean.split_whitespace() {
        if !out.is_empty() && out.chars().count() + 1 + w.chars().count() > 60 {
            break;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(w);
    }
    out.chars().take(60).collect::<String>().trim_start_matches('.').trim().to_string()
}

/// `name.md` in `dir`, or `name 2.md`, `name 3.md`... when taken.
fn unique(dir: &Path, name: &str) -> PathBuf {
    let mut p = dir.join(format!("{name}.md"));
    let mut i = 2;
    while p.exists() {
        p = dir.join(format!("{name} {i}.md"));
        i += 1;
    }
    p
}

fn folder(vault: &Path, rel: &str) -> Result<PathBuf, String> {
    let dir = vault.join(rel.trim().trim_matches('/'));
    std::fs::create_dir_all(&dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    Ok(dir)
}

fn write(path: &Path, text: &str) -> Result<(), String> {
    std::fs::write(path, text).map_err(|e| format!("{}: {e}", path.display()))
}

/// A new note in the inbox, named after its first line. Returns the file.
pub fn capture_note(vault: &Path, inbox: &str, text: &str, now: DateTime<Local>) -> Result<PathBuf, String> {
    let text = text.trim();
    if text.is_empty() {
        return Err(t("la note est vide"));
    }
    let dir = folder(vault, inbox)?;
    let name = safe_name(text.lines().next().unwrap_or("").trim_start_matches('#'));
    let name = if name.is_empty() { format!("Capture {}", now.format("%Y-%m-%d %H%M")) } else { name };
    let path = unique(&dir, &name);
    write(&path, &format!("---\ncreated: {}\nsource: landry\n---\n{text}\n", now.format("%Y-%m-%d")))?;
    Ok(path)
}

/// A task line in the dataview format read by the Tasks plugin.
pub fn task_line(text: &str, due: Option<DateTime<Local>>) -> String {
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    match due {
        Some(d) => format!("- [ ] {text} [due:: {}]", d.format("%Y-%m-%d")),
        None => format!("- [ ] {text}"),
    }
}

/// Appends the task to the inbox's task file. Returns the file.
pub fn capture_task(vault: &Path, inbox: &str, text: &str, due: Option<DateTime<Local>>) -> Result<PathBuf, String> {
    if text.trim().is_empty() {
        return Err(t("la tâche est vide"));
    }
    let path = folder(vault, inbox)?.join(TASK_FILE);
    let content = std::fs::read_to_string(&path).unwrap_or_else(|_| format!("# Inbox\n\n{TASK_HEADING}\n"));
    write(&path, &insert_under(&content, TASK_HEADING, &task_line(text, due)))?;
    Ok(path)
}

/// `line` at the end of the section under `heading` (the section is added when missing).
pub fn insert_under(content: &str, heading: &str, line: &str) -> String {
    let mut lines: Vec<&str> = content.lines().collect();
    let Some(start) = lines.iter().position(|l| l.trim() == heading) else {
        return format!("{}\n\n{heading}\n{line}\n", content.trim_end());
    };
    let level = |l: &str| {
        let n = l.chars().take_while(|c| *c == '#').count();
        if n > 0 && l[n..].starts_with(' ') { n } else { 0 }
    };
    let own = level(heading);
    let end = lines[start + 1..].iter().position(|l| (1..=own).contains(&level(l))).map_or(lines.len(), |i| start + 1 + i);
    let mut at = end;
    while at > start + 1 && lines[at - 1].trim().is_empty() {
        at -= 1;
    }
    lines.insert(at, line);
    lines.join("\n") + "\n"
}

fn mirror_text(n: &Note) -> String {
    format!("---\nlandry_note: {}\nsource: landry\n---\n{MARK}\n\n{}\n", n.id, n.body.trim())
}

/// Note id and body of a mirror file; None for any other file.
fn parse_mirror(text: &str) -> Option<(i64, String)> {
    let (fm, body) = text.strip_prefix("---\n")?.split_once("\n---\n")?;
    let id = fm.lines().find_map(|l| l.strip_prefix("landry_note:"))?.trim().parse().ok()?;
    let body = body.trim_start();
    Some((id, body.strip_prefix(MARK).unwrap_or(body).trim().to_string()))
}

fn mirror_name(n: &Note) -> String {
    let name = safe_name(n.body.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim_start_matches('#'));
    if name.is_empty() { format!("Note {}", n.id) } else { name }
}

/// Mirror files in the folder, by note id.
fn mirrors(dir: &Path) -> HashMap<i64, (PathBuf, String, i64)> {
    let mut out = HashMap::new();
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let path = e.path();
        if path.extension().is_none_or(|x| x != "md") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        if let Some((id, body)) = parse_mirror(&text) {
            let mtime = e.metadata().and_then(|m| m.modified()).ok().and_then(|m| m.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs() as i64);
            out.insert(id, (path, body, mtime));
        }
    }
    out
}

/// The mirror file of a note, if it exists.
pub fn mirror_path(vault: &Path, rel: &str, id: i64) -> Option<PathBuf> {
    mirrors(&vault.join(rel.trim().trim_matches('/'))).remove(&id).map(|m| m.0)
}

/// Brings the mirror folder in line with Landry's notes: new and changed notes are written,
/// renamed notes follow their title, deleted ones go to the vault's `.trash`. Returns the notes
/// edited in Obsidian since Landry last saved them, as (id, body), for Landry to take back.
pub fn sync_notes(vault: &Path, rel: &str, notes: &[Note]) -> Result<Vec<(i64, String)>, String> {
    let dir = folder(vault, rel)?;
    let mut found = mirrors(&dir);
    let mut back = Vec::new();
    for n in notes {
        let want = dir.join(format!("{}.md", mirror_name(n)));
        match found.remove(&n.id) {
            Some((path, body, mtime)) => {
                let changed = body != n.body.trim();
                if changed && mtime > n.updated {
                    back.push((n.id, body));
                    continue;
                }
                if changed {
                    write(&path, &mirror_text(n))?;
                }
                if path != want && !want.exists() {
                    std::fs::rename(&path, &want).map_err(|e| format!("{}: {e}", path.display()))?;
                }
            }
            None => write(&if want.exists() { unique(&dir, &mirror_name(n)) } else { want }, &mirror_text(n))?,
        }
    }
    if !found.is_empty() {
        let trash = folder(vault, ".trash")?;
        for (path, _, _) in found.into_values() {
            let name = path.file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default();
            std::fs::rename(&path, unique(&trash, &name)).map_err(|e| format!("{}: {e}", path.display()))?;
        }
    }
    Ok(back)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("landry-obsidian-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    fn note(id: i64, body: &str, updated: i64) -> Note {
        Note { id, body: body.into(), updated }
    }

    #[test]
    fn names_and_lines() {
        assert_eq!(safe_name("  Idée: [[lien]] #tag / test  "), "Idée lien tag test");
        assert!(safe_name(&"mot ".repeat(40)).chars().count() <= 60);
        assert_eq!(safe_name(".cache"), "cache");
        let due = Local.with_ymd_and_hms(2026, 11, 2, 10, 0, 0).unwrap();
        assert_eq!(task_line("appeler  la  banque", Some(due)), "- [ ] appeler la banque [due:: 2026-11-02]");
        assert_eq!(task_line("lire", None), "- [ ] lire");
        assert_eq!(open_uri(Path::new("/v/00 Inbox/a.md")), "obsidian://open?path=%2Fv%2F00%20Inbox%2Fa%2Emd");
    }

    #[test]
    fn insert_keeps_sections() {
        let doc = "# Inbox\n\n## Tasks\n- [ ] a\n\n## Later\n- b\n";
        assert_eq!(insert_under(doc, "## Tasks", "- [ ] c"), "# Inbox\n\n## Tasks\n- [ ] a\n- [ ] c\n\n## Later\n- b\n");
        assert_eq!(insert_under("# Inbox\n", "## Tasks", "- [ ] c"), "# Inbox\n\n## Tasks\n- [ ] c\n");
        assert_eq!(insert_under("## Tasks\n### Sub\n- x\n", "## Tasks", "- [ ] c"), "## Tasks\n### Sub\n- x\n- [ ] c\n");
    }

    #[test]
    fn captures() {
        let v = tmp("capture");
        let now = Local.with_ymd_and_hms(2026, 10, 26, 9, 30, 0).unwrap();
        let a = capture_note(&v, "00 Inbox", "# Idée de preset\ncouleurs du soir", now).unwrap();
        assert_eq!(a, v.join("00 Inbox/Idée de preset.md"));
        assert_eq!(std::fs::read_to_string(&a).unwrap(), "---\ncreated: 2026-10-26\nsource: landry\n---\n# Idée de preset\ncouleurs du soir\n");
        assert_eq!(capture_note(&v, "00 Inbox", "Idée de preset", now).unwrap(), v.join("00 Inbox/Idée de preset 2.md"));
        assert!(capture_note(&v, "00 Inbox", "  ", now).is_err());
        capture_task(&v, "00 Inbox", "payer le loyer", Some(now)).unwrap();
        capture_task(&v, "00 Inbox", "lire", None).unwrap();
        assert_eq!(std::fs::read_to_string(v.join("00 Inbox/Inbox.md")).unwrap(), "# Inbox\n\n## Tasks\n- [ ] payer le loyer [due:: 2026-10-26]\n- [ ] lire\n");
        let _ = std::fs::remove_dir_all(v);
    }

    #[test]
    fn mirror_both_ways() {
        let v = tmp("mirror");
        let dir = v.join("Landry");
        let notes = vec![note(1, "# Courses\n- pain", 100), note(2, "Sans titre ici", 100)];
        assert!(sync_notes(&v, "Landry", &notes).unwrap().is_empty());
        let a = dir.join("Courses.md");
        assert_eq!(parse_mirror(&std::fs::read_to_string(&a).unwrap()), Some((1, "# Courses\n- pain".into())));
        assert_eq!(mirror_path(&v, "Landry", 2), Some(dir.join("Sans titre ici.md")));
        // A file of the user in the same folder is never touched.
        std::fs::write(dir.join("Mine.md"), "# mine\n").unwrap();

        // Changed and renamed in Landry (newer than the file): the mirror follows.
        let notes = vec![note(1, "# Achats\n- pain\n- lait", i64::MAX), note(2, "Sans titre ici", 100)];
        assert!(sync_notes(&v, "Landry", &notes).unwrap().is_empty());
        assert!(!a.exists());
        assert!(std::fs::read_to_string(dir.join("Achats.md")).unwrap().contains("- lait"));

        // Edited in Obsidian (file newer than the note): Landry takes the new body back.
        let b = dir.join("Sans titre ici.md");
        std::fs::write(&b, std::fs::read_to_string(&b).unwrap().replace("Sans titre ici", "Sans titre ici, complété")).unwrap();
        let notes = vec![note(1, "# Achats\n- pain\n- lait", i64::MAX), note(2, "Sans titre ici", 100)];
        assert_eq!(sync_notes(&v, "Landry", &notes).unwrap(), vec![(2, "Sans titre ici, complété".to_string())]);

        // Deleted in Landry: moved to the vault's trash, not destroyed.
        let notes = vec![note(1, "# Achats\n- pain\n- lait", i64::MAX)];
        sync_notes(&v, "Landry", &notes).unwrap();
        assert!(!b.exists());
        assert!(v.join(".trash/Sans titre ici.md").exists());
        assert!(dir.join("Mine.md").exists());
        let _ = std::fs::remove_dir_all(v);
    }
}
