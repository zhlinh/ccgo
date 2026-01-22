//! Benchmark command implementation
//!
//! Provides comprehensive benchmarking functionality including:
//! - Google Benchmark execution
//! - Result storage and comparison
//! - Regression detection
//! - CI integration

use std::path::PathBuf;

use anyhow::Result;
use clap::Args;

use crate::build::platforms::benches::BenchesBuilder;
use crate::build::{BuildContext, BuildOptions, PlatformBuilder};
use crate::commands::build::{BuildTarget, LinkType, WindowsToolchain};
use crate::config::CcgoConfig;
use crate::testing::benchmark::{BenchmarkStore, ComparisonReport};
use crate::testing::ci::{CiFormat, CiReporter};

/// Run Google Benchmark benchmarks
#[derive(Args, Debug)]
pub struct BenchCommand {
    /// Benchmark filter pattern (e.g., "BM_Sort*")
    #[arg(long)]
    pub filter: Option<String>,

    /// Generate IDE project for benchmarks instead of running
    #[arg(long)]
    pub ide_project: bool,

    /// Build benchmarks without running them
    #[arg(long)]
    pub build_only: bool,

    /// Run benchmarks without building (assumes already built)
    #[arg(long)]
    pub run_only: bool,

    /// Number of parallel jobs for building
    #[arg(short, long)]
    pub jobs: Option<usize>,

    /// Build in release mode (recommended for benchmarks)
    #[arg(long, default_value = "true")]
    pub release: bool,

    /// Save results with this identifier (e.g., git commit hash)
    #[arg(long)]
    pub save: Option<String>,

    /// Compare results with a baseline run
    #[arg(long)]
    pub baseline: Option<String>,

    /// Compare with the latest saved run
    #[arg(long)]
    pub compare_latest: bool,

    /// Regression threshold percentage (default: 5%)
    #[arg(long, default_value = "5.0")]
    pub threshold: f64,

    /// Fail if regressions are detected
    #[arg(long)]
    pub fail_on_regression: bool,

    /// Output format for CI integration
    #[arg(long)]
    pub ci_format: Option<String>,

    /// List all saved benchmark runs
    #[arg(long)]
    pub list_runs: bool,

    /// Export comparison as Markdown
    #[arg(long)]
    pub export_markdown: Option<PathBuf>,

    /// Export comparison as JSON
    #[arg(long)]
    pub export_json: Option<PathBuf>,
}

impl BenchCommand {
    /// Execute the bench command
    pub fn execute(self, verbose: bool) -> Result<()> {
        // Handle list runs
        if self.list_runs {
            return self.list_saved_runs();
        }

        // Load project configuration
        let config = CcgoConfig::load()?;
        let project_root = std::env::current_dir()?;

        // Create build context for benchmarks
        let options = BuildOptions {
            target: BuildTarget::Linux, // Placeholder, not used by benchmarks
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
        let builder = BenchesBuilder::new();

        // Determine build directory
        let release_subdir = if self.release { "release" } else { "debug" };
        let build_dir = project_root
            .join("cmake_build")
            .join(release_subdir)
            .join("benches");

        if self.ide_project {
            // Generate IDE project
            return builder.generate_ide_project(&ctx);
        }

        // Build benchmarks if needed
        if !self.run_only {
            if verbose {
                eprintln!("Building benchmarks...");
            }
            builder.build(&ctx)?;
        }

        // Run benchmarks if needed
        if !self.build_only {
            if verbose {
                eprintln!("Running benchmarks...");
            }
            builder.run_benchmarks(&ctx, self.filter.as_deref())?;

            // Handle result storage and comparison
            self.process_results(&project_root, &build_dir, verbose)?;
        }

        Ok(())
    }

    /// List saved benchmark runs
    fn list_saved_runs(&self) -> Result<()> {
        let store = BenchmarkStore::new(BenchmarkStore::default_path());
        let runs = store.list()?;

        if runs.is_empty() {
            println!("No saved benchmark runs found.");
            println!("Use --save <id> to save benchmark results.");
        } else {
            println!("Saved benchmark runs:");
            for run_id in runs {
                println!("  - {}", run_id);
            }
        }

        Ok(())
    }

    /// Process benchmark results (save, compare, report)
    fn process_results(
        &self,
        project_root: &PathBuf,
        build_dir: &PathBuf,
        verbose: bool,
    ) -> Result<()> {
        let store = BenchmarkStore::new(project_root.join(".ccgo").join("benchmarks"));

        // Find the latest benchmark JSON output
        let json_file = self.find_benchmark_json(build_dir)?;

        if let Some(json_path) = json_file {
            // Parse the results
            let mut current_run = store.parse_gbench_json(&json_path)?;

            // Set run ID
            if let Some(ref save_id) = self.save {
                current_run.run_id = save_id.clone();
            } else {
                // Use timestamp as default ID
                current_run.run_id = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
            }

            // Save if requested
            if self.save.is_some() {
                let saved_path = store.save(&current_run)?;
                eprintln!("📁 Benchmark results saved: {}", saved_path.display());
            }

            // Compare with baseline
            let comparison = if let Some(ref baseline_id) = self.baseline {
                Some(self.compare_with_baseline(&store, baseline_id, &current_run)?)
            } else if self.compare_latest {
                match store.load_latest() {
                    Ok(baseline) => {
                        if baseline.run_id != current_run.run_id {
                            Some(ComparisonReport::new(&baseline, &current_run, self.threshold))
                        } else {
                            None
                        }
                    }
                    Err(_) => None,
                }
            } else {
                None
            };

            // Process comparison
            if let Some(report) = comparison {
                self.report_comparison(&report, verbose)?;
            }
        } else if verbose {
            eprintln!("Warning: No benchmark JSON output found in {}", build_dir.display());
        }

        Ok(())
    }

    /// Find benchmark JSON output file
    fn find_benchmark_json(&self, build_dir: &PathBuf) -> Result<Option<PathBuf>> {
        if !build_dir.exists() {
            return Ok(None);
        }

        let mut latest: Option<(PathBuf, std::time::SystemTime)> = None;

        for entry in std::fs::read_dir(build_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let name = path.file_name().unwrap().to_string_lossy();
                if name.contains("_result") && name.ends_with(".json") {
                    if let Ok(metadata) = path.metadata() {
                        if let Ok(modified) = metadata.modified() {
                            match &latest {
                                None => latest = Some((path, modified)),
                                Some((_, latest_time)) if modified > *latest_time => {
                                    latest = Some((path, modified))
                                }
                                _ => {}
                            }
                        }
                    }
                }
            }
        }

        Ok(latest.map(|(p, _)| p))
    }

    /// Compare with a baseline run
    fn compare_with_baseline(
        &self,
        store: &BenchmarkStore,
        baseline_id: &str,
        current: &crate::testing::benchmark::BenchmarkRun,
    ) -> Result<ComparisonReport> {
        let baseline = store.load(baseline_id)?;
        Ok(ComparisonReport::new(&baseline, current, self.threshold))
    }

    /// Report comparison results
    fn report_comparison(&self, report: &ComparisonReport, _verbose: bool) -> Result<()> {
        // Print report
        report.print_report();

        // Export to Markdown if requested
        if let Some(ref md_path) = self.export_markdown {
            std::fs::write(md_path, report.to_markdown())?;
            eprintln!("📄 Markdown report: {}", md_path.display());
        }

        // Export to JSON if requested
        if let Some(ref json_path) = self.export_json {
            std::fs::write(json_path, report.to_json()?)?;
            eprintln!("📄 JSON report: {}", json_path.display());
        }

        // CI reporting
        if self.ci_format.is_some() {
            let format = if let Some(ref fmt) = self.ci_format {
                fmt.parse::<CiFormat>()?
            } else {
                CiFormat::detect()
            };
            let reporter = CiReporter::new(format);
            reporter.report_benchmarks(report);
        }

        // Fail if regressions detected
        if self.fail_on_regression && report.has_regressions() {
            anyhow::bail!(
                "{} benchmark regression(s) detected (threshold: {:.1}%)",
                report.regressions,
                self.threshold
            );
        }

        Ok(())
    }
}
