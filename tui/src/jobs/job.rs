//! Job value type. A `Job` represents a single execution of an action,
//! from start to finish.

use chrono::{DateTime, Local};
use std::sync::atomic::{AtomicU64, Ordering};

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
#[allow(dead_code)] // id/started_at/log_buffer are surfaced in the M7 history view.
pub struct Job {
    pub id: u64,
    pub action_id: String,
    pub status: JobStatus,
    pub started_at: DateTime<Local>,
    pub finished_at: Option<DateTime<Local>>,
    pub exit_code: Option<i32>,
    /// Captured log lines, in arrival order. Includes child stdout/stderr
    /// and any app-emitted Info/Success/Failure lines.
    pub log_buffer: Vec<LogLine>,
}

impl Job {
    pub fn new(action_id: &str) -> Self {
        Self {
            id: next_job_id(),
            action_id: action_id.to_string(),
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
