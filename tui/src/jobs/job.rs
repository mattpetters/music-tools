//! Job value type. A `Job` represents a single execution of an action,
//! from start to finish.

use chrono::{DateTime, Local};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::actions::ResolvedInputs;
use crate::log::LogLine;
use crate::process::LogEvent;

/// Lifecycle status of a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JobStatus {
    /// Spawned but not yet observed to finish.
    Running,
    /// Exited 0.
    Done,
    /// Exited non-zero.
    Failed,
    /// Cancelled by the user (SIGTERM/SIGKILL).
    Cancelled,
}

impl JobStatus {
    #[allow(dead_code)] // Used in M7 history view.
    pub fn badge(self) -> &'static str {
        match self {
            JobStatus::Running => "…",
            JobStatus::Done => "✓",
            JobStatus::Failed => "✗",
            JobStatus::Cancelled => "↻",
        }
    }
}

/// A finished or running job.
#[derive(Debug, Clone)]
pub struct Job {
    pub id: u64,
    pub action_id: String,
    /// The inputs the user resolved when starting this job. Persisted
    /// on the Job (rather than re-resolved from the browser state) so
    /// the M7 history view can re-run jobs with the exact same inputs.
    pub inputs: ResolvedInputs,
    pub status: JobStatus,
    pub started_at: DateTime<Local>,
    pub finished_at: Option<DateTime<Local>>,
    pub exit_code: Option<i32>,
    /// Captured log lines, in arrival order. Includes child stdout/stderr
    /// and any app-emitted Info/Success/Failure lines.
    pub log_buffer: Vec<LogLine>,
}

impl Job {
    pub fn new(action_id: &str, inputs: ResolvedInputs) -> Self {
        Self {
            id: next_job_id(),
            action_id: action_id.to_string(),
            inputs,
            status: JobStatus::Running,
            started_at: Local::now(),
            finished_at: None,
            exit_code: None,
            log_buffer: Vec::new(),
        }
    }

    pub fn complete(&mut self, status: JobStatus, exit_code: Option<i32>) {
        self.status = status;
        self.finished_at = Some(Local::now());
        self.exit_code = exit_code;
    }

    /// Write the job's log buffer to `~/Library/Logs/mtui/<id>.log`.
    /// Best-effort; returns Err on IO failure but the caller is free
    /// to ignore.
    pub fn write_log_file(&self) -> std::io::Result<()> {
        let Some(dir) = log_dir() else {
            return Ok(());
        };
        std::fs::create_dir_all(&dir)?;
        let path = dir.join(format!("{}.log", self.id));
        let mut content = String::new();
        content.push_str(&format!(
            "# Job {} ({}): {}\n",
            self.id,
            self.action_id,
            status_label(self.status)
        ));
        if let Some(code) = self.exit_code {
            content.push_str(&format!("# exit code: {code}\n"));
        }
        for line in &self.log_buffer {
            content.push_str(&format!(
                "[{}] [{}] {}\n",
                line.timestamp.format("%Y-%m-%d %H:%M:%S"),
                line.stream.badge(),
                line.message
            ));
        }
        std::fs::write(path, content)
    }
}

fn log_dir() -> Option<PathBuf> {
    dirs::home_dir().map(|h| h.join("Library").join("Logs").join("mtui"))
}

fn status_label(s: JobStatus) -> &'static str {
    match s {
        JobStatus::Running => "running",
        JobStatus::Done => "done",
        JobStatus::Failed => "failed",
        JobStatus::Cancelled => "cancelled",
    }
}

/// Monotonic job ID.
fn next_job_id() -> u64 {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    COUNTER.fetch_add(1, Ordering::Relaxed)
}

// Bridge from process::LogEvent to log::LogLine, defined here because
// `Job::log_buffer` uses `LogLine` and `pump_lines` produces `LogEvent`.
impl LogLine {
    pub fn from_event(ev: LogEvent) -> Self {
        Self {
            timestamp: Local::now(),
            stream: ev.stream,
            message: ev.line,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn job_round_trip() {
        let j = Job::new("test", ResolvedInputs::default());
        assert_eq!(j.action_id, "test");
        assert_eq!(j.status, JobStatus::Running);
        assert!(j.finished_at.is_none());
    }

    #[test]
    fn write_log_file_creates_file() {
        let dir = tempfile::tempdir().expect("tempdir");
        // Override home to a tempdir by creating a fake $HOME.
        // (log_dir uses dirs::home_dir which respects $HOME on Unix.)
        std::env::set_var("HOME", dir.path());
        let j = Job::new("test", ResolvedInputs::default());
        // Manually inject a log line.
        let mut j = j;
        j.log_buffer.push(LogLine {
            timestamp: Local::now(),
            stream: crate::log::LogStream::Info,
            message: "hello".to_string(),
        });
        j.write_log_file().expect("write log");
        let path = dir
            .path()
            .join("Library")
            .join("Logs")
            .join("mtui")
            .join(format!("{}.log", j.id));
        assert!(path.exists(), "log file should exist at {path:?}");
        let content = std::fs::read_to_string(&path).unwrap();
        assert!(content.contains("hello"));
    }
}
