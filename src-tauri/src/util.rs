//! Process helpers shared by modules.

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Own process group on Unix, so the child outlives Landry; no console window on Windows.
pub fn detach(c: &mut Command) -> &mut Command {
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        c.process_group(0)
    }
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(CREATE_NO_WINDOW)
    }
}

/// No console window flashing up when the daemon (a GUI program on Windows) runs a console tool.
pub fn quiet(c: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        c.creation_flags(CREATE_NO_WINDOW);
    }
    c
}

/// Start a program detached from Landry (own process group, no stdio).
pub fn spawn(program: &str, args: &[String], cwd: Option<&Path>) -> Result<(), String> {
    let mut c = Command::new(program);
    detach(c.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()));
    if let Some(d) = cwd {
        c.current_dir(d);
    }
    c.spawn().map(|_| ()).map_err(|e| format!("{program} : {e}"))
}

/// Run and capture stdout, for at most 2 minutes. A non-zero exit becomes an error carrying stderr.
pub fn output(program: &str, args: &[&str], cwd: Option<&Path>) -> Result<String, String> {
    output_within(program, args, cwd, Duration::from_secs(120))
}

/// What a pipe has delivered so far, filled by a thread of its own; `done` fires at end of file.
struct Drain {
    buf: std::sync::Arc<std::sync::Mutex<Vec<u8>>>,
    done: Receiver<()>,
}

fn drain(pipe: Option<impl Read + Send + 'static>) -> Drain {
    let buf = std::sync::Arc::new(std::sync::Mutex::new(Vec::new()));
    let (tx, done) = mpsc::channel();
    if let Some(mut p) = pipe {
        let b = buf.clone();
        std::thread::spawn(move || {
            let mut chunk = [0u8; 8192];
            while let Ok(n @ 1..) = p.read(&mut chunk) {
                b.lock().unwrap_or_else(|e| e.into_inner()).extend_from_slice(&chunk[..n]);
            }
            let _ = tx.send(());
        });
    }
    Drain { buf, done }
}

impl Drain {
    /// Everything read, waiting at most `grace` for the end of file.
    fn take(self, grace: Duration) -> String {
        let _ = self.done.recv_timeout(grace);
        String::from_utf8_lossy(&self.buf.lock().unwrap_or_else(|e| e.into_inner())).into_owned()
    }
}

/// Like `output`, killed after `limit`. Only the program itself is waited for: an app it starts
/// inherits the pipes and may keep them open for hours (`gio open` starting Spotify), so after the
/// program exits its output is read for 200 ms more, never until the pipes close.
pub fn output_within(program: &str, args: &[&str], cwd: Option<&Path>, limit: Duration) -> Result<String, String> {
    let mut c = Command::new(program);
    quiet(c.args(args).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped()));
    if let Some(d) = cwd {
        c.current_dir(d);
    }
    let mut child = c.spawn().map_err(|e| format!("{program} : {e}"))?;
    let (out, err) = (drain(child.stdout.take()), drain(child.stderr.take()));
    let start = Instant::now();
    let status = loop {
        match child.try_wait() {
            Ok(Some(s)) => break s,
            Ok(None) if start.elapsed() > limit => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(crate::i18n::tf("{p} : pas de réponse après {n} s", &[("p", &program), ("n", &limit.as_secs())]));
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(5)),
            Err(e) => return Err(format!("{program} : {e}")),
        }
    };
    let grace = Duration::from_millis(200);
    let stdout = out.take(grace);
    if status.success() {
        Ok(stdout)
    } else {
        let stderr = err.take(grace);
        Err(format!("{program} : {}", stderr.trim().lines().last().unwrap_or(&crate::i18n::t("échec"))))
    }
}

/// Whether a program is on the PATH.
pub fn has(program: &str) -> bool {
    let mut c = Command::new(if cfg!(windows) { "where" } else { "sh" });
    if cfg!(windows) { c.args(["/q", program]) } else { c.args(["-c", "command -v \"$1\" >/dev/null", "sh", program]) };
    quiet(&mut c).status().is_ok_and(|s| s.success())
}

/// Open a file, folder or URL with the default application.
pub fn open(target: &str) -> Result<(), String> {
    #[cfg(unix)]
    return spawn("gio", &["open".into(), target.into()], None);
    // Explorer hands files, folders, URLs and .lnk shortcuts to their default handler.
    #[cfg(windows)]
    spawn("explorer", &[if target.contains("://") { target.into() } else { target.replace('/', "\\") }], None)
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

    #[cfg(unix)]
    #[test]
    fn output_does_not_wait_for_what_the_program_starts() {
        // The program exits at once but leaves a child holding stdout for 5 s.
        let start = Instant::now();
        let out = output("sh", &["-c", "echo ok; sleep 5 &"], None).unwrap();
        assert_eq!(out.trim(), "ok");
        assert!(start.elapsed() < Duration::from_secs(2), "{:?}", start.elapsed());
    }

    #[cfg(unix)]
    #[test]
    fn output_is_killed_after_its_limit() {
        let start = Instant::now();
        assert!(output_within("sleep", &["5"], None, Duration::from_millis(300)).is_err());
        assert!(start.elapsed() < Duration::from_secs(2));
        assert!(output("sh", &["-c", "echo boom >&2; exit 3"], None).unwrap_err().contains("boom"));
    }

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
