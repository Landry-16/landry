//! Fuzzy ranking shared by every module: nucleo score, initials bonus, frecency boost.

use nucleo::pattern::{CaseMatching, Normalization, Pattern};
use nucleo::{Config, Matcher, Utf32Str};
use serde::Serialize;
use std::collections::HashMap;

/// A secondary action shown with Ctrl+K. The first action of an item is its Enter action.
#[derive(Serialize, Clone, Debug, PartialEq, Default)]
pub struct Action {
    pub id: String,
    pub title: String,
    /// When set, the bar asks for a text argument with this placeholder before running.
    pub arg: Option<String>,
    /// Prefilled argument (e.g. the current name for a rename).
    pub value: String,
    pub danger: bool,
}

impl Action {
    pub fn new(id: &str, title: &str) -> Action {
        Action { id: id.into(), title: title.into(), ..Default::default() }
    }
    pub fn ask(mut self, placeholder: &str, value: &str) -> Action {
        self.arg = Some(placeholder.into());
        self.value = value.into();
        self
    }
    pub fn danger(mut self) -> Action {
        self.danger = true;
        self
    }
}

#[derive(Serialize, Clone, Debug, PartialEq, Default)]
pub struct Item {
    /// Stable id, prefixed by module: "app:code.desktop", "cmd:settings".
    pub id: String,
    pub module: String,
    pub title: String,
    pub subtitle: String,
    /// Data URL, or empty.
    pub icon: String,
    /// Extra searchable text (keywords, program name), weighted lower than the title.
    #[serde(skip)]
    pub keywords: Vec<String>,
    pub actions: Vec<Action>,
    /// Module payload (path, value to copy...), never sent to the UI.
    #[serde(skip)]
    pub data: String,
}

/// Lowercase and strip French and common Latin accents.
pub fn fold(s: &str) -> String {
    s.chars()
        .flat_map(|c| c.to_lowercase())
        .map(|c| match c {
            'à' | 'â' | 'ä' | 'á' | 'ã' | 'å' => 'a',
            'ç' => 'c',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'î' | 'ï' | 'í' | 'ì' => 'i',
            'ô' | 'ö' | 'ó' | 'ò' | 'õ' => 'o',
            'ù' | 'û' | 'ü' | 'ú' => 'u',
            'ÿ' => 'y',
            'ñ' => 'n',
            c => c,
        })
        .collect()
}

/// "Visual Studio Code" -> "vsc".
pub fn initials(s: &str) -> String {
    fold(s)
        .split(|c: char| c.is_whitespace() || c == '-' || c == '_' || c == '.')
        .filter_map(|w| w.chars().next())
        .collect()
}

/// Bonus from usage: grows slowly so a good text match still wins over a frequent bad one.
fn usage_bonus(frecency: f64) -> u32 {
    ((frecency.max(0.0) + 1.0).ln() * 25.0).min(120.0) as u32
}

pub struct Ranker {
    matcher: Matcher,
    buf: Vec<char>,
}

impl Default for Ranker {
    fn default() -> Self {
        Ranker { matcher: Matcher::new(Config::DEFAULT), buf: Vec::new() }
    }
}

impl Ranker {
    fn score(&mut self, pattern: &Pattern, haystack: &str) -> Option<u32> {
        pattern.score(Utf32Str::new(&fold(haystack), &mut self.buf), &mut self.matcher)
    }

    /// Indexes of `items` matching `query`, best first, at most `limit`.
    /// Empty query: most used items first.
    pub fn rank(
        &mut self,
        items: &[Item],
        query: &str,
        frecency: &HashMap<String, f64>,
        limit: usize,
    ) -> Vec<usize> {
        let q = fold(query.trim());
        let mut scored: Vec<(usize, u32)> = if q.is_empty() {
            items
                .iter()
                .enumerate()
                .filter_map(|(i, it)| frecency.get(&it.id).filter(|f| **f > 0.0).map(|f| (i, usage_bonus(*f) + 1)))
                .collect()
        } else {
            let pattern = Pattern::parse(&q, CaseMatching::Ignore, Normalization::Smart);
            items
                .iter()
                .enumerate()
                .filter_map(|(i, it)| {
                    let title = self.score(&pattern, &it.title);
                    let extra = it
                        .keywords
                        .iter()
                        .filter_map(|k| self.score(&pattern, k))
                        .max()
                        .map(|s| s * 6 / 10);
                    let mut best = title.max(extra)?;
                    if q.chars().count() >= 2 && initials(&it.title).starts_with(&q) {
                        best += 200;
                    }
                    if fold(&it.title).starts_with(&q) {
                        best += 100;
                    }
                    Some((i, best + usage_bonus(frecency.get(&it.id).copied().unwrap_or(0.0))))
                })
                .collect()
        };
        // Ties keep the items' own order (stable sort).
        scored.sort_by(|a, b| b.1.cmp(&a.1));
        // Drop scattered subsequence matches far below the best one ("vsc" in a long file name).
        if let Some(&(_, best)) = scored.first() {
            if !q.is_empty() {
                scored.retain(|&(_, s)| s * 3 >= best);
            }
        }
        scored.into_iter().take(limit).map(|(i, _)| i).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(id: &str, title: &str, kw: &[&str]) -> Item {
        Item {
            id: id.into(),
            module: "apps".into(),
            title: title.into(),
            keywords: kw.iter().map(|s| s.to_string()).collect(),
            ..Default::default()
        }
    }

    fn titles(items: &[Item], q: &str, f: &HashMap<String, f64>) -> Vec<String> {
        Ranker::default().rank(items, q, f, 10).into_iter().map(|i| items[i].title.clone()).collect()
    }

    fn sample() -> Vec<Item> {
        vec![
            item("a", "Visual Studio Code", &["code", "vscode"]),
            item("b", "Paramètres", &["gnome-control-center", "settings"]),
            item("c", "Vidéos", &["totem"]),
            item("d", "Terminal", &["gnome-terminal", "shell"]),
            item("e", "Calculatrice", &["gnome-calculator"]),
        ]
    }

    #[test]
    fn initials_find_vscode() {
        assert_eq!(titles(&sample(), "vsc", &HashMap::new())[0], "Visual Studio Code");
    }

    #[test]
    fn accents_are_ignored_both_ways() {
        assert_eq!(titles(&sample(), "parametres", &HashMap::new())[0], "Paramètres");
        assert_eq!(titles(&sample(), "vidéo", &HashMap::new())[0], "Vidéos");
    }

    #[test]
    fn keywords_match() {
        assert_eq!(titles(&sample(), "shell", &HashMap::new())[0], "Terminal");
        assert_eq!(titles(&sample(), "settings", &HashMap::new())[0], "Paramètres");
    }

    #[test]
    fn typo_tolerant_subsequence() {
        assert_eq!(titles(&sample(), "calcl", &HashMap::new())[0], "Calculatrice");
        assert!(titles(&sample(), "zzz", &HashMap::new()).is_empty());
    }

    #[test]
    fn usage_breaks_ties_and_drives_empty_query() {
        let items = vec![item("x", "Firefox", &[]), item("y", "Files", &[])];
        let mut f = HashMap::new();
        f.insert("y".to_string(), 8.0);
        assert_eq!(titles(&items, "fi", &f)[0], "Files");
        assert_eq!(titles(&items, "", &f), vec!["Files"]);
        assert!(titles(&items, "", &HashMap::new()).is_empty());
    }

    #[test]
    fn weak_scattered_matches_are_dropped() {
        let items = vec![
            item("a", "Visual Studio Code", &[]),
            item("b", "Convention-stage-Tech2-2-mois-Sandes-SAVARIMUTHU.pdf", &[]),
        ];
        assert_eq!(titles(&items, "vsc", &HashMap::new()), vec!["Visual Studio Code"]);
    }

    #[test]
    fn usage_does_not_beat_a_much_better_match() {
        let items = vec![item("t", "Terminal", &[]), item("o", "Text Editor Formatter", &[])];
        let mut f = HashMap::new();
        f.insert("o".to_string(), 50.0);
        assert_eq!(titles(&items, "term", &f)[0], "Terminal");
    }
}
