//! `ccgo self` — manage the ccgo tool itself
//!
//! Usage:
//!   ccgo self update   # Update ccgo to the latest version

use anyhow::Result;
use clap::{Args, Subcommand};

/// Manage the ccgo tool itself
#[derive(Args, Debug)]
pub struct SelfCmdCommand {
    #[command(subcommand)]
    pub command: SelfSubCommand,
}

#[derive(Subcommand, Debug)]
pub enum SelfSubCommand {
    /// Update ccgo to the latest version
    Update,
}

impl SelfCmdCommand {
    pub fn execute(self, verbose: bool) -> Result<()> {
        match self.command {
            SelfSubCommand::Update => crate::commands::self_update::execute_self_update(verbose),
        }
    }
}
