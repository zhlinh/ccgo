//! Init command implementation

use anyhow::{bail, Context, Result};
use clap::Args;
use std::env;

use crate::exec::subprocess::{command_exists, run_command};

/// Default CCGO template from GitHub
const DEFAULT_TEMPLATE: &str = "https://github.com/zhlinh/ccgo-template";

/// Initialize library project in current directory
#[derive(Args, Debug)]
pub struct InitCommand {
    /// Use default values (no prompts)
    #[arg(long)]
    pub defaults: bool,

    /// Custom template URL or path
    #[arg(long)]
    pub template: Option<String>,

    /// Use latest template version (HEAD)
    #[arg(long)]
    pub use_latest: bool,
}

impl InitCommand {
    /// Execute the init command
    pub fn execute(self, verbose: bool) -> Result<()> {
        // Check if copier is installed
        if !command_exists("copier") {
            bail!(
                "Copier not found. Please install it:\n\n\
                 pip install copier\n\
                 # or\n\
                 pipx install copier"
            );
        }

        // Determine template source
        let template_src = self.template.as_deref().unwrap_or(DEFAULT_TEMPLATE);

        // Get current directory name for project name
        let current_dir = env::current_dir()
            .context("Failed to get current directory")?;
        let dir_name = current_dir
            .file_name()
            .context("Failed to get directory name")?
            .to_str()
            .context("Directory name contains invalid UTF-8")?;

        // Build copier arguments
        let mut args = vec!["copy".to_string()];

        // Add flags
        if self.defaults {
            args.push("--defaults".to_string());
        }

        // Use --trust flag for template extensions
        args.push("--trust".to_string());

        // Pass project name to template
        args.push("-d".to_string());
        args.push(format!("cpy_project_name={}", dir_name));

        // Add version reference
        if !self.use_latest {
            // Use stable HEAD by default
            args.push("--vcs-ref".to_string());
            args.push("HEAD".to_string());
        }

        // Add template source and current directory as destination
        args.push(template_src.to_string());
        args.push(".".to_string());

        if verbose {
            eprintln!(
                "Initializing project in current directory from template: {}",
                template_src
            );
            eprintln!("Running: copier {}", args.join(" "));
        }

        // Execute copier
        let result = run_command("copier", &args, true, None)
            .context("Failed to execute copier")?;

        if !result.success {
            bail!("Project initialization failed with exit code: {}", result.exit_code);
        }

        if verbose {
            eprintln!("✅ Project initialized successfully!");
        }

        Ok(())
    }
}
