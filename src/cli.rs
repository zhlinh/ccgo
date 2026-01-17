//! CLI argument parsing using clap derive macros

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands::{
    add::AddCommand, bench::BenchCommand, build::BuildCommand, check::CheckCommand,
    clean::CleanCommand, collection::CollectionCommand, doc::DocCommand, init::InitCommand,
    install::InstallCommand, new::NewCommand, package::PackageCommand, publish::PublishCommand,
    remove::RemoveCommand, search::SearchCommand, tag::TagCommand, test::TestCommand,
    tree::TreeCommand, update::UpdateCommand,
};

/// CCGO - C++ Cross-platform Build Tool
///
/// A high-performance CLI for building C++ libraries across multiple platforms.
#[derive(Parser, Debug)]
#[command(name = "ccgo")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    /// Enable verbose output
    #[arg(short, long, global = true)]
    pub verbose: bool,

    /// Disable colored output
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Commands,
}

/// Available commands
#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Build library for specific platform
    Build(BuildCommand),

    /// Create a new library project
    New(NewCommand),

    /// Initialize library project in current directory
    Init(InitCommand),

    /// Run GoogleTest unit tests
    Test(TestCommand),

    /// Run Google Benchmark benchmarks
    Bench(BenchCommand),

    /// Generate Doxygen documentation
    Doc(DocCommand),

    /// Publish library to repository
    Publish(PublishCommand),

    /// Check platform dependencies
    Check(CheckCommand),

    /// Clean build artifacts
    Clean(CleanCommand),

    /// Create version tag from CCGO.toml
    Tag(TagCommand),

    /// Package SDK for distribution
    Package(PackageCommand),

    /// Install project dependencies
    Install(InstallCommand),

    /// Add a dependency to CCGO.toml
    Add(AddCommand),

    /// Remove a dependency from CCGO.toml
    Remove(RemoveCommand),

    /// Update dependencies to latest versions
    Update(UpdateCommand),

    /// Display dependency tree
    Tree(TreeCommand),

    /// Manage package collections
    Collection(CollectionCommand),

    /// Search for packages
    Search(SearchCommand),
}

impl Cli {
    /// Execute the CLI command
    pub fn execute(self) -> Result<()> {
        // Set up terminal colors
        if self.no_color {
            console::set_colors_enabled(false);
            console::set_colors_enabled_stderr(false);
        }

        // Execute the subcommand
        match self.command {
            Commands::Build(cmd) => cmd.execute(self.verbose),
            Commands::New(cmd) => cmd.execute(self.verbose),
            Commands::Init(cmd) => cmd.execute(self.verbose),
            Commands::Test(cmd) => cmd.execute(self.verbose),
            Commands::Bench(cmd) => cmd.execute(self.verbose),
            Commands::Doc(cmd) => cmd.execute(self.verbose),
            Commands::Publish(cmd) => cmd.execute(self.verbose),
            Commands::Check(cmd) => cmd.execute(self.verbose),
            Commands::Clean(cmd) => cmd.execute(self.verbose),
            Commands::Tag(cmd) => cmd.execute(self.verbose),
            Commands::Package(cmd) => cmd.execute(self.verbose),
            Commands::Install(cmd) => cmd.execute(self.verbose),
            Commands::Add(cmd) => cmd.execute(self.verbose),
            Commands::Remove(cmd) => cmd.execute(self.verbose),
            Commands::Update(cmd) => cmd.execute(self.verbose),
            Commands::Tree(cmd) => cmd.execute(self.verbose),
            Commands::Collection(cmd) => cmd.execute(self.verbose),
            Commands::Search(cmd) => cmd.execute(self.verbose),
        }
    }
}
