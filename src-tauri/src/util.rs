//! Process helpers shared by modules.

use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Command, Stdio};

/// Start a program detached from Landry (own process group, no stdio).
pub fn spawn(program: &str, args: &[String], cwd: Option<&Path>) -> Result<(), String> {
    let mut c = Command::new(program);
    c.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).process_group(0);
    if let Some(d) = cwd {
        c.current_dir(d);
    }
    c.spawn().map(|_| ()).map_err(|e| format!("{program} : {e}"))
}

/// Run and capture stdout. A non-zero exit becomes an error carrying stderr.
pub fn output(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<String, String> {
    let mut c = Command::new(program);
    c.args(args).stdin(Stdio::null());
    if let Some(d) = cwd {
        c.current_dir(d);
    }
    let o = c.output().map_err(|e| format!("{program} : {e}"))?;
    if o.status.success() {
        Ok(String::from_utf8_lossy(&o.stdout).into_owned())
    } else {
        let err = String::from_utf8_lossy(&o.stderr);
        Err(format!("{program} : {}", err.trim().lines().last().unwrap_or("échec")))
    }
}

/// Open a file, folder or URL with the default application.
pub fn open(target: &str) -> Result<(), String> {
    spawn("gio", &["open".into(), target.into()], None)
}

/// Expand a command template word by word: `{name}` placeholders become exactly one argument
/// each, so values never go through a shell.
pub fn template(tpl: &str, vars: &[(&str, &str)]) -> Vec<String> {
    tpl.split_whitespace()
        .map(|w| vars.iter().fold(w.to_string(), |acc, (k, v)| acc.replace(&format!("{{{k}}}"), v)))
        .collect()
}

pub fn run_template(tpl: &str, vars: &[(&str, &str)], cwd: Option<&Path>) -> Result<(), String> {
    let argv = template(tpl, vars);
    let (program, args) = argv.split_first().ok_or_else(|| crate::i18n::t("commande vide"))?;
    spawn(program, args, cwd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_keeps_values_as_single_arguments() {
        let argv = template(
            "gnome-terminal --working-directory={dir} -- bash -lc {cmd}",
            &[("dir", "/home/me/my dir"), ("cmd", "npm run dev; exec bash")],
        );
        assert_eq!(argv[1], "--working-directory=/home/me/my dir");
        assert_eq!(argv.last().unwrap(), "npm run dev; exec bash");
        assert_eq!(argv.len(), 6);
    }
}
