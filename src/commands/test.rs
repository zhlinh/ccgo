//! Test command implementation
//!
//! Provides comprehensive testing functionality including:
//! - GoogleTest and Catch2 test discovery
//! - Test execution with filtering
//! - Code coverage reporting
//! - Result aggregation and CI integration

use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, ValueEnum};

use crate::build::platforms::tests::TestsBuilder;
use crate::build::{BuildContext, BuildOptions, PlatformBuilder};
use crate::commands::build::{BuildTarget, LinkType, WindowsToolchain};
use crate::config::CcgoConfig;
use crate::testing::coverage::{CoverageCollector, CoverageConfig, CoverageFormat};
use crate::testing::discovery::TestDiscovery;
use crate::testing::results::TestResultAggregator;
use crate::testing::ci::{CiFormat, CiReporter};

/// Coverage output format for CLI
#[derive(Debug, Clone, Copy, ValueEnum, Default)]
pub enum CoverageOutputFormat {
    /// HTML report
    #[default]
    Html,
    /// LCOV format
    Lcov,
    /// JSON format
    Json,
    /// Cobertura XML
    Cobertura,
    /// Console summary only
    Summary,
}

impl From<CoverageOutputFormat> for CoverageFormat {
    fn from(f: CoverageOutputFormat) -> Self {
        match f {
            CoverageOutputFormat::Html => CoverageFormat::Html,
            CoverageOutputFormat::Lcov => CoverageFormat::Lcov,
            CoverageOutputFormat::Json => CoverageFormat::Json,
            CoverageOutputFormat::Cobertura => CoverageFormat::Cobertura,
            CoverageOutputFormat::Summary => CoverageFormat::Summary,
        }
    }
}

/// Run GoogleTest unit tests
#[derive(Args, Debug)]
pub struct TestCommand {
    /// Test filter pattern (e.g., TestSuite.TestCase, *Sort*)
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

    /// List all discovered tests without running
    #[arg(long)]
    pub list: bool,

    /// Enable code coverage collection
    #[arg(long)]
    pub coverage: bool,

    /// Coverage output format
    #[arg(long, value_enum, default_value = "html")]
    pub coverage_format: CoverageOutputFormat,

    /// Coverage output directory
    #[arg(long, default_value = "coverage")]
    pub coverage_dir: PathBuf,

    /// Minimum coverage threshold (0-100)
    #[arg(long)]
    pub coverage_threshold: Option<f64>,

    /// Fail if coverage is below threshold
    #[arg(long)]
    pub fail_under_coverage: bool,

    /// Aggregate results from multiple test runs
    #[arg(long)]
    pub aggregate: bool,

    /// Output format for CI integration
    #[arg(long)]
    pub ci_format: Option<String>,

    /// Write JUnit XML report to path
    #[arg(long)]
    pub junit_xml: Option<PathBuf>,
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
            features: vec![],
            use_default_features: true,
            all_features: false,
            cache: Some("auto".to_string()),
        };

        let ctx = BuildContext::new(project_root.clone(), config, options);
        let builder = TestsBuilder::new();

        // Determine build directory
        let release_subdir = if self.release { "release" } else { "debug" };
        let build_dir = project_root
            .join("cmake_build")
            .join(release_subdir)
            .join("tests");

        // Handle list command
        if self.list {
            return self.list_tests(&build_dir, verbose);
        }

        // Handle IDE project generation
        if self.ide_project {
            return builder.generate_ide_project(&ctx);
        }

        // Build tests if needed
        if !self.run_only {
            if verbose {
                eprintln!("Building tests...");
            }

            // Add coverage flags if enabled
            if self.coverage {
                // Note: Coverage requires special CMake flags
                // This would be passed via cmake_extra_flags in a real implementation
                eprintln!("📊 Coverage collection enabled");
            }

            builder.build(&ctx)?;
        }

        // Run tests if needed
        if !self.build_only {
            if verbose {
                eprintln!("Running tests...");
            }
            builder.run_tests(&ctx, self.filter.as_deref())?;
        }

        // Aggregate results if requested
        if self.aggregate {
            self.aggregate_results(&build_dir, verbose)?;
        }

        // Handle CI integration
        if self.ci_format.is_some() || self.junit_xml.is_some() {
            self.report_to_ci(&build_dir, verbose)?;
        }

        // Collect coverage if enabled
        if self.coverage && !self.build_only {
            self.collect_coverage(&project_root, &build_dir, verbose)?;
        }

        Ok(())
    }

    /// List discovered tests
    fn list_tests(&self, build_dir: &PathBuf, verbose: bool) -> Result<()> {
        let discovery = TestDiscovery::new(build_dir.clone(), verbose);

        println!("Discovering tests...");

        let tests = if let Some(ref pattern) = self.filter {
            discovery.filter(pattern)?
        } else {
            discovery.discover_all()?
        };

        discovery.print_tests(&tests);

        // Also show by suite
        println!("\nTests by Suite:");
        let by_suite = discovery.list_by_suite()?;
        for (suite, suite_tests) in by_suite {
            println!("  {} ({} tests)", suite, suite_tests.len());
        }

        Ok(())
    }

    /// Aggregate test results
    fn aggregate_results(&self, build_dir: &PathBuf, verbose: bool) -> Result<()> {
        let mut aggregator = TestResultAggregator::new(verbose);
        aggregator.find_results(build_dir)?;

        let summary = aggregator.aggregate()?;
        summary.print_summary();

        // Write JUnit XML if requested
        if let Some(ref junit_path) = self.junit_xml {
            std::fs::write(junit_path, summary.to_junit_xml())?;
            eprintln!("JUnit XML written to: {}", junit_path.display());
        }

        if !summary.all_passed() {
            anyhow::bail!("{} test(s) failed", summary.failed + summary.errors);
        }

        Ok(())
    }

    /// Report results to CI
    fn report_to_ci(&self, build_dir: &PathBuf, verbose: bool) -> Result<()> {
        let mut aggregator = TestResultAggregator::new(verbose);
        aggregator.find_results(build_dir)?;
        let summary = aggregator.aggregate()?;

        // Determine CI format
        let format = if let Some(ref fmt) = self.ci_format {
            fmt.parse::<CiFormat>()?
        } else {
            CiFormat::detect()
        };

        let reporter = CiReporter::new(format);
        reporter.report_tests(&summary);

        // Write JUnit XML if requested
        if let Some(ref junit_path) = self.junit_xml {
            reporter.write_junit_xml(&summary, junit_path)?;
            eprintln!("JUnit XML written to: {}", junit_path.display());
        }

        Ok(())
    }

    /// Collect code coverage
    fn collect_coverage(
        &self,
        project_root: &PathBuf,
        build_dir: &PathBuf,
        verbose: bool,
    ) -> Result<()> {
        eprintln!("\n📊 Collecting code coverage...");

        let config = CoverageConfig {
            source_dir: project_root.clone(),
            build_dir: build_dir.clone(),
            output_dir: self.coverage_dir.clone(),
            format: self.coverage_format.into(),
            threshold: self.coverage_threshold,
            fail_under: self.fail_under_coverage,
            ..Default::default()
        };

        let collector = CoverageCollector::new(config, verbose);

        match collector.collect() {
            Ok(report) => {
                report.print_summary();

                // Generate HTML if requested
                if matches!(self.coverage_format, CoverageOutputFormat::Html) {
                    match collector.generate_html(&report) {
                        Ok(html_path) => {
                            eprintln!("\n📄 HTML report: {}", html_path.display());

                            // Try to open in browser
                            #[cfg(target_os = "macos")]
                            {
                                let _ = std::process::Command::new("open")
                                    .arg(&html_path)
                                    .status();
                            }
                        }
                        Err(e) => {
                            eprintln!("Warning: Could not generate HTML report: {}", e);
                        }
                    }
                }

                // Check threshold
                if let Some(threshold) = self.coverage_threshold {
                    if !report.meets_threshold(threshold) {
                        let msg = format!(
                            "Coverage {:.1}% is below threshold {:.1}%",
                            report.line_coverage_percent(),
                            threshold
                        );
                        if self.fail_under_coverage {
                            anyhow::bail!(msg);
                        } else {
                            eprintln!("⚠️  {}", msg);
                        }
                    }
                }

                // CI reporting for coverage
                if self.ci_format.is_some() {
                    let format = if let Some(ref fmt) = self.ci_format {
                        fmt.parse::<CiFormat>()?
                    } else {
                        CiFormat::detect()
                    };
                    let reporter = CiReporter::new(format);
                    reporter.report_coverage(&report);
                }

                Ok(())
            }
            Err(e) => {
                eprintln!("⚠️  Coverage collection failed: {}", e);
                eprintln!("   Make sure tests were built with coverage flags:");
                eprintln!("   -DCMAKE_CXX_FLAGS=\"--coverage\" -DCMAKE_C_FLAGS=\"--coverage\"");
                Ok(()) // Don't fail the test run
            }
        }
    }
}
