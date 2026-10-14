//! Dictionary and synonyms: `/def maison`, `/syn rapide`, `/def en house`.
//! French from Wiktionnaire, English from Wiktionary (definitions) and Datamuse (synonyms).
//! Fetched in the background only when asked, through the cache of the cloud module.

use crate::config::Settings;
use crate::engine::Outcome;
use crate::i18n::{english, t, tf};
use crate::search::{Action, Item};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

/// "en house" -> ("en", "house"); "maison" -> (interface language, "maison").
pub fn language_and_word(arg: &str) -> (&'static str, String) {
    let default = if english() { "en" } else { "fr" };
    match arg.split_once(' ') {
        Some(("fr", w)) if !w.trim().is_empty() => ("fr", w.trim().to_string()),
        Some(("en", w)) if !w.trim().is_empty() => ("en", w.trim().to_string()),
        _ => (default, arg.trim().to_string()),
    }
}

fn page(lang: &str, word: &str) -> String {
    let host = if lang == "fr" { "fr.wiktionary.org" } else { "en.wiktionary.org" };
    format!("https://{host}/wiki/{}", utf8_percent_encode(word, NON_ALPHANUMERIC))
}

fn item(id: String, title: String, subtitle: String, actions: Vec<Action>, data: String) -> Item {
    Item { id, module: "dict".into(), title, subtitle, actions, data, ..Default::default() }
}

fn hint(id: &str, title: &str, subtitle: String) -> Item {
    item(format!("dict:{id}"), title.into(), subtitle, vec![], String::new())
}

pub fn definitions(settings: &Settings, arg: &str) -> Vec<Item> {
    let (lang, word) = language_and_word(arg);
    if word.is_empty() {
        return vec![hint("def", "/def <mot>", t("Définitions du Wiktionnaire ; /def en <word> pour l'anglais"))];
    }
    crate::modules::cloud::cached(&format!("def\t{lang}\t{word}"), settings, fetch_definitions)
}

pub fn synonyms(settings: &Settings, arg: &str) -> Vec<Item> {
    let (lang, word) = language_and_word(arg);
    if word.is_empty() {
        return vec![hint("syn", "/syn <mot>", t("Synonymes ; /syn en <word> pour l'anglais"))];
    }
    crate::modules::cloud::cached(&format!("syn\t{lang}\t{word}"), settings, fetch_synonyms)
}

fn get(url: &str) -> Result<String, String> {
    crate::modules::cloud::client()?.get(url).send().and_then(|r| r.error_for_status()).and_then(|r| r.text()).map_err(|e| e.to_string())
}

/// The French section of a Wiktionnaire page (other languages share the page).
fn french_section(wikitext: &str) -> &str {
    let start = wikitext.find("== {{langue|fr}} ==").map(|i| i + 19).unwrap_or(0);
    let rest = &wikitext[start..];
    rest.find("\n== ").map(|i| &rest[..i]).unwrap_or(rest)
}

/// Wikitext of a definition line -> plain text: links keep their label, `{{lien|x|fr}}` gives x,
/// a domain label such as `{{édifices|fr}}` gives "(Édifices)", other templates disappear.
pub fn clean_wikitext(line: &str) -> String {
    let template = regex::Regex::new(r"\{\{([^{}]*)\}\}").expect("regex valide");
    let link = regex::Regex::new(r"\[\[(?:[^\]|]*\|)?([^\]|]*)\]\]").expect("regex valide");
    let tags = regex::Regex::new(r"(?s)<ref[^>]*>.*?</ref>|<ref[^>]*/>|<[^>]+>").expect("regex valide");
    let mut s = tags.replace_all(line, "").into_owned();
    // Innermost templates first, until none is left.
    while template.is_match(&s) {
        s = template
            .replace_all(&s, |c: &regex::Captures| {
                let params: Vec<&str> = c[1].split('|').map(str::trim).filter(|p| !p.contains('=')).collect();
                match params.as_slice() {
                    [name, word, ..] if matches!(*name, "lien" | "l" | "w" | "ws" | "term") => word.to_string(),
                    ["lexique", domain, ..] => format!("({})", capitalize(domain)),
                    [label, "fr"] => format!("({})", capitalize(label)),
                    _ => String::new(),
                }
            })
            .into_owned();
    }
    let s = link.replace_all(&s, "$1").replace("'''", "").replace("''", "");
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    c.next().map(|f| f.to_uppercase().chain(c).collect()).unwrap_or_default()
}

/// French Wiktionnaire wikitext -> (part of speech, definition), at most 3 per part of speech.
/// Definitions are the `#` lines; `#*` (examples) and `#:` (notes) are skipped.
pub fn parse_french_definitions(wikitext: &str) -> Vec<(String, String)> {
    const NOT_POS: &[&str] = &[
        "étymologie", "prononciation", "voir aussi", "références", "traductions", "anagrammes", "homophones", "paronymes", "synonymes",
        "antonymes", "dérivés", "apparentés", "vocabulaire", "hyperonymes", "hyponymes", "méronymes", "holonymes", "variantes", "notes",
        "proverbes", "quasi-synonymes", "compos", "abréviations", "attestations", "gentilés", "variantes orthographiques", "faux-amis",
    ];
    let heading = regex::Regex::new(r"^===\s*\{\{S\|([^|}]+)").expect("regex valide");
    let mut out = Vec::new();
    let mut pos: Option<String> = None;
    let mut count = 0;
    for line in french_section(wikitext).lines() {
        if line.starts_with("==") {
            pos = heading.captures(line).map(|c| c[1].trim().to_string()).filter(|p| !NOT_POS.contains(&p.as_str())).map(|p| capitalize(&p));
            count = 0;
            continue;
        }
        let Some(p) = &pos else { continue };
        if !line.starts_with('#') || line[1..].starts_with(['*', ':']) || count >= 3 {
            continue;
        }
        let text = clean_wikitext(line.trim_start_matches('#'));
        if !text.is_empty() {
            out.push((p.clone(), text));
            count += 1;
        }
    }
    out
}

/// French Wiktionnaire wikitext -> synonyms listed under {{S|synonymes}}.
pub fn parse_french_synonyms(wikitext: &str) -> Vec<String> {
    let link = regex::Regex::new(r"\[\[([^\]|#]+)(?:\|[^\]]*)?\]\]").expect("regex valide");
    let mut out: Vec<String> = Vec::new();
    for section in french_section(wikitext).split("{{S|synonymes").skip(1) {
        let body = section.split("\n=").next().unwrap_or("");
        for line in body.lines().filter(|l| l.starts_with('*')) {
            for c in link.captures_iter(line) {
                let w = c[1].trim().to_string();
                if !out.contains(&w) {
                    out.push(w);
                }
            }
        }
    }
    out
}

fn strip_html(s: &str) -> String {
    // <style> blocks carry CSS that would leak into the text.
    let tag = regex::Regex::new(r"(?s)<style[^>]*>.*?</style>|<[^>]*>").expect("regex valide");
    tag.replace_all(s, "").replace("&nbsp;", " ").replace("&amp;", "&").replace("&quot;", "\"").replace("&#39;", "'").trim().to_string()
}

fn fetch_definitions(_: &Settings, key: &str) -> Result<Vec<Item>, String> {
    let mut parts = key.split('\t').skip(1);
    let (lang, word) = (parts.next().unwrap_or("fr"), parts.next().unwrap_or(""));
    let enc = utf8_percent_encode(word, NON_ALPHANUMERIC).to_string();
    let defs: Vec<(String, String)> = if lang == "fr" {
        let json: serde_json::Value =
            serde_json::from_str(&get(&format!("https://fr.wiktionary.org/w/api.php?action=parse&prop=wikitext&redirects=1&format=json&formatversion=2&page={enc}"))?)
                .map_err(|e| e.to_string())?;
        parse_french_definitions(json["parse"]["wikitext"].as_str().unwrap_or(""))
    } else {
        let json: serde_json::Value = serde_json::from_str(&get(&format!("https://en.wiktionary.org/api/rest_v1/page/definition/{enc}"))?).map_err(|e| e.to_string())?;
        json["en"]
            .as_array()
            .into_iter()
            .flatten()
            .flat_map(|e| {
                let pos = e["partOfSpeech"].as_str().unwrap_or("").to_string();
                e["definitions"].as_array().cloned().unwrap_or_default().into_iter().take(3).map(move |d| (pos.clone(), strip_html(d["definition"].as_str().unwrap_or(""))))
            })
            .filter(|(_, d)| !d.is_empty())
            .collect()
    };
    if defs.is_empty() {
        return Ok(vec![item(
            "dict:none".into(),
            tf("Pas de définition pour « {w} »", &[("w", &word)]),
            t("Vérifie l'orthographe, ou ouvre le Wiktionnaire"),
            vec![Action::new("open", "Ouvrir le Wiktionnaire")],
            page(lang, word),
        )]);
    }
    Ok(defs
        .into_iter()
        .take(8)
        .enumerate()
        .map(|(i, (pos, def))| {
            item(
                format!("dict:def:{i}"),
                def.clone(),
                format!("{word} · {pos}"),
                vec![Action::new("open", "Ouvrir le Wiktionnaire"), Action::new("copy", "Copier la définition")],
                format!("{}\t{def}", page(lang, word)),
            )
        })
        .collect())
}

fn fetch_synonyms(_: &Settings, key: &str) -> Result<Vec<Item>, String> {
    let mut parts = key.split('\t').skip(1);
    let (lang, word) = (parts.next().unwrap_or("fr"), parts.next().unwrap_or(""));
    let enc = utf8_percent_encode(word, NON_ALPHANUMERIC).to_string();
    let words: Vec<String> = if lang == "fr" {
        let json: serde_json::Value =
            serde_json::from_str(&get(&format!("https://fr.wiktionary.org/w/api.php?action=parse&prop=wikitext&redirects=1&format=json&formatversion=2&page={enc}"))?)
                .map_err(|e| e.to_string())?;
        parse_french_synonyms(json["parse"]["wikitext"].as_str().unwrap_or(""))
    } else {
        let json: serde_json::Value = serde_json::from_str(&get(&format!("https://api.datamuse.com/words?rel_syn={enc}&max=15"))?).map_err(|e| e.to_string())?;
        json.as_array().into_iter().flatten().filter_map(|w| w["word"].as_str().map(String::from)).collect()
    };
    if words.is_empty() {
        return Ok(vec![item(
            "dict:nosyn".into(),
            tf("Pas de synonyme pour « {w} »", &[("w", &word)]),
            t("Essaie /def pour la définition"),
            vec![Action::new("open", "Ouvrir le Wiktionnaire")],
            page(lang, word),
        )]);
    }
    let def = if lang == "fr" { "/def" } else { "/def en" };
    Ok(words
        .into_iter()
        .take(15)
        .map(|w| {
            item(
                format!("dict:syn:{w}"),
                w.clone(),
                tf("Synonyme de « {w} »", &[("w", &word)]),
                vec![Action::new("paste", "Coller"), Action::new("copy", "Copier"), Action::new("define", "Définition")],
                format!("{def} {w}\t{w}"),
            )
        })
        .collect())
}

pub fn run(item: &Item, action: &str) -> Result<Outcome, String> {
    let (first, text) = item.data.split_once('\t').unwrap_or((&item.data, ""));
    Ok(match action {
        "open" => {
            crate::util::open(first)?;
            Outcome::Hide
        }
        "copy" => Outcome::Copy(text.to_string()),
        "paste" => Outcome::Paste { text: text.to_string(), back: 0 },
        "define" => Outcome::Fill(first.to_string()),
        _ => return Err(tf("action inconnue : {a}", &[("a", &action)])),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_prefix() {
        assert_eq!(language_and_word("en house").0, "en");
        assert_eq!(language_and_word("en house").1, "house");
        assert_eq!(language_and_word("pomme de terre").1, "pomme de terre");
    }

    #[test]
    fn french_definitions_from_wikitext() {
        let w = "{{voir/maison}}\n== {{langue|fr}} ==\n=== {{S|étymologie}} ===\n: Du latin.\n\n=== {{S|nom|fr}} ===\n'''maison''' {{pron|me.zɔ̃|fr}} {{f}}\n# {{édifices|fr}} [[bâtiment|Bâtiment]] servant de [[logis]].<ref>Note</ref>\n#*{{exemple|lang=fr|Les '''maisons''' peuvent brûler.}}\n# Ménage, {{lien|foyer|fr}}.\n#: Note d'usage.\n\n=== {{S|adjectif|fr}} ===\n# Fait à la maison.\n\n== {{langue|fro}} ==\n=== {{S|nom|fro}} ===\n# Demeure.\n";
        assert_eq!(parse_french_definitions(w), [
            ("Nom".to_string(), "(Édifices) Bâtiment servant de logis.".to_string()),
            ("Nom".to_string(), "Ménage, foyer.".to_string()),
            ("Adjectif".to_string(), "Fait à la maison.".to_string()),
        ]);
    }

    #[test]
    fn english_html_loses_its_css() {
        assert_eq!(strip_html("<style>.mw-parser-output .defdate{font-size:smaller}</style>A <a href=\"x\">house</a>."), "A house.");
    }

    #[test]
    fn french_synonyms_from_wikitext() {
        let w = "==== {{S|synonymes}} ====\n{{(}}\n* [[bref]]\n* [[diligent]] {{familier|fr}}\n* [[expéditif|expéditive]]\n{{)}}\n\n==== {{S|antonymes}} ====\n* [[lent]]\n";
        assert_eq!(parse_french_synonyms(w), ["bref", "diligent", "expéditif"]);
    }

    /// Real requests: `cargo test real_sources -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn real_sources() {
        let s = Settings::default();
        for key in ["def\tfr\tmaison", "def\ten\thouse"] {
            let d = fetch_definitions(&s, key).unwrap();
            println!("{key}: {:?}", d.iter().take(3).map(|i| (&i.subtitle, &i.title)).collect::<Vec<_>>());
            assert!(d.len() >= 2, "{key}");
        }
        for key in ["syn\tfr\trapide", "syn\ten\tfast"] {
            let d = fetch_synonyms(&s, key).unwrap();
            println!("{key}: {:?}", d.iter().take(6).map(|i| &i.title).collect::<Vec<_>>());
            assert!(d.len() >= 3, "{key}");
        }
    }
}
