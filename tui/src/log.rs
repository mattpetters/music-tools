//! Log line types shared between the App's on-screen log pane, the job
//! manager's per-job log buffer, and the process layer's child output
//! streaming. Defined in its own module to avoid a circular import
//! between `app` and `jobs`.

use chrono::{DateTime, Local};

/// Visual stream classification for a log line. Drives color and glyph.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogStream {
    /// App-emitted informational line (cyan ·).
    Info,
    /// Child process stdout (white ›).
    Out,
    /// Child process stderr (yellow !).
    Err,
    /// Job finished successfully (green ✓).
    Success,
    /// Job failed (red ✗).
    Failure,
}

impl LogStream {
    pub fn badge(self) -> &'static str {
        match self {
            LogStream::Info => "·",
            LogStream::Out => "›",
            LogStream::Err => "!",
            LogStream::Success => "✓",
            LogStream::Failure => "✗",
        }
    }
}

/// A single log line: timestamp + stream + message.
#[derive(Debug, Clone)]
pub struct LogLine {
    pub timestamp: DateTime<Local>,
    pub stream: LogStream,
    pub message: String,
}
