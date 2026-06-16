//! Job lifecycle: spawn, stream, wait, cancel, history.

pub mod job;

use std::collections::VecDeque;

use color_eyre::eyre::Result;
use tokio::sync::mpsc;

use crate::actions::{Action, ResolvedInputs};
use crate::log::LogLine;
use crate::process::{spawn_bare, spawn_with_sudo, ChildHandle, LogEvent};

pub use job::{Job, JobStatus};

/// Owns the lifecycle of all jobs. The [`App`](crate::app::App) holds one
/// of these and drains log events from the current job on each tick.
///
/// In M2 the JobManager supports a single running job at a time and a
/// small history ring. Concurrent jobs are out of scope per spec §5.6.
pub struct JobManager {
    pub current: Option<RunningJob>,
    pub history: VecDeque<Job>,
}

const MAX_HISTORY: usize = 50;

pub struct RunningJob {
    pub job: Job,
    pub handle: Option<ChildHandle>,
    pub rx: mpsc::Receiver<LogEvent>,
    /// All log lines captured so far. The App drains this via
    /// [`JobManager::take_new_logs`]. On job completion, the full buffer
    /// is moved to [`Job::log_buffer`] for history.
    pub log_buffer: Vec<LogLine>,
    /// Index of the next line in `log_buffer` that the App hasn't seen.
    /// `take_new_logs` returns `log_buffer[drained_to..]` and advances
    /// `drained_to`.
    pub drained_to: usize,
}

impl JobManager {
    pub fn new() -> Self {
        Self {
            current: None,
            history: VecDeque::new(),
        }
    }

    pub fn is_running(&self) -> bool {
        self.current.is_some()
    }

    /// Start a new job. Errors if one is already running.
    pub async fn start(&mut self, action: &dyn Action, inputs: ResolvedInputs) -> Result<()> {
        if self.is_running() {
            return Err(color_eyre::eyre::eyre!(
                "a job is already running; cancel it first"
            ));
        }

        let info = action.info();
        let spec = action.build_command(inputs);

        let askpass = crate::sudo::ensure_askpass()?;
        let (handle, rx) = if action.requires_sudo() {
            spawn_with_sudo(spec, &askpass)?
        } else {
            spawn_bare(spec)?
        };

        let job = Job::new(info.id);
        self.current = Some(RunningJob {
            job,
            handle: Some(handle),
            rx,
            log_buffer: Vec::new(),
            drained_to: 0,
        });
        Ok(())
    }

    /// Cancel the current job (if any). Idempotent.
    pub async fn cancel_current(&mut self) -> Result<()> {
        if let Some(mut running) = self.current.take() {
            if let Some(handle) = running.handle.take() {
                handle.cancel(std::time::Duration::from_secs(3)).await?;
            }
            // Drain any remaining log events.
            while let Ok(ev) = running.rx.try_recv() {
                running.log_buffer.push(crate::log::LogLine::from_event(ev));
            }
            running.job.complete(JobStatus::Cancelled, None);
            self.history.push_front(running.job);
            self.trim_history();
        }
        Ok(())
    }

    /// Non-blocking poll: check whether the current job has exited and
    /// update state. Returns the job's new status if it just finished,
    /// else None.
    ///
    /// Log draining is the responsibility of [`take_new_logs`]; the App
    /// calls that first so the App's on-screen log pane sees every line
    /// before the buffer moves to history.
    pub fn poll(&mut self) -> Result<Option<JobStatus>> {
        let mut running = match self.current.take() {
            Some(r) => r,
            None => return Ok(None),
        };

        if let Some(handle) = running.handle.as_mut() {
            match handle.child.try_wait() {
                Ok(Some(status)) => {
                    // Final drain in case lines arrived between the last
                    // take_new_logs and the child's exit.
                    while let Ok(ev) = running.rx.try_recv() {
                        running.log_buffer.push(LogLine::from_event(ev));
                    }
                    let final_status = if status.success() {
                        JobStatus::Done
                    } else {
                        JobStatus::Failed
                    };
                    running.job.complete(final_status, status.code());
                    // Move the buffer to the history entry so the M7
                    // history view can show the full transcript.
                    let mut job = running.job;
                    job.log_buffer = std::mem::take(&mut running.log_buffer);
                    self.history.push_front(job);
                    self.trim_history();
                    return Ok(Some(final_status));
                }
                Ok(None) => {
                    // Still running.
                    self.current = Some(running);
                }
                Err(e) => {
                    self.current = Some(running);
                    return Err(color_eyre::eyre::eyre!("try_wait failed: {e}"));
                }
            }
        } else {
            // No handle (shouldn't happen, but be safe).
            self.current = Some(running);
        }
        Ok(None)
    }

    /// Take log lines the App hasn't seen yet. The App calls this once
    /// per tick to populate the on-screen log pane.
    ///
    /// As a side effect, drains any pending events from the current
    /// job's channel into the buffer. This means the App sees every
    /// line even on the tick the job finishes (the next `poll()` then
    /// moves the (now-empty-from-App's-perspective) buffer to history).
    pub fn take_new_logs(&mut self) -> Vec<LogLine> {
        if let Some(running) = self.current.as_mut() {
            // Drain any pending log events.
            while let Ok(ev) = running.rx.try_recv() {
                running.log_buffer.push(LogLine::from_event(ev));
            }
            if running.drained_to < running.log_buffer.len() {
                let new: Vec<LogLine> = running.log_buffer[running.drained_to..].to_vec();
                running.drained_to = running.log_buffer.len();
                return new;
            }
        }
        Vec::new()
    }

    fn trim_history(&mut self) {
        while self.history.len() > MAX_HISTORY {
            self.history.pop_back();
        }
    }
}

impl Default for JobManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::actions::TestAction;
    use crate::log::LogStream;

    #[tokio::test]
    async fn start_and_finish_a_test_job() {
        let mut mgr = JobManager::new();
        let action = TestAction::new("echo hi");
        mgr.start(&action, ResolvedInputs::default())
            .await
            .expect("start");
        assert!(mgr.is_running());

        // Poll until the job finishes.
        let mut finished = false;
        for _ in 0..200 {
            tokio::time::sleep(std::time::Duration::from_millis(25)).await;
            if let Ok(Some(status)) = mgr.poll() {
                assert_eq!(status, JobStatus::Done);
                finished = true;
                break;
            }
        }
        assert!(finished, "job did not finish in time");
        assert!(!mgr.is_running());
        assert_eq!(mgr.history.len(), 1);
        assert_eq!(mgr.history[0].status, JobStatus::Done);
    }

    #[tokio::test]
    async fn cancel_running_job() {
        let mut mgr = JobManager::new();
        let action = TestAction::new("sleep 30");
        mgr.start(&action, ResolvedInputs::default())
            .await
            .expect("start");
        assert!(mgr.is_running());

        mgr.cancel_current().await.expect("cancel");
        assert!(!mgr.is_running());
        assert_eq!(mgr.history.len(), 1);
        assert_eq!(mgr.history[0].status, JobStatus::Cancelled);
    }

    #[tokio::test]
    async fn take_new_logs_returns_lines_then_empty() {
        let mut mgr = JobManager::new();
        let action = TestAction::new("echo line1; echo line2; echo line3");
        mgr.start(&action, ResolvedInputs::default())
            .await
            .expect("start");

        let mut saw_lines = false;
        for _ in 0..100 {
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
            let logs = mgr.take_new_logs();
            if !logs.is_empty() {
                assert!(logs.iter().all(|l| l.stream == LogStream::Out));
                saw_lines = true;
            }
            if let Ok(Some(_status)) = mgr.poll() {
                assert!(saw_lines, "should have seen log lines before completion");
                let again = mgr.take_new_logs();
                assert!(again.is_empty(), "no logs after completion");
                return;
            }
        }
        panic!("did not observe expected log lines or job completion");
    }
}
