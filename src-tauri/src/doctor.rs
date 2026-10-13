//! `landry doctor`: what Landry needs on this machine, and how to fix what is missing.

use crate::config::Store;
use crate::util;

pub struct Check {
    pub label: String,
    pub ok: bool,
    /// What to do when it fails.
    pub hint: String,
    /// Missing but optional: only some features are affected.
    pub optional: bool,
}

fn check(label: &str, ok: bool, hint: &str, optional: bool) -> Check {
    Check { label: label.into(), ok, hint: hint.into(), optional }
}

fn has(program: &str) -> bool {
    std::process::Command::new("sh")
        .args(["-c", "command -v \"$1\" >/dev/null", "sh", program])
        .status()
        .is_ok_and(|s| s.success())
}

fn portal_interfaces() -> String {
    util::output(
        "gdbus",
        &["introspect", "--session", "--dest", "org.freedesktop.portal.Desktop", "--object-path", "/org/freedesktop/portal/desktop"],
        None,
    )
    .unwrap_or_default()
}

fn first_word(template: &str) -> String {
    template.split_whitespace().next().unwrap_or("").to_string()
}

pub fn checks() -> Vec<Check> {
    let settings = Store::new(crate::paths::config_dir()).settings().0;
    let session = std::env::var("XDG_SESSION_TYPE").unwrap_or_default();
    let desktop = std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default();
    let portals = portal_interfaces();
    let keybindings = util::output("gsettings", &["get", "org.gnome.settings-daemon.plugins.media-keys", "custom-keybindings"], None).unwrap_or_default();
    let shortcut_cmd = util::output(
        "gsettings",
        &["get", "org.gnome.settings-daemon.plugins.media-keys.custom-keybinding:/org/gnome/settings-daemon/plugins/media-keys/custom-keybindings/landry/", "command"],
        None,
    )
    .unwrap_or_default();
    let tess_langs = util::output("tesseract", &["--list-langs"], None).unwrap_or_default();
    let editor = first_word(&settings.editor);
    let terminal = first_word(&settings.terminal);

    vec![
        check("Session GNOME", desktop.contains("GNOME"), &format!("bureau détecté : « {desktop} » ; Landry vise GNOME"), false),
        check(&format!("Session {session}"), !session.is_empty(), "XDG_SESSION_TYPE absent", true),
        check("gio et gsettings", has("gio") && has("gsettings"), "sudo apt install libglib2.0-bin", false),
        check(
            "Raccourci GNOME vers landry toggle",
            keybindings.contains("/landry/") && shortcut_cmd.contains("toggle"),
            "scripts/install-shortcut.sh /usr/bin/landry '<Control><Alt>space'",
            false,
        ),
        check(
            "Démarrage avec la session",
            crate::paths::expand("~/.config/autostart/landry.desktop").exists(),
            "Réglages, onglet Général, « Démarrer Landry avec la session »",
            true,
        ),
        check("Portail Screenshot (pipette, capture)", portals.contains("org.freedesktop.portal.Screenshot"), "sudo apt install xdg-desktop-portal-gnome", true),
        check("Portail RemoteDesktop (collage automatique)", portals.contains("org.freedesktop.portal.RemoteDesktop"), "sudo apt install xdg-desktop-portal-gnome", true),
        check("Trousseau (Secret Service)", keyring::Entry::store_status().is_ok(), "sudo apt install gnome-keyring", true),
        check(&format!("Éditeur : {editor}"), has(&editor), "Réglages, onglet Général, champ Éditeur", true),
        check(&format!("Terminal : {terminal}"), has(&terminal), "Réglages, onglet Général, champ Terminal", true),
        check("git (projets)", has("git"), "sudo apt install git", true),
        check("zip (compresser un fichier)", has("zip"), "sudo apt install zip", true),
        check("tesseract (OCR)", has("tesseract"), "sudo apt install tesseract-ocr tesseract-ocr-fra", true),
        check("Langue française pour l'OCR", tess_langs.lines().any(|l| l.trim() == "fra"), "sudo apt install tesseract-ocr-fra", true),
        check("docker (conteneurs)", has("docker"), "facultatif : seulement pour « docker »", true),
        check("playerctl (/spotify, /media)", has("playerctl"), "sudo apt install playerctl", true),
        check("wpctl (/volume, /muet)", has("wpctl"), "sudo apt install wireplumber", true),
        check("nmcli (/wifi)", has("nmcli"), "sudo apt install network-manager", true),
        check("Processus résident", std::os::unix::net::UnixStream::connect(crate::ipc::socket_path()).is_ok(), "landry toggle le démarre", true),
    ]
}

pub fn run() -> i32 {
    let mut blocking = 0;
    for c in checks() {
        let mark = match (c.ok, c.optional) {
            (true, _) => "ok      ",
            (false, true) => "manque  ",
            (false, false) => {
                blocking += 1;
                "ERREUR  "
            }
        };
        println!("{mark}{}", c.label);
        if !c.ok {
            println!("        -> {}", c.hint);
        }
    }
    if blocking > 0 {
        println!("\n{blocking} problème(s) bloquant(s).");
        1
    } else {
        println!("\nRien de bloquant.");
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helpers() {
        assert_eq!(first_word("gnome-terminal --working-directory={dir}"), "gnome-terminal");
        assert_eq!(first_word(""), "");
        assert!(has("sh"));
        assert!(!has("landry-programme-inexistant"));
    }
}
