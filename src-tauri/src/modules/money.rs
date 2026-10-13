//! Amounts for a freelance activity: HT <-> TTC and micro-entreprise contributions.
//! Every rate comes from settings.json; none is hardcoded here.

use crate::config::Settings;
use crate::search::{Action, Item};

/// "1234.5" -> "1 234,50 €" (narrow no-break space as thousands separator).
pub fn euros(v: f64) -> String {
    let cents = (v.abs() * 100.0).round() as u64;
    let (int, dec) = (cents / 100, cents % 100);
    let digits = int.to_string();
    let mut grouped = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            grouped.push('\u{202f}');
        }
        grouped.push(c);
    }
    format!("{}{grouped},{dec:02}\u{a0}€", if v < 0.0 { "-" } else { "" })
}

fn rate(v: f64) -> String {
    let s = format!("{v}").replace('.', ",");
    format!("{s}\u{a0}%")
}

/// "1 234,50", "1234.5", "1234 €" -> 1234.5
pub fn parse_amount(s: &str) -> Option<f64> {
    let cleaned: String = s
        .trim()
        .trim_end_matches(['€', ' '])
        .trim_end_matches("euros")
        .trim_end_matches("eur")
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '\u{202f}' && *c != '\u{a0}')
        .collect();
    let v: f64 = cleaned.replace(',', ".").parse().ok()?;
    (v.is_finite() && v >= 0.0).then_some(v)
}

fn item(key: &str, title: String, subtitle: String, copy: String) -> Item {
    Item {
        id: format!("money:{key}"),
        module: "calc".into(),
        title,
        subtitle,
        actions: vec![Action::new("copy", "Copier le montant")],
        data: copy,
        ..Default::default()
    }
}

fn plain(v: f64) -> String {
    format!("{v:.2}").replace('.', ",")
}

pub fn query(settings: &Settings, q: &str) -> Vec<Item> {
    let lower = q.trim().to_lowercase();
    // "100 ht", "120 € ttc"
    for (suffix, is_ht) in [(" ht", true), (" ttc", false)] {
        if let Some(amount) = lower.strip_suffix(suffix).and_then(parse_amount) {
            let t = settings.vat_rate / 100.0;
            let (ht, ttc) = if is_ht { (amount, amount * (1.0 + t)) } else { (amount / (1.0 + t), amount) };
            let tva = ttc - ht;
            let sub = format!("TVA {} : {}", rate(settings.vat_rate), euros(tva));
            return if is_ht {
                vec![item("ttc", format!("{} TTC", euros(ttc)), format!("{} HT · {sub}", euros(ht)), plain(ttc))]
            } else {
                vec![item("ht", format!("{} HT", euros(ht)), format!("{} TTC · {sub}", euros(ttc)), plain(ht))]
            };
        }
    }
    // "ca 3000", "cotisations 3000"
    let (word, rest) = lower.split_once(' ').unwrap_or((&lower, ""));
    if matches!(word, "ca" | "cotisations" | "cotisation" | "urssaf") {
        let Some(amount) = parse_amount(rest) else { return vec![] };
        let Some(r) = settings.micro_rate else {
            return vec![Item {
                id: "money:norate".into(),
                module: "calc".into(),
                title: "Taux de cotisations non renseigné".into(),
                subtitle: "Ajoute micro_rate (en %) dans Réglages, onglet Général".into(),
                actions: vec![Action::new("settings", "Ouvrir les réglages")],
                ..Default::default()
            }];
        };
        let contrib = amount * r / 100.0;
        let tax = settings.micro_tax_rate.map(|t| amount * t / 100.0).unwrap_or(0.0);
        let net = amount - contrib - tax;
        let mut sub = format!("CA {} · cotisations {}", euros(amount), rate(r));
        if let Some(t) = settings.micro_tax_rate {
            sub.push_str(&format!(" · versement libératoire {} : {}", rate(t), euros(tax)));
        }
        return vec![
            item("contrib", format!("{} de cotisations", euros(contrib)), sub.clone(), plain(contrib)),
            item("net", format!("{} restants", euros(net)), sub, plain(net)),
        ];
    }
    vec![]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn settings() -> Settings {
        Settings { vat_rate: 20.0, micro_rate: Some(24.6), micro_tax_rate: None, ..Settings::default() }
    }

    #[test]
    fn formatting_and_parsing() {
        assert_eq!(euros(1234.5), "1\u{202f}234,50\u{a0}€");
        assert_eq!(euros(0.004), "0,00\u{a0}€");
        assert_eq!(euros(1_000_000.0), "1\u{202f}000\u{202f}000,00\u{a0}€");
        assert_eq!(parse_amount("1 234,50 €"), Some(1234.5));
        assert_eq!(parse_amount("1234.5"), Some(1234.5));
        assert_eq!(parse_amount("abc"), None);
        assert_eq!(parse_amount("-5"), None);
    }

    #[test]
    fn vat_both_ways() {
        let r = query(&settings(), "100 ht");
        assert_eq!(r[0].title, format!("{} TTC", euros(120.0)));
        assert_eq!(r[0].data, "120,00");
        let r = query(&settings(), "120 € ttc");
        assert_eq!(r[0].title, format!("{} HT", euros(100.0)));
    }

    #[test]
    fn contributions_need_a_rate() {
        let r = query(&settings(), "ca 3000");
        assert_eq!(r[0].data, "738,00");
        assert_eq!(r[1].data, "2262,00");
        let none = Settings { micro_rate: None, ..settings() };
        assert_eq!(query(&none, "ca 3000")[0].title, "Taux de cotisations non renseigné");
        let tax = Settings { micro_tax_rate: Some(2.2), ..settings() };
        assert_eq!(query(&tax, "ca 1000")[1].data, "732,00");
    }

    #[test]
    fn unrelated() {
        assert!(query(&settings(), "chat").is_empty());
        assert!(query(&settings(), "ca va").is_empty());
    }
}
