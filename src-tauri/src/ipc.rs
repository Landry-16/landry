//! CLI <-> resident process link over a Unix socket. One request line, one reply line.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::{UnixListener, UnixStream};
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

pub fn socket_path() -> PathBuf {
    let dir = std::env::var_os("XDG_RUNTIME_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    dir.join("landry.sock")
}

/// A request sent by the CLI: the command words plus the activation token, if the
/// launcher (GNOME shortcut, terminal) gave us one.
#[derive(Debug, PartialEq)]
pub struct Request {
    pub args: Vec<String>,
    pub token: String,
}

impl Request {
    pub fn encode(&self) -> String {
        // Tab separated: token first, then args. Args never contain tabs or newlines.
        let mut line = self.token.replace(['\t', '\n'], " ");
        for a in &self.args {
            line.push('\t');
            line.push_str(&a.replace(['\t', '\n'], " "));
        }
        line.push('\n');
        line
    }

    pub fn decode(line: &str) -> Request {
        let mut parts = line.trim_end_matches('\n').split('\t');
        let token = parts.next().unwrap_or("").to_string();
        let args = parts.filter(|s| !s.is_empty()).map(String::from).collect();
        Request { args, token }
    }
}

fn send(line: &str) -> std::io::Result<String> {
    let mut stream = UnixStream::connect(socket_path())?;
    stream.set_read_timeout(Some(Duration::from_secs(5)))?;
    stream.write_all(line.as_bytes())?;
    let mut reply = String::new();
    BufReader::new(stream).read_line(&mut reply)?;
    Ok(reply.trim_end().to_string())
}

fn spawn_daemon() -> std::io::Result<()> {
    Command::new(std::env::current_exe()?)
        .arg("daemon")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .process_group(0)
        .spawn()?;
    Ok(())
}

/// Entry point for every non-daemon invocation. Returns the process exit code.
pub fn client(args: &[String]) -> i32 {
    let args: Vec<String> = if args.is_empty() { vec!["toggle".into()] } else { args.to_vec() };
    let token = std::env::var("XDG_ACTIVATION_TOKEN")
        .or_else(|_| std::env::var("DESKTOP_STARTUP_ID"))
        .unwrap_or_default();
    let line = Request { args: args.clone(), token }.encode();
    let start = Instant::now();

    let reply = match send(&line) {
        Ok(r) => Ok(r),
        Err(_) => {
            if let Err(e) = spawn_daemon() {
                eprintln!("landry: impossible de démarrer le processus résident : {e}");
                return 1;
            }
            // Wait for the socket of the freshly started daemon.
            let deadline = Instant::now() + Duration::from_secs(10);
            loop {
                std::thread::sleep(Duration::from_millis(50));
                match send(&line) {
                    Ok(r) => break Ok(r),
                    Err(e) if Instant::now() > deadline => break Err(e),
                    Err(_) => {}
                }
            }
        }
    };

    match reply {
        Ok(r) => {
            if std::env::var_os("LANDRY_TIMING").is_some() {
                eprintln!("landry: {} ms aller-retour", start.elapsed().as_millis());
            }
            if let Some(err) = r.strip_prefix("err ") {
                eprintln!("landry: {err}");
                return 1;
            }
            let body = r.strip_prefix("ok").unwrap_or(&r).trim();
            if !body.is_empty() {
                println!("{body}");
            }
            0
        }
        Err(e) => {
            eprintln!("landry: pas de réponse du processus résident : {e}");
            1
        }
    }
}

/// Bind the socket, refusing to start if another daemon already answers on it.
pub fn bind() -> std::io::Result<UnixListener> {
    let path = socket_path();
    if UnixStream::connect(&path).is_ok() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::AddrInUse,
            "un processus résident Landry tourne déjà",
        ));
    }
    let _ = std::fs::remove_file(&path); // stale socket from a crash
    UnixListener::bind(&path)
}

/// Serve requests forever; `handle` returns the reply line (without newline).
pub fn serve(listener: UnixListener, handle: impl Fn(Request) -> String + Send + Sync + 'static) {
    let handle = std::sync::Arc::new(handle);
    for stream in listener.incoming().flatten() {
        let handle = handle.clone();
        std::thread::spawn(move || {
            let mut line = String::new();
            let mut reader = BufReader::new(&stream);
            if reader.read_line(&mut line).is_err() {
                return;
            }
            let reply = handle(Request::decode(&line));
            let _ = (&stream).write_all(format!("{}\n", reply.replace('\n', " ")).as_bytes());
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_roundtrip() {
        let r = Request { args: vec!["run".into(), "deploy nacre".into()], token: "abc".into() };
        assert_eq!(Request::decode(&r.encode()), r);
    }

    #[test]
    fn empty_token_and_sanitized_args() {
        let r = Request { args: vec!["a\tb".into()], token: String::new() };
        let d = Request::decode(&r.encode());
        assert_eq!(d.token, "");
        assert_eq!(d.args, vec!["a b".to_string()]);
    }
}
