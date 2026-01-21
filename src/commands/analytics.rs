//! Analytics command implementation

use anyhow::Result;
use clap::{Args, Subcommand};

use crate::build::analytics::BuildAnalytics;
use crate::config::CcgoConfig;

/// Build analytics and performance metrics
#[derive(Args, Debug)]
pub struct AnalyticsCommand {
    #[command(subcommand)]
    pub subcommand: AnalyticsSubcommand,
}

#[derive(Subcommand, Debug)]
pub enum AnalyticsSubcommand {
    /// Show build analytics for current project
    Show {
        /// Number of recent builds to show (default: 10)
        #[arg(short = 'n', long, default_value = "10")]
        count: usize,
    },

    /// Show summary statistics across all builds
    Summary,

    /// Clear analytics history for current project
    Clear {
        /// Skip confirmation prompt
        #[arg(short = 'y', long)]
        yes: bool,
    },

    /// Export analytics data to JSON file
    Export {
        /// Output file path
        #[arg(short, long)]
        output: String,
    },

    /// List all projects with analytics data
    List,
}

impl AnalyticsCommand {
    /// Execute the analytics command
    pub fn execute(self, _verbose: bool) -> Result<()> {
        match self.subcommand {
            AnalyticsSubcommand::Show { count } => self.show(count),
            AnalyticsSubcommand::Summary => self.summary(),
            AnalyticsSubcommand::Clear { yes } => self.clear(yes),
            AnalyticsSubcommand::Export { ref output } => self.export(output),
            AnalyticsSubcommand::List => self.list(),
        }
    }

    /// Show recent build analytics
    fn show(&self, count: usize) -> Result<()> {
        let config = CcgoConfig::load()?;
        let package = config.require_package()?;

        let history = BuildAnalytics::load_history(&package.name)?;

        if history.is_empty() {
            println!("No build analytics available for '{}'", package.name);
            println!("Run a build to collect analytics data.");
            return Ok(());
        }

        println!("\n{}", "=".repeat(80));
        println!("Build Analytics for {}", package.name);
        println!("{}", "=".repeat(80));
        println!();

        // Show most recent builds (up to count)
        let start = if history.len() > count {
            history.len() - count
        } else {
            0
        };

        for (idx, analytics) in history[start..].iter().enumerate() {
            let build_num = start + idx + 1;
            println!("Build #{} - {} ({})",
                build_num,
                analytics.platform,
                analytics.timestamp
            );
            println!("  Duration:    {:.2}s", analytics.total_duration_secs);
            println!("  Jobs:        {}", analytics.parallel_jobs);
            println!("  Success:     {}", if analytics.success { "✓" } else { "✗" });

            if let Some(tool) = &analytics.cache_stats.tool {
                println!("  Cache Tool:  {}", tool);
                println!("  Cache Rate:  {:.1}%", analytics.cache_stats.hit_rate);
            }

            if analytics.error_count > 0 || analytics.warning_count > 0 {
                println!("  Errors:      {}", analytics.error_count);
                println!("  Warnings:    {}", analytics.warning_count);
            }

            println!();
        }

        Ok(())
    }

    /// Show summary statistics
    fn summary(&self) -> Result<()> {
        let config = CcgoConfig::load()?;
        let package = config.require_package()?;

        let history = BuildAnalytics::load_history(&package.name)?;

        if history.is_empty() {
            println!("No build analytics available for '{}'", package.name);
            return Ok(());
        }

        println!("\n{}", "=".repeat(80));
        println!("Build Analytics Summary for {}", package.name);
        println!("{}", "=".repeat(80));
        println!();

        // Calculate statistics
        let total_builds = history.len();
        let successful_builds = history.iter().filter(|a| a.success).count();
        let success_rate = (successful_builds as f64 / total_builds as f64) * 100.0;

        let total_duration: f64 = history.iter().map(|a| a.total_duration_secs).sum();
        let avg_duration = total_duration / total_builds as f64;

        let min_duration = history
            .iter()
            .map(|a| a.total_duration_secs)
            .min_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        let max_duration = history
            .iter()
            .map(|a| a.total_duration_secs)
            .max_by(|a, b| a.partial_cmp(b).unwrap())
            .unwrap_or(0.0);

        println!("Total Builds:      {}", total_builds);
        println!("Successful:        {} ({:.1}%)", successful_builds, success_rate);
        println!();
        println!("Build Duration:");
        println!("  Average:         {:.2}s", avg_duration);
        println!("  Fastest:         {:.2}s", min_duration);
        println!("  Slowest:         {:.2}s", max_duration);
        println!();

        // Cache statistics (if available)
        let builds_with_cache: Vec<_> = history
            .iter()
            .filter(|a| a.cache_stats.tool.is_some())
            .collect();

        if !builds_with_cache.is_empty() {
            let avg_hit_rate: f64 = builds_with_cache
                .iter()
                .map(|a| a.cache_stats.hit_rate)
                .sum::<f64>() / builds_with_cache.len() as f64;

            println!("Cache Statistics:");
            println!("  Builds with cache: {}", builds_with_cache.len());
            println!("  Avg Hit Rate:      {:.1}%", avg_hit_rate);
            println!();
        }

        // Platform breakdown
        use std::collections::HashMap;
        let mut platform_counts: HashMap<String, usize> = HashMap::new();
        for analytics in &history {
            *platform_counts.entry(analytics.platform.clone()).or_insert(0) += 1;
        }

        if !platform_counts.is_empty() {
            println!("Platform Breakdown:");
            for (platform, count) in platform_counts.iter() {
                println!("  {:.<20} {}", platform, count);
            }
            println!();
        }

        println!("{}", "=".repeat(80));

        Ok(())
    }

    /// Clear analytics history
    fn clear(&self, yes: bool) -> Result<()> {
        let config = CcgoConfig::load()?;
        let package = config.require_package()?;

        let history = BuildAnalytics::load_history(&package.name)?;

        if history.is_empty() {
            println!("No analytics to clear for '{}'", package.name);
            return Ok(());
        }

        if !yes {
            println!("This will delete {} build analytics entries for '{}'",
                history.len(), package.name);
            print!("Continue? [y/N] ");

            use std::io::{self, Write};
            io::stdout().flush()?;

            let mut input = String::new();
            io::stdin().read_line(&mut input)?;

            if !input.trim().eq_ignore_ascii_case("y") {
                println!("Cancelled.");
                return Ok(());
            }
        }

        BuildAnalytics::clear_history(&package.name)?;
        println!("✓ Cleared analytics for '{}'", package.name);

        Ok(())
    }

    /// Export analytics to JSON file
    fn export(&self, output: &str) -> Result<()> {
        let config = CcgoConfig::load()?;
        let package = config.require_package()?;

        let history = BuildAnalytics::load_history(&package.name)?;

        if history.is_empty() {
            println!("No analytics to export for '{}'", package.name);
            return Ok(());
        }

        let json = serde_json::to_string_pretty(&history)?;
        std::fs::write(output, json)?;

        println!("✓ Exported {} build analytics to {}", history.len(), output);

        Ok(())
    }

    /// List all projects with analytics
    fn list(&self) -> Result<()> {
        let analytics_dir = BuildAnalytics::analytics_dir()?;

        if !analytics_dir.exists() {
            println!("No analytics data available.");
            return Ok(());
        }

        let mut projects = Vec::new();

        for entry in std::fs::read_dir(&analytics_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().map_or(false, |ext| ext == "json") {
                if let Some(stem) = path.file_stem() {
                    if let Some(project_name) = stem.to_str() {
                        // Load history to get build count
                        if let Ok(history) = BuildAnalytics::load_history(project_name) {
                            projects.push((project_name.to_string(), history.len()));
                        }
                    }
                }
            }
        }

        if projects.is_empty() {
            println!("No analytics data available.");
            return Ok(());
        }

        projects.sort();

        println!("\n{}", "=".repeat(80));
        println!("Projects with Analytics");
        println!("{}", "=".repeat(80));
        println!();

        for (project, count) in projects {
            println!("  {:.<40} {} builds", project, count);
        }

        println!();
        println!("Use 'ccgo analytics show' from a project directory to view details.");
        println!("{}", "=".repeat(80));

        Ok(())
    }
}
