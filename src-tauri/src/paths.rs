//! XDG locations used by Landry, and their Windows counterparts.

use std::path::PathBuf;

fn home() -> PathBuf {
    std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir)
}

/// %APPDATA% or %LOCALAPPDATA% on Windows.
#[cfg(windows)]
fn windows_dir(var: &str) -> Option<PathBuf> {
    std::env::var_os(var).map(PathBuf::from).filter(|p| p.is_absolute())
}

fn xdg(var: &str, fallback: &str) -> PathBuf {
    std::env::var_os(var)
        .map(PathBuf::from)
        .filter(|p| p.is_absolute())
        .unwrap_or_else(|| home().join(fallback))
}

pub fn config_dir() -> PathBuf {
    #[cfg(windows)]
    if let Some(d) = windows_dir("APPDATA") {
        return d.join("landry");
    }
    xdg("XDG_CONFIG_HOME", ".config").join("landry")
}

pub fn data_dir() -> PathBuf {
    #[cfg(windows)]
    if let Some(d) = windows_dir("LOCALAPPDATA") {
        return d.join("landry");
    }
    xdg("XDG_DATA_HOME", ".local/share").join("landry")
}

/// Folders holding the installed applications: XDG `applications` dirs (user first), or the
/// Start Menu on Windows.
pub fn app_dirs() -> Vec<PathBuf> {
    #[cfg(windows)]
    return ["APPDATA", "ProgramData"]
        .into_iter()
        .filter_map(windows_dir)
        .map(|d| d.join("Microsoft").join("Windows").join("Start Menu").join("Programs"))
        .collect();
    #[cfg(not(windows))]
    data_dirs().into_iter().map(|d| d.join("applications")).collect()
}

/// All XDG data dirs, user first.
#[cfg(not(windows))]
fn data_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![xdg("XDG_DATA_HOME", ".local/share")];
    let system = std::env::var("XDG_DATA_DIRS")
        .ok()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "/usr/local/share:/usr/share".into());
    dirs.extend(system.split(':').filter(|s| !s.is_empty()).map(PathBuf::from));
    // Flatpak and Snap exports are not always in XDG_DATA_DIRS of the session.
    for extra in [
        home().join(".local/share/flatpak/exports/share"),
        PathBuf::from("/var/lib/flatpak/exports/share"),
        PathBuf::from("/var/lib/snapd/desktop"),
    ] {
        if !dirs.contains(&extra) {
            dirs.push(extra);
        }
    }
    dirs
}

/// Expand a leading `~/`.
pub fn expand(p: &str) -> PathBuf {
    match p.strip_prefix("~/") {
        Some(rest) => home().join(rest),
        None if p == "~" => home(),
        None => PathBuf::from(p),
    }
}
