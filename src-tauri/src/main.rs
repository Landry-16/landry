mod apps;
mod backup;
mod config;
mod doctor;
mod engine;
mod i18n;
use i18n::{t, tf};
mod ipc;
mod modules;
mod organizer;
mod paste;
mod paths;
mod search;
mod session;
mod usage;
mod util;

use base64::Engine as _;
use config::{Preset, PresetInfo};
use engine::{Engine, Outcome};
use serde::Serialize;
use std::collections::HashMap;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Mutex, MutexGuard};
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, State, WebviewWindow, WindowEvent};

/// Set by `show_bar`, fired by the frontend once the first frame after showing is painted.
struct Painted(Mutex<Option<mpsc::Sender<()>>>);
struct EngineState(Mutex<Engine>);
static SYSTEM_DARK: AtomicBool = AtomicBool::new(false);

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("daemon") => daemon(),
        Some("doctor") => std::process::exit(doctor::run()),
        _ => std::process::exit(ipc::client(&args)),
    }
}

fn engine(app: &AppHandle) -> MutexGuard<'_, Engine> {
    app.state::<EngineState>().inner().0.lock().unwrap_or_else(|e| e.into_inner())
}

/// Layout used by the last opening, for when the engine is busy.
static LAST_LAYER: Mutex<Option<(config::Layer, u32, u32)>> = Mutex::new(None);

fn bar(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("bar")
}

#[derive(Serialize, Clone)]
struct PresetEvent {
    preset: Preset,
    warning: Option<String>,
}

fn emit_preset(app: &AppHandle) {
    let ev = {
        let e = engine(app);
        PresetEvent { preset: e.preset.clone(), warning: e.warning.clone() }
    };
    if let Some(w) = bar(app) {
        let l = &ev.preset.layout;
        fit_layer(&w, (l.layer, l.width, l.max_height));
    }
    let _ = app.emit("landry://preset", ev);
}

/// Show and focus the bar. Returns a status line with timings and focus state.
fn show_bar(app: &AppHandle, token: String) -> String {
    let Some(w) = bar(app) else { return "err fenêtre introuvable".into() };
    let start = Instant::now();
    let (tx, rx) = mpsc::channel();
    *app.state::<Painted>().0.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);

    // Never wait for the engine here: a long action holding it must not keep the bar from opening.
    let layer = match app.state::<EngineState>().inner().0.try_lock() {
        Ok(e) => {
            let l = (e.preset.layout.layer, e.preset.layout.width, e.preset.layout.max_height);
            *LAST_LAYER.lock().unwrap_or_else(|e| e.into_inner()) = Some(l);
            l
        }
        Err(_) => LAST_LAYER.lock().unwrap_or_else(|e| e.into_inner()).unwrap_or((config::Layer::Screen, 720, 480)),
    };
    let win = w.clone();
    let _ = app.run_on_main_thread(move || {
        fit_layer(&win, layer);
        // On Wayland, an xdg-activation token from the launcher lets mutter grant focus.
        if !token.is_empty() {
            if let Ok(gw) = win.gtk_window() {
                use gtk::prelude::GtkWindowExt;
                gw.set_startup_id(&token);
            }
        }
        let _ = win.show();
        let _ = win.set_focus();
        let _ = win.emit("landry://shown", ());
    });

    let painted = rx.recv_timeout(Duration::from_secs(1)).is_ok();
    // The text selected in the previous app, for the text transformations. Read after the
    // first paint so it never delays the bar; a slow source app only delays this.
    let h = app.clone();
    std::thread::spawn(move || {
        let wanted = engine(&h).settings.read_selection;
        let sel = if wanted { read_selection(&h, true).filter(|s| !s.trim().is_empty() && s.len() < 100_000) } else { None };
        engine(&h).selection = sel;
        // Open windows change all the time: read them at each opening (empty without the extension).
        let wins = modules::shell::windows().unwrap_or_default();
        engine(&h).set_windows(&wins);
    });
    let paint_ms = start.elapsed().as_millis();
    // Focus arrives asynchronously from the compositor; give it a short window.
    let mut focused = false;
    while start.elapsed() < Duration::from_millis(paint_ms as u64 + 300) {
        if w.is_focused().unwrap_or(false) {
            focused = true;
            break;
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    let size = w.inner_size().map(|s| format!("{}x{}", s.width, s.height)).unwrap_or_default();
    let status = format!(
        "ok shown painted={painted} paint_ms={paint_ms} focused={focused} focus_ms={} size={size}",
        start.elapsed().as_millis()
    );
    eprintln!("landry: {status}");
    status
}

/// Wayland clients cannot place their windows. Screen layer: a transparent fullscreen window,
/// the bar placed in CSS. Fullscreen, because a plain window the size of the screen gets pushed
/// aside by the dock and the top bar, which shifts the bar off centre. Window layer: a window
/// the size of the bar, placed by GNOME, faster to map (less surface).
fn fit_layer(w: &WebviewWindow, (layer, width, max_height): (config::Layer, u32, u32)) {
    let screen = layer == config::Layer::Screen;
    if w.is_fullscreen().unwrap_or(!screen) != screen {
        let _ = w.set_fullscreen(screen);
    }
    if screen {
        return;
    }
    let target = tauri::LogicalSize::new(f64::from(width) + 64.0, f64::from(max_height) + 64.0);
    let scale = w.scale_factor().unwrap_or(1.0);
    let current = w.inner_size().map(|s| s.to_logical::<f64>(scale)).ok();
    if current.is_none_or(|c| (c.width - target.width).abs() > 1.0 || (c.height - target.height).abs() > 1.0) {
        let _ = w.set_size(target);
    }
}

fn hide_bar(app: &AppHandle) {
    if let Some(w) = bar(app) {
        let _ = w.hide();
    }
}

/// After an activation: hide the bar, keep it, open settings, or quit.
fn apply_outcome(app: &AppHandle, outcome: &Outcome) {
    if engine(app).quit_requested {
        app.exit(0);
        return;
    }
    match outcome {
        Outcome::Hide => hide_bar(app),
        Outcome::Settings => {
            let _ = app.emit("landry://open-settings", ());
        }
        Outcome::Copy(text) => {
            set_clipboard(app, text.clone());
            hide_bar(app);
        }
        Outcome::Pick => {
            hide_bar(app);
            let h = app.clone();
            std::thread::spawn(move || pick_color(&h));
        }
        Outcome::Paste { text, back } => {
            set_clipboard(app, text.clone());
            hide_bar(app);
            let back = *back;
            std::thread::spawn(move || {
                // Let the previous window get the focus back before typing into it.
                std::thread::sleep(Duration::from_millis(150));
                if let Err(e) = tauri::async_runtime::block_on(paste::send(back)) {
                    eprintln!("landry: collage automatique impossible : {e}");
                    notify("Copié dans le presse-papiers", "Colle avec Ctrl+V (collage automatique refusé ou indisponible).");
                }
            });
        }
        Outcome::Capture(kind) => {
            hide_bar(app);
            let (h, kind) = (app.clone(), kind.clone());
            std::thread::spawn(move || capture(&h, &kind));
        }
        Outcome::Run { program, args, label, silent, limit } => {
            if *silent {
                hide_bar(app);
            }
            let (h, program, args, label, silent, limit) = (app.clone(), program.clone(), args.clone(), label.clone(), *silent, *limit);
            std::thread::spawn(move || run_script(&h, &program, &args, &label, silent, limit));
        }
        // Shown or applied by the frontend.
        Outcome::Keep | Outcome::Message(_) | Outcome::Fill(_) | Outcome::View(_) | Outcome::Ask(_) => {}
    }
}

/// Summary and body may be French keys: translated here.
fn notify(summary: &str, body: &str) {
    if let Err(e) = notify_rust::Notification::new().appname("Landry").summary(&t(summary)).body(&t(body)).show() {
        eprintln!("landry: notification impossible : {e}");
    }
}

/// Read a GTK clipboard (CLIPBOARD or PRIMARY). The request is asynchronous: the owner of the
/// selection may never answer (Spotify under XWayland does not), and `wait_for_text` would then
/// freeze the main loop, so the bar could not open again until that app quit.
fn read_selection(app: &AppHandle, primary: bool) -> Option<String> {
    let (tx, rx) = mpsc::channel();
    let _ = app.run_on_main_thread(move || {
        let atom = if primary { gtk::gdk::SELECTION_PRIMARY } else { gtk::gdk::SELECTION_CLIPBOARD };
        gtk::Clipboard::get(&atom).request_text(move |_, text| {
            let _ = tx.send(text.map(str::to_string));
        });
    });
    rx.recv_timeout(Duration::from_millis(500)).ok().flatten()
}

/// The clipboard belongs to the resident process, so it survives hiding the bar.
fn set_clipboard(app: &AppHandle, text: String) {
    let (tx, rx) = mpsc::channel();
    let _ = app.run_on_main_thread(move || {
        let cb = gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD);
        cb.set_text(&text);
        cb.store();
        let _ = tx.send(());
    });
    let _ = rx.recv_timeout(Duration::from_secs(1));
}

/// Run a user script with a time limit; show its output in the bar (or a notification).
fn run_script(app: &AppHandle, program: &str, args: &[String], label: &str, silent: bool, limit: u64) {
    use std::io::Read;
    let child = std::process::Command::new(program)
        .args(args)
        .current_dir(paths::expand("~"))
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn();
    let mut child = match child {
        Ok(c) => c,
        Err(e) => return notify(label, &tf("Lancement impossible : {e}", &[("e", &e)])),
    };
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break Some(s),
            Ok(None) if start.elapsed() > Duration::from_secs(limit) => {
                let _ = child.kill();
                break None;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(_) => break None,
        }
    };
    let mut out = String::new();
    let mut err = String::new();
    if let Some(mut o) = child.stdout.take() {
        let _ = o.read_to_string(&mut out);
    }
    if let Some(mut e) = child.stderr.take() {
        let _ = e.read_to_string(&mut err);
    }
    let text = match status {
        None => tf("{l} : arrêté après {n} s", &[("l", &label), ("n", &limit)]),
        Some(s) if s.success() => {
            let o = out.trim();
            if o.is_empty() { tf("{l} : terminé", &[("l", &label)]) } else { o.chars().take(2000).collect() }
        }
        Some(s) => tf("{l} : échec ({s}) {e}", &[("l", &label), ("s", &s), ("e", &err.trim().chars().take(500).collect::<String>())]),
    };
    let visible = bar(app).and_then(|w| w.is_visible().ok()).unwrap_or(false);
    if silent || !visible {
        notify(label, &text);
    } else {
        let _ = app.emit("landry://notice", text);
    }
}

/// Interactive screenshot through the portal (GNOME saves it in Pictures/Screenshots), then
/// copy the image, or the text Tesseract reads in it.
fn capture(app: &AppHandle, kind: &str) {
    // Let the bar disappear before GNOME's screenshot UI freezes the screen.
    std::thread::sleep(Duration::from_millis(250));
    let shot = tauri::async_runtime::block_on(async {
        ashpd::desktop::screenshot::Screenshot::request().interactive(true).modal(true).send().await?.response()
    });
    let path = match shot.map(|s| url::Url::parse(s.uri().as_str()).ok().and_then(|u| u.to_file_path().ok())) {
        Ok(Some(p)) => p,
        Ok(None) => return notify("Capture", "Chemin de capture inattendu."),
        Err(e) => return eprintln!("landry: capture annulée ou indisponible : {e}"),
    };
    if kind == "ocr" {
        let langs = util::output("tesseract", &["--list-langs"], None).unwrap_or_default();
        let lang = if langs.lines().any(|l| l.trim() == "fra") { "fra+eng" } else { "eng" };
        match util::output("tesseract", &[&path.to_string_lossy(), "stdout", "-l", lang], None) {
            Ok(text) if !text.trim().is_empty() => {
                // Wayland only lets a focused window set the clipboard: show the bar, which also
                // lets the user see what was read.
                let text = text.trim().to_string();
                show_bar(app, String::new());
                set_clipboard(app, text.clone());
                let preview: String = text.chars().take(300).collect();
                let _ = app.emit("landry://notice", tf("Texte copié : {t}", &[("t", &preview)]));
            }
            Ok(_) => notify("OCR", "Aucun texte reconnu dans cette zone."),
            Err(e) => notify("OCR indisponible", &tf("{e}. Installe tesseract-ocr et tesseract-ocr-fra (relance ./install.sh).", &[("e", &e)])),
        }
        return;
    }
    let (tx, rx) = mpsc::channel();
    let p = path.clone();
    let _ = app.run_on_main_thread(move || {
        let ok = match gtk::gdk_pixbuf::Pixbuf::from_file(&p) {
            Ok(pix) => {
                let cb = gtk::Clipboard::get(&gtk::gdk::SELECTION_CLIPBOARD);
                cb.set_image(&pix);
                cb.store();
                true
            }
            Err(_) => false,
        };
        let _ = tx.send(ok);
    });
    let copied = rx.recv_timeout(Duration::from_secs(2)).unwrap_or(false);
    notify(
        "Capture enregistrée",
        &if copied { tf("{p} (copiée dans le presse-papiers)", &[("p", &path.display())]) } else { path.display().to_string() },
    );
}

/// Screen color picker through the Screenshot portal, then show the bar with the color.
fn pick_color(app: &AppHandle) {
    let picked = tauri::async_runtime::block_on(async {
        ashpd::desktop::Color::pick().send().await?.response()
    });
    match picked {
        Ok(c) => {
            let hex = modules::color::Rgb { r: c.red(), g: c.green(), b: c.blue() }.hex();
            show_bar(app, String::new());
            let _ = app.emit("landry://fill", hex);
        }
        Err(e) => eprintln!("landry: pipette annulée ou indisponible : {e}"),
    }
}

fn handle(app: &AppHandle, req: ipc::Request) -> String {
    let words: Vec<&str> = req.args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["toggle"] => {
            let w = bar(app);
            let visible = w.as_ref().and_then(|w| w.is_visible().ok()).unwrap_or(false);
            let focused = w.as_ref().and_then(|w| w.is_focused().ok()).unwrap_or(false);
            if visible && focused {
                hide_bar(app);
                "ok hidden".into()
            } else {
                show_bar(app, req.token)
            }
        }
        ["show"] => show_bar(app, req.token),
        ["setup"] => {
            let r = show_bar(app, req.token);
            let _ = app.emit("landry://view", "setup");
            r
        }
        ["hide"] => {
            hide_bar(app);
            "ok hidden".into()
        }
        ["quit"] => {
            app.exit(0);
            "ok".into()
        }
        ["ping"] => "ok pong".into(),
        ["reload"] => {
            engine(app).reload_config();
            emit_preset(app);
            "ok configuration rechargée".into()
        }
        ["run", rest @ ..] if !rest.is_empty() => {
            let result = engine(app).run_query(&rest.join(" "));
            match result {
                Ok((title, outcome)) => {
                    apply_outcome(app, &outcome);
                    match &outcome {
                        Outcome::Settings => {
                            show_bar(app, req.token);
                            let _ = app.emit("landry://open-settings", ());
                        }
                        Outcome::Fill(q) => {
                            show_bar(app, req.token);
                            let _ = app.emit("landry://fill", q.clone());
                        }
                        Outcome::View(v) => {
                            show_bar(app, req.token);
                            let _ = app.emit("landry://view", v.clone());
                        }
                        _ => {}
                    }
                    emit_preset(app);
                    match outcome {
                        Outcome::Message(m) => format!("ok {title} : {m}"),
                        _ => format!("ok {title}"),
                    }
                }
                Err(e) => format!("err {e}"),
            }
        }
        ["search", rest @ ..] => {
            let items = engine(app).search(&rest.join(" "));
            let lines: Vec<String> = items.iter().map(|i| format!("[{}] {} ({})", i.module, i.title, i.subtitle)).collect();
            format!("ok {}", lines.join(" | "))
        }
        ["preset", "list"] => {
            let names: Vec<String> = engine(app).store.list().into_iter().map(|i| i.name).collect();
            format!("ok {}", names.join(" | "))
        }
        ["preset", "show", name @ ..] if !name.is_empty() => {
            let r = engine(app).store.get(&name.join(" "));
            match r.and_then(|p| serde_json::to_string(&p).map_err(|e| e.to_string())) {
                Ok(json) => format!("ok {json}"),
                Err(e) => format!("err {e}"),
            }
        }
        ["preset", "use", name @ ..] if !name.is_empty() => {
            let r = engine(app).use_preset(&name.join(" "));
            match r {
                Ok(()) => {
                    emit_preset(app);
                    format!("ok preset actif : {}", name.join(" "))
                }
                Err(e) => format!("err {e}"),
            }
        }
        _ => format!(
            "err commande inconnue : {}. Commandes : toggle, show, hide, setup, search <texte>, run <texte>, preset list, preset show <nom>, preset use <nom>, reload, quit",
            req.args.join(" ")
        ),
    }
}

// ---------- Commands called by the frontend ----------

#[derive(Serialize)]
struct UiState {
    preset: Preset,
    warning: Option<String>,
    history: Vec<String>,
    system_dark: bool,
    locale: String,
    /// The first-run assistant has never been closed.
    first_run: bool,
}

// Commands that touch the engine, a process or the clipboard are async: Tauri runs sync commands
// on the main thread, where a slow call would freeze the bar.

#[tauri::command]
async fn ui_state(app: AppHandle) -> UiState {
    let e = engine(&app);
    UiState {
        preset: e.preset.clone(),
        warning: e.warning.clone(),
        history: e.history(),
        system_dark: SYSTEM_DARK.load(Ordering::Relaxed),
        locale: e.settings.locale.clone(),
        first_run: session::first_run(),
    }
}

#[tauri::command]
async fn search(app: AppHandle, query: String) -> Vec<search::Item> {
    engine(&app).search(&query)
}

#[tauri::command]
async fn activate(app: AppHandle, id: String, action: String, arg: String, query: String) -> Result<Outcome, String> {
    if id.starts_with("snip:") {
        // {presse-papiers} needs the clipboard as it is now, before Landry replaces it.
        let clip = read_selection(&app, false).unwrap_or_default();
        engine(&app).clipboard = clip;
    }
    let outcome = engine(&app).activate(&id, &action, &arg, &query)?;
    if id.starts_with("cmd:preset:") || id == "cmd:reload" {
        emit_preset(&app);
    }
    apply_outcome(&app, &outcome);
    Ok(outcome)
}

#[tauri::command]
async fn item_actions(app: AppHandle, id: String) -> Vec<search::Action> {
    engine(&app).actions(&id)
}

#[tauri::command]
async fn preview(app: AppHandle, id: String) -> modules::Preview {
    engine(&app).preview(&id)
}

#[derive(Serialize)]
struct DiffLine {
    tag: &'static str,
    text: String,
}

#[tauri::command]
fn diff_texts(a: String, b: String) -> Vec<DiffLine> {
    similar::TextDiff::from_lines(&a, &b)
        .iter_all_changes()
        .map(|c| DiffLine {
            tag: match c.tag() {
                similar::ChangeTag::Delete => "del",
                similar::ChangeTag::Insert => "ins",
                similar::ChangeTag::Equal => "eq",
            },
            text: c.value().trim_end_matches('\n').to_string(),
        })
        .collect()
}

#[derive(Serialize)]
struct Account {
    id: &'static str,
    label: &'static str,
    configured: bool,
}

/// Which accounts have a token in the keyring (never the tokens themselves).
#[tauri::command]
fn accounts() -> Vec<Account> {
    modules::cloud::ACCOUNTS
        .iter()
        .map(|(id, label)| Account { id, label, configured: modules::cloud::token(id).is_some() })
        .collect()
}

#[tauri::command]
fn get_settings(state: State<EngineState>) -> config::Settings {
    state.0.lock().unwrap_or_else(|e| e.into_inner()).settings.clone()
}

/// Save global settings, then re-index projects and files in the background.
/// shortcut: the file watcher keeps the folders it had at startup; restart Landry to watch new ones.
#[tauri::command]
fn save_settings(app: AppHandle, settings: config::Settings) -> Result<(), String> {
    engine(&app).save_settings(settings.clone())?;
    let h = app.clone();
    std::thread::spawn(move || {
        let projects = modules::projects::scan(&settings, true);
        let files = modules::files::scan(&settings);
        let bookmarks = modules::web::bookmarks();
        let mut e = engine(&h);
        e.set_index("projects", projects);
        e.set_index("files", files);
        // Indexed items carry fixed texts: rebuild them in case the language changed.
        e.set_index("web", bookmarks);
        e.set_index("tools", modules::tools::catalog());
        e.set_index("system", modules::system::ssh_items());
        e.set_index("scripts", modules::scripts::scan());
        e.reload_organizer();
    });
    Ok(())
}

/// Empty token removes it.
#[tauri::command]
fn set_token(account: String, token: String) -> Result<(), String> {
    modules::cloud::set_token(&account, &token)
}

#[tauri::command]
fn snippets_list(state: State<EngineState>) -> Result<Vec<usage::Snippet>, String> {
    state.0.lock().unwrap_or_else(|e| e.into_inner()).snippets()
}

#[tauri::command]
fn snippet_save(state: State<EngineState>, snippet: usage::Snippet) -> Result<i64, String> {
    state.0.lock().unwrap_or_else(|e| e.into_inner()).save_snippet(&snippet)
}

#[tauri::command]
fn snippet_delete(state: State<EngineState>, id: i64) -> Result<(), String> {
    state.0.lock().unwrap_or_else(|e| e.into_inner()).delete_snippet(id)
}

#[tauri::command]
fn notes_list(state: State<EngineState>) -> Result<Vec<usage::Note>, String> {
    let e = state.0.lock().unwrap_or_else(|e| e.into_inner());
    e.db_ref().ok_or_else(|| crate::i18n::t("base de données indisponible"))?.notes().map_err(|e| e.to_string())
}

/// Save (id 0 creates); returns the id.
#[tauri::command]
fn note_save(state: State<EngineState>, id: i64, body: String) -> Result<i64, String> {
    let mut e = state.0.lock().unwrap_or_else(|e| e.into_inner());
    let id = e.db_ref().ok_or_else(|| crate::i18n::t("base de données indisponible"))?.save_note(id, &body).map_err(|e| e.to_string())?;
    e.reload_organizer();
    Ok(id)
}

#[tauri::command]
fn note_delete(state: State<EngineState>, id: i64) -> Result<(), String> {
    let mut e = state.0.lock().unwrap_or_else(|e| e.into_inner());
    e.db_ref().ok_or_else(|| crate::i18n::t("base de données indisponible"))?.delete_note(id).map_err(|e| e.to_string())?;
    e.reload_organizer();
    Ok(())
}

#[tauri::command]
fn backup_export(state: State<EngineState>) -> Result<String, String> {
    let e = state.0.lock().unwrap_or_else(|e| e.into_inner());
    e.export_backup(&downloads_dir()).map(|p| p.display().to_string())
}

#[tauri::command]
fn backup_import(app: AppHandle, path: String) -> Result<String, String> {
    let msg = engine(&app).import_backup(&paths::expand(path.trim()))?;
    emit_preset(&app);
    Ok(msg)
}

#[tauri::command]
async fn autostart_get() -> bool {
    session::autostart()
}

/// Start the resident process with the session (systemd user service, restarted after a crash).
#[tauri::command]
async fn autostart_set(on: bool) -> Result<(), String> {
    session::set_autostart(on)
}

#[derive(Serialize)]
struct SetupState {
    shortcut: Option<String>,
    autostart: bool,
    /// Optional tools that are missing, with the command that installs them.
    missing: Vec<(String, String)>,
    extension: session::Extension,
}

/// Everything the first-run assistant shows. The doctor checks run on a plain thread: the
/// keyring check talks D-Bus through its own blocking runtime, which must not run inside Tauri's.
#[tauri::command]
async fn setup_state() -> SetupState {
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let missing = doctor::checks().into_iter().filter(|c| !c.ok && c.hint.starts_with("sudo ")).map(|c| (c.label, c.hint)).collect();
        let _ = tx.send(SetupState { shortcut: session::shortcut(), autostart: session::autostart(), missing, extension: session::extension() });
    });
    let fallback = || SetupState { shortcut: None, autostart: false, missing: vec![], extension: session::Extension::Missing };
    tauri::async_runtime::spawn_blocking(move || rx.recv_timeout(Duration::from_secs(10)).unwrap_or_else(|_| fallback())).await.unwrap_or_else(|_| fallback())
}

#[tauri::command]
async fn extension_enable() -> Result<(), String> {
    session::enable_extension()
}

#[tauri::command]
async fn shortcut_set(accel: String) -> Result<(), String> {
    session::set_shortcut(&accel)
}

#[tauri::command]
fn setup_done() -> Result<(), String> {
    session::mark_done()
}

#[tauri::command]
async fn copy_text(app: AppHandle, text: String) {
    set_clipboard(&app, text);
}

#[tauri::command]
fn painted(state: State<Painted>) {
    if let Some(tx) = state.0.lock().unwrap_or_else(|e| e.into_inner()).take() {
        let _ = tx.send(());
    }
}

#[tauri::command]
fn hide(app: AppHandle) {
    hide_bar(&app);
}

/// The settings view needs room even when the preset uses the small window layer.
#[tauri::command]
fn settings_layer(app: AppHandle, on: bool) {
    let Some(w) = bar(&app) else { return };
    let e = engine(&app);
    let l = &e.preset.layout;
    let layer = if on { config::Layer::Screen } else { l.layer };
    fit_layer(&w, (layer, l.width, l.max_height));
}

#[tauri::command]
fn preset_list(state: State<EngineState>) -> Vec<PresetInfo> {
    state.0.lock().unwrap_or_else(|e| e.into_inner()).store.list()
}

#[tauri::command]
fn preset_get(state: State<EngineState>, name: String) -> Result<Preset, String> {
    state.0.lock().unwrap_or_else(|e| e.into_inner()).store.get(&name)
}

/// Save from the form or from raw JSON. Re-applies the preset if it is the active one.
#[tauri::command]
fn preset_save(app: AppHandle, json: String, previous: Option<String>) -> Result<Preset, String> {
    let p = Preset::parse(&json)?;
    {
        let mut e = engine(&app);
        e.store.save(&p, previous.as_deref())?;
        let active = e.preset.name.clone();
        if active == p.name || previous.as_deref() == Some(active.as_str()) {
            e.use_preset(&p.name)?;
        }
    }
    emit_preset(&app);
    Ok(p)
}

#[tauri::command]
fn preset_check(json: String) -> Result<(), String> {
    Preset::parse(&json).map(|_| ())
}

#[tauri::command]
fn preset_duplicate(state: State<EngineState>, name: String, new_name: String) -> Result<Preset, String> {
    state.0.lock().unwrap_or_else(|e| e.into_inner()).store.duplicate(&name, &new_name)
}

#[tauri::command]
fn preset_delete(app: AppHandle, name: String) -> Result<(), String> {
    {
        let mut e = engine(&app);
        e.store.delete(&name)?;
        e.reload_config();
    }
    emit_preset(&app);
    Ok(())
}

#[tauri::command]
async fn preset_use(app: AppHandle, name: String) -> Result<(), String> {
    engine(&app).use_preset(&name)?;
    emit_preset(&app);
    Ok(())
}

fn downloads_dir() -> PathBuf {
    let home = paths::expand("~");
    // xdg-user-dir knows the localized name (Téléchargements).
    std::process::Command::new("xdg-user-dir")
        .arg("DOWNLOAD")
        .output()
        .ok()
        .map(|o| PathBuf::from(String::from_utf8_lossy(&o.stdout).trim()))
        .filter(|p| p.is_dir() && *p != home)
        .unwrap_or(home)
}

/// Export to the Downloads folder; returns the written path.
#[tauri::command]
fn preset_export(state: State<EngineState>, name: String) -> Result<String, String> {
    let slug: String =
        name.chars().map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    let dest = downloads_dir().join(format!("landry-preset-{slug}.json"));
    state.0.lock().unwrap_or_else(|e| e.into_inner()).store.export(&name, &dest)?;
    Ok(dest.display().to_string())
}

#[tauri::command]
fn preset_import(state: State<EngineState>, path: String) -> Result<String, String> {
    state.0.lock().unwrap_or_else(|e| e.into_inner()).store.import(&paths::expand(path.trim()))
}

// ---------- Background work ----------

fn data_url(p: &Path) -> Option<String> {
    let mime = match p.extension()?.to_str()? {
        "png" => "image/png",
        "svg" => "image/svg+xml",
        _ => return None,
    };
    let bytes = std::fs::read(p).ok()?;
    if bytes.len() > 200_000 {
        return None;
    }
    Some(format!("data:{mime};base64,{}", base64::engine::general_purpose::STANDARD.encode(bytes)))
}

/// GTK icon theme lookups must run on the main thread; file reads do not.
fn resolve_icons(app: &AppHandle, names: Vec<String>) -> HashMap<String, String> {
    let (tx, rx) = mpsc::channel();
    let _ = app.run_on_main_thread(move || {
        use gtk::prelude::*;
        let theme = gtk::IconTheme::default();
        let found: Vec<(String, Option<PathBuf>)> = names
            .into_iter()
            .map(|n| {
                let p = if n.starts_with('/') {
                    Some(PathBuf::from(&n))
                } else {
                    theme
                        .as_ref()
                        .and_then(|t| t.lookup_icon(&n, 48, gtk::IconLookupFlags::empty()))
                        .and_then(|i| i.filename())
                };
                (n, p)
            })
            .collect();
        let _ = tx.send(found);
    });
    rx.recv_timeout(Duration::from_secs(10))
        .unwrap_or_default()
        .into_iter()
        .filter_map(|(n, p)| Some((n, data_url(&p?)?)))
        .collect()
}

fn index_apps(app: &AppHandle) {
    let start = Instant::now();
    let apps = apps::scan(&paths::data_dirs());
    let mut names: Vec<String> = apps.iter().map(|a| a.icon.clone()).filter(|n| !n.is_empty()).collect();
    names.sort();
    names.dedup();
    let icons = resolve_icons(app, names);
    let count = apps.len();
    engine(app).set_apps(apps, &icons);
    eprintln!("landry: {count} applications indexées en {} ms", start.elapsed().as_millis());
}

/// Tools, SSH hosts, projects and files; files stay watched afterwards.
fn index_others(app: &AppHandle) {
    let settings = engine(app).settings.clone();
    std::thread::spawn(modules::calc::refresh_rates);
    {
        let mut e = engine(app);
        e.set_index("tools", modules::tools::catalog());
        e.set_index("system", modules::system::ssh_items());
    }
    let bookmarks = modules::web::bookmarks();
    engine(app).set_index("web", bookmarks);
    modules::scripts::seed();
    engine(app).set_index("scripts", modules::scripts::scan());
    let h = app.clone();
    std::thread::spawn(move || watch_scripts(&h));
    // Projects: names first, git states a moment later.
    let fast = modules::projects::scan(&settings, false);
    engine(app).set_index("projects", fast);
    let h = app.clone();
    let s2 = settings.clone();
    std::thread::spawn(move || {
        let full = modules::projects::scan(&s2, true);
        engine(&h).set_index("projects", full);
    });

    let start = Instant::now();
    let files = modules::files::scan(&settings);
    let n = files.len();
    engine(app).set_index("files", files);
    eprintln!("landry: {n} fichiers indexés en {} ms", start.elapsed().as_millis());
    let h = app.clone();
    modules::files::watch(settings, move |files| engine(&h).set_index("files", files));
}

/// Every second: timers and reminders, and a live refresh while timers run on screen.
fn clock_loop(app: &AppHandle) {
    loop {
        std::thread::sleep(Duration::from_secs(1));
        let (notes, running) = {
            let mut e = engine(app);
            (e.tick(usage::now()), e.has_timers())
        };
        for (title, body) in notes {
            notify(&title, &body);
        }
        if running && bar(app).and_then(|w| w.is_visible().ok()).unwrap_or(false) {
            let _ = app.emit("landry://refresh", ());
        }
    }
}

/// Icon in the top bar (needs the AppIndicator extension, enabled by default on Ubuntu).
fn tray(app: &AppHandle) -> tauri::Result<()> {
    use tauri::menu::{Menu, MenuItem};
    let open = MenuItem::with_id(app, "open", "Ouvrir Landry", true, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Réglages", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quitter Landry", true, None::<&str>)?;
    let menu = Menu::with_items(app, &[&open, &settings, &quit])?;
    let mut builder = tauri::tray::TrayIconBuilder::with_id("landry").tooltip("Landry").menu(&menu);
    if let Some(icon) = app.default_window_icon() {
        builder = builder.icon(icon.clone());
    }
    builder
        .on_menu_event(|app, e| {
            let h = app.clone();
            match e.id().as_ref() {
                // show_bar waits for the first frame: never on the event loop thread.
                "open" => {
                    std::thread::spawn(move || {
                        show_bar(&h, String::new());
                    });
                }
                "settings" => {
                    std::thread::spawn(move || {
                        show_bar(&h, String::new());
                        let _ = h.emit("landry://open-settings", ());
                    });
                }
                "quit" => app.exit(0),
                _ => {}
            }
        })
        .build(app)?;
    Ok(())
}

/// Hot reload: rescan the scripts folder when it changes.
fn watch_scripts(app: &AppHandle) {
    use notify::{RecursiveMode, Watcher};
    let (tx, rx) = mpsc::channel();
    let Ok(mut w) = notify::recommended_watcher(move |r: notify::Result<notify::Event>| {
        if r.is_ok_and(|e| !e.kind.is_access()) {
            let _ = tx.send(());
        }
    }) else {
        return;
    };
    if w.watch(&modules::scripts::dir(), RecursiveMode::NonRecursive).is_err() {
        return;
    }
    while rx.recv().is_ok() {
        while rx.recv_timeout(Duration::from_millis(300)).is_ok() {}
        engine(app).set_index("scripts", modules::scripts::scan());
    }
}

fn read_system_dark() -> bool {
    std::process::Command::new("gsettings")
        .args(["get", "org.gnome.desktop.interface", "color-scheme"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).contains("dark"))
        .unwrap_or(false)
}

/// Follow GNOME's light or dark choice live.
fn watch_theme(app: AppHandle) {
    SYSTEM_DARK.store(read_system_dark(), Ordering::Relaxed);
    let Ok(child) = std::process::Command::new("gsettings")
        .args(["monitor", "org.gnome.desktop.interface", "color-scheme"])
        .stdout(std::process::Stdio::piped())
        .spawn()
    else {
        return;
    };
    let Some(out) = child.stdout else { return };
    for line in std::io::BufReader::new(out).lines().map_while(Result::ok) {
        let dark = line.contains("dark");
        SYSTEM_DARK.store(dark, Ordering::Relaxed);
        let _ = app.emit("landry://theme", dark);
    }
}

fn daemon() {
    let listener = match ipc::bind() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("landry: {e}");
            // Another daemon already answers: not a failure, so systemd does not restart us in a loop.
            std::process::exit(if e.kind() == std::io::ErrorKind::AddrInUse { 0 } else { 1 });
        }
    };
    let db = match usage::Db::open(&paths::data_dir().join("landry.db")) {
        Ok(d) => Some(d),
        Err(e) => {
            eprintln!("landry: historique indisponible : {e}");
            None
        }
    };
    let core = Engine::new(config::Store::new(paths::config_dir()), db);

    tauri::Builder::default()
        .manage(Painted(Mutex::new(None)))
        .manage(EngineState(Mutex::new(core)))
        .invoke_handler(tauri::generate_handler![
            ui_state,
            search,
            activate,
            item_actions,
            preview,
            diff_texts,
            copy_text,
            accounts,
            set_token,
            snippets_list,
            notes_list,
            backup_export,
            autostart_get,
            autostart_set,
            setup_state,
            extension_enable,
            shortcut_set,
            setup_done,
            backup_import,
            note_save,
            note_delete,
            snippet_save,
            snippet_delete,
            get_settings,
            save_settings,
            painted,
            hide,
            settings_layer,
            preset_list,
            preset_get,
            preset_save,
            preset_check,
            preset_duplicate,
            preset_delete,
            preset_use,
            preset_export,
            preset_import
        ])
        .setup(move |app| {
            if let Err(e) = tray(app.handle()) {
                eprintln!("landry: icône de barre système indisponible : {e}");
            }
            let h = app.handle().clone();
            std::thread::spawn(move || ipc::serve(listener, move |req| handle(&h, req)));
            let h = app.handle().clone();
            std::thread::spawn(move || {
                index_apps(&h);
                // Installed or removed apps show up without restarting Landry.
                let h2 = h.clone();
                apps::watch(&paths::data_dirs(), move || index_apps(&h2));
            });
            let h = app.handle().clone();
            std::thread::spawn(move || index_others(&h));
            let h = app.handle().clone();
            std::thread::spawn(move || clock_loop(&h));
            let h = app.handle().clone();
            modules::cloud::on_update(move || {
                let _ = h.emit("landry://refresh", ());
            });
            let h = app.handle().clone();
            std::thread::spawn(move || watch_theme(h));
            Ok(())
        })
        .on_window_event(|window, event| {
            // A launcher bar disappears when you switch elsewhere.
            if let WindowEvent::Focused(false) = event {
                if std::env::var_os("LANDRY_KEEP_OPEN").is_none() {
                    let _ = window.hide();
                }
            }
        })
        .build(tauri::generate_context!())
        .expect("échec du démarrage de Tauri")
        .run(|_, event| {
            // The daemon stays alive with every window hidden.
            if let tauri::RunEvent::ExitRequested { api, code: None, .. } = event {
                api.prevent_exit();
            }
        });
}
