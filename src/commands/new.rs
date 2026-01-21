//! New project command implementation

use anyhow::{bail, Context, Result};
use clap::Args;

use crate::exec::subprocess::{command_exists, run_command};

/// Default CCGO template from GitHub
const DEFAULT_TEMPLATE: &str = "https://github.com/zhlinh/ccgo-template";

/// Create a new library project
#[derive(Args, Debug)]
pub struct NewCommand {
    /// Project name
    pub name: String,

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

impl NewCommand {
    /// Execute the new command
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
        args.push(format!("cpy_project_name={}", self.name));

        // Add version reference
        if !self.use_latest {
            // Use stable HEAD by default
            args.push("--vcs-ref".to_string());
            args.push("HEAD".to_string());
        }

        // Add template source and destination
        args.push(template_src.to_string());
        args.push(self.name.clone());

        if verbose {
            eprintln!(
                "Creating new project '{}' from template: {}",
                self.name, template_src
            );
            eprintln!("Running: copier {}", args.join(" "));
        }

        // Execute copier
        let result = run_command("copier", &args, true, None)
            .context("Failed to execute copier")?;

        if !result.success {
            bail!("Project creation failed with exit code: {}", result.exit_code);
        }

        if verbose {
            eprintln!("✅ Project '{}' created successfully!", self.name);
        }

        Ok(())
    }
}
