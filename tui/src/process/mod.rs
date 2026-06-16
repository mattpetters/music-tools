//! Child process spawning and streaming.
//!
//! `spawn_with_sudo()` wraps a [`CommandSpec`] in `sudo -A -E` (using
//! `SUDO_ASKPASS` to pop the macOS password dialog instead of reading from
//! the tty), and returns:
//!   - a [`ChildHandle`] that owns the spawned child
//!   - an `mpsc::Receiver<LogEvent>` that yields each line of stdout/stderr
//!     in real time, classified by stream
//!
//! See [`cancel`] for SIGTERM → SIGKILL escalation.

use std::path::PathBuf;
use std::process::Stdio;

use color_eyre::eyre::Result;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::{Child, Command};
use tokio::sync::mpsc;

use crate::log::LogStream;

pub mod cancel;

/// A spec for a child process: program, args, env, cwd.
#[derive(Debug, Clone)]
pub struct CommandSpec {
    pub program: String,
    pub args: Vec<String>,
    pub env: Vec<(String, String)>,
    pub cwd: Option<PathBuf>,
}

impl CommandSpec {
    pub fn new(program: impl Into<String>) -> Self {
        Self {
            program: program.into(),
            args: Vec::new(),
            env: Vec::new(),
            cwd: None,
        }
    }

    #[allow(dead_code)] // M3 actions use this; M2 only uses args().
    pub fn arg(mut self, a: impl Into<String>) -> Self {
        self.args.push(a.into());
        self
    }

    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.args.extend(args.into_iter().map(Into::into));
        self
    }

    #[allow(dead_code)] // M3 actions use this.
    pub fn env(mut self, key: impl Into<String>, val: impl Into<String>) -> Self {
        self.env.push((key.into(), val.into()));
        self
    }

    #[allow(dead_code)] // M3 actions use this.
    pub fn cwd(mut self, path: impl Into<PathBuf>) -> Self {
        self.cwd = Some(path.into());
        self
    }
}

/// A line of output from a child process, classified by stream.
#[derive(Debug, Clone)]
pub struct LogEvent {
    pub stream: LogStream,
    pub line: String,
}

/// Handle to a running child. Drop = SIGKILL (via `kill_on_drop`).
pub struct ChildHandle {
    pub child: Child,
    pub pid: Option<u32>,
}

impl ChildHandle {
    /// Wait for the child to exit naturally. Returns the exit status.
    #[allow(dead_code)] // used by tests; production code uses JobManager::poll
    pub async fn wait(&mut self) -> Result<std::process::ExitStatus> {
        Ok(self.child.wait().await?)
    }

    /// Best-effort cancel: SIGTERM the process group, then SIGKILL after
    /// `grace` if it's still alive. Idempotent. See [`cancel::cancel`].
    pub async fn cancel(self, grace: std::time::Duration) -> Result<()> {
        cancel::cancel(self, grace).await
    }
}

/// Spawn a `CommandSpec` wrapped in `sudo -A -E`, with stdout/stderr piped
/// into a channel of [`LogEvent`]s. The returned [`ChildHandle`] can be
/// awaited for completion or cancelled.
pub fn spawn_with_sudo(
    spec: CommandSpec,
    askpass: &std::path::Path,
) -> Result<(ChildHandle, mpsc::Receiver<LogEvent>)> {
    let mut cmd = Command::new("sudo");
    cmd.arg("-A")
        .arg("-E")
        .arg(&spec.program)
        .args(&spec.args)
        .env("SUDO_ASKPASS", askpass)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .process_group(0);

    for (k, v) in &spec.env {
        cmd.env(k, v);
    }
    if let Some(cwd) = &spec.cwd {
        cmd.current_dir(cwd);
    }

    let mut child = cmd.spawn()?;
    let pid = child.id();

    let (tx, rx) = mpsc::channel(256);

    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(pump_lines(
            BufReader::new(stdout),
            LogStream::Out,
            tx.clone(),
        ));
    }
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(pump_lines(
            BufReader::new(stderr),
            LogStream::Err,
            tx.clone(),
        ));
    }
    // Drop the sender in this task so the receiver finishes when both
    // reader tasks exit.
    drop(tx);

    Ok((ChildHandle { child, pid }, rx))
}

/// Spawn an unsudued command (for tests and the no-sudo `TestAction`).
pub fn spawn_bare(spec: CommandSpec) -> Result<(ChildHandle, mpsc::Receiver<LogEvent>)> {
    let mut cmd = Command::new(&spec.program);
    cmd.args(&spec.args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    for (k, v) in &spec.env {
        cmd.env(k, v);
    }
    if let Some(cwd) = &spec.cwd {
        cmd.current_dir(cwd);
    }

    let mut child = cmd.spawn()?;
    let pid = child.id();

    let (tx, rx) = mpsc::channel(256);

    if let Some(stdout) = child.stdout.take() {
        tokio::spawn(pump_lines(
            BufReader::new(stdout),
            LogStream::Out,
            tx.clone(),
        ));
    }
    if let Some(stderr) = child.stderr.take() {
        tokio::spawn(pump_lines(
            BufReader::new(stderr),
            LogStream::Err,
            tx.clone(),
        ));
    }
    drop(tx);

    Ok((ChildHandle { child, pid }, rx))
}

async fn pump_lines<R>(reader: R, stream: LogStream, tx: mpsc::Sender<LogEvent>)
where
    R: tokio::io::AsyncRead + Unpin + Send + 'static,
{
    let mut buf = BufReader::new(reader);
    let mut line = String::new();
    loop {
        line.clear();
        match buf.read_line(&mut line).await {
            Ok(0) => break, // EOF
            Ok(_) => {
                let trimmed = line.trim_end_matches(['\n', '\r']);
                if tx
                    .send(LogEvent {
                        stream,
                        line: trimmed.to_string(),
                    })
                    .await
                    .is_err()
                {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A bare `bash -c 'echo hello; echo world >&2; exit 0'` should produce
    /// exactly one OUT line and one ERR line and exit 0.
    #[tokio::test]
    async fn bare_spawn_streams_both_streams() {
        let spec = CommandSpec::new("bash").args(["-c", "echo hello; echo world >&2; exit 0"]);
        let (mut handle, mut rx) = spawn_bare(spec).expect("spawn");

        // Wait for the child to exit naturally, then drop the handle so the
        // stdout/stderr pipes close and the pump_lines tasks reach EOF.
        let status = handle.wait().await.expect("wait");
        drop(handle);

        // Now drain all log events until the channel closes.
        let mut out_lines = Vec::new();
        let mut err_lines = Vec::new();
        while let Some(ev) = rx.recv().await {
            match ev.stream {
                LogStream::Out => out_lines.push(ev.line),
                LogStream::Err => err_lines.push(ev.line),
                _ => {}
            }
        }

        assert!(status.success(), "exit status should be 0");
        assert!(
            out_lines.contains(&"hello".to_string()),
            "stdout: {out_lines:?}"
        );
        assert!(
            err_lines.contains(&"world".to_string()),
            "stderr: {err_lines:?}"
        );
    }

    /// A command that exits non-zero should be observable via the exit status.
    #[tokio::test]
    async fn bare_spawn_preserves_exit_code() {
        let spec = CommandSpec::new("bash").args(["-c", "exit 42"]);
        let (mut handle, mut _rx) = spawn_bare(spec).expect("spawn");
        let status = handle.wait().await.expect("wait");
        assert_eq!(status.code(), Some(42));
    }
}
