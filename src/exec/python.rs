//! Python script invocation for ccgo build scripts

#![allow(dead_code)]

use std::path::Path;
use std::process::{Command, ExitStatus};

use anyhow::{bail, Context, Result};

use super::subprocess::{command_exists, run_command, CommandResult};

/// Python runner for executing ccgo Python CLI
pub struct PythonRunner {
    /// ccgo command path
    ccgo_path: String,
    /// Python interpreter path (for build scripts)
    python_path: String,
}

impl PythonRunner {
    /// Create a new Python runner
    pub fn new() -> Result<Self> {
        let ccgo_path = Self::find_ccgo()?;
        let python_path = Self::find_python()?;
        Ok(Self { ccgo_path, python_path })
    }

    /// Find ccgo command
    fn find_ccgo() -> Result<String> {
        if command_exists("ccgo") {
            return Ok("ccgo".to_string());
        }
        bail!("ccgo Python CLI not found. Please install: pip install ccgo")
    }

    /// Find Python 3 interpreter
    fn find_python() -> Result<String> {
        // Try python3 first, then python
        for python in &["python3", "python"] {
            if command_exists(python) {
                // Verify it's Python 3
                let result = run_command(python, &["--version".to_string()], false, None)?;
                if result.success && result.stdout.contains("Python 3") {
                    return Ok(python.to_string());
                }
            }
        }

        bail!("Python 3 not found. Please install Python 3.8 or later.")
    }

    /// Run the ccgo Python CLI with the given arguments
    pub fn run_ccgo(&self, args: &[String], verbose: bool) -> Result<CommandResult> {
        let mut full_args = Vec::new();

        if verbose {
            full_args.push("-v".to_string());
        }

        full_args.extend(args.iter().cloned());

        if verbose {
            eprintln!(
                "Executing: {} {}",
                self.ccgo_path,
                full_args.join(" ")
            );
        }

        run_command(&self.ccgo_path, &full_args, true, None)
            .context("Failed to execute ccgo Python CLI")
    }

    /// Run a specific Python build script
    pub fn run_build_script(
        &self,
        script: &str,
        args: &[String],
        verbose: bool,
    ) -> Result<CommandResult> {
        let mut full_args = vec![script.to_string()];
        full_args.extend(args.iter().cloned());

        if verbose {
            eprintln!(
                "Executing: {} {}",
                self.python_path,
                full_args.join(" ")
            );
        }

        run_command(&self.python_path, &full_args, true, None)
            .context("Failed to execute Python build script")
    }
}

/// Run a Python script directly
///
/// This is a simpler alternative to PythonRunner for one-off script execution.
pub fn run_python_script<P: AsRef<Path>>(
    script: P,
    args: &[&str],
    cwd: Option<P>,
) -> Result<ExitStatus> {
    // Find Python 3 interpreter
    let python = if command_exists("python3") {
        "python3"
    } else if command_exists("python") {
        "python"
    } else {
        bail!("Python 3 not found. Please install Python 3.8 or later.");
    };

    let mut cmd = Command::new(python);
    cmd.arg(script.as_ref());
    cmd.args(args);

    if let Some(dir) = cwd {
        cmd.current_dir(dir.as_ref());
    }

    // Inherit stdout/stderr to show build progress
    let status = cmd.status().context("Failed to execute Python script")?;

    Ok(status)
}
