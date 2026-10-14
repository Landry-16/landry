//! Tasks with reminders, timers (countdown, stopwatch, Pomodoro) and time tracking.
//! Pure parsing and formatting here; storage lives in `usage.rs`, timers in the engine.

use crate::usage::TimeEntry;
use chrono::{DateTime, Datelike, Duration, Local, NaiveDate, NaiveTime, TimeZone, Weekday};

const DAYS: [(&str, Weekday); 7] = [
    ("lundi", Weekday::Mon),
    ("mardi", Weekday::Tue),
    ("mercredi", Weekday::Wed),
    ("jeudi", Weekday::Thu),
    ("vendredi", Weekday::Fri),
    ("samedi", Weekday::Sat),
    ("dimanche", Weekday::Sun),
];

fn at(date: NaiveDate, h: u32, m: u32) -> Option<DateTime<Local>> {
    Local.from_local_datetime(&date.and_time(NaiveTime::from_hms_opt(h, m, 0)?)).earliest()
}

fn hm(c: &regex::Captures, h: usize, m: usize, default: (u32, u32)) -> (u32, u32) {
    match c.get(h) {
        Some(hh) => (hh.as_str().parse().unwrap_or(default.0), c.get(m).and_then(|v| v.as_str().parse().ok()).unwrap_or(0)),
        None => default,
    }
}

/// Split "appeler la boulangerie demain 10h" into the task text and its due time.
pub fn parse_task(input: &str, now: DateTime<Local>) -> (String, Option<DateTime<Local>>) {
    let today = now.date_naive();
    let time = r"(?:\s+(?:à\s+)?(\d{1,2})\s*h\s*(\d{2})?)?";
    let rules: [(&str, usize); 4] = [
        (r"(?i)\bdans\s+(\d+)\s*(min|minutes?|h|heures?|j|jours?)\b", 0),
        (&format!(r"(?i)\b(aujourd'hui|aujourdhui|demain|après-demain|apres-demain|ce soir|ce matin|lundi|mardi|mercredi|jeudi|vendredi|samedi|dimanche){time}"), 1),
        (&format!(r"(?i)\b(?:le\s+)?(\d{{1,2}})/(\d{{1,2}})(?:/(\d{{4}}))?{time}"), 2),
        (r"(?i)(?:^|\s)(?:à\s+)?(\d{1,2})\s*h\s*(\d{2})?\b", 3),
    ];
    for (pattern, kind) in rules {
        let Ok(re) = regex::Regex::new(pattern) else { continue };
        let Some(c) = re.captures(input) else { continue };
        let due = match kind {
            0 => {
                let n: i64 = c[1].parse().unwrap_or(0);
                let unit = c[2].to_lowercase();
                Some(now + if unit.starts_with('m') { Duration::minutes(n) } else if unit.starts_with('h') { Duration::hours(n) } else { Duration::days(n) })
            }
            1 => {
                let word = crate::search::fold(&c[1]);
                let (date, default) = match word.as_str() {
                    "aujourd'hui" | "aujourdhui" => (today, (now.hour_u32() + 1, 0)),
                    "demain" => (today + Duration::days(1), (9, 0)),
                    "apres-demain" => (today + Duration::days(2), (9, 0)),
                    "ce soir" => (today, (19, 0)),
                    "ce matin" => (today, (9, 0)),
                    day => {
                        let wd = DAYS.iter().find(|(n, _)| *n == day).map(|(_, w)| *w).unwrap_or(Weekday::Mon);
                        // Next occurrence, never today.
                        let ahead = (wd.num_days_from_monday() as i64 - today.weekday().num_days_from_monday() as i64).rem_euclid(7);
                        (today + Duration::days(if ahead == 0 { 7 } else { ahead }), (9, 0))
                    }
                };
                let (h, m) = hm(&c, 2, 3, default);
                at(date, h, m)
            }
            2 => {
                let (d, mo): (u32, u32) = (c[1].parse().unwrap_or(0), c[2].parse().unwrap_or(0));
                let year = c.get(3).and_then(|y| y.as_str().parse().ok()).unwrap_or(today.year());
                let date = NaiveDate::from_ymd_opt(year, mo, d);
                // "le 05/01" typed in December means next January.
                let date = match (date, c.get(3)) {
                    (Some(dt), None) if dt < today => NaiveDate::from_ymd_opt(year + 1, mo, d),
                    (dt, _) => dt,
                };
                let (h, m) = hm(&c, 4, 5, (9, 0));
                date.and_then(|dt| at(dt, h, m))
            }
            _ => {
                let (h, m) = hm(&c, 1, 2, (9, 0));
                at(today, h, m).map(|t| if t <= now { t + Duration::days(1) } else { t })
            }
        };
        if let Some(due) = due {
            let text = format!("{}{}", &input[..c.get(0).map_or(0, |m| m.start())], &input[c.get(0).map_or(0, |m| m.end())..]);
            let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
            return (text, Some(due));
        }
    }
    (input.trim().to_string(), None)
}

trait HourU32 {
    fn hour_u32(&self) -> u32;
}

impl HourU32 for DateTime<Local> {
    fn hour_u32(&self) -> u32 {
        use chrono::Timelike;
        self.hour()
    }
}

/// "lundi 14 octobre à 10:00", "demain à 10:00", "aujourd'hui à 18:30".
pub fn when(ts: i64, now: DateTime<Local>) -> String {
    let Some(t) = Local.timestamp_opt(ts, 0).single() else { return String::new() };
    let days = (t.date_naive() - now.date_naive()).num_days();
    let day = match days {
        0 => crate::i18n::t("aujourd'hui"),
        1 => crate::i18n::t("demain"),
        -1 => crate::i18n::t("hier"),
        _ => super::dates::long_date(t.date_naive()),
    };
    crate::i18n::tf("{day} à {h}", &[("day", &day), ("h", &t.format("%H:%M"))])
}

/// "10 min", "5m", "1h30", "90s", "25" (minutes) -> seconds.
pub fn parse_duration(s: &str) -> Option<i64> {
    let s = s.trim().to_lowercase().replace(' ', "");
    if s.is_empty() {
        return None;
    }
    if let Ok(n) = s.parse::<i64>() {
        return (n > 0).then_some(n * 60);
    }
    if let Some(n) = s.strip_suffix('s').and_then(|v| v.parse::<i64>().ok()) {
        return (n > 0).then_some(n);
    }
    let re = regex::Regex::new(r"^(?:(\d+)h)?(?:(\d+)(?:min|mn|m)?)?(?:(\d+)s)?$").ok()?;
    let c = re.captures(&s)?;
    let num = |i: usize| c.get(i).and_then(|v| v.as_str().parse::<i64>().ok()).unwrap_or(0);
    // "1h30": the bare number after hours is minutes.
    let total = num(1) * 3600 + num(2) * 60 + num(3);
    (total > 0).then_some(total)
}

/// 3725 -> "1:02:05", 125 -> "2:05".
pub fn clock(secs: i64) -> String {
    let s = secs.max(0);
    if s >= 3600 { format!("{}:{:02}:{:02}", s / 3600, s % 3600 / 60, s % 60) } else { format!("{}:{:02}", s / 60, s % 60) }
}

/// 5400 -> "1 h 30", 3600 -> "1 h", 600 -> "10 min", 45 -> "45 s".
pub fn human(secs: i64) -> String {
    let s = secs.max(0);
    let m = s / 60;
    match (s, m) {
        (s, _) if s < 60 => format!("{s} s"),
        (_, m) if m >= 60 && m % 60 == 0 => format!("{} h", m / 60),
        (_, m) if m >= 60 => format!("{} h {:02}", m / 60, m % 60),
        (_, m) => format!("{m} min"),
    }
}

fn csv_field(s: &str) -> String {
    if s.contains([',', '"', '\n', ';']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() }
}

/// CSV with one line per entry; running entries count up to `now`.
pub fn tracking_csv(entries: &[TimeEntry], now: i64) -> String {
    let mut out = String::from("client,debut,fin,duree,heures\n");
    for e in entries {
        let end = e.end.unwrap_or(now);
        let fmt = |t: i64| Local.timestamp_opt(t, 0).single().map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default();
        let secs = end - e.start;
        out.push_str(&format!(
            "{},{},{},{},{:.2}\n",
            csv_field(&e.label),
            fmt(e.start),
            if e.end.is_some() { fmt(end) } else { "en cours".into() },
            clock(secs),
            secs as f64 / 3600.0
        ));
    }
    out
}

/// Total per label since `since`, biggest first.
pub fn totals(entries: &[TimeEntry], since: i64, now: i64) -> Vec<(String, i64)> {
    let mut map: std::collections::BTreeMap<String, i64> = Default::default();
    for e in entries.iter().filter(|e| e.start >= since) {
        *map.entry(e.label.clone()).or_default() += e.end.unwrap_or(now) - e.start;
    }
    let mut v: Vec<_> = map.into_iter().collect();
    v.sort_by(|a, b| b.1.cmp(&a.1));
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> DateTime<Local> {
        // Tuesday 13 October 2026, 14:20.
        Local.with_ymd_and_hms(2026, 10, 13, 14, 20, 0).unwrap()
    }

    fn due(s: &str) -> (String, String) {
        let (t, d) = parse_task(s, now());
        (t, d.map(|d| d.format("%Y-%m-%d %H:%M").to_string()).unwrap_or_default())
    }

    #[test]
    fn task_due_dates() {
        assert_eq!(due("appeler la boulangerie demain 10h"), ("appeler la boulangerie".into(), "2026-10-14 10:00".into()));
        assert_eq!(due("payer l'Urssaf vendredi"), ("payer l'Urssaf".into(), "2026-10-16 09:00".into()));
        assert_eq!(due("revue mardi à 8h30"), ("revue".into(), "2026-10-20 08:30".into()), "mardi prochain, pas aujourd'hui");
        assert_eq!(due("facture le 24/10 14h"), ("facture".into(), "2026-10-24 14:00".into()));
        assert_eq!(due("bilan 05/01"), ("bilan".into(), "2027-01-05 09:00".into()));
        assert_eq!(due("pause dans 30 min"), ("pause".into(), "2026-10-13 14:50".into()));
        assert_eq!(due("sport 9h"), ("sport".into(), "2026-10-14 09:00".into()), "heure passée : demain");
        assert_eq!(due("sport à 18h"), ("sport".into(), "2026-10-13 18:00".into()));
        assert_eq!(due("lire un livre"), ("lire un livre".into(), String::new()));
    }

    #[test]
    fn durations() {
        assert_eq!(parse_duration("10 min"), Some(600));
        assert_eq!(parse_duration("5m"), Some(300));
        assert_eq!(parse_duration("1h30"), Some(5400));
        assert_eq!(parse_duration("90s"), Some(90));
        assert_eq!(parse_duration("25"), Some(1500));
        assert_eq!(parse_duration("abc"), None);
        assert_eq!(parse_duration("0"), None);
        assert_eq!(clock(3725), "1:02:05");
        assert_eq!(clock(125), "2:05");
        assert_eq!(human(5400), "1 h 30");
        assert_eq!(human(3600), "1 h");
        assert_eq!(human(45), "45 s");
    }

    #[test]
    fn csv_and_totals() {
        let e = vec![
            TimeEntry { id: 1, label: "Nacre".into(), start: 0, end: Some(5400) },
            TimeEntry { id: 2, label: "Client, \"B\"".into(), start: 6000, end: None },
        ];
        let csv = tracking_csv(&e, 7800);
        let lines: Vec<&str> = csv.lines().collect();
        assert_eq!(lines[0], "client,debut,fin,duree,heures");
        assert!(lines[1].starts_with("Nacre,") && lines[1].ends_with(",1:30:00,1.50"));
        assert!(lines[2].starts_with("\"Client, \"\"B\"\"\",") && lines[2].contains(",en cours,"));
        assert_eq!(totals(&e, 0, 7800), vec![("Nacre".into(), 5400), ("Client, \"B\"".into(), 1800)]);
    }

    #[test]
    fn when_labels() {
        let n = now();
        assert_eq!(when((n + Duration::days(1)).timestamp(), n), format!("demain à {}", (n + Duration::days(1)).format("%H:%M")));
    }
}
