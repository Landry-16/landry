//! Slash commands. "/" lists them all, "/spotify daft punk" acts at once.
//! Any other "/word text" is the plain query "word text", so existing keywords ("/note", "/minuteur",
//! "/g") work with a slash without being declared twice.

use crate::config::Settings;
use crate::engine::Outcome;
use crate::i18n::{t, tf};
use crate::search::{fold, Action, Item};
use crate::util;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};

pub enum Slash {
    Items(Vec<Item>),
    /// Search this query instead (the slash dropped).
    Rewrite(String),
}

struct Cmd {
    name: &'static str,
    aliases: &'static [&'static str],
    usage: &'static str,
    desc: &'static str,
}

const NATIVE: &[Cmd] = &[
    Cmd { name: "spotify", aliases: &[], usage: "/spotify <titre, artiste ou playlist>", desc: "Chercher et écouter dans Spotify" },
    Cmd { name: "media", aliases: &["musique", "lecture"], usage: "/media [pause | suivant | précédent]", desc: "Morceau en cours, tous lecteurs" },
    Cmd { name: "volume", aliases: &["vol", "son"], usage: "/volume <0-100 | +10 | -10 | muet>", desc: "Régler le volume" },
    Cmd { name: "muet", aliases: &["mute"], usage: "/muet", desc: "Couper ou rétablir le son" },
    Cmd { name: "luminosite", aliases: &["brightness"], usage: "/luminosite <0-100>", desc: "Luminosité de l'écran" },
    Cmd { name: "sombre", aliases: &["dark"], usage: "/sombre", desc: "Thème sombre du système" },
    Cmd { name: "clair", aliases: &["light"], usage: "/clair", desc: "Thème clair du système" },
    Cmd { name: "dnd", aliases: &["deranger", "nepasderanger"], usage: "/dnd", desc: "Ne pas déranger : cacher les bannières de notification" },
    Cmd { name: "wifi", aliases: &[], usage: "/wifi [on | off]", desc: "Activer ou couper le wifi" },
    Cmd { name: "verrouiller", aliases: &["lock"], usage: "/verrouiller", desc: "Verrouiller la session" },
    Cmd { name: "veille", aliases: &["suspend"], usage: "/veille", desc: "Mettre en veille" },
    Cmd { name: "eteindre", aliases: &["shutdown"], usage: "/eteindre", desc: "Éteindre (GNOME demande confirmation)" },
    Cmd { name: "redemarrer", aliases: &["reboot"], usage: "/redemarrer", desc: "Redémarrer (GNOME demande confirmation)" },
    Cmd { name: "deconnexion", aliases: &["logout"], usage: "/deconnexion", desc: "Fermer la session (GNOME demande confirmation)" },
    Cmd { name: "reglages", aliases: &["parametres"], usage: "/reglages <panneau>", desc: "Ouvrir un panneau des paramètres GNOME" },
    Cmd { name: "tuer", aliases: &["kill", "processus"], usage: "/tuer <processus>", desc: "Trouver et arrêter un processus" },
    Cmd { name: "meteo", aliases: &["weather"], usage: "/meteo [ville]", desc: "Météo du jour et des deux suivants" },
    Cmd { name: "fenetres", aliases: &["fenetre", "windows"], usage: "/fenetres [texte]", desc: "Passer à une fenêtre ouverte" },
    Cmd { name: "presse-papiers", aliases: &["clip", "historique", "pressepapiers"], usage: "/presse-papiers [texte]", desc: "Historique du presse-papiers" },
    Cmd { name: "aide", aliases: &["help"], usage: "/aide", desc: "Raccourcis clavier et exemples" },
    Cmd { name: "bienvenue", aliases: &["setup", "assistant"], usage: "/bienvenue", desc: "Raccourci, démarrage, apparence : l'assistant de premier lancement" },
];

/// Keywords of other modules, listed in the palette. "/word text" runs as "word text".
const KEYWORDS: &[(&str, &str, &str)] = &[
    ("note", "/note <texte>", "Créer une note"),
    ("todo", "/todo <tâche> [échéance]", "Ajouter une tâche"),
    ("minuteur", "/minuteur <durée> [libellé]", "Lancer un minuteur"),
    ("chrono", "/chrono", "Lancer un chronomètre"),
    ("pomodoro", "/pomodoro", "25 min de travail, 5 min de pause, en boucle"),
    ("suivi", "/suivi <client>", "Suivre le temps passé"),
    ("tr", "/tr [langue] <texte>", "Traduire"),
    ("qr", "/qr <texte>", "Générer un QR code"),
    ("capture", "/capture", "Capturer une zone de l'écran"),
    ("ocr", "/ocr", "Lire le texte d'une zone de l'écran"),
    ("pipette", "/pipette", "Prendre une couleur à l'écran"),
    ("port", "/port [numéro]", "Ports occupés"),
    ("ssh", "/ssh", "Hôtes SSH"),
    ("docker", "/docker", "Conteneurs Docker"),
    ("preset", "/preset <nom>", "Changer de preset"),
];

/// GNOME Settings panels: id for gnome-control-center, French name.
const PANELS: &[(&str, &str)] = &[
    ("wifi", "Wi-Fi"),
    ("network", "Réseau"),
    ("bluetooth", "Bluetooth"),
    ("display", "Écrans"),
    ("sound", "Son"),
    ("power", "Énergie"),
    ("keyboard", "Clavier"),
    ("mouse", "Souris et pavé tactile"),
    ("notifications", "Notifications"),
    ("background", "Fond d'écran"),
    ("ubuntu", "Apparence"),
    ("applications", "Applications"),
    ("privacy", "Confidentialité"),
    ("printers", "Imprimantes"),
    ("online-accounts", "Comptes en ligne"),
    ("multitasking", "Multitâche"),
    ("sharing", "Partage"),
    ("system", "Système"),
];

/// What an action does, stored in the item: ["run", message, program, args...] waits for the
/// program, ["spawn", "", program, args...] starts it detached, ["open", target, fallback],
/// ["fill", text], ["view", name].
type Step = Vec<String>;

fn step(kind: &str, parts: &[&str]) -> Step {
    std::iter::once(kind).chain(parts.iter().copied()).map(String::from).collect()
}

/// A status read (player, volume, wifi, processes): never more than 2 s, the bar is waiting.
fn quick(program: &str, args: &[&str]) -> Result<String, String> {
    util::output_within(program, args, None, std::time::Duration::from_secs(2))
}

fn item(id: &str, title: String, subtitle: String, acts: Vec<(Action, Step)>) -> Item {
    let (mut actions, steps): (Vec<Action>, Vec<Step>) = acts.into_iter().unzip();
    actions.iter_mut().enumerate().for_each(|(i, a)| a.id = format!("a{i}"));
    Item {
        id: format!("slash:{id}"),
        module: "slash".into(),
        title,
        subtitle,
        actions,
        data: serde_json::to_string(&steps).unwrap_or_default(),
        ..Default::default()
    }
}

fn info(id: &str, title: String, subtitle: String) -> Item {
    item(id, title, subtitle, vec![])
}

fn native(word: &str) -> Option<&'static Cmd> {
    let w = fold(word);
    NATIVE.iter().find(|c| c.name == w || c.aliases.contains(&w.as_str()))
}

pub fn query(settings: &Settings, rest: &str) -> Slash {
    let (word, arg) = rest.split_once(' ').map(|(a, b)| (a, b.trim())).unwrap_or((rest, ""));
    if let Some(cmd) = native(word) {
        return Slash::Items(run_native(settings, cmd.name, arg));
    }
    if arg.is_empty() {
        let p = palette(settings, word);
        if !p.is_empty() || word.is_empty() {
            return Slash::Items(p);
        }
    }
    Slash::Rewrite(format!("{word} {arg}").trim().to_string())
}

/// Every command whose name starts with what was typed. Enter completes it.
fn palette(settings: &Settings, typed: &str) -> Vec<Item> {
    let typed = fold(typed);
    let fill = |name: &str| vec![(Action::new("", "Compléter"), step("fill", &[&format!("/{name} ")]))];
    let mut out: Vec<Item> = NATIVE
        .iter()
        .filter(|c| c.name.starts_with(&typed) || c.aliases.iter().any(|a| a.starts_with(&typed)))
        .map(|c| item(&format!("cmd:{}", c.name), c.usage.into(), t(c.desc), fill(c.name)))
        .collect();
    out.extend(
        KEYWORDS
            .iter()
            .filter(|(k, _, _)| k.starts_with(&typed))
            .map(|(k, usage, desc)| item(&format!("cmd:{k}"), (*usage).into(), t(desc), fill(k))),
    );
    out.extend(settings.search_prefixes.iter().filter(|(k, _)| k.starts_with(&typed)).map(|(k, url)| {
        let host = url::Url::parse(url).ok().and_then(|u| u.host_str().map(|h| h.trim_start_matches("www.").to_string())).unwrap_or_default();
        item(&format!("cmd:{k}"), format!("/{k} <texte>"), tf("Recherche sur {h}", &[("h", &host)]), fill(k))
    }));
    out
}

fn run_native(settings: &Settings, name: &str, arg: &str) -> Vec<Item> {
    match name {
        "spotify" => spotify(arg),
        "media" => media(None, arg),
        "volume" => volume(arg),
        "muet" => volume("muet"),
        "luminosite" => brightness(arg),
        "sombre" => vec![scheme(true)],
        "clair" => vec![scheme(false)],
        "dnd" => vec![dnd()],
        "wifi" => vec![wifi(arg)],
        "verrouiller" => vec![item("lock", t("Verrouiller la session"), t("Écran de verrouillage de GNOME"), vec![(Action::new("", "Verrouiller"), step("spawn", &["", "loginctl", "lock-session"]))])],
        // Suspend acts at once, hence the confirmation. GNOME asks itself for the three others.
        "veille" => vec![item("suspend", t("Mettre en veille"), t("Entrée deux fois pour confirmer"), vec![(Action::new("", "Mettre en veille").danger(), step("spawn", &["", "systemctl", "suspend"]))])],
        "eteindre" => vec![session("power-off", "Éteindre", "--power-off")],
        "redemarrer" => vec![session("reboot", "Redémarrer", "--reboot")],
        "deconnexion" => vec![session("logout", "Fermer la session", "--logout")],
        "reglages" => panels(arg),
        "tuer" => processes(arg),
        "meteo" => crate::modules::cloud::cached(&format!("meteo\t{arg}"), settings, weather),
        "bienvenue" => vec![item("setup", t("Assistant de premier lancement"), t("Raccourci, démarrage, apparence, outils"), vec![(Action::new("", "Ouvrir"), step("view", &["setup"]))])],
        "fenetres" => crate::modules::shell::window_items(arg),
        "presse-papiers" => crate::modules::shell::clipboard_items(arg),
        "aide" => vec![item("help", t("Aide"), t("Raccourcis clavier et exemples"), vec![(Action::new("", "Ouvrir"), step("view", &["help"]))])],
        _ => vec![],
    }
}

fn session(id: &str, title: &str, flag: &str) -> Item {
    item(id, t(title), t("GNOME demande confirmation"), vec![(Action::new("", title), step("spawn", &["", "gnome-session-quit", flag]))])
}

// ---------- Music ----------

fn spotify(arg: &str) -> Vec<Item> {
    let a = fold(arg);
    if arg.is_empty() || matches!(a.as_str(), "pause" | "play" | "lecture" | "suivant" | "next" | "precedent" | "prev") {
        let mut out = media(Some("spotify"), arg);
        if arg.is_empty() {
            out.push(info("spotify:hint", "/spotify <titre, artiste ou playlist>".into(), t("Ouvre la recherche dans l'application Spotify")));
        }
        return out;
    }
    let enc = utf8_percent_encode(arg, NON_ALPHANUMERIC).to_string();
    let web = format!("https://open.spotify.com/search/{enc}");
    vec![
        item(
            "spotify:search",
            tf("Chercher « {q} » dans Spotify", &[("q", &arg)]),
            t("Titres, artistes, albums et playlists"),
            vec![
                (Action::new("", "Ouvrir dans Spotify"), step("open", &[&format!("spotify:search:{arg}"), &web])),
                (Action::new("", "Ouvrir dans le navigateur"), step("open", &[&web, ""])),
                (Action::new("", "Seulement les playlists"), step("open", &[&format!("{web}/playlists"), ""])),
            ],
        ),
        item(
            "spotify:playlist",
            tf("Playlists « {q} »", &[("q", &arg)]),
            t("Dans le navigateur, filtré sur les playlists"),
            vec![(Action::new("", "Ouvrir"), step("open", &[&format!("{web}/playlists"), ""]))],
        ),
    ]
}

/// `playerctl` drives any MPRIS player (Spotify, Firefox, Rhythmbox...).
fn media(player: Option<&str>, arg: &str) -> Vec<Item> {
    let base: Vec<&str> = player.map(|p| vec!["-p", p]).unwrap_or_default();
    let ctl = |cmd: &str, msg: &str| -> Step {
        let mut s = step("run", &[msg, "playerctl"]);
        s.extend(base.iter().chain([&cmd]).map(|x| x.to_string()));
        s
    };
    let id = player.unwrap_or("media");
    let control = |verb: &str| -> Option<(Action, Step)> {
        Some(match verb {
            "pause" | "play" | "lecture" => (Action::new("", "Lecture ou pause"), ctl("play-pause", &t("Lecture ou pause"))),
            "suivant" | "next" => (Action::new("", "Morceau suivant"), ctl("next", &t("Morceau suivant"))),
            "precedent" | "prev" => (Action::new("", "Morceau précédent"), ctl("previous", &t("Morceau précédent"))),
            _ => return None,
        })
    };
    if let Some(a) = control(&fold(arg)) {
        let title = a.0.title.clone();
        return vec![item(&format!("{id}:ctl"), t(&title), player.map(String::from).unwrap_or_else(|| t("Lecteur actif")), vec![a])];
    }
    let mut meta = base.clone();
    meta.extend(["metadata", "--format", "{{status}}\t{{artist}}\t{{title}}"]);
    match quick("playerctl", &meta) {
        Ok(out) => {
            let mut f = out.trim().splitn(3, '\t');
            let (status, artist, title) = (f.next().unwrap_or(""), f.next().unwrap_or(""), f.next().unwrap_or(""));
            let state = if status == "Playing" { t("En lecture") } else { t("En pause") };
            vec![item(
                &format!("{id}:now"),
                if title.is_empty() { t("Morceau en cours") } else { title.to_string() },
                format!("{artist} · {state}"),
                ["pause", "suivant", "precedent"].iter().filter_map(|v| control(v)).collect(),
            )]
        }
        Err(e) if e.contains("No such file") || e.contains("os error 2") => {
            vec![info(&format!("{id}:missing"), t("playerctl manquant"), t("Installe-le : sudo apt install playerctl"))]
        }
        Err(_) => vec![info(&format!("{id}:none"), t("Aucun lecteur en cours"), t("Lance une musique, puis reviens ici"))],
    }
}

// ---------- Sound, screen, theme ----------

/// "Volume: 0.40 [MUTED]" from `wpctl get-volume` -> (40, muted).
pub fn parse_volume(s: &str) -> Option<(u32, bool)> {
    let v: f64 = s.split_whitespace().nth(1)?.parse().ok()?;
    Some(((v * 100.0).round() as u32, s.contains("MUTED")))
}

fn volume(arg: &str) -> Vec<Item> {
    const SINK: &str = "@DEFAULT_AUDIO_SINK@";
    let set = |value: &str, msg: String| step("run", &[&msg, "wpctl", "set-volume", "-l", "1.0", SINK, value]);
    let mute = step("run", &[&t("Son coupé ou rétabli"), "wpctl", "set-mute", SINK, "toggle"]);
    let a = fold(arg);
    if matches!(a.as_str(), "muet" | "mute" | "0ff") {
        return vec![item("volume:mute", t("Couper ou rétablir le son"), String::new(), vec![(Action::new("", "Basculer"), mute)])];
    }
    let target = if let Some(n) = a.strip_prefix('+').and_then(|n| n.parse::<u32>().ok()) {
        Some((format!("{}%+", n.min(100)), tf("Volume +{n} %", &[("n", &n)])))
    } else if let Some(n) = a.strip_prefix('-').and_then(|n| n.parse::<u32>().ok()) {
        Some((format!("{}%-", n.min(100)), tf("Volume -{n} %", &[("n", &n)])))
    } else {
        a.trim_end_matches('%').parse::<u32>().ok().map(|n| (format!("{}%", n.min(100)), tf("Volume à {n} %", &[("n", &n.min(100))])))
    };
    if let Some((value, label)) = target {
        return vec![item("volume:set", label.clone(), String::new(), vec![(Action::new("", "Appliquer"), set(&value, label))])];
    }
    let current = quick("wpctl", &["get-volume", SINK]).ok().and_then(|s| parse_volume(&s));
    let subtitle = match current {
        Some((v, true)) => tf("{v} %, coupé", &[("v", &v)]),
        Some((v, false)) => tf("{v} %", &[("v", &v)]),
        None => t("wpctl introuvable (PipeWire)"),
    };
    vec![item(
        "volume:now",
        t("Volume"),
        subtitle,
        vec![
            (Action::new("", "Monter de 10 %"), set("10%+", tf("Volume +{n} %", &[("n", &10)]))),
            (Action::new("", "Baisser de 10 %"), set("10%-", tf("Volume -{n} %", &[("n", &10)]))),
            (Action::new("", "Couper ou rétablir le son"), mute),
        ],
    )]
}

fn brightness(arg: &str) -> Vec<Item> {
    let Ok(n) = arg.trim_end_matches('%').parse::<u32>() else {
        return vec![info("brightness:hint", "/luminosite <0-100>".into(), t("Écrans de portable seulement"))];
    };
    let n = n.clamp(1, 100);
    let msg = tf("Luminosité à {n} %", &[("n", &n)]);
    let value = format!("<int32 {n}>");
    vec![item(
        "brightness:set",
        msg.clone(),
        t("Écrans de portable seulement"),
        vec![(
            Action::new("", "Appliquer"),
            step(
                "run",
                &[
                    &msg,
                    "gdbus",
                    "call",
                    "--session",
                    "--dest",
                    "org.gnome.SettingsDaemon.Power",
                    "--object-path",
                    "/org/gnome/SettingsDaemon/Power",
                    "--method",
                    "org.freedesktop.DBus.Properties.Set",
                    "org.gnome.SettingsDaemon.Power.Screen",
                    "Brightness",
                    &value,
                ],
            ),
        )],
    )]
}

fn scheme(dark: bool) -> Item {
    let (value, msg) = if dark { ("prefer-dark", t("Thème sombre activé")) } else { ("default", t("Thème clair activé")) };
    item(
        if dark { "scheme:dark" } else { "scheme:light" },
        if dark { t("Thème sombre") } else { t("Thème clair") },
        t("Pour tout le système"),
        vec![(Action::new("", "Appliquer"), step("run", &[&msg, "gsettings", "set", "org.gnome.desktop.interface", "color-scheme", value]))],
    )
}

fn dnd() -> Item {
    let banners = quick("gsettings", &["get", "org.gnome.desktop.notifications", "show-banners"]).map(|s| s.trim() == "true").unwrap_or(true);
    let (title, value, msg) = if banners {
        (t("Activer ne pas déranger"), "false", t("Ne pas déranger activé"))
    } else {
        (t("Désactiver ne pas déranger"), "true", t("Notifications rétablies"))
    };
    item("dnd", title, if banners { t("Notifications affichées") } else { t("Ne pas déranger est actif") }, vec![(Action::new("", "Basculer"), step("run", &[&msg, "gsettings", "set", "org.gnome.desktop.notifications", "show-banners", value]))])
}

fn wifi(arg: &str) -> Item {
    let on = quick("nmcli", &["radio", "wifi"]).map(|s| s.trim() == "enabled").ok();
    let want = match fold(arg).as_str() {
        "on" | "oui" => true,
        "off" | "non" => false,
        _ => !on.unwrap_or(false),
    };
    let msg = if want { t("Wifi activé") } else { t("Wifi coupé") };
    let state = match on {
        Some(true) => t("Actuellement activé"),
        Some(false) => t("Actuellement coupé"),
        None => t("nmcli introuvable (NetworkManager)"),
    };
    item("wifi", if want { t("Activer le wifi") } else { t("Couper le wifi") }, state, vec![(Action::new("", "Appliquer"), step("run", &[&msg, "nmcli", "radio", "wifi", if want { "on" } else { "off" }]))])
}

fn panels(arg: &str) -> Vec<Item> {
    let a = fold(arg);
    PANELS
        .iter()
        .filter(|(id, name)| a.is_empty() || fold(&t(name)).contains(&a) || id.contains(&a))
        .map(|(id, name)| {
            item(&format!("panel:{id}"), tf("Paramètres : {p}", &[("p", &t(name))]), "gnome-control-center".into(), vec![(Action::new("", "Ouvrir"), step("spawn", &["", "gnome-control-center", id]))])
        })
        .collect()
}

// ---------- Processes ----------

/// `ps -eo pid=,comm=,%cpu=,rss=` lines -> (pid, name, cpu, rss in KiB).
pub fn parse_ps(out: &str) -> Vec<(u32, String, f32, u64)> {
    out.lines()
        .filter_map(|l| {
            let mut f = l.split_whitespace();
            let pid = f.next()?.parse().ok()?;
            let rest: Vec<&str> = f.collect();
            let (rss, rest) = rest.split_last()?;
            let (cpu, name) = rest.split_last()?;
            Some((pid, name.join(" "), cpu.parse().ok()?, rss.parse().ok()?))
        })
        .collect()
}

fn processes(arg: &str) -> Vec<Item> {
    if arg.is_empty() {
        return vec![info("kill:hint", "/tuer <processus>".into(), t("Les plus gourmands en premier"))];
    }
    let out = match quick("ps", &["-eo", "pid=,comm=,%cpu=,rss=", "--sort=-%cpu"]) {
        Ok(o) => o,
        Err(e) => return vec![info("kill:err", t("Processus illisibles"), e)],
    };
    let me = std::process::id();
    let a = fold(arg);
    parse_ps(&out)
        .into_iter()
        .filter(|(pid, name, _, _)| *pid != me && fold(name).contains(&a))
        .take(8)
        .map(|(pid, name, cpu, rss)| {
            let p = pid.to_string();
            item(
                &format!("kill:{pid}"),
                format!("{name} · {pid}"),
                tf("CPU {c} % · {m} Mo", &[("c", &format!("{cpu:.0}")), ("m", &(rss / 1024))]),
                vec![
                    (Action::new("", "Arrêter le processus").danger(), step("run", &[&tf("Signal envoyé au processus {pid}", &[("pid", &p)]), "kill", "-TERM", &p])),
                    (Action::new("", "Forcer l'arrêt (SIGKILL)").danger(), step("run", &[&tf("Signal envoyé au processus {pid}", &[("pid", &p)]), "kill", "-KILL", &p])),
                ],
            )
        })
        .collect()
}

// ---------- Weather ----------

/// wttr.in JSON (format j1) -> one item for now and one per day.
pub fn parse_weather(v: &serde_json::Value, place: &str) -> Vec<Item> {
    let s = |v: &serde_json::Value| v.as_str().unwrap_or("").to_string();
    let desc = |v: &serde_json::Value| {
        v.get("lang_fr").and_then(|l| l.get(0)).map(|d| s(&d["value"])).filter(|d| !d.is_empty()).unwrap_or_else(|| s(&v["weatherDesc"][0]["value"]))
    };
    let url = format!("https://wttr.in/{}", utf8_percent_encode(place, NON_ALPHANUMERIC));
    let open = || vec![(Action::new("", "Ouvrir la prévision complète"), step("open", &[&url, ""]))];
    let mut out = Vec::new();
    let area = v["nearest_area"][0]["areaName"][0]["value"].as_str().unwrap_or(place).to_string();
    if let Some(c) = v["current_condition"].get(0) {
        out.push(item(
            "meteo:now",
            tf("{p} : {t} °C, {d}", &[("p", &area), ("t", &s(&c["temp_C"])), ("d", &desc(c))]),
            tf("Ressenti {f} °C · humidité {h} % · vent {w} km/h", &[("f", &s(&c["FeelsLikeC"])), ("h", &s(&c["humidity"])), ("w", &s(&c["windspeedKmph"]))]),
            open(),
        ));
    }
    for (i, d) in v["weather"].as_array().into_iter().flatten().take(3).enumerate() {
        let noon = &d["hourly"][4];
        out.push(item(
            &format!("meteo:day{i}"),
            tf("{day} : {min} à {max} °C", &[("day", &s(&d["date"])), ("min", &s(&d["mintempC"])), ("max", &s(&d["maxtempC"]))]),
            desc(noon),
            open(),
        ));
    }
    out
}

fn weather(settings: &Settings, key: &str) -> Result<Vec<Item>, String> {
    let place = key.split('\t').nth(1).unwrap_or("");
    let lang = if settings.locale == "fr" { "fr" } else { "en" };
    let url = format!("https://wttr.in/{}?format=j1&lang={lang}", utf8_percent_encode(place, NON_ALPHANUMERIC));
    let v: serde_json::Value = crate::modules::cloud::client()?.get(url).send().and_then(|r| r.error_for_status()).and_then(|r| r.json()).map_err(|e| e.to_string())?;
    Ok(parse_weather(&v, place))
}

// ---------- Run ----------

pub fn run(item: &Item, action: &str) -> Result<Outcome, String> {
    let steps: Vec<Step> = serde_json::from_str(&item.data).map_err(|e| e.to_string())?;
    let i: usize = action.strip_prefix('a').and_then(|n| n.parse().ok()).unwrap_or(0);
    let s = steps.get(i).ok_or_else(|| tf("action inconnue : {a}", &[("a", &action)]))?;
    let arg = |n: usize| s.get(n).map(String::as_str).unwrap_or("");
    match arg(0) {
        "run" => {
            let args: Vec<&str> = s[3..].iter().map(String::as_str).collect();
            util::output_within(arg(2), &args, None, std::time::Duration::from_secs(10))?;
            Ok(if arg(1).is_empty() { Outcome::Hide } else { Outcome::Message(arg(1).to_string()) })
        }
        "spawn" => {
            util::spawn(arg(2), &s[3..], None)?;
            Ok(Outcome::Hide)
        }
        "open" => {
            // gio exits non-zero when no application handles the scheme (Spotify not installed).
            if util::output_within("gio", &["open", arg(1)], None, std::time::Duration::from_secs(10)).is_err() {
                if arg(2).is_empty() {
                    return Err(tf("impossible d'ouvrir {u}", &[("u", &arg(1))]));
                }
                util::open(arg(2))?;
            }
            Ok(Outcome::Hide)
        }
        "fill" => Ok(Outcome::Fill(arg(1).to_string())),
        "view" => Ok(Outcome::View(arg(1).to_string())),
        k => Err(tf("action inconnue : {a}", &[("a", &k)])),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items(q: &str) -> Vec<Item> {
        match query(&Settings::default(), q) {
            Slash::Items(v) => v,
            Slash::Rewrite(r) => panic!("réécrit en {r}"),
        }
    }

    #[test]
    fn palette_lists_and_filters() {
        let all = items("");
        assert!(all.len() > NATIVE.len() + KEYWORDS.len(), "commandes natives, mots-clés et préfixes web");
        let sp = items("sp");
        assert_eq!(sp[0].title, "/spotify <titre, artiste ou playlist>");
        assert_eq!(run(&sp[0], "a0").unwrap(), Outcome::Fill("/spotify ".into()));
    }

    #[test]
    fn keywords_are_rewritten() {
        assert!(matches!(query(&Settings::default(), "minuteur 10 min"), Slash::Rewrite(q) if q == "minuteur 10 min"));
        assert!(matches!(query(&Settings::default(), "gh landry"), Slash::Rewrite(q) if q == "gh landry"));
    }

    #[test]
    fn spotify_search_opens_the_app_then_the_web() {
        let it = &items("spotify daft punk")[0];
        assert_eq!(it.title, "Chercher « daft punk » dans Spotify");
        let steps: Vec<Step> = serde_json::from_str(&it.data).unwrap();
        assert_eq!(steps[0], vec!["open", "spotify:search:daft punk", "https://open.spotify.com/search/daft%20punk"]);
        assert_eq!(it.actions.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), ["a0", "a1", "a2"]);
    }

    #[test]
    fn volume_arguments() {
        let it = &items("volume 140")[0];
        assert_eq!(it.title, "Volume à 100 %");
        assert!(it.data.contains("\"100%\""));
        assert!(items("vol +5")[0].data.contains("5%+"));
        assert_eq!(parse_volume("Volume: 0.40 [MUTED]\n"), Some((40, true)));
        assert_eq!(parse_volume("Volume: 1.00"), Some((100, false)));
    }

    #[test]
    fn suspend_needs_confirmation() {
        assert!(items("veille")[0].actions[0].danger);
    }

    #[test]
    fn ps_and_weather_parsing() {
        assert_eq!(parse_ps(" 42 Web Content  12.5 204800\n"), vec![(42, "Web Content".into(), 12.5, 204800)]);
        let v: serde_json::Value = serde_json::from_str(
            r#"{"current_condition":[{"temp_C":"14","FeelsLikeC":"12","humidity":"80","windspeedKmph":"9","lang_fr":[{"value":"Bruine"}]}],
                "nearest_area":[{"areaName":[{"value":"Lyon"}]}],
                "weather":[{"date":"2026-10-13","mintempC":"9","maxtempC":"17","hourly":[{},{},{},{},{"weatherDesc":[{"value":"Sunny"}]}]}]}"#,
        )
        .unwrap();
        let w = parse_weather(&v, "lyon");
        assert_eq!(w[0].title, "Lyon : 14 °C, Bruine");
        assert_eq!(w[1].subtitle, "Sunny");
    }
}
