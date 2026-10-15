//! Desktop integration: start with the session (systemd user service), the GNOME keyboard
//! shortcut, and the first-run marker. Every step is idempotent: running it again changes nothing.
//! On Windows: the Run registry key, and a global shortcut registered by the daemon itself.

use crate::util;
use std::path::PathBuf;

const DEFAULT_SHORTCUT: &str = "<Control><Alt>space";

#[cfg(not(windows))]
const SCHEMA: &str = "org.gnome.settings-daemon.plugins.media-keys";
#[cfg(not(windows))]
const ENTRY: &str = "/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/landry/";

#[cfg(not(windows))]
fn unit_path() -> PathBuf {
    crate::paths::expand("~/.config/systemd/user/landry.service")
}

#[cfg(not(windows))]
/// The XDG autostart entry written by Landry 0.2; replaced by the service when possible.
fn legacy_autostart() -> PathBuf {
    crate::paths::expand("~/.config/autostart/landry.desktop")
}

fn marker() -> PathBuf {
    crate::paths::config_dir().join(".setup-done")
}

#[cfg(not(windows))]
pub fn unit(exe: &str) -> String {
    format!(
        "[Unit]\nDescription=Landry, compagnon du quotidien\nPartOf=graphical-session.target\nAfter=graphical-session.target\n\n\
         [Service]\nExecStart=\"{exe}\" daemon\n# Restarted after a crash, not after « Quitter ».\nRestart=on-failure\nRestartSec=2\n\n\
         [Install]\nWantedBy=graphical-session.target\n"
    )
}

#[cfg(not(windows))]
fn systemctl(args: &[&str]) -> Result<String, String> {
    let mut a = vec!["--user"];
    a.extend_from_slice(args);
    util::output("systemctl", &a, None)
}

#[cfg(not(windows))]
pub fn service_enabled() -> bool {
    unit_path().exists() && systemctl(&["is-enabled", "landry.service"]).is_ok_and(|s| s.trim() == "enabled")
}

#[cfg(not(windows))]
/// On with the session: true when the service or the old autostart entry is in place.
pub fn autostart() -> bool {
    service_enabled() || legacy_autostart().exists()
}

#[cfg(not(windows))]
/// Enable or disable the start with the session. Without a systemd user session, fall back to
/// an XDG autostart entry (no restart after a crash then).
pub fn set_autostart(on: bool) -> Result<(), String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let exe = exe.display().to_string();
    let remove = |p: PathBuf| match std::fs::remove_file(&p) {
        Err(e) if e.kind() != std::io::ErrorKind::NotFound => Err(format!("{}: {e}", p.display())),
        _ => Ok(()),
    };
    if !on {
        if unit_path().exists() {
            let _ = systemctl(&["disable", "landry.service"]);
            remove(unit_path())?;
            let _ = systemctl(&["daemon-reload"]);
        }
        return remove(legacy_autostart());
    }
    let path = unit_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, unit(&exe)).map_err(|e| e.to_string())?;
    // Enable only: the daemon already running keeps serving; systemd takes over at the next login.
    match systemctl(&["daemon-reload"]).and_then(|_| systemctl(&["enable", "landry.service"])) {
        Ok(_) => remove(legacy_autostart()),
        Err(_) => {
            let _ = std::fs::remove_file(&path);
            let entry = format!(
                "[Desktop Entry]\nType=Application\nName=Landry\nComment=Compagnon du quotidien\nExec=\"{exe}\" daemon\nIcon=landry\nNoDisplay=true\nX-GNOME-Autostart-enabled=true\n"
            );
            let p = legacy_autostart();
            if let Some(dir) = p.parent() {
                std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
            }
            std::fs::write(&p, entry).map_err(|e| e.to_string())
        }
    }
}

#[cfg(not(windows))]
/// Start the daemon through systemd when the service exists, so it is supervised.
pub fn start_service() -> bool {
    service_enabled() && systemctl(&["start", "--no-block", "landry.service"]).is_ok()
}

#[cfg(not(windows))]
/// Add `entry` to a gsettings string list ("@as []", "['/a/']"). None when already there.
pub fn add_to_list(current: &str, entry: &str) -> Option<String> {
    let current = current.trim();
    if current.contains(&format!("'{entry}'")) {
        return None;
    }
    let inner = current.trim_start_matches("@as").trim().trim_start_matches('[').trim_end_matches(']').trim();
    Some(if inner.is_empty() { format!("['{entry}']") } else { format!("[{inner}, '{entry}']") })
}

#[cfg(not(windows))]
/// Relocatable schema path of Landry's own shortcut.
fn keybinding() -> String {
    format!("{SCHEMA}.custom-keybinding:{ENTRY}")
}

#[cfg(not(windows))]
/// The accelerator of Landry's GNOME shortcut, if it is installed.
pub fn shortcut() -> Option<String> {
    let list = util::output("gsettings", &["get", SCHEMA, "custom-keybindings"], None).ok()?;
    if !list.contains(ENTRY) {
        return None;
    }
    let b = util::output("gsettings", &["get", &keybinding(), "binding"], None).ok()?;
    let b = b.trim().trim_matches('\'').to_string();
    (!b.is_empty()).then_some(b)
}

#[cfg(not(windows))]
/// Create or update the shortcut: `accel` runs `landry toggle`.
pub fn set_shortcut(accel: &str) -> Result<(), String> {
    let accel = accel.trim();
    if accel.is_empty() || accel.contains(['\'', '\n']) {
        return Err(crate::i18n::t("raccourci invalide"));
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let list = util::output("gsettings", &["get", SCHEMA, "custom-keybindings"], None)?;
    if let Some(new) = add_to_list(&list, ENTRY) {
        util::output("gsettings", &["set", SCHEMA, "custom-keybindings", &new], None)?;
    }
    let kb = keybinding();
    util::output("gsettings", &["set", &kb, "name", "Landry"], None)?;
    util::output("gsettings", &["set", &kb, "command", &format!("{} toggle", exe.display())], None)?;
    util::output("gsettings", &["set", &kb, "binding", accel], None)?;
    Ok(())
}

#[cfg(not(windows))]
/// Ctrl+Alt+Space out of the box: installed once, when Landry has no shortcut yet. A shortcut
/// the user removes or changes afterwards is left alone.
pub fn default_shortcut() {
    let marker = crate::paths::config_dir().join(".shortcut-default");
    if marker.exists() {
        return;
    }
    if shortcut().is_none() {
        if let Err(e) = set_shortcut(DEFAULT_SHORTCUT) {
            // No GNOME settings (another desktop): try again at the next start.
            return eprintln!("landry: raccourci par défaut impossible : {e}");
        }
    }
    let _ = std::fs::create_dir_all(crate::paths::config_dir());
    let _ = std::fs::write(marker, "1\n");
}

/// State of Landry's GNOME Shell extension (open windows, clipboard history).
#[derive(serde::Serialize, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
#[allow(dead_code)] // each platform reports only some of these states
pub enum Extension {
    /// Answers on D-Bus.
    Active,
    /// Enabled, waiting for the next login (Wayland loads new extensions only then).
    Pending,
    /// Installed with the package, not enabled.
    Off,
    /// Not installed (Landry built without the package).
    Missing,
    /// No GNOME Shell on this system (Windows).
    Unsupported,
}

#[cfg(not(windows))]
pub fn extension() -> Extension {
    use crate::modules::shell::{available, UUID};
    if available() {
        return Extension::Active;
    }
    let installed = ["/usr/share/gnome-shell/extensions", "~/.local/share/gnome-shell/extensions"]
        .iter()
        .any(|d| crate::paths::expand(d).join(UUID).join("extension.js").exists());
    let enabled = util::output("gsettings", &["get", "org.gnome.shell", "enabled-extensions"], None).is_ok_and(|l| l.contains(UUID));
    match (installed, enabled) {
        (false, _) => Extension::Missing,
        (true, true) => Extension::Pending,
        (true, false) => Extension::Off,
    }
}

#[cfg(not(windows))]
/// Add the extension to GNOME's enabled list. It starts at the next login under Wayland.
pub fn enable_extension() -> Result<(), String> {
    let uuid = crate::modules::shell::UUID;
    let list = util::output("gsettings", &["get", "org.gnome.shell", "enabled-extensions"], None)?;
    if let Some(new) = add_to_list(&list, uuid) {
        util::output("gsettings", &["set", "org.gnome.shell", "enabled-extensions", &new], None)?;
    }
    // Starts at once when the Shell already knows the extension (X11, or installed before login).
    let _ = util::output("gnome-extensions", &["enable", uuid], None);
    Ok(())
}

// ---------- Windows ----------

#[cfg(windows)]
const RUN: &str = r"HKCU\Software\Microsoft\Windows\CurrentVersion\Run";

#[cfg(windows)]
pub fn autostart() -> bool {
    util::output("reg", &["query", RUN, "/v", "Landry"], None).is_ok()
}

/// The Run key starts the daemon at login (no restart after a crash, unlike systemd).
#[cfg(windows)]
pub fn set_autostart(on: bool) -> Result<(), String> {
    if !on {
        return if autostart() { util::output("reg", &["delete", RUN, "/v", "Landry", "/f"], None).map(drop) } else { Ok(()) };
    }
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let command = format!("\"{}\" daemon", exe.display());
    util::output("reg", &["add", RUN, "/v", "Landry", "/t", "REG_SZ", "/d", &command, "/f"], None).map(drop)
}

#[cfg(windows)]
pub fn start_service() -> bool {
    false
}


#[cfg(windows)]
fn shortcut_file() -> PathBuf {
    crate::paths::config_dir().join("shortcut")
}

/// The saved accelerator, Ctrl+Alt+Space until one is chosen: the daemon registers it at start.
#[cfg(windows)]
pub fn shortcut() -> Option<String> {
    let saved = std::fs::read_to_string(shortcut_file()).unwrap_or_default();
    Some(if saved.trim().is_empty() { DEFAULT_SHORTCUT.into() } else { saved.trim().into() })
}

#[cfg(windows)]
pub fn set_shortcut(accel: &str) -> Result<(), String> {
    let accel = accel.trim();
    hotkey(accel).parse::<tauri_plugin_global_shortcut::Shortcut>().map_err(|_| crate::i18n::t("raccourci invalide"))?;
    let path = shortcut_file();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&path, accel).map_err(|e| e.to_string())
}

#[cfg(windows)]
pub fn extension() -> Extension {
    Extension::Unsupported
}

#[cfg(windows)]
pub fn enable_extension() -> Result<(), String> {
    Err(crate::i18n::t("L'extension GNOME n'existe que sous Linux."))
}

/// GNOME accelerator ("<Control><Alt>space", the format the interface records) to the
/// global-hotkey syntax ("Control+Alt+space").
#[cfg_attr(not(windows), allow(dead_code))]
pub fn hotkey(accel: &str) -> String {
    accel.replace("<Primary>", "<Control>").replace('>', "+").replace('<', "").replace("Return", "Enter")
}

pub fn first_run() -> bool {
    !marker().exists()
}

pub fn mark_done() -> Result<(), String> {
    let m = marker();
    if let Some(dir) = m.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    std::fs::write(&m, "1\n").map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hotkey_syntax() {
        assert_eq!(hotkey("<Control><Alt>space"), "Control+Alt+space");
        assert_eq!(hotkey("<Primary><Shift>Return"), "Control+Shift+Enter");
        assert_eq!(hotkey("F12"), "F12");
    }

    #[cfg(not(windows))]
    #[test]
    fn keybinding_list() {
        assert_eq!(add_to_list("@as []\n", "/x/"), Some("['/x/']".into()));
        assert_eq!(add_to_list("['/a/']", "/x/"), Some("['/a/', '/x/']".into()));
        assert_eq!(add_to_list("['/a/', '/x/']", "/x/"), None);
    }

    #[cfg(not(windows))]
    #[test]
    fn unit_restarts_only_after_a_crash() {
        let u = unit("/usr/bin/landry");
        assert!(u.contains("ExecStart=\"/usr/bin/landry\" daemon"));
        assert!(u.contains("Restart=on-failure"));
        assert!(u.contains("WantedBy=graphical-session.target"));
    }
}
