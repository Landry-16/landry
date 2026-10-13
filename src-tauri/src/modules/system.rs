//! Busy ports, SSH hosts and Docker containers.

use crate::config::Settings;
use crate::engine::Outcome;
use crate::search::{Action, Item};
use crate::util;

#[derive(Debug, PartialEq)]
pub struct Listener {
    pub port: u16,
    pub addr: String,
    pub process: String,
    pub pid: Option<u32>,
}

/// Parse `ss -ltnpH` lines.
pub fn parse_ss(out: &str) -> Vec<Listener> {
    let proc_re = regex::Regex::new(r#"\(\("([^"]+)",pid=(\d+)"#).expect("regex valide");
    let mut v: Vec<Listener> = out
        .lines()
        .filter_map(|l| {
            let cols: Vec<&str> = l.split_whitespace().collect();
            let local = cols.get(3)?;
            let (addr, port) = local.rsplit_once(':')?;
            let caps = proc_re.captures(l);
            Some(Listener {
                port: port.parse().ok()?,
                addr: addr.trim_matches(['[', ']']).to_string(),
                process: caps.as_ref().map(|c| c[1].to_string()).unwrap_or_else(|| "?".into()),
                pid: caps.and_then(|c| c[2].parse().ok()),
            })
        })
        .collect();
    v.sort_by_key(|l| (l.port, l.addr.clone()));
    v.dedup_by_key(|l| (l.port, l.pid));
    v
}

fn ports(filter: &str) -> Vec<Item> {
    let out = match util::output("ss", &["-ltnpH"], None) {
        Ok(o) => o,
        Err(e) => return vec![info("port:err", "Ports illisibles", &e)],
    };
    parse_ss(&out)
        .into_iter()
        .filter(|l| filter.is_empty() || l.port.to_string().starts_with(filter) || l.process.contains(filter))
        .map(|l| {
            let mut actions = vec![Action::new("open-port", "Ouvrir dans le navigateur")];
            if l.pid.is_some() {
                actions.push(Action::new("kill", "Arrêter le processus").danger());
                actions.push(Action::new("kill9", "Forcer l'arrêt (SIGKILL)").danger());
            }
            Item {
                id: format!("port:{}:{}", l.port, l.addr),
                module: "system".into(),
                title: format!("Port {} · {}", l.port, l.process),
                subtitle: match l.pid {
                    Some(p) => format!("pid {p} · écoute sur {}", l.addr),
                    None => format!("écoute sur {} (processus d'un autre utilisateur)", l.addr),
                },
                actions,
                data: format!("{}\t{}", l.port, l.pid.map(|p| p.to_string()).unwrap_or_default()),
                ..Default::default()
            }
        })
        .collect()
}

/// Concrete `Host` aliases from an ssh config (wildcards skipped).
pub fn parse_ssh_hosts(config: &str) -> Vec<String> {
    let mut v = Vec::new();
    for line in config.lines() {
        let l = line.split('#').next().unwrap_or("").trim();
        let mut words = l.split_whitespace();
        if words.next().is_some_and(|w| w.eq_ignore_ascii_case("host")) {
            for h in words {
                if !h.contains(['*', '?', '!']) && !v.contains(&h.to_string()) {
                    v.push(h.to_string());
                }
            }
        }
    }
    v
}

/// SSH hosts are indexed (fuzzy-searchable without a keyword).
pub fn ssh_items() -> Vec<Item> {
    let cfg = std::fs::read_to_string(crate::paths::expand("~/.ssh/config")).unwrap_or_default();
    parse_ssh_hosts(&cfg)
        .into_iter()
        .map(|h| Item {
            id: format!("ssh:{h}"),
            module: "system".into(),
            title: format!("ssh {h}"),
            subtitle: "Se connecter dans un terminal".into(),
            keywords: vec![h.clone(), "ssh".into(), "serveur".into()],
            actions: vec![Action::new("ssh", "Se connecter"), Action::new("copy", "Copier la commande")],
            data: h,
            ..Default::default()
        })
        .collect()
}

fn docker(filter: &str) -> Vec<Item> {
    let out = match util::output("docker", &["ps", "-a", "--format", "{{json .}}"], None) {
        Ok(o) => o,
        Err(e) => return vec![info("docker:err", "Docker indisponible", &e)],
    };
    let items: Vec<Item> = out
        .lines()
        .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
        .filter_map(|c| {
            let name = c.get("Names")?.as_str()?.to_string();
            let image = c.get("Image").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let state = c.get("State").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let status = c.get("Status").and_then(|v| v.as_str()).unwrap_or("").to_string();
            if !filter.is_empty() && !name.contains(filter) && !image.contains(filter) {
                return None;
            }
            let running = state == "running";
            let mut actions = if running {
                vec![Action::new("stop", "Arrêter"), Action::new("restart", "Redémarrer")]
            } else {
                vec![Action::new("start", "Démarrer")]
            };
            actions.push(Action::new("logs", "Voir les logs"));
            if running {
                actions.push(Action::new("exec", "Ouvrir un shell"));
            }
            Some(Item {
                id: format!("docker:{name}"),
                module: "system".into(),
                title: format!("{} {name}", if running { "●" } else { "○" }),
                subtitle: format!("{image} · {status}"),
                actions,
                data: name,
                ..Default::default()
            })
        })
        .collect();
    if items.is_empty() {
        vec![info("docker:none", "Aucun conteneur", "docker ps -a ne renvoie rien")]
    } else {
        items
    }
}

fn info(key: &str, title: &str, subtitle: &str) -> Item {
    Item { id: format!("sys:{key}"), module: "system".into(), title: title.into(), subtitle: subtitle.into(), ..Default::default() }
}

pub fn query(_settings: &Settings, q: &str) -> Vec<Item> {
    let (word, rest) = q.split_once(' ').map(|(a, b)| (a, b.trim())).unwrap_or((q, ""));
    match word.to_lowercase().as_str() {
        "port" | "ports" => ports(rest),
        "docker" => docker(rest),
        _ => vec![],
    }
}

fn home() -> String {
    crate::paths::expand("~").display().to_string()
}

pub fn run(settings: &Settings, item: &Item, action: &str, _arg: &str) -> Result<Outcome, String> {
    let term = |cmd: &str| util::run_template(&settings.terminal_run, &[("dir", &home()), ("cmd", cmd)], None);
    Ok(match action {
        "open-port" => {
            let port = item.data.split('\t').next().unwrap_or("");
            util::open(&format!("http://localhost:{port}"))?;
            Outcome::Hide
        }
        "kill" | "kill9" => {
            let pid = item.data.split('\t').nth(1).filter(|p| !p.is_empty()).ok_or("processus inconnu")?;
            let sig = if action == "kill9" { "-KILL" } else { "-TERM" };
            util::output("kill", &[sig, pid], None)?;
            Outcome::Message(format!("Signal envoyé au processus {pid}"))
        }
        "ssh" => {
            // `--` keeps a host name from being read as an ssh option.
            term(&format!("ssh -- {}; exec bash", shell_quote(&item.data)))?;
            Outcome::Hide
        }
        "copy" => Outcome::Copy(format!("ssh {}", item.data)),
        "start" | "stop" | "restart" => {
            util::output("docker", &[action, &item.data], None)?;
            Outcome::Message(format!("docker {action} {} : fait", item.data))
        }
        "logs" => {
            term(&format!("docker logs -f --tail 200 {}; exec bash", shell_quote(&item.data)))?;
            Outcome::Hide
        }
        "exec" => {
            term(&format!("docker exec -it {} sh; exec bash", shell_quote(&item.data)))?;
            Outcome::Hide
        }
        "" => return Err(item.subtitle.clone()),
        a => return Err(format!("action inconnue : {a}")),
    })
}

/// Single-quote for bash; the terminal template runs `bash -lc <cmd>`.
pub fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ss_output() {
        let out = "LISTEN 0      511        127.0.0.1:5173      0.0.0.0:*    users:((\"node\",pid=4242,fd=23))\nLISTEN 0      4096           [::]:22            [::]:*\nLISTEN 0      511            [::1]:5173         [::]:*    users:((\"node\",pid=4242,fd=24))\n";
        let v = parse_ss(out);
        assert_eq!(v[0], Listener { port: 22, addr: "::".into(), process: "?".into(), pid: None });
        assert_eq!(v[1].port, 5173);
        assert_eq!(v[1].pid, Some(4242));
        assert_eq!(v.len(), 2, "même port et même pid : une seule entrée");
    }

    #[test]
    fn ssh_config() {
        let cfg = "Host *\n  ForwardAgent no\nHost prod staging\n  HostName 1.2.3.4\nhost dev-box # perso\nHost !bad web-?\n";
        assert_eq!(parse_ssh_hosts(cfg), vec!["prod", "staging", "dev-box"]);
    }

    #[test]
    fn quoting() {
        assert_eq!(shell_quote("a'b"), r"'a'\''b'");
    }
}
