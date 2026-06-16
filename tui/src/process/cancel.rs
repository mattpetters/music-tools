//! Child process cancellation: SIGTERM the process group, wait up to a
//! grace period, then SIGKILL.
//!
//! Requires the child to be the leader of its own process group (set via
//! `tokio::process::Command::process_group(0)` when spawning). On macOS
//! and Linux this puts the child in a new pgrp whose pgid == child pid.

use std::time::Duration;

use color_eyre::eyre::Result;
use nix::sys::signal::{killpg, Signal};
use nix::unistd::Pid;

use super::ChildHandle;

/// Cancel a child: SIGTERM, wait up to `grace`, then SIGKILL.
/// Idempotent. Returns once the child has been reaped.
pub async fn cancel(mut handle: ChildHandle, grace: Duration) -> Result<()> {
    let pid = match handle.pid {
        Some(p) => p,
        None => {
            // We don't have the pid (child reaped between spawn and cancel).
            // Best we can do is reap it.
            let _ = handle.child.wait().await;
            return Ok(());
        }
    };

    // Send SIGTERM to the process group. If the child is not actually a
    // process group leader (process_group(0) failed or we're on a platform
    // where it isn't supported), killpg returns ESRCH; we fall back to
    // a direct kill on the pid.
    if killpg(Pid::from_raw(pid as i32), Signal::SIGTERM).is_err() {
        let _ = nix::sys::signal::kill(Pid::from_raw(pid as i32), Signal::SIGTERM);
    }

    // Poll for exit up to `grace`.
    let poll = Duration::from_millis(100);
    let mut waited = Duration::ZERO;
    while waited < grace {
        if matches!(handle.child.try_wait(), Ok(Some(_))) {
            return Ok(());
        }
        tokio::time::sleep(poll).await;
        waited += poll;
    }

    // Still alive — SIGKILL.
    handle.child.start_kill()?;
    let _ = handle.child.wait().await;
    Ok(())
}
