//! Test command implementation

use anyhow::Result;
use clap::Args;

use crate::build::platforms::tests::TestsBuilder;
use crate::build::{BuildContext, BuildOptions, PlatformBuilder};
use crate::commands::build::{BuildTarget, LinkType, WindowsToolchain};
use crate::config::CcgoConfig;

/// Run GoogleTest unit tests
#[derive(Args, Debug)]
pub struct TestCommand {
    /// Test filter pattern (e.g., TestSuite.TestCase)
    #[arg(long)]
    pub filter: Option<String>,

    /// Generate IDE project for tests instead of running
    #[arg(long)]
    pub ide_project: bool,

    /// Build tests without running them
    #[arg(long)]
    pub build_only: bool,

    /// Run tests without building (assumes already built)
    #[arg(long)]
    pub run_only: bool,

    /// Number of parallel jobs for building
    #[arg(short, long)]
    pub jobs: Option<usize>,

    /// Build in release mode
    #[arg(long)]
    pub release: bool,
}

impl TestCommand {
    /// Execute the test command
    pub fn execute(self, verbose: bool) -> Result<()> {
        // Load project configuration
        let config = CcgoConfig::load()?;
        let project_root = std::env::current_dir()?;

        // Create build context for tests
        let options = BuildOptions {
            target: BuildTarget::Linux, // Placeholder, not used by tests
            architectures: vec![],
            link_type: LinkType::Both,
            use_docker: false,
            auto_docker: false,
            jobs: self.jobs,
            ide_project: self.ide_project,
            release: self.release,
            native_only: false,
            toolchain: WindowsToolchain::Auto,
            verbose,
            dev: false,
        };

        let ctx = BuildContext::new(project_root, config, options);
        let builder = TestsBuilder::new();

        if self.ide_project {
            // Generate IDE project
            builder.generate_ide_project(&ctx)?;
        } else if self.run_only {
            // Run tests without building
            if verbose {
                eprintln!("Running tests (without rebuilding)...");
            }
            builder.run_tests(&ctx, self.filter.as_deref())?;
        } else if self.build_only {
            // Build tests without running
            if verbose {
                eprintln!("Building tests...");
            }
            builder.build(&ctx)?;
        } else {
            // Build and run tests (default)
            if verbose {
                eprintln!("Building and running tests...");
            }
            builder.build(&ctx)?;
            builder.run_tests(&ctx, self.filter.as_deref())?;
        }

        Ok(())
    }
}
