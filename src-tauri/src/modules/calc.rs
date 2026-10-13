//! Calculator and unit conversions (fend), currencies from the ECB daily rates (cached).

use crate::i18n::{t, tf};
use crate::search::{Action, Item};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::RwLock;
use std::time::{Duration, Instant, SystemTime};

const ECB_URL: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml";
const MAX_AGE: Duration = Duration::from_secs(12 * 3600);

/// Units of each currency for 1 EUR, and the ECB date.
#[derive(Debug, Default, Clone, PartialEq)]
pub struct Rates {
    pub per_eur: HashMap<String, f64>,
    pub date: String,
}

static RATES: RwLock<Option<Rates>> = RwLock::new(None);

pub fn parse_ecb(xml: &str) -> Option<Rates> {
    let re = regex::Regex::new(r"currency='([A-Z]{3})'\s+rate='([0-9.]+)'").ok()?;
    let per_eur: HashMap<String, f64> = re.captures_iter(xml).filter_map(|c| Some((c[1].to_string(), c[2].parse().ok()?))).collect();
    let date = regex::Regex::new(r"time='([0-9-]+)'").ok()?.captures(xml).map(|c| c[1].to_string()).unwrap_or_default();
    (!per_eur.is_empty()).then_some(Rates { per_eur, date })
}

fn cache_path() -> PathBuf {
    crate::paths::data_dir().join("ecb.xml")
}

fn install(r: Rates) {
    *RATES.write().unwrap_or_else(|e| e.into_inner()) = Some(r);
}

/// Load the cached rates, then download fresh ones if the cache is old. Runs in the background.
pub fn refresh_rates() {
    let path = cache_path();
    if let Some(r) = std::fs::read_to_string(&path).ok().as_deref().and_then(parse_ecb) {
        install(r);
    }
    let fresh = std::fs::metadata(&path)
        .and_then(|m| m.modified())
        .ok()
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age < MAX_AGE);
    if fresh {
        return;
    }
    let body = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(15))
        .build()
        .and_then(|c| c.get(ECB_URL).send())
        .and_then(|r| r.error_for_status())
        .and_then(|r| r.text());
    match body {
        Ok(xml) => {
            if let Some(r) = parse_ecb(&xml) {
                write_cache(&path, &xml);
                install(r);
            }
        }
        Err(e) => eprintln!("landry: taux BCE non téléchargés (cache utilisé) : {e}"),
    }
}

fn write_cache(path: &Path, xml: &str) {
    if let Some(dir) = path.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(path, xml);
}

struct Ecb;

impl fend_core::ExchangeRateFnV2 for Ecb {
    /// Units of `currency` for one unit of the base currency (EUR here), as the ECB publishes them.
    fn relative_to_base_currency(
        &self,
        currency: &str,
        _options: &fend_core::ExchangeRateFnV2Options,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync + 'static>> {
        if currency == "EUR" {
            return Ok(1.0);
        }
        let guard = RATES.read().unwrap_or_else(|e| e.into_inner());
        let rate = guard
            .as_ref()
            .and_then(|r| r.per_eur.get(currency))
            .ok_or_else(|| tf("taux {c} indisponible (hors ligne et pas de cache)", &[("c", &currency)]))?;
        Ok(*rate)
    }
}

struct Deadline(Instant);

impl fend_core::Interrupt for Deadline {
    fn should_interrupt(&self) -> bool {
        Instant::now() > self.0
    }
}

/// French spellings fend does not know.
fn normalize(q: &str) -> String {
    let mut s = format!(" {} ", q.trim());
    for (from, to) in [(" en ", " to "), (" vers ", " to "), (" dans ", " to "), (" → ", " to ")] {
        s = s.replace(from, to);
    }
    // Case-sensitive on purpose: "To" (téraoctet) must not catch the English "to".
    let units = regex::Regex::new(r"(\d|\s)(Kio|Mio|Gio|Tio|[KkMmGg]o|To|octets?)\b").expect("regex valide");
    s = units
        .replace_all(&s, |c: &regex::Captures| {
            let u = match c[2].to_lowercase().as_str() {
                "kio" => "KiB",
                "mio" => "MiB",
                "gio" => "GiB",
                "tio" => "TiB",
                "ko" => "kB",
                "mo" => "MB",
                "go" => "GB",
                "to" => "TB",
                _ => "bytes",
            };
            format!("{}{u}", &c[1])
        })
        .into_owned();
    s.replace('€', " EUR ").replace('$', " USD ").replace('£', " GBP ").trim().to_string()
}

/// A comma between digits ("3,5") means French decimals.
fn decimal_comma(q: &str) -> bool {
    let b = q.as_bytes();
    (1..b.len().saturating_sub(1)).any(|i| b[i] == b',' && b[i - 1].is_ascii_digit() && b[i + 1].is_ascii_digit())
}

pub fn evaluate(q: &str) -> Option<String> {
    let q = q.trim();
    // Only things that look like math: a digit, and not a bare number.
    if !q.chars().any(|c| c.is_ascii_digit()) || q.parse::<f64>().is_ok() {
        return None;
    }
    let mut ctx = fend_core::Context::new();
    ctx.set_exchange_rate_handler_v2(Ecb);
    if decimal_comma(q) {
        ctx.set_decimal_separator_style(fend_core::DecimalSeparatorStyle::Comma);
    }
    let input = normalize(q);
    let deadline = Deadline(Instant::now() + Duration::from_millis(150));
    let mut out = fend_core::evaluate_with_interrupt(&input, &mut ctx, &deadline).ok()?.get_main_result().trim().to_string();
    // "200 * 15%" is 3000% for fend; a plain number is meant when the input mixes both.
    let plain_number = regex::Regex::new(r"\d(\s*[^\d\s%.,]|\s*$)").expect("regex valide");
    if out.ends_with('%') && plain_number.is_match(&input) {
        if let Ok(r) = fend_core::evaluate_with_interrupt(&format!("({input}) to unitless"), &mut ctx, &deadline) {
            out = r.get_main_result().trim().to_string();
        }
    }
    // fend echoes some inputs unchanged (plain words, dates): nothing computed.
    (!out.is_empty() && out != input && out != q).then_some(out)
}

pub fn query(q: &str) -> Vec<Item> {
    let Some(result) = evaluate(q) else { return vec![] };
    let rates = RATES.read().unwrap_or_else(|e| e.into_inner()).as_ref().map(|r| r.date.clone());
    let currency = ["EUR", "USD", "GBP", "CHF", "JPY", "CAD"].iter().any(|c| normalize(q).to_uppercase().contains(c));
    let subtitle = match (currency, rates) {
        (true, Some(d)) => tf("Taux BCE du {d} · Entrée pour copier", &[("d", &d)]),
        _ => t("Calcul · Entrée pour copier"),
    };
    vec![Item {
        id: "calc:result".into(),
        module: "calc".into(),
        title: format!("= {result}"),
        subtitle,
        actions: vec![Action::new("copy", "Copier le résultat")],
        data: result,
        ..Default::default()
    }]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic_and_percent() {
        assert_eq!(evaluate("12*7").as_deref(), Some("84"));
        assert_eq!(evaluate("(2+3)^2").as_deref(), Some("25"));
        assert_eq!(evaluate("200 * 15%").as_deref(), Some("30"));
        assert_eq!(evaluate("3,5 + 2").as_deref(), Some("5,5"));
        assert_eq!(evaluate("sqrt(16)").as_deref(), Some("4"));
        assert_eq!(evaluate("12*7%").as_deref(), Some("0.84"));
        assert_eq!(evaluate("20% + 5%").as_deref(), Some("25%"));
    }

    #[test]
    fn not_math() {
        assert_eq!(evaluate("firefox"), None);
        assert_eq!(evaluate("2026"), None);
        assert_eq!(evaluate("port 5173"), None);
    }

    #[test]
    fn units_in_french() {
        assert_eq!(evaluate("5 km en m").as_deref(), Some("5000 m"));
        assert_eq!(evaluate("1 Gio en Mio").as_deref(), Some("1024 MiB"));
        assert_eq!(evaluate("1 Go en Mo").as_deref(), Some("1000 MB"));
        assert!(evaluate("100 °C en °F").unwrap().starts_with("212"));
        assert!(evaluate("90 km/h en m/s").unwrap().starts_with("25"));
    }

    #[test]
    fn ecb_parsing_and_currency() {
        let xml = "<Cube time='2026-10-12'><Cube currency='USD' rate='1.1000'/><Cube currency='JPY' rate='160.00'/></Cube>";
        let r = parse_ecb(xml).unwrap();
        assert_eq!(r.date, "2026-10-12");
        assert_eq!(r.per_eur["USD"], 1.1);
        install(r);
        assert_eq!(evaluate("10 EUR en USD").as_deref(), Some("11 USD"));
        assert_eq!(evaluate("11 $ en €").as_deref(), Some("10 EUR"));
    }
}

