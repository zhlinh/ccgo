//! Subprocess execution with timeout support

#![allow(dead_code)]

use std::process::{Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

/// Result of a subprocess execution
#[derive(Debug)]
pub struct CommandResult {
    /// Whether the command succeeded (exit code 0)
    pub success: bool,

    /// Process exit code
    pub exit_code: i32,

    /// Captured standard output
    pub stdout: String,

    /// Captured standard error
    pub stderr: String,

    /// Execution duration
    pub duration: Duration,
}

impl CommandResult {
    /// Create a CommandResult from an exit status
    pub fn from_status(status: ExitStatus, stdout: String, stderr: String, duration: Duration) -> Self {
        let exit_code = status.code().unwrap_or(-1);
        Self {
            success: status.success(),
            exit_code,
            stdout,
            stderr,
            duration,
        }
    }
}

/// Run a command with optional timeout
pub fn run_command(
    program: &str,
    args: &[String],
    inherit_io: bool,
    _timeout: Option<Duration>,
) -> Result<CommandResult> {
    let start = Instant::now();

    let mut cmd = Command::new(program);
    cmd.args(args);

    if inherit_io {
        // Inherit stdin/stdout/stderr for interactive commands
        cmd.stdin(Stdio::inherit());
        cmd.stdout(Stdio::inherit());
        cmd.stderr(Stdio::inherit());

        let status = cmd
            .status()
            .with_context(|| format!("Failed to execute {}", program))?;

        let duration = start.elapsed();
        Ok(CommandResult::from_status(
            status,
            String::new(),
            String::new(),
            duration,
        ))
    } else {
        // Capture output
        let output = cmd
            .output()
            .with_context(|| format!("Failed to execute {}", program))?;

        let duration = start.elapsed();
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        Ok(CommandResult::from_status(
            output.status,
            stdout,
            stderr,
            duration,
        ))
    }
}

/// Check if a command exists in PATH
pub fn command_exists(program: &str) -> bool {
    which::which(program).is_ok()
}
