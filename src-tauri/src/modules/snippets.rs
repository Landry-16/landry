//! Snippets with variables: {date}, {heure}, {date:%A %d %B}, {presse-papiers}, {curseur},
//! {champ:Nom} (asked when inserting).

use crate::search::{Action, Item};
use crate::usage::Snippet;
use chrono::{DateTime, Local};
use std::collections::HashMap;
use std::fmt::Write as _;

const CURSOR: &str = "{curseur}";

fn field_re() -> regex::Regex {
    regex::Regex::new(r"\{champ:([^}]+)\}").expect("regex valide")
}

/// Names of the fields to ask, in order of appearance, without duplicates.
pub fn fields(body: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for c in field_re().captures_iter(body) {
        let name = c[1].trim().to_string();
        if !out.contains(&name) {
            out.push(name);
        }
    }
    out
}

/// Expanded text, and how many characters sit after the cursor marker (Left presses needed).
pub fn expand(body: &str, now: DateTime<Local>, clipboard: &str, values: &HashMap<String, String>) -> (String, usize) {
    let fmt_re = regex::Regex::new(r"\{date:([^}]+)\}").expect("regex valide");
    let mut text = fmt_re
        .replace_all(body, |c: &regex::Captures| {
            let mut out = String::new();
            // An invalid chrono format must not panic: keep the placeholder instead.
            match write!(out, "{}", now.format(&c[1])) {
                Ok(()) => out,
                Err(_) => c[0].to_string(),
            }
        })
        .into_owned();
    text = text
        .replace("{date}", &now.format("%d/%m/%Y").to_string())
        .replace("{heure}", &now.format("%H:%M").to_string())
        .replace("{presse-papiers}", clipboard);
    text = field_re()
        .replace_all(&text, |c: &regex::Captures| values.get(c[1].trim()).cloned().unwrap_or_default())
        .into_owned();
    match text.find(CURSOR) {
        Some(i) => {
            let after = text[i + CURSOR.len()..].chars().count();
            (text.replacen(CURSOR, "", 1).replace(CURSOR, ""), after)
        }
        None => (text, 0),
    }
}

pub fn items(snippets: &[Snippet]) -> Vec<Item> {
    snippets
        .iter()
        .map(|s| {
            let first = s.body.lines().find(|l| !l.trim().is_empty()).unwrap_or("").trim();
            let mut keywords = vec!["extrait".to_string()];
            if !s.keyword.is_empty() {
                keywords.push(s.keyword.clone());
            }
            Item {
                id: format!("snip:{}", s.id),
                module: "snippets".into(),
                title: s.name.clone(),
                subtitle: if s.keyword.is_empty() { first.to_string() } else { format!("{} · {first}", s.keyword) },
                keywords,
                actions: vec![
                    Action::new("paste", "Coller"),
                    Action::new("copy", "Copier"),
                    Action::new("edit", "Modifier l'extrait"),
                ],
                data: s.body.clone(),
                ..Default::default()
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn t() -> DateTime<Local> {
        Local.with_ymd_and_hms(2026, 10, 13, 9, 5, 0).unwrap()
    }

    #[test]
    fn variables_and_fields() {
        let body = "Bonjour {champ:Nom},\nle {date} à {heure}.\n{champ:Nom} : {presse-papiers}";
        assert_eq!(fields(body), vec!["Nom"]);
        let mut v = HashMap::new();
        v.insert("Nom".to_string(), "Camille".to_string());
        let (text, back) = expand(body, t(), "lien", &v);
        assert_eq!(text, "Bonjour Camille,\nle 13/10/2026 à 09:05.\nCamille : lien");
        assert_eq!(back, 0);
    }

    #[test]
    fn cursor_and_custom_format() {
        let (text, back) = expand("<b>{curseur}</b> {date:%Y}", t(), "", &HashMap::new());
        assert_eq!(text, "<b></b> 2026");
        assert_eq!(back, 9);
        // Invalid format: no panic, placeholder kept.
        let (text, _) = expand("{date:%Q}", t(), "", &HashMap::new());
        assert_eq!(text, "{date:%Q}");
    }
}
