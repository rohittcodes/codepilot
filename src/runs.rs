use std::path::Path;
use std::process::Stdio;
use std::time::Duration;

use anyhow::Result;
use tokio::process::Command;
use tokio::time::timeout;

/// What a `Run` executes. Closed on purpose — see PLAN.md non-goals: this is not
/// a generic task-runner, just the fixed set of JS/TS verification gates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunKind {
    TypeCheck,
}

/// Outcome of a single `Run`.
#[derive(Debug, Clone)]
pub enum RunStatus {
    Succeeded,
    Failed(String),
}

const RUN_TIMEOUT: Duration = Duration::from_secs(120);

/// Execute the given gate against `repo_path`, bounded by `RUN_TIMEOUT`.
pub async fn execute(kind: RunKind, repo_path: &Path) -> Result<RunStatus> {
    match kind {
        RunKind::TypeCheck => run_tsc(repo_path).await,
    }
}

async fn run_tsc(repo_path: &Path) -> Result<RunStatus> {
    let mut cmd = tsc_command(repo_path);
    cmd.current_dir(repo_path)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let child = cmd.spawn()?;
    let output = match timeout(RUN_TIMEOUT, child.wait_with_output()).await {
        Ok(result) => result?,
        Err(_) => {
            return Ok(RunStatus::Failed(format!(
                "tsc timed out after {}s",
                RUN_TIMEOUT.as_secs()
            )))
        }
    };

    if output.status.success() {
        Ok(RunStatus::Succeeded)
    } else {
        let combined = format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        Ok(RunStatus::Failed(combined.trim().to_string()))
    }
}

/// Prefer the repo's own `node_modules/.bin/tsc` (no network resolution, exact
/// pinned version); fall back to `npx tsc`. On Windows, `.cmd` shims must be
/// run through `cmd /C` since `CreateProcess` doesn't resolve `PATHEXT` itself.
fn tsc_command(repo_path: &Path) -> Command {
    let local_tsc = repo_path
        .join("node_modules")
        .join(".bin")
        .join(if cfg!(windows) { "tsc.cmd" } else { "tsc" });

    if local_tsc.exists() {
        if cfg!(windows) {
            let mut cmd = Command::new("cmd");
            cmd.args(["/C", local_tsc.to_string_lossy().as_ref(), "--noEmit"]);
            cmd
        } else {
            let mut cmd = Command::new(local_tsc);
            cmd.arg("--noEmit");
            cmd
        }
    } else if cfg!(windows) {
        let mut cmd = Command::new("cmd");
        cmd.args(["/C", "npx", "tsc", "--noEmit"]);
        cmd
    } else {
        let mut cmd = Command::new("npx");
        cmd.args(["tsc", "--noEmit"]);
        cmd
    }
}
