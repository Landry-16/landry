//! Time zones ("15h Paris en New York", "heure à Tokyo"), relative dates ("dans 3 semaines")
//! and day counts ("jours ouvrés entre 01/10/2026 et 24/12/2026", French public holidays).

use crate::search::{fold, Action, Item};
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone, Weekday};
use chrono_tz::Tz;

const CITIES: &[(&str, &str)] = &[
    ("paris", "Europe/Paris"), ("lyon", "Europe/Paris"), ("marseille", "Europe/Paris"), ("france", "Europe/Paris"),
    ("londres", "Europe/London"), ("london", "Europe/London"), ("dublin", "Europe/Dublin"), ("lisbonne", "Europe/Lisbon"),
    ("madrid", "Europe/Madrid"), ("barcelone", "Europe/Madrid"), ("rome", "Europe/Rome"), ("milan", "Europe/Rome"),
    ("berlin", "Europe/Berlin"), ("bruxelles", "Europe/Brussels"), ("amsterdam", "Europe/Amsterdam"),
    ("geneve", "Europe/Zurich"), ("zurich", "Europe/Zurich"), ("vienne", "Europe/Vienna"), ("varsovie", "Europe/Warsaw"),
    ("athenes", "Europe/Athens"), ("istanbul", "Europe/Istanbul"), ("moscou", "Europe/Moscow"), ("kiev", "Europe/Kyiv"),
    ("new york", "America/New_York"), ("nyc", "America/New_York"), ("boston", "America/New_York"), ("miami", "America/New_York"),
    ("montreal", "America/Toronto"), ("toronto", "America/Toronto"), ("quebec", "America/Toronto"), ("chicago", "America/Chicago"),
    ("denver", "America/Denver"), ("san francisco", "America/Los_Angeles"), ("los angeles", "America/Los_Angeles"),
    ("seattle", "America/Los_Angeles"), ("vancouver", "America/Vancouver"), ("mexico", "America/Mexico_City"),
    ("sao paulo", "America/Sao_Paulo"), ("rio", "America/Sao_Paulo"), ("buenos aires", "America/Argentina/Buenos_Aires"),
    ("martinique", "America/Martinique"), ("guadeloupe", "America/Guadeloupe"), ("cayenne", "America/Cayenne"),
    ("dakar", "Africa/Dakar"), ("abidjan", "Africa/Abidjan"), ("casablanca", "Africa/Casablanca"), ("alger", "Africa/Algiers"),
    ("tunis", "Africa/Tunis"), ("le caire", "Africa/Cairo"), ("lagos", "Africa/Lagos"), ("nairobi", "Africa/Nairobi"),
    ("johannesburg", "Africa/Johannesburg"), ("la reunion", "Indian/Reunion"), ("reunion", "Indian/Reunion"),
    ("dubai", "Asia/Dubai"), ("mumbai", "Asia/Kolkata"), ("delhi", "Asia/Kolkata"), ("bangalore", "Asia/Kolkata"),
    ("bangkok", "Asia/Bangkok"), ("singapour", "Asia/Singapore"), ("hong kong", "Asia/Hong_Kong"), ("pekin", "Asia/Shanghai"),
    ("shanghai", "Asia/Shanghai"), ("taipei", "Asia/Taipei"), ("seoul", "Asia/Seoul"), ("tokyo", "Asia/Tokyo"),
    ("sydney", "Australia/Sydney"), ("melbourne", "Australia/Melbourne"), ("auckland", "Pacific/Auckland"),
    ("noumea", "Pacific/Noumea"), ("tahiti", "Pacific/Tahiti"), ("utc", "UTC"), ("gmt", "UTC"),
];

const JOURS: [&str; 7] = ["lundi", "mardi", "mercredi", "jeudi", "vendredi", "samedi", "dimanche"];
const MOIS: [&str; 12] = ["janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août", "septembre", "octobre", "novembre", "décembre"];

pub fn long_date(d: NaiveDate) -> String {
    format!("{} {} {} {}", JOURS[d.weekday().num_days_from_monday() as usize], d.day(), MOIS[d.month0() as usize], d.year())
}

/// City name or IANA zone ("Asia/Tokyo"); the label is the name as typed.
pub fn zone(name: &str) -> Option<Tz> {
    let n = fold(name.trim());
    if let Some((_, z)) = CITIES.iter().find(|(c, _)| *c == n) {
        return z.parse().ok();
    }
    Tz::from_str_insensitive(name.trim()).ok()
}

fn local_zone() -> Option<Tz> {
    std::env::var("TZ")
        .ok()
        .or_else(|| std::fs::read_to_string("/etc/timezone").ok())
        .and_then(|z| z.trim().parse().ok())
        .or_else(|| std::fs::read_link("/etc/localtime").ok()?.to_str()?.split("zoneinfo/").nth(1)?.parse().ok())
}

/// "15h", "15h30", "15:30", "3pm", "9 am".
pub fn parse_time(s: &str) -> Option<NaiveTime> {
    let re = regex::Regex::new(r"^(\d{1,2})(?:\s*[h:]\s*(\d{2})?)?\s*(am|pm)?$").ok()?;
    let c = re.captures(s.trim())?;
    if c.get(2).is_none() && c.get(3).is_none() && !s.contains(['h', ':']) {
        return None;
    }
    let mut h: u32 = c[1].parse().ok()?;
    let m: u32 = c.get(2).map_or(Some(0), |m| m.as_str().parse().ok())?;
    match c.get(3).map(|m| m.as_str()) {
        Some("pm") if h < 12 => h += 12,
        Some("am") if h == 12 => h = 0,
        _ => {}
    }
    NaiveTime::from_hms_opt(h, m, 0)
}

fn day_shift(from: NaiveDate, to: NaiveDate) -> &'static str {
    match (to - from).num_days() {
        0 => "",
        1 => " (lendemain)",
        -1 => " (veille)",
        _ => "",
    }
}

fn result(key: &str, title: String, subtitle: String) -> Item {
    Item {
        id: format!("date:{key}"),
        module: "calc".into(),
        data: title.clone(),
        title,
        subtitle,
        actions: vec![Action::new("copy", "Copier")],
        ..Default::default()
    }
}

/// "<heure> [ville] en|à <ville>" and "heure à <ville>".
fn timezones(q: &str) -> Option<Item> {
    let f = fold(q);
    let f = f.trim();
    if let Some(city) = f.strip_prefix("heure").map(|r| r.trim().trim_start_matches("a ").trim_start_matches("en ").trim()) {
        let tz = zone(city)?;
        let now = chrono::Utc::now().with_timezone(&tz);
        return Some(result("now", format!("{} à {}", now.format("%H:%M"), title_case(city)), long_date(now.date_naive())));
    }
    let re = regex::Regex::new(r"^(.+?)\s+(?:en|a|in|to|vers)\s+(.+)$").ok()?;
    let c = re.captures(f)?;
    let (left, target) = (c[1].trim(), c[2].trim());
    let to = zone(target)?;
    // Left side: "15h" or "15h paris".
    let (time_part, from_city) = match left.split_once(' ') {
        Some((t, city)) if parse_time(t).is_some() => (t, Some(city)),
        _ => (left, None),
    };
    let time = parse_time(time_part)?;
    let today = Local::now().date_naive();
    let (from_dt, from_label): (DateTime<Tz>, String) = match from_city {
        Some(city) => (zone(city)?.from_local_datetime(&today.and_time(time)).single()?, title_case(city)),
        None => (local_zone()?.from_local_datetime(&today.and_time(time)).single()?, "ici".into()),
    };
    let there = from_dt.with_timezone(&to);
    Some(result(
        "tz",
        format!("{} à {}{}", there.format("%H:%M"), title_case(target), day_shift(from_dt.date_naive(), there.date_naive())),
        format!("{} {}, {}", from_dt.format("%H:%M"), if from_label == "ici" { "ici".into() } else { format!("à {from_label}") }, long_date(from_dt.date_naive())),
    ))
}

fn title_case(s: &str) -> String {
    s.split(' ')
        .map(|w| {
            let mut c = w.chars();
            c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
        })
        .collect::<Vec<_>>()
        .join(" ")
}

/// "dans 3 semaines", "il y a 10 jours".
fn relative(q: &str, today: NaiveDate) -> Option<Item> {
    let f = fold(q);
    let re = regex::Regex::new(r"^(dans|il y a)\s+(\d+)\s*(jours?|j|semaines?|sem|mois|ans?|annees?)$").ok()?;
    let c = re.captures(f.trim())?;
    let n: i64 = c[2].parse().ok()?;
    let sign = if &c[1] == "dans" { 1 } else { -1 };
    let unit = &c[3];
    let d = if unit.starts_with('j') {
        today + Duration::days(sign * n)
    } else if unit.starts_with("sem") {
        today + Duration::weeks(sign * n)
    } else {
        let months = if unit.starts_with("mois") { n } else { n * 12 } as i32 * sign as i32;
        let total = today.year() * 12 + today.month0() as i32 + months;
        let (y, m) = (total.div_euclid(12), total.rem_euclid(12) as u32 + 1);
        // Clamp the day (31 janvier + 1 mois = 28 ou 29 février).
        (0..4).find_map(|back| NaiveDate::from_ymd_opt(y, m, today.day() - back))?
    };
    Some(result("rel", long_date(d), format!("{} · {}", d.format("%d/%m/%Y"), q.trim())))
}

pub fn easter(year: i32) -> NaiveDate {
    let a = year % 19;
    let b = year / 100;
    let c = year % 100;
    let d = b / 4;
    let e = b % 4;
    let f = (b + 8) / 25;
    let g = (b - f + 1) / 3;
    let h = (19 * a + b - d - g + 15) % 30;
    let i = c / 4;
    let k = c % 4;
    let l = (32 + 2 * e + 2 * i - h - k) % 7;
    let m = (a + 11 * h + 22 * l) / 451;
    let month = (h + l - 7 * m + 114) / 31;
    let day = (h + l - 7 * m + 114) % 31 + 1;
    NaiveDate::from_ymd_opt(year, month as u32, day as u32).expect("date de Pâques valide")
}

pub fn is_french_holiday(d: NaiveDate) -> bool {
    let fixed = [(1, 1), (5, 1), (5, 8), (7, 14), (8, 15), (11, 1), (11, 11), (12, 25)];
    if fixed.contains(&(d.month(), d.day())) {
        return true;
    }
    let e = easter(d.year());
    [1, 39, 50].iter().any(|off| e + Duration::days(*off) == d)
}

/// Working days in [from, to], both included.
pub fn business_days(from: NaiveDate, to: NaiveDate) -> i64 {
    let (a, b) = if from <= to { (from, to) } else { (to, from) };
    a.iter_days()
        .take_while(|d| *d <= b)
        .filter(|d| !matches!(d.weekday(), Weekday::Sat | Weekday::Sun) && !is_french_holiday(*d))
        .count() as i64
}

pub fn parse_date(s: &str, today: NaiveDate) -> Option<NaiveDate> {
    let s = fold(s.trim());
    match s.as_str() {
        "aujourd'hui" | "aujourdhui" | "auj" => return Some(today),
        "demain" => return Some(today + Duration::days(1)),
        _ => {}
    }
    NaiveDate::parse_from_str(&s, "%d/%m/%Y")
        .or_else(|_| NaiveDate::parse_from_str(&s, "%Y-%m-%d"))
        .or_else(|_| NaiveDate::parse_from_str(&format!("{s}/{}", today.year()), "%d/%m/%Y"))
        .ok()
}

/// "jours ouvrés entre A et B", "jours entre A et B".
fn day_counts(q: &str, today: NaiveDate) -> Option<Item> {
    let f = fold(q);
    let re = regex::Regex::new(r"^jours?\s*(ouvres|ouvrables)?\s+(?:entre\s+)?(\S+)\s+(?:et\s+)?(\S+)$").ok()?;
    let c = re.captures(f.trim())?;
    let (a, b) = (parse_date(&c[2], today)?, parse_date(&c[3], today)?);
    if c.get(1).is_some() {
        let n = business_days(a, b);
        let label = if n == 1 { "1 jour ouvré".to_string() } else { format!("{n} jours ouvrés") };
        Some(result("work", label, format!("du {} au {}, week-ends et fériés exclus", long_date(a.min(b)), long_date(a.max(b)))))
    } else {
        let n = (b - a).num_days().abs();
        Some(result("days", format!("{n} jours"), format!("entre le {} et le {}", long_date(a.min(b)), long_date(a.max(b)))))
    }
}

pub fn query(q: &str) -> Vec<Item> {
    let today = Local::now().date_naive();
    [timezones(q), relative(q, today), day_counts(q, today)].into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(y: i32, m: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(y, m, day).unwrap()
    }

    #[test]
    fn times() {
        assert_eq!(parse_time("15h"), NaiveTime::from_hms_opt(15, 0, 0));
        assert_eq!(parse_time("15h30"), NaiveTime::from_hms_opt(15, 30, 0));
        assert_eq!(parse_time("9:05"), NaiveTime::from_hms_opt(9, 5, 0));
        assert_eq!(parse_time("3pm"), NaiveTime::from_hms_opt(15, 0, 0));
        assert_eq!(parse_time("12 am"), NaiveTime::from_hms_opt(0, 0, 0));
        assert_eq!(parse_time("15"), None);
        assert_eq!(parse_time("25h"), None);
    }

    #[test]
    fn zones_by_city() {
        assert_eq!(zone("New York"), Some(chrono_tz::America::New_York));
        assert_eq!(zone("Séoul"), Some(chrono_tz::Asia::Seoul));
        assert_eq!(zone("asia/tokyo"), Some(chrono_tz::Asia::Tokyo));
        assert_eq!(zone("Atlantide"), None);
    }

    #[test]
    fn conversion_between_cities() {
        let it = timezones("15h Paris en New York").unwrap();
        // 6 hours apart, except during the few weeks when only one side has switched DST.
        assert!(it.title == "09:00 à New York" || it.title == "10:00 à New York", "{}", it.title);
        assert!(it.subtitle.starts_with("15:00 à Paris"));
        let it = timezones("23h paris en tokyo").unwrap();
        assert!(it.title.ends_with("(lendemain)"), "{}", it.title);
        assert!(timezones("heure à Tokyo").unwrap().title.ends_with("à Tokyo"));
        assert!(timezones("firefox en mode sombre").is_none());
    }

    #[test]
    fn relative_dates() {
        let t = d(2026, 1, 31);
        assert_eq!(relative("dans 3 semaines", t).unwrap().title, "samedi 21 février 2026");
        assert_eq!(relative("dans 1 mois", t).unwrap().title, "samedi 28 février 2026");
        assert_eq!(relative("il y a 10 jours", t).unwrap().title, "mercredi 21 janvier 2026");
        assert_eq!(relative("dans 2 ans", t).unwrap().title, "lundi 31 janvier 2028");
    }

    #[test]
    fn easter_and_holidays() {
        assert_eq!(easter(2026), d(2026, 4, 5));
        assert_eq!(easter(2024), d(2024, 3, 31));
        assert!(is_french_holiday(d(2026, 4, 6)), "lundi de Pâques");
        assert!(is_french_holiday(d(2026, 5, 14)), "Ascension");
        assert!(is_french_holiday(d(2026, 5, 25)), "lundi de Pentecôte");
        assert!(!is_french_holiday(d(2026, 5, 15)));
    }

    #[test]
    fn counting_days() {
        // Week of 2026-11-09: Wednesday 11 is a holiday.
        assert_eq!(business_days(d(2026, 11, 9), d(2026, 11, 15)), 4);
        let t = d(2026, 10, 13);
        assert_eq!(day_counts("jours ouvrés entre 09/11/2026 et 15/11/2026", t).unwrap().title, "4 jours ouvrés");
        assert_eq!(day_counts("jours entre 2026-01-01 et 2026-12-31", t).unwrap().title, "364 jours");
        assert_eq!(day_counts("jours ouvres 24/12 26/12", t).unwrap().title, "1 jour ouvré");
    }
}
