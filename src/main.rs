//! CCGO CLI - A high-performance C++ cross-platform build tool
//!
//! This is the Rust implementation of the ccgo CLI, providing fast startup
//! and native build orchestration with ZERO Python dependency.
//!
//! ## Architecture
//!
//! ```text
//! Rust CLI → build/ modules → CMake/Gradle/Hvigor (direct)
//! ```

mod build;
mod cli;
mod commands;
mod config;
mod exec;
mod utils;

use anyhow::Result;
use clap::Parser;

use cli::Cli;

fn main() -> Result<()> {
    let cli = Cli::parse();
    cli.execute()
}
