mod apps;
mod config;
mod engine;
mod ipc;
mod paths;
mod search;
mod usage;

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
        _ => std::process::exit(ipc::client(&args)),
    }
}

fn engine(app: &AppHandle) -> MutexGuard<'_, Engine> {
    app.state::<EngineState>().inner().0.lock().unwrap_or_else(|e| e.into_inner())
}

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

    let layer = {
        let e = engine(app);
        (e.preset.layout.layer, e.preset.layout.width, e.preset.layout.max_height)
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

/// Wayland clients cannot place their windows. Screen layer: a transparent window the size
/// of the screen, the bar placed in CSS. Window layer: a window the size of the bar, placed by
/// GNOME, faster to map (less surface).
fn fit_layer(w: &WebviewWindow, (layer, width, max_height): (config::Layer, u32, u32)) {
    let target = match layer {
        config::Layer::Window => tauri::LogicalSize::new(f64::from(width) + 64.0, f64::from(max_height) + 64.0),
        config::Layer::Screen => {
            let monitor = w.current_monitor().ok().flatten().or_else(|| w.primary_monitor().ok().flatten());
            let Some(m) = monitor else { return };
            m.size().to_logical::<f64>(m.scale_factor())
        }
    };
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
        Outcome::Keep => {}
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
                    if outcome == Outcome::Settings {
                        show_bar(app, req.token);
                    }
                    apply_outcome(app, &outcome);
                    emit_preset(app);
                    format!("ok {title}")
                }
                Err(e) => format!("err {e}"),
            }
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
            "err commande inconnue : {}. Commandes : toggle, show, hide, run <texte>, preset list, preset show <nom>, preset use <nom>, reload, quit",
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
}

#[tauri::command]
fn ui_state(state: State<EngineState>) -> UiState {
    let e = state.0.lock().unwrap_or_else(|e| e.into_inner());
    UiState {
        preset: e.preset.clone(),
        warning: e.warning.clone(),
        history: e.history(),
        system_dark: SYSTEM_DARK.load(Ordering::Relaxed),
    }
}

#[tauri::command]
fn search(state: State<EngineState>, query: String) -> Vec<search::Item> {
    state.0.lock().unwrap_or_else(|e| e.into_inner()).search(&query)
}

#[tauri::command]
fn activate(app: AppHandle, id: String, query: String) -> Result<Outcome, String> {
    let outcome = engine(&app).activate(&id, &query)?;
    if id.starts_with("cmd:preset:") || id == "cmd:reload" {
        emit_preset(&app);
    }
    apply_outcome(&app, &outcome);
    Ok(outcome)
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
fn preset_use(app: AppHandle, name: String) -> Result<(), String> {
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
            std::process::exit(1);
        }
    };
    let db = match usage::Db::open(&paths::data_dir().join("landry.db")) {
        Ok(d) => Some(d),
        Err(e) => {
            eprintln!("landry: historique indisponible : {e}");
            None
        }
    };
    let engine = Engine::new(config::Store::new(paths::config_dir()), db);

    tauri::Builder::default()
        .manage(Painted(Mutex::new(None)))
        .manage(EngineState(Mutex::new(engine)))
        .invoke_handler(tauri::generate_handler![
            ui_state,
            search,
            activate,
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
            let h = app.handle().clone();
            std::thread::spawn(move || ipc::serve(listener, move |req| handle(&h, req)));
            let h = app.handle().clone();
            std::thread::spawn(move || index_apps(&h));
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
