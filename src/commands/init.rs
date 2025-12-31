//! Init command implementation

use anyhow::Result;
use clap::Args;

use crate::exec::python::PythonRunner;

/// Initialize library project in current directory
#[derive(Args, Debug)]
pub struct InitCommand {
    /// Use default values (no prompts)
    #[arg(long)]
    pub defaults: bool,

    /// Custom template URL
    #[arg(long)]
    pub template: Option<String>,

    /// Use latest template version
    #[arg(long)]
    pub use_latest: bool,
}

impl InitCommand {
    /// Execute the init command
    pub fn execute(self, verbose: bool) -> Result<()> {
        let mut args = vec!["init".to_string()];

        if self.defaults {
            args.push("--defaults".to_string());
        }

        if let Some(template) = &self.template {
            args.push("--template".to_string());
            args.push(template.clone());
        }

        if self.use_latest {
            args.push("--use-latest".to_string());
        }

        let runner = PythonRunner::new()?;
        let result = runner.run_ccgo(&args, verbose)?;

        std::process::exit(result.exit_code);
    }
}
