//! CLI argument parsing using clap derive macros

use anyhow::Result;
use clap::{Parser, Subcommand};

use crate::commands::{
    add::AddCommand, analytics::AnalyticsCommand, bench::BenchCommand, build::BuildCommand,
    check::CheckCommand, clean::CleanCommand, collection::CollectionCommand, doc::DocCommand,
    doctor::DoctorCommand, fetch::FetchCommand, init::InitCommand, install::InstallCommand,
    new::NewCommand, package::PackageCommand, publish::PublishCommand, registry::RegistryCommand,
    release::ReleaseCommand, remove::RemoveCommand, run::RunCommand, search::SearchCommand,
    self_cmd::SelfCmdCommand, tag::TagCommand, test::TestCommand, tree::TreeCommand,
    uninstall::UninstallCommand, update::UpdateCommand, vendor::VendorCommand,
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

    /// Run an example or binary target
    Run(RunCommand),

    /// Run GoogleTest unit tests
    Test(TestCommand),

    /// Run Google Benchmark benchmarks
    Bench(BenchCommand),

    /// Generate Doxygen documentation
    Doc(DocCommand),

    /// Publish library to repository
    Publish(PublishCommand),

    /// Check compilation without generating binaries
    Check(CheckCommand),

    /// Diagnose platform dependencies and environment setup
    Doctor(DoctorCommand),

    /// Clean build artifacts
    Clean(CleanCommand),

    /// Create version tag from CCGO.toml
    Tag(TagCommand),

    /// Bump project version, sync platform manifests, update CHANGELOG, and commit
    Release(ReleaseCommand),

    /// Package SDK for distribution
    Package(PackageCommand),

    /// Install the current project into the global CCGO package cache
    /// (analog of `cargo install --path .` / `mvn install`)
    Install(InstallCommand),

    /// Fetch this project's dependencies into `.ccgo/deps/`
    /// (was `ccgo install` prior to v3.5)
    Fetch(FetchCommand),

    /// Uninstall a package previously installed with `ccgo install`
    Uninstall(UninstallCommand),

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

    /// Manage package registries (index repositories)
    Registry(RegistryCommand),

    /// Search for packages
    Search(SearchCommand),

    /// Vendor dependencies to local directory for offline builds
    Vendor(VendorCommand),

    /// View build performance analytics
    Analytics(AnalyticsCommand),

    /// Manage the ccgo tool itself (e.g. update)
    #[command(name = "self")]
    SelfCmd(SelfCmdCommand),
}

impl Cli {
    /// Execute the CLI command
    pub fn execute(self) -> Result<()> {
        // Set up terminal colors
        if self.no_color {
            console::set_colors_enabled(false);
            console::set_colors_enabled_stderr(false);
        }

        dispatch_command(self.command, self.verbose)
    }
}

/// Dispatch build/project-lifecycle commands, delegating dependency-management
/// commands to [`dispatch_dependency_command`].
fn dispatch_command(command: Commands, verbose: bool) -> Result<()> {
    match command {
        Commands::Build(cmd) => cmd.execute(verbose),
        Commands::New(cmd) => cmd.execute(verbose),
        Commands::Init(cmd) => cmd.execute(verbose),
        Commands::Run(cmd) => cmd.execute(verbose),
        Commands::Test(cmd) => cmd.execute(verbose),
        Commands::Bench(cmd) => cmd.execute(verbose),
        Commands::Doc(cmd) => cmd.execute(verbose),
        Commands::Publish(cmd) => cmd.execute(verbose),
        Commands::Check(cmd) => cmd.execute(verbose),
        Commands::Doctor(cmd) => cmd.execute(verbose),
        Commands::Clean(cmd) => cmd.execute(verbose),
        Commands::Tag(cmd) => cmd.execute(verbose),
        Commands::Release(cmd) => cmd.execute(verbose),
        Commands::Package(cmd) => cmd.execute(verbose),
        Commands::Analytics(cmd) => cmd.execute(verbose),
        Commands::SelfCmd(cmd) => cmd.execute(verbose),
        other => dispatch_dependency_command(other, verbose),
    }
}

/// Dispatch dependency- and registry-management commands.
fn dispatch_dependency_command(command: Commands, verbose: bool) -> Result<()> {
    match command {
        Commands::Install(cmd) => cmd.execute(verbose),
        Commands::Fetch(cmd) => cmd.execute(verbose),
        Commands::Uninstall(cmd) => cmd.execute(verbose),
        Commands::Add(cmd) => cmd.execute(verbose),
        Commands::Remove(cmd) => cmd.execute(verbose),
        Commands::Update(cmd) => cmd.execute(verbose),
        Commands::Tree(cmd) => cmd.execute(verbose),
        Commands::Collection(cmd) => cmd.execute(verbose),
        Commands::Registry(cmd) => cmd.execute(verbose),
        Commands::Search(cmd) => cmd.execute(verbose),
        Commands::Vendor(cmd) => cmd.execute(verbose),
        _ => unreachable!("dispatch_dependency_command called with non-dependency command"),
    }
}
