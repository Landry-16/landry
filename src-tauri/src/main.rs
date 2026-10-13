mod apps;
mod config;
mod ipc;
mod paths;
mod search;
mod usage;

use std::sync::mpsc;
use std::sync::Mutex;
use std::time::{Duration, Instant};
use tauri::{AppHandle, Emitter, Manager, WebviewWindow, WindowEvent};

/// Set by `show_bar`, fired by the frontend once the first frame after showing is painted.
struct Painted(Mutex<Option<mpsc::Sender<()>>>);

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("daemon") => daemon(),
        _ => std::process::exit(ipc::client(&args)),
    }
}

fn bar(app: &AppHandle) -> Option<WebviewWindow> {
    app.get_webview_window("bar")
}

/// Show and focus the bar. Returns a status line with timings and focus state.
fn show_bar(app: &AppHandle, token: String) -> String {
    let Some(w) = bar(app) else { return "err fenêtre introuvable".into() };
    let start = Instant::now();
    let (tx, rx) = mpsc::channel();
    *app.state::<Painted>().0.lock().unwrap_or_else(|e| e.into_inner()) = Some(tx);

    let win = w.clone();
    let _ = app.run_on_main_thread(move || {
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
    let status = format!(
        "ok shown painted={painted} paint_ms={paint_ms} focused={focused} focus_ms={}",
        start.elapsed().as_millis()
    );
    eprintln!("landry: {status}");
    status
}

fn hide_bar(app: &AppHandle) {
    if let Some(w) = bar(app) {
        let _ = w.hide();
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
        _ => format!("err commande inconnue : {}", req.args.join(" ")),
    }
}

#[tauri::command]
fn painted(state: tauri::State<Painted>) {
    if let Some(tx) = state.0.lock().unwrap_or_else(|e| e.into_inner()).take() {
        let _ = tx.send(());
    }
}

#[tauri::command]
fn hide(app: AppHandle) {
    hide_bar(&app);
}

fn daemon() {
    let listener = match ipc::bind() {
        Ok(l) => l,
        Err(e) => {
            eprintln!("landry: {e}");
            std::process::exit(1);
        }
    };

    tauri::Builder::default()
        .manage(Painted(Mutex::new(None)))
        .invoke_handler(tauri::generate_handler![painted, hide])
        .setup(move |app| {
            let app_handle = app.handle().clone();
            std::thread::spawn(move || ipc::serve(listener, move |req| handle(&app_handle, req)));
            Ok(())
        })
        .on_window_event(|window, event| {
            // A launcher bar disappears when you click elsewhere.
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
