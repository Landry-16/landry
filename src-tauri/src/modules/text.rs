//! Transformations of the selected text (or of the text typed after the keyword).

use crate::i18n::{t, tf};
use crate::search::{Action, Item};

const SMALL_WORDS: &[&str] = &["de", "du", "des", "la", "le", "les", "l", "d", "et", "en", "à", "au", "aux", "un", "une", "pour", "par", "sur", "ou", "of", "the", "and", "a", "an", "in", "on"];

pub fn title_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut first = true;
    for word in s.split_inclusive(char::is_whitespace) {
        let core = word.trim_end();
        let lower = core.to_lowercase();
        if !first && SMALL_WORDS.contains(&lower.as_str()) {
            out.push_str(&lower);
        } else {
            let mut c = lower.chars();
            if let Some(f) = c.next() {
                out.extend(f.to_uppercase());
                out.push_str(c.as_str());
            }
        }
        out.push_str(&word[core.len()..]);
        if !core.is_empty() {
            first = false;
        }
    }
    out
}

/// French typography: narrow no-break space before ; ! ?, no-break space before : and inside « ».
pub fn french_typography(s: &str) -> String {
    let chars: Vec<char> = s.chars().collect();
    let mut out = String::with_capacity(s.len() + 8);
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        let tight = matches!(c, ';' | '!' | '?');
        // ':' in times (15:30) and URLs (https://) stays untouched.
        let colon = c == ':' && next != Some('/') && !out.chars().last().is_some_and(|p| p.is_ascii_digit());
        if tight || colon {
            while out.ends_with([' ', '\u{a0}', '\u{202f}']) {
                out.pop();
            }
            let prev_is_text = out.chars().last().is_some_and(|p| !p.is_whitespace() && !matches!(p, ';' | '!' | '?' | ':'));
            if prev_is_text {
                out.push(if tight { '\u{202f}' } else { '\u{a0}' });
            }
            out.push(c);
        } else if c == '«' {
            out.push('«');
            out.push('\u{a0}');
            while chars.get(i + 1).is_some_and(|n| n.is_whitespace()) {
                i += 1;
            }
        } else if c == '»' {
            while out.ends_with([' ', '\u{a0}']) {
                out.pop();
            }
            out.push('\u{a0}');
            out.push('»');
        } else {
            out.push(c);
        }
        i += 1;
    }
    out
}

pub fn stats(s: &str) -> String {
    let words = s.split_whitespace().count();
    let chars = s.chars().count();
    let lines = if s.is_empty() { 0 } else { s.lines().count() };
    if crate::i18n::english() {
        format!("{words} word{}, {chars} character{}, {lines} line{}", plural(words), plural(chars), plural(lines))
    } else {
        format!("{words} mot{}, {chars} caractère{}, {lines} ligne{}", plural(words), plural(chars), plural(lines))
    }
}

fn plural(n: usize) -> &'static str {
    if n > 1 { "s" } else { "" }
}

fn preview(s: &str) -> String {
    let flat: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > 60 { format!("{}…", flat.chars().take(59).collect::<String>()) } else { flat }
}

/// `selection`: the text selected in another app when the bar opened, if any.
pub fn query(q: &str, selection: Option<&str>) -> Vec<Item> {
    let (word, rest) = q.split_once(' ').map(|(a, b)| (a, b)).unwrap_or((q, ""));
    let (source, from_selection) = match (rest.trim().is_empty(), selection) {
        (false, _) => (rest.to_string(), false),
        (true, Some(sel)) if !sel.is_empty() => (sel.to_string(), true),
        _ => return vec![],
    };
    let (kind, label, out) = match crate::search::fold(word).as_str() {
        "maj" | "majuscules" | "upper" => ("upper", t("En majuscules"), source.to_uppercase()),
        "min" | "minuscules" | "lower" => ("lower", t("En minuscules"), source.to_lowercase()),
        "titre" | "title" => ("title", t("Casse de titre"), title_case(&source)),
        "brut" | "plain" => ("plain", t("Sans mise en forme"), source.clone()),
        "typo" | "insecables" => ("typo", t("Typographie française"), french_typography(&source)),
        "mots" | "compter" | "count" => {
            return vec![Item {
                id: "text:count".into(),
                module: "text".into(),
                title: stats(&source),
                subtitle: tf("{what} : « {text} »", &[("what", &if from_selection { t("Sélection") } else { t("Texte") }), ("text", &preview(&source))]),
                actions: vec![Action::new("copy", "Copier")],
                data: stats(&source),
                ..Default::default()
            }];
        }
        _ => return vec![],
    };
    let mut actions = Vec::new();
    if from_selection {
        actions.push(Action::new("paste", "Remplacer la sélection"));
    }
    actions.push(Action::new("copy", "Copier"));
    vec![Item {
        id: format!("text:{kind}"),
        module: "text".into(),
        title: preview(&out),
        subtitle: if from_selection { tf("{label} · appliqué à la sélection", &[("label", &label)]) } else { label },
        actions,
        data: out,
        ..Default::default()
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn titles() {
        assert_eq!(title_case("le seigneur des anneaux"), "Le Seigneur des Anneaux");
        assert_eq!(title_case("HELLO  wORLD"), "Hello  World");
    }

    #[test]
    fn typography() {
        assert_eq!(french_typography("Vraiment ? Oui ! Note : ceci ; cela"), "Vraiment\u{202f}? Oui\u{202f}! Note\u{a0}: ceci\u{202f}; cela");
        assert_eq!(french_typography("Quoi?!"), "Quoi\u{202f}?!");
        assert_eq!(french_typography("à 15:30 sur https://x.fr"), "à 15:30 sur https://x.fr");
        assert_eq!(french_typography("« bonjour »"), "«\u{a0}bonjour\u{a0}»");
    }

    #[test]
    fn counting() {
        assert_eq!(stats("un deux\ntrois"), "3 mots, 13 caractères, 2 lignes");
        assert_eq!(stats("x"), "1 mot, 1 caractère, 1 ligne");
    }

    #[test]
    fn sources() {
        assert_eq!(query("maj salut", None)[0].data, "SALUT");
        let r = query("titre", Some("la belle vie"));
        assert_eq!(r[0].data, "La Belle Vie");
        assert_eq!(r[0].actions[0].id, "paste");
        assert!(query("maj", None).is_empty());
        assert!(query("majestic", Some("x")).is_empty());
    }
}
