//! GitHub and Vercel. Network calls run in the background; the bar shows cached results
//! (or a loading line) and refreshes when the data arrives.

use crate::config::Settings;
use crate::engine::Outcome;
use crate::i18n::{t, tf};
use crate::search::{Action, Item};
use crate::util;
use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

const SERVICE: &str = "landry";
const TTL: Duration = Duration::from_secs(60);

pub const ACCOUNTS: &[(&str, &str)] = &[("github", "GitHub"), ("vercel", "Vercel"), ("translate", "Traduction")];

// ---------- Secrets ----------

fn entry(account: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, account).map_err(|e| tf("trousseau indisponible : {e}", &[("e", &e)]))
}

pub fn token(account: &str) -> Option<String> {
    entry(account).ok()?.get_password().ok().filter(|t| !t.is_empty())
}

pub fn set_token(account: &str, token: &str) -> Result<(), String> {
    if !ACCOUNTS.iter().any(|(a, _)| *a == account) {
        return Err(tf("compte inconnu : {a}", &[("a", &account)]));
    }
    let e = entry(account)?;
    let t = token.trim();
    let r = if t.is_empty() { e.delete_credential() } else { e.set_password(t) };
    clear_cache();
    r.map_err(|e| tf("trousseau : {e}", &[("e", &e)]))
}

// ---------- Background fetch with cache ----------

pub type Fetch = fn(&Settings, &str) -> Result<Vec<Item>, String>;

struct Cached {
    at: Instant,
    items: Vec<Item>,
}

fn cache() -> &'static Mutex<HashMap<String, Cached>> {
    static C: OnceLock<Mutex<HashMap<String, Cached>>> = OnceLock::new();
    C.get_or_init(|| Mutex::new(HashMap::new()))
}

fn in_flight() -> &'static Mutex<Vec<String>> {
    static F: OnceLock<Mutex<Vec<String>>> = OnceLock::new();
    F.get_or_init(|| Mutex::new(Vec::new()))
}

/// Called when fresh data arrives, so the UI searches again.
static ON_UPDATE: OnceLock<Box<dyn Fn() + Send + Sync>> = OnceLock::new();

pub fn on_update(f: impl Fn() + Send + Sync + 'static) {
    let _ = ON_UPDATE.set(Box::new(f));
}

fn clear_cache() {
    cache().lock().unwrap_or_else(|e| e.into_inner()).clear();
}

/// Title and subtitle are French keys, translated here.
fn info(key: &str, title: &str, subtitle: &str) -> Item {
    Item { id: format!("cloud:{}", key.replace('\t', ":")), module: "cloud".into(), title: t(title), subtitle: t(subtitle), ..Default::default() }
}

fn missing_token(name: &str) -> Item {
    Item {
        actions: vec![Action::new("settings", "Ouvrir les réglages")],
        ..info(&format!("{name}:token"), &tf("Jeton {name} manquant", &[("name", &name)]), "Réglages Landry, section Comptes")
    }
}

/// Cached items for `key`, fetching in the background when stale.
pub fn cached(key: &str, settings: &Settings, fetch: Fetch) -> Vec<Item> {
    let fresh = {
        let c = cache().lock().unwrap_or_else(|e| e.into_inner());
        c.get(key).map(|v| (v.at.elapsed() < TTL, v.items.clone()))
    };
    if let Some((true, items)) = fresh {
        return items;
    }
    {
        let mut f = in_flight().lock().unwrap_or_else(|e| e.into_inner());
        if !f.contains(&key.to_string()) {
            f.push(key.to_string());
            let (key, settings) = (key.to_string(), settings.clone());
            std::thread::spawn(move || {
                let items = fetch(&settings, &key).unwrap_or_else(|e| vec![info(&format!("{key}:err"), "Erreur réseau", &e)]);
                cache().lock().unwrap_or_else(|e| e.into_inner()).insert(key.clone(), Cached { at: Instant::now(), items });
                in_flight().lock().unwrap_or_else(|e| e.into_inner()).retain(|k| *k != key);
                if let Some(cb) = ON_UPDATE.get() {
                    cb();
                }
            });
        }
    }
    // Stale data is better than nothing while refreshing.
    fresh.map(|(_, items)| items).unwrap_or_else(|| vec![info(&format!("{key}:loading"), "Chargement…", &key.replace('\t', " · "))])
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent("landry")
        .timeout(Duration::from_secs(15))
        .build()
        .map_err(|e| e.to_string())
}

fn check(r: reqwest::blocking::Response) -> Result<serde_json::Value, String> {
    let status = r.status();
    let body: serde_json::Value = r.json().map_err(|e| e.to_string())?;
    if status.is_success() {
        Ok(body)
    } else {
        let msg = body.get("message").or_else(|| body.pointer("/error/message")).and_then(|m| m.as_str()).unwrap_or("");
        Err(format!("{status} {msg}"))
    }
}

// ---------- GitHub ----------

const PR_QUERY: &str = "query { viewer { pullRequests(first: 20, states: OPEN, orderBy: {field: UPDATED_AT, direction: DESC}) { nodes { title number url isDraft repository { nameWithOwner } commits(last: 1) { nodes { commit { statusCheckRollup { state } } } } } } } }";

fn ci_label(state: Option<&str>) -> String {
    match state {
        Some("SUCCESS") => t("CI réussie"),
        Some("FAILURE") | Some("ERROR") => t("CI en échec"),
        Some("PENDING") | Some("EXPECTED") => t("CI en cours"),
        _ => t("sans CI"),
    }
}

pub fn parse_prs(v: &serde_json::Value) -> Vec<Item> {
    v.pointer("/data/viewer/pullRequests/nodes")
        .and_then(|n| n.as_array())
        .map(|nodes| {
            nodes
                .iter()
                .filter_map(|n| {
                    let url = n.get("url")?.as_str()?.to_string();
                    let repo = n.pointer("/repository/nameWithOwner").and_then(|r| r.as_str()).unwrap_or("");
                    let ci = n.pointer("/commits/nodes/0/commit/statusCheckRollup/state").and_then(|s| s.as_str());
                    let draft = if n.get("isDraft").and_then(|d| d.as_bool()) == Some(true) { format!(" · {}", t("brouillon")) } else { String::new() };
                    Some(Item {
                        id: format!("gh:{url}"),
                        module: "cloud".into(),
                        title: n.get("title")?.as_str()?.to_string(),
                        subtitle: format!("{repo}#{} · {}{draft}", n.get("number")?.as_u64()?, ci_label(ci)),
                        keywords: vec![repo.into(), "pr".into()],
                        actions: vec![Action::new("open", "Ouvrir sur GitHub"), Action::new("copy", "Copier le lien")],
                        data: url,
                        ..Default::default()
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn github_prs(_: &Settings, _: &str) -> Result<Vec<Item>, String> {
    let Some(t) = token("github") else { return Ok(vec![missing_token("GitHub")]) };
    let body = serde_json::json!({ "query": PR_QUERY });
    let v = check(client()?.post("https://api.github.com/graphql").bearer_auth(t).json(&body).send().map_err(|e| e.to_string())?)?;
    let items = parse_prs(&v);
    Ok(if items.is_empty() { vec![info("gh:none", "Aucune pull request ouverte", "")] } else { items })
}

pub fn parse_issues(v: &serde_json::Value) -> Vec<Item> {
    v.get("items")
        .and_then(|i| i.as_array())
        .map(|items| {
            items
                .iter()
                .filter_map(|i| {
                    let url = i.get("html_url")?.as_str()?.to_string();
                    let repo = i.get("repository_url")?.as_str()?.trim_start_matches("https://api.github.com/repos/").to_string();
                    Some(Item {
                        id: format!("gh:{url}"),
                        module: "cloud".into(),
                        title: i.get("title")?.as_str()?.to_string(),
                        subtitle: format!("{repo}#{} · {}", i.get("number")?.as_u64()?, t("issue assignée")),
                        keywords: vec![repo, "issue".into()],
                        actions: vec![Action::new("open", "Ouvrir sur GitHub"), Action::new("copy", "Copier le lien")],
                        data: url,
                        ..Default::default()
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn github_issues(_: &Settings, _: &str) -> Result<Vec<Item>, String> {
    let Some(t) = token("github") else { return Ok(vec![missing_token("GitHub")]) };
    let v = check(
        client()?
            .get("https://api.github.com/search/issues")
            .query(&[("q", "is:issue is:open assignee:@me"), ("per_page", "20"), ("sort", "updated")])
            .bearer_auth(t)
            .send()
            .map_err(|e| e.to_string())?,
    )?;
    let items = parse_issues(&v);
    Ok(if items.is_empty() { vec![info("gh:noissue", "Aucune issue assignée", "")] } else { items })
}

// ---------- Vercel ----------

fn ago(ms: i64) -> String {
    let mins = (chrono::Utc::now().timestamp_millis() - ms).max(0) / 60_000;
    match mins {
        0 => t("à l'instant"),
        m if m < 60 => tf("il y a {n} min", &[("n", &m)]),
        m if m < 60 * 24 => tf("il y a {n} h", &[("n", &(m / 60))]),
        m => tf("il y a {n} j", &[("n", &(m / (60 * 24)))]),
    }
}

fn state_label(s: &str) -> String {
    match s {
        "READY" => t("en ligne"),
        "ERROR" => t("en erreur"),
        "BUILDING" | "INITIALIZING" | "QUEUED" => t("en cours"),
        "CANCELED" => t("annulé"),
        other => other.to_string(),
    }
}

pub fn parse_deployments(v: &serde_json::Value) -> Vec<Item> {
    v.get("deployments")
        .and_then(|d| d.as_array())
        .map(|deps| {
            deps.iter()
                .filter_map(|d| {
                    let uid = d.get("uid")?.as_str()?.to_string();
                    let name = d.get("name")?.as_str()?.to_string();
                    let url = d.get("url").and_then(|u| u.as_str()).unwrap_or("");
                    let state = d.get("state").or_else(|| d.get("readyState")).and_then(|s| s.as_str()).unwrap_or("?");
                    let target = d.get("target").and_then(|t| t.as_str()).unwrap_or("preview");
                    let created = d.get("created").or_else(|| d.get("createdAt")).and_then(|c| c.as_i64()).unwrap_or(0);
                    let inspector = d.get("inspectorUrl").and_then(|u| u.as_str()).unwrap_or("");
                    Some(Item {
                        id: format!("vercel:{uid}"),
                        module: "cloud".into(),
                        title: format!("{name} · {}", state_label(state)),
                        subtitle: format!("{target} · {} · {url}", ago(created)),
                        keywords: vec![name.clone(), "vercel".into(), "deploy".into()],
                        actions: vec![
                            Action::new("open", "Ouvrir le site"),
                            Action::new("logs", "Ouvrir les logs"),
                            Action::new("redeploy", "Redéployer").danger(),
                        ],
                        // uid, name, site URL, inspector URL.
                        data: format!("{uid}\t{name}\thttps://{url}\t{inspector}"),
                        ..Default::default()
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn team_query(settings: &Settings) -> Vec<(&'static str, String)> {
    if settings.vercel_team.trim().is_empty() { vec![] } else { vec![("teamId", settings.vercel_team.trim().to_string())] }
}

fn vercel_deployments(settings: &Settings, _: &str) -> Result<Vec<Item>, String> {
    let Some(t) = token("vercel") else { return Ok(vec![missing_token("Vercel")]) };
    let mut q = team_query(settings);
    q.push(("limit", "20".into()));
    let v = check(client()?.get("https://api.vercel.com/v6/deployments").query(&q).bearer_auth(t).send().map_err(|e| e.to_string())?)?;
    let items = parse_deployments(&v);
    Ok(if items.is_empty() { vec![info("vercel:none", "Aucun déploiement", "")] } else { items })
}

fn redeploy(settings: &Settings, uid: &str, name: &str) -> Result<String, String> {
    let t = token("vercel").ok_or_else(|| t("jeton Vercel manquant"))?;
    let body = serde_json::json!({ "name": name, "deploymentId": uid });
    let v = check(
        client()?
            .post("https://api.vercel.com/v13/deployments")
            .query(&team_query(settings))
            .bearer_auth(t)
            .json(&body)
            .send()
            .map_err(|e| e.to_string())?,
    )?;
    clear_cache();
    Ok(v.get("url").and_then(|u| u.as_str()).unwrap_or("").to_string())
}

// ---------- Module entry points ----------

pub fn query(settings: &Settings, q: &str) -> Vec<Item> {
    let (word, rest) = q.split_once(' ').map(|(a, b)| (a, b.trim().to_lowercase())).unwrap_or((q, String::new()));
    let items = match word.to_lowercase().as_str() {
        "pr" | "prs" | "ci" => cached("github:prs", settings, github_prs),
        "issue" | "issues" => cached("github:issues", settings, github_issues),
        "vercel" | "deploy" | "deploys" => cached("vercel:deployments", settings, vercel_deployments),
        _ => return vec![],
    };
    if rest.is_empty() {
        return items;
    }
    items.into_iter().filter(|i| i.title.to_lowercase().contains(&rest) || i.subtitle.to_lowercase().contains(&rest)).collect()
}

pub fn run(settings: &Settings, item: &Item, action: &str) -> Result<Outcome, String> {
    let field = |n: usize| item.data.split('\t').nth(n).unwrap_or("").to_string();
    Ok(match (item.id.split(':').next().unwrap_or(""), action) {
        (_, "settings") => Outcome::Settings,
        ("gh", "open") => {
            util::open(&item.data)?;
            Outcome::Hide
        }
        ("gh", "copy") => Outcome::Copy(item.data.clone()),
        ("vercel", "open") => {
            util::open(&field(2))?;
            Outcome::Hide
        }
        ("vercel", "logs") => {
            let url = field(3);
            if url.is_empty() {
                return Err(t("pas de lien vers les logs"));
            }
            util::open(&url)?;
            Outcome::Hide
        }
        ("vercel", "redeploy") => Outcome::Message(tf("Redéploiement lancé : {u}", &[("u", &redeploy(settings, &field(0), &field(1))?)])),
        (_, "") => return Err(item.subtitle.clone()),
        (_, a) => return Err(tf("action inconnue : {a}", &[("a", &a)])),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prs_from_graphql() {
        let v = serde_json::json!({"data": {"viewer": {"pullRequests": {"nodes": [
            {"title": "Ajoute les presets", "number": 12, "url": "https://github.com/Landry-16/landry/pull/12", "isDraft": false,
             "repository": {"nameWithOwner": "Landry-16/landry"},
             "commits": {"nodes": [{"commit": {"statusCheckRollup": {"state": "FAILURE"}}}]}},
            {"title": "WIP", "number": 3, "url": "https://github.com/a/b/pull/3", "isDraft": true,
             "repository": {"nameWithOwner": "a/b"}, "commits": {"nodes": [{"commit": {"statusCheckRollup": null}}]}}
        ]}}}});
        let items = parse_prs(&v);
        assert_eq!(items[0].subtitle, "Landry-16/landry#12 · CI en échec");
        assert_eq!(items[1].subtitle, "a/b#3 · sans CI · brouillon");
        assert_eq!(items[0].data, "https://github.com/Landry-16/landry/pull/12");
    }

    #[test]
    fn issues_from_search() {
        let v = serde_json::json!({"items": [{"title": "Bug", "number": 7, "html_url": "https://github.com/a/b/issues/7", "repository_url": "https://api.github.com/repos/a/b"}]});
        assert_eq!(parse_issues(&v)[0].subtitle, "a/b#7 · issue assignée");
    }

    #[test]
    fn deployments() {
        let v = serde_json::json!({"deployments": [{"uid": "dpl_1", "name": "nacre", "url": "nacre-abc.vercel.app", "state": "READY",
            "target": "production", "created": chrono::Utc::now().timestamp_millis() - 3 * 3_600_000, "inspectorUrl": "https://vercel.com/x/nacre/dpl_1"}]});
        let it = &parse_deployments(&v)[0];
        assert_eq!(it.title, "nacre · en ligne");
        assert!(it.subtitle.starts_with("production · il y a 3 h"));
        assert_eq!(it.data, "dpl_1\tnacre\thttps://nacre-abc.vercel.app\thttps://vercel.com/x/nacre/dpl_1");
    }

    #[test]
    fn unknown_account_is_rejected() {
        assert!(set_token("gitlab", "x").is_err());
    }
}
