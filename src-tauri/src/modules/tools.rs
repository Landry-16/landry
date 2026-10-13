//! Developer toolbox: results computed from the query itself ("b64 bonjour", "#ff8800"...).

use super::color;
use crate::search::{Action, Item};
use base64::engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD};
use base64::Engine as _;
use chrono::{DateTime, Local, NaiveDate, NaiveDateTime, TimeZone, Utc};
use md5::Md5;
use percent_encoding::{percent_decode_str, utf8_percent_encode, NON_ALPHANUMERIC};
use sha1::Sha1;
use sha2::{Digest, Sha256};

/// Tool catalog: searchable entries that fill the bar with the tool keyword.
const CATALOG: &[(&str, &str, &str, &[&str])] = &[
    ("uuid", "UUID", "Générer un UUID v4", &["identifiant", "guid"]),
    ("b64 ", "Base64 : encoder", "b64 <texte>", &["base64", "encode"]),
    ("b64d ", "Base64 : décoder", "b64d <texte>", &["base64", "decode"]),
    ("url ", "URL : encoder", "url <texte>", &["percent", "encode", "urlencode"]),
    ("urld ", "URL : décoder", "urld <texte>", &["percent", "decode", "urldecode"]),
    ("json ", "JSON : formater", "json <texte> ou jsonmin <texte>", &["format", "pretty", "minify"]),
    ("sha256 ", "Hachage SHA-256", "sha256 <texte> (aussi sha1, md5)", &["hash", "sha1", "md5", "empreinte"]),
    ("jwt ", "JWT : décoder", "jwt <jeton>", &["token", "jeton"]),
    ("ts ", "Timestamp", "ts, ts 1700000000 ou ts 2026-10-13 14:30", &["timestamp", "epoch", "date", "unix"]),
    ("regex ", "Tester une regex", "regex /motif/ texte", &["expression", "regulière", "regexp"]),
    ("contraste ", "Contraste WCAG", "contraste #fff #1c1a2e", &["contrast", "accessibilite", "wcag"]),
    ("#", "Couleurs", "#ff8800, rgb(...), hsl(...), oklch(...)", &["color", "couleur", "hex", "oklch", "hsl", "rgb"]),
];

pub fn catalog() -> Vec<Item> {
    let mut out: Vec<Item> = CATALOG
        .iter()
        .map(|(fill, title, sub, kw)| Item {
            id: format!("tool:catalog:{}", fill.trim()),
            module: "tools".into(),
            title: (*title).into(),
            subtitle: (*sub).into(),
            keywords: kw.iter().map(|s| s.to_string()).chain([fill.trim().to_string()]).collect(),
            actions: vec![Action::new("fill", "Utiliser")],
            data: (*fill).into(),
            ..Default::default()
        })
        .collect();
    out.push(Item {
        id: "tool:diff".into(),
        module: "tools".into(),
        title: "Comparer deux textes".into(),
        subtitle: "Diff ligne par ligne".into(),
        keywords: vec!["diff".into(), "comparer".into(), "difference".into()],
        actions: vec![Action::new("view", "Ouvrir")],
        data: "diff".into(),
        ..Default::default()
    });
    for (id, title, sub, kw, action) in [
        ("tool:capture", "Capture d'une zone", "Enregistrée dans Images et copiée", &["capture", "screenshot", "ecran"][..], "capture"),
        ("tool:ocr", "Texte d'une zone (OCR)", "Copie le texte reconnu à l'écran", &["ocr", "texte", "tesseract", "reconnaissance"][..], "ocr"),
    ] {
        out.push(Item {
            id: id.into(),
            module: "tools".into(),
            title: title.into(),
            subtitle: sub.into(),
            keywords: kw.iter().map(|s| s.to_string()).collect(),
            actions: vec![Action::new(action, title)],
            ..Default::default()
        });
    }
    out.push(Item {
        id: "tool:catalog:qr".into(),
        module: "tools".into(),
        title: "QR code".into(),
        subtitle: "qr <texte ou URL>".into(),
        keywords: vec!["qr".into(), "qrcode".into()],
        actions: vec![Action::new("fill", "Utiliser")],
        data: "qr ".into(),
        ..Default::default()
    });
    out.push(Item {
        id: "tool:pick".into(),
        module: "tools".into(),
        title: "Pipette couleur".into(),
        subtitle: "Prendre une couleur à l'écran".into(),
        keywords: vec!["pipette".into(), "color picker".into(), "couleur".into(), "pick".into()],
        actions: vec![Action::new("pick", "Choisir une couleur")],
        ..Default::default()
    });
    out
}

fn one_line(s: &str) -> String {
    let flat: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() > 120 {
        format!("{}…", flat.chars().take(119).collect::<String>())
    } else {
        flat
    }
}

/// A result to copy. `key` keeps ids distinct when one query yields several results.
fn copy(key: &str, title: &str, subtitle: &str, value: String) -> Item {
    Item {
        id: format!("tool:{key}"),
        module: "tools".into(),
        title: one_line(title),
        subtitle: subtitle.into(),
        actions: vec![Action::new("copy", "Copier")],
        data: value,
        ..Default::default()
    }
}

fn info(key: &str, title: &str, subtitle: &str) -> Item {
    Item { id: format!("tool:{key}"), module: "tools".into(), title: one_line(title), subtitle: subtitle.into(), ..Default::default() }
}

fn b64_decode(s: &str) -> Option<Vec<u8>> {
    let s: String = s.chars().filter(|c| !c.is_whitespace()).collect();
    STANDARD
        .decode(&s)
        .ok()
        .or_else(|| URL_SAFE_NO_PAD.decode(s.trim_end_matches('=')).ok())
        .or_else(|| base64::engine::general_purpose::STANDARD_NO_PAD.decode(s.trim_end_matches('=')).ok())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn local(dt: DateTime<Utc>) -> String {
    dt.with_timezone(&Local).format("%d/%m/%Y %H:%M:%S").to_string()
}

fn timestamp(rest: &str) -> Vec<Item> {
    let now = Utc::now();
    if rest.is_empty() {
        return vec![
            copy("ts:now", &now.timestamp().to_string(), &format!("Timestamp actuel, {}", local(now)), now.timestamp().to_string()),
            copy("ts:ms", &now.timestamp_millis().to_string(), "En millisecondes", now.timestamp_millis().to_string()),
            copy("ts:iso", &now.to_rfc3339(), "ISO 8601 (UTC)", now.to_rfc3339()),
        ];
    }
    if let Ok(n) = rest.parse::<i64>() {
        // 13 digits and more: milliseconds.
        let dt = if n.abs() >= 100_000_000_000 { Utc.timestamp_millis_opt(n).single() } else { Utc.timestamp_opt(n, 0).single() };
        return match dt {
            Some(dt) => vec![
                copy("ts:local", &local(dt), "Date locale", local(dt)),
                copy("ts:iso", &dt.to_rfc3339(), "ISO 8601 (UTC)", dt.to_rfc3339()),
            ],
            None => vec![info("ts:err", "Timestamp hors limites", rest)],
        };
    }
    let parsed = NaiveDateTime::parse_from_str(rest, "%Y-%m-%d %H:%M")
        .or_else(|_| NaiveDateTime::parse_from_str(rest, "%Y-%m-%d %H:%M:%S"))
        .or_else(|_| NaiveDate::parse_from_str(rest, "%Y-%m-%d").map(|d| d.and_hms_opt(0, 0, 0).unwrap_or_default()))
        .or_else(|_| NaiveDate::parse_from_str(rest, "%d/%m/%Y").map(|d| d.and_hms_opt(0, 0, 0).unwrap_or_default()));
    match parsed.ok().and_then(|n| Local.from_local_datetime(&n).single()) {
        Some(dt) => vec![
            copy("ts:sec", &dt.timestamp().to_string(), "Timestamp (heure locale)", dt.timestamp().to_string()),
            copy("ts:ms", &dt.timestamp_millis().to_string(), "En millisecondes", dt.timestamp_millis().to_string()),
        ],
        None => vec![info("ts:err", "Date non reconnue", "ts 1700000000, ts 2026-10-13 14:30 ou ts 13/10/2026")],
    }
}

fn jwt(token: &str) -> Vec<Item> {
    let parts: Vec<&str> = token.trim().split('.').collect();
    if parts.len() < 2 {
        return vec![info("jwt:err", "JWT invalide", "attendu : en-tête.charge.signature")];
    }
    let decode = |p: &str| -> Option<serde_json::Value> { serde_json::from_slice(&b64_decode(p)?).ok() };
    let (Some(header), Some(payload)) = (decode(parts[0]), decode(parts[1])) else {
        return vec![info("jwt:err", "JWT illisible", "en-tête ou charge non décodable")];
    };
    let mut sub = Vec::new();
    for key in ["exp", "iat"] {
        if let Some(dt) = payload.get(key).and_then(|v| v.as_i64()).and_then(|n| Utc.timestamp_opt(n, 0).single()) {
            let label = if key == "exp" { if dt < Utc::now() { "expiré le" } else { "expire le" } } else { "émis le" };
            sub.push(format!("{label} {}", local(dt)));
        }
    }
    let pretty = serde_json::to_string_pretty(&serde_json::json!({ "header": header, "payload": payload })).unwrap_or_default();
    let alg = header.get("alg").and_then(|v| v.as_str()).unwrap_or("?");
    let who = payload.get("sub").and_then(|v| v.as_str()).map(|s| format!(", sub {s}")).unwrap_or_default();
    vec![copy("jwt", &format!("JWT {alg}{who}"), &sub.join(" · "), pretty)]
}

fn regex_test(rest: &str) -> Vec<Item> {
    // "/pattern/flags text" (pattern may contain spaces) or "pattern text".
    let (pattern, flags, text) = if let Some(r) = rest.strip_prefix('/') {
        match r.rfind('/') {
            Some(end) => {
                let after = &r[end + 1..];
                let (flags, text) = after.split_once(' ').unwrap_or((after, ""));
                (&r[..end], flags, text)
            }
            None => return vec![info("regex:err", "Regex incomplète", "regex /motif/ texte")],
        }
    } else {
        let (p, t) = rest.split_once(' ').unwrap_or((rest, ""));
        (p, "", t)
    };
    let full = if flags.contains('i') { format!("(?i){pattern}") } else { pattern.to_string() };
    match regex::Regex::new(&full) {
        Err(e) => vec![info("regex:err", "Regex invalide", &one_line(&e.to_string()))],
        Ok(re) => {
            let found: Vec<&str> = re.find_iter(text).map(|m| m.as_str()).collect();
            let title = match found.len() {
                0 => "Aucune correspondance".to_string(),
                1 => format!("1 correspondance : {}", found[0]),
                n => format!("{n} correspondances : {}", found.iter().take(8).cloned().collect::<Vec<_>>().join(" · ")),
            };
            vec![copy("regex", &title, &format!("/{pattern}/{flags}"), found.join("\n"))]
        }
    }
}

fn colors(c: color::Rgb, prefix: &str) -> Vec<Item> {
    let w = color::contrast(c, color::Rgb { r: 1.0, g: 1.0, b: 1.0 });
    let k = color::contrast(c, color::Rgb { r: 0.0, g: 0.0, b: 0.0 });
    let sub = format!("Contraste {w:.2} sur blanc, {k:.2} sur noir");
    [("hex", c.hex()), ("rgb", c.rgb()), ("hsl", c.hsl()), ("oklch", c.oklch())]
        .into_iter()
        .map(|(k2, v)| copy(&format!("{prefix}color:{k2}"), &v, &sub, v.clone()))
        .collect()
}

pub fn query(q: &str) -> Vec<Item> {
    let (word, rest) = q.split_once(' ').map(|(a, b)| (a, b.trim())).unwrap_or((q, ""));
    let digest = |kind: &str, bytes: Vec<u8>| copy(kind, &hex(&bytes), &format!("{} de « {} »", kind.to_uppercase(), one_line(rest)), hex(&bytes));
    match (word.to_lowercase().as_str(), rest.is_empty()) {
        ("uuid", _) => {
            let id = uuid::Uuid::new_v4().to_string();
            vec![copy("uuid", &id, "UUID v4 aléatoire", id.clone())]
        }
        ("b64" | "base64", false) => {
            let v = STANDARD.encode(rest);
            vec![copy("b64", &v, "Base64", v.clone())]
        }
        ("b64d" | "unbase64", false) => match b64_decode(rest).map(String::from_utf8) {
            Some(Ok(s)) => vec![copy("b64d", &s, "Base64 décodé", s.clone())],
            Some(Err(e)) => {
                let h = hex(e.as_bytes());
                vec![copy("b64d", &h, "Base64 décodé (binaire, en hexadécimal)", h.clone())]
            }
            None => vec![info("b64d:err", "Base64 invalide", rest)],
        },
        ("url", false) => {
            let v = utf8_percent_encode(rest, NON_ALPHANUMERIC).to_string();
            vec![copy("url", &v, "Encodage URL", v.clone())]
        }
        ("urld", false) => {
            let v = percent_decode_str(&rest.replace('+', " ")).decode_utf8_lossy().into_owned();
            vec![copy("urld", &v, "URL décodée", v.clone())]
        }
        ("json" | "jsonmin", false) => match serde_json::from_str::<serde_json::Value>(rest) {
            Ok(v) => {
                let out = if word == "jsonmin" { v.to_string() } else { serde_json::to_string_pretty(&v).unwrap_or_default() };
                vec![copy("json", &out, "JSON valide, Entrée pour copier", out.clone())]
            }
            Err(e) => vec![info("json:err", "JSON invalide", &e.to_string())],
        },
        ("sha256", false) => vec![digest("sha256", Sha256::digest(rest.as_bytes()).to_vec())],
        ("sha1", false) => vec![digest("sha1", Sha1::digest(rest.as_bytes()).to_vec())],
        ("md5", false) => vec![digest("md5", Md5::digest(rest.as_bytes()).to_vec())],
        ("jwt", false) => jwt(rest),
        ("qr" | "qrcode", false) => vec![Item {
            id: "tool:qr".into(),
            module: "tools".into(),
            title: format!("QR code : {}", one_line(rest)),
            subtitle: "Entrée pour l'enregistrer en SVG dans Téléchargements".into(),
            actions: vec![Action::new("qr-save", "Enregistrer en SVG"), Action::new("copy", "Copier le texte")],
            data: rest.to_string(),
            ..Default::default()
        }],
        ("ts" | "timestamp", _) => timestamp(rest),
        ("regex", false) => regex_test(rest),
        ("contraste" | "contrast", false) => {
            let parts: Vec<&str> = rest.split_whitespace().collect();
            match (parts.first().and_then(|p| color::parse(p)), parts.get(1).and_then(|p| color::parse(p))) {
                (Some(a), Some(b)) => {
                    let r = color::contrast(a, b);
                    vec![copy("contrast", &format!("Contraste {r:.2}:1"), &color::wcag(r), format!("{r:.2}"))]
                }
                _ => vec![info("contrast:err", "Deux couleurs attendues", "contraste #ffffff #1c1a2e")],
            }
        }
        _ => color::parse(q).map(|c| colors(c, "")).unwrap_or_default(),
    }
}

/// SVG of a QR code (dark modules on white, with a quiet zone).
pub fn qr_svg(text: &str) -> Result<String, String> {
    let code = qrcode::QrCode::new(text.as_bytes()).map_err(|e| format!("QR code impossible : {e}"))?;
    Ok(code.render::<qrcode::render::svg::Color>().min_dimensions(256, 256).quiet_zone(true).build())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn qr() {
        let svg = qr_svg("https://landry.dev").unwrap();
        assert!(svg.starts_with("<?xml") && svg.contains("<svg"));
        assert_eq!(first("qr https://landry.dev").data, "https://landry.dev");
    }

    fn first(q: &str) -> Item {
        query(q).into_iter().next().unwrap_or_else(|| panic!("aucun résultat pour {q}"))
    }

    #[test]
    fn encodings() {
        assert_eq!(first("b64 bonjour").data, "Ym9uam91cg==");
        assert_eq!(first("b64d Ym9uam91cg==").data, "bonjour");
        assert_eq!(first("b64d Ym9uam91cg").data, "bonjour");
        assert_eq!(first("url a b&c").data, "a%20b%26c");
        assert_eq!(first("urld a%20b%26c").data, "a b&c");
    }

    #[test]
    fn json_and_hashes() {
        assert_eq!(first(r#"jsonmin { "a" : [1, 2] }"#).data, r#"{"a":[1,2]}"#);
        assert!(first(r#"json {"a":1}"#).data.contains("\n"));
        assert_eq!(first("json {nope").title, "JSON invalide");
        assert_eq!(first("sha256 abc").data, "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad");
        assert_eq!(first("md5 abc").data, "900150983cd24fb0d6963f7d28e17f72");
        assert_eq!(first("sha1 abc").data, "a9993e364706816aba3e25717850c26c9cd0d89d");
    }

    #[test]
    fn uuid_and_timestamps() {
        assert_eq!(first("uuid").data.len(), 36);
        assert_eq!(first("ts 0").actions.len(), 1);
        assert_eq!(query("ts 1700000000").into_iter().nth(1).unwrap().data, "2023-11-14T22:13:20+00:00");
        assert_eq!(query("ts 1700000000000").into_iter().nth(1).unwrap().data, "2023-11-14T22:13:20+00:00");
        assert!(first("ts 2026-10-13 14:30").data.parse::<i64>().is_ok());
        assert_eq!(query("ts").len(), 3);
    }

    #[test]
    fn jwt_decodes() {
        // {"alg":"HS256","typ":"JWT"} . {"sub":"1234567890","name":"John Doe","iat":1516239022}
        let t = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIiwibmFtZSI6IkpvaG4gRG9lIiwiaWF0IjoxNTE2MjM5MDIyfQ.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c";
        let it = first(&format!("jwt {t}"));
        assert_eq!(it.title, "JWT HS256, sub 1234567890");
        assert!(it.data.contains("John Doe"));
        assert!(it.subtitle.contains("émis le"));
    }

    #[test]
    fn regex_and_colors() {
        assert_eq!(first(r"regex /\d+/ a1 b22 c333").title, "3 correspondances : 1 · 22 · 333");
        assert_eq!(first("regex /BON/i bonjour").title, "1 correspondance : bon");
        assert_eq!(first("regex /(/ x").title, "Regex invalide");
        let c = query("#ff8800");
        assert_eq!(c.len(), 4);
        assert_eq!(c[1].data, "rgb(255, 136, 0)");
        assert!(first("contraste #fff #000").title.starts_with("Contraste 21.00"));
    }

    #[test]
    fn unrelated_query_yields_nothing() {
        assert!(query("firefox").is_empty());
        assert!(query("b64").is_empty());
    }
}
