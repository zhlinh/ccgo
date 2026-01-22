//! Benchmark result comparison module
//!
//! Stores benchmark results and compares across runs.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// A single benchmark result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkResult {
    /// Benchmark name
    pub name: String,
    /// Number of iterations
    pub iterations: u64,
    /// Real time per iteration (nanoseconds)
    pub real_time: f64,
    /// CPU time per iteration (nanoseconds)
    pub cpu_time: f64,
    /// Time unit
    pub time_unit: String,
    /// Bytes per second (if applicable)
    pub bytes_per_second: Option<f64>,
    /// Items per second (if applicable)
    pub items_per_second: Option<f64>,
    /// Custom counters
    pub counters: HashMap<String, f64>,
}

impl BenchmarkResult {
    /// Format time with appropriate unit
    pub fn format_time(&self) -> String {
        if self.real_time >= 1_000_000_000.0 {
            format!("{:.2} s", self.real_time / 1_000_000_000.0)
        } else if self.real_time >= 1_000_000.0 {
            format!("{:.2} ms", self.real_time / 1_000_000.0)
        } else if self.real_time >= 1_000.0 {
            format!("{:.2} us", self.real_time / 1_000.0)
        } else {
            format!("{:.2} ns", self.real_time)
        }
    }
}

/// Benchmark run (collection of results)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkRun {
    /// Run timestamp
    pub timestamp: String,
    /// Run identifier (e.g., git commit, version)
    pub run_id: String,
    /// System information
    pub system_info: SystemInfo,
    /// Benchmark results
    pub results: Vec<BenchmarkResult>,
}

impl BenchmarkRun {
    /// Create a new benchmark run
    pub fn new(run_id: &str) -> Self {
        Self {
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            run_id: run_id.to_string(),
            system_info: SystemInfo::current(),
            results: Vec::new(),
        }
    }

    /// Add a result
    pub fn add_result(&mut self, result: BenchmarkResult) {
        self.results.push(result);
    }

    /// Get result by name
    pub fn get(&self, name: &str) -> Option<&BenchmarkResult> {
        self.results.iter().find(|r| r.name == name)
    }
}

/// System information for benchmark context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemInfo {
    /// Operating system
    pub os: String,
    /// CPU model
    pub cpu: String,
    /// Number of CPU cores
    pub cores: u32,
    /// Total memory in bytes
    pub memory: u64,
}

impl SystemInfo {
    /// Get current system info
    pub fn current() -> Self {
        Self {
            os: std::env::consts::OS.to_string(),
            cpu: "Unknown".to_string(), // Would need sysinfo crate
            cores: std::thread::available_parallelism()
                .map(|p| p.get() as u32)
                .unwrap_or(1),
            memory: 0, // Would need sysinfo crate
        }
    }
}

/// Comparison between two benchmark results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkComparison {
    /// Benchmark name
    pub name: String,
    /// Baseline result
    pub baseline: BenchmarkResult,
    /// Current result
    pub current: BenchmarkResult,
    /// Time difference (percentage, positive = regression)
    pub time_diff_percent: f64,
    /// Absolute time difference (nanoseconds)
    pub time_diff_abs: f64,
    /// Whether this is a regression
    pub is_regression: bool,
    /// Whether this is an improvement
    pub is_improvement: bool,
    /// Significance threshold used
    pub threshold: f64,
}

impl BenchmarkComparison {
    /// Create comparison between baseline and current
    pub fn compare(baseline: &BenchmarkResult, current: &BenchmarkResult, threshold: f64) -> Self {
        let time_diff_abs = current.real_time - baseline.real_time;
        let time_diff_percent = if baseline.real_time > 0.0 {
            (time_diff_abs / baseline.real_time) * 100.0
        } else {
            0.0
        };

        let is_regression = time_diff_percent > threshold;
        let is_improvement = time_diff_percent < -threshold;

        Self {
            name: current.name.clone(),
            baseline: baseline.clone(),
            current: current.clone(),
            time_diff_percent,
            time_diff_abs,
            is_regression,
            is_improvement,
            threshold,
        }
    }

    /// Format the comparison result
    pub fn format(&self) -> String {
        let status = if self.is_regression {
            "🔴 REGRESSION"
        } else if self.is_improvement {
            "🟢 IMPROVEMENT"
        } else {
            "⚪ NO CHANGE"
        };

        format!(
            "{}: {} -> {} ({:+.1}%) {}",
            self.name,
            self.baseline.format_time(),
            self.current.format_time(),
            self.time_diff_percent,
            status
        )
    }
}

/// Benchmark comparison report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparisonReport {
    /// Baseline run info
    pub baseline_id: String,
    /// Current run info
    pub current_id: String,
    /// Timestamp
    pub timestamp: String,
    /// Comparisons
    pub comparisons: Vec<BenchmarkComparison>,
    /// Number of regressions
    pub regressions: usize,
    /// Number of improvements
    pub improvements: usize,
    /// Regression threshold
    pub threshold: f64,
}

impl ComparisonReport {
    /// Create a new comparison report
    pub fn new(baseline: &BenchmarkRun, current: &BenchmarkRun, threshold: f64) -> Self {
        let mut comparisons = Vec::new();

        for current_result in &current.results {
            if let Some(baseline_result) = baseline.get(&current_result.name) {
                comparisons.push(BenchmarkComparison::compare(
                    baseline_result,
                    current_result,
                    threshold,
                ));
            }
        }

        let regressions = comparisons.iter().filter(|c| c.is_regression).count();
        let improvements = comparisons.iter().filter(|c| c.is_improvement).count();

        Self {
            baseline_id: baseline.run_id.clone(),
            current_id: current.run_id.clone(),
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            comparisons,
            regressions,
            improvements,
            threshold,
        }
    }

    /// Check if there are any regressions
    pub fn has_regressions(&self) -> bool {
        self.regressions > 0
    }

    /// Print comparison report
    pub fn print_report(&self) {
        println!("\n{}", "═".repeat(70));
        println!("BENCHMARK COMPARISON REPORT");
        println!("{}", "═".repeat(70));
        println!("Baseline: {}", self.baseline_id);
        println!("Current:  {}", self.current_id);
        println!("Threshold: {:.1}%", self.threshold);
        println!();

        // Summary
        println!("Summary:");
        println!("  Total comparisons: {}", self.comparisons.len());
        println!("  🟢 Improvements:    {}", self.improvements);
        println!("  🔴 Regressions:     {}", self.regressions);
        println!(
            "  ⚪ No change:       {}",
            self.comparisons.len() - self.improvements - self.regressions
        );
        println!();

        // Details
        println!("{}", "─".repeat(70));
        println!("Details:");
        println!();

        // Show regressions first
        if self.regressions > 0 {
            println!("REGRESSIONS:");
            for comp in self.comparisons.iter().filter(|c| c.is_regression) {
                println!("  {}", comp.format());
            }
            println!();
        }

        // Show improvements
        if self.improvements > 0 {
            println!("IMPROVEMENTS:");
            for comp in self.comparisons.iter().filter(|c| c.is_improvement) {
                println!("  {}", comp.format());
            }
            println!();
        }

        // Show no change (abbreviated if many)
        let no_change: Vec<_> = self
            .comparisons
            .iter()
            .filter(|c| !c.is_regression && !c.is_improvement)
            .collect();

        if !no_change.is_empty() {
            println!("NO SIGNIFICANT CHANGE:");
            for comp in no_change.iter().take(10) {
                println!("  {}", comp.format());
            }
            if no_change.len() > 10 {
                println!("  ... and {} more", no_change.len() - 10);
            }
        }

        println!("{}", "═".repeat(70));

        if self.has_regressions() {
            println!("⚠️  {} regression(s) detected!", self.regressions);
        } else {
            println!("✓ No regressions detected");
        }
    }

    /// Export to JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize comparison report")
    }

    /// Export to Markdown
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();

        md.push_str("# Benchmark Comparison Report\n\n");
        md.push_str(&format!("**Baseline:** {}\n", self.baseline_id));
        md.push_str(&format!("**Current:** {}\n", self.current_id));
        md.push_str(&format!("**Threshold:** {:.1}%\n\n", self.threshold));

        md.push_str("## Summary\n\n");
        md.push_str(&format!("| Metric | Count |\n"));
        md.push_str(&format!("|--------|-------|\n"));
        md.push_str(&format!("| Total | {} |\n", self.comparisons.len()));
        md.push_str(&format!("| 🟢 Improvements | {} |\n", self.improvements));
        md.push_str(&format!("| 🔴 Regressions | {} |\n", self.regressions));
        md.push_str(&format!(
            "| ⚪ No change | {} |\n\n",
            self.comparisons.len() - self.improvements - self.regressions
        ));

        md.push_str("## Details\n\n");
        md.push_str("| Benchmark | Baseline | Current | Change | Status |\n");
        md.push_str("|-----------|----------|---------|--------|--------|\n");

        for comp in &self.comparisons {
            let status = if comp.is_regression {
                "🔴"
            } else if comp.is_improvement {
                "🟢"
            } else {
                "⚪"
            };

            md.push_str(&format!(
                "| {} | {} | {} | {:+.1}% | {} |\n",
                comp.name,
                comp.baseline.format_time(),
                comp.current.format_time(),
                comp.time_diff_percent,
                status
            ));
        }

        md
    }
}

/// Benchmark result store
pub struct BenchmarkStore {
    /// Storage directory
    store_dir: PathBuf,
}

impl BenchmarkStore {
    /// Create a new benchmark store
    pub fn new(store_dir: PathBuf) -> Self {
        Self { store_dir }
    }

    /// Default store location
    pub fn default_path() -> PathBuf {
        PathBuf::from(".ccgo").join("benchmarks")
    }

    /// Save a benchmark run
    pub fn save(&self, run: &BenchmarkRun) -> Result<PathBuf> {
        std::fs::create_dir_all(&self.store_dir)?;

        let filename = format!(
            "{}_{}.json",
            run.run_id.replace(['/', '\\', ' '], "_"),
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        );
        let path = self.store_dir.join(filename);

        let json = serde_json::to_string_pretty(run)?;
        std::fs::write(&path, json)?;

        Ok(path)
    }

    /// Load a benchmark run
    pub fn load(&self, run_id: &str) -> Result<BenchmarkRun> {
        // Find file matching run_id
        for entry in std::fs::read_dir(&self.store_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().map(|e| e == "json").unwrap_or(false) {
                let name = path.file_stem().unwrap().to_string_lossy();
                if name.starts_with(&run_id.replace(['/', '\\', ' '], "_")) {
                    let content = std::fs::read_to_string(&path)?;
                    return serde_json::from_str(&content)
                        .context("Failed to parse benchmark file");
                }
            }
        }

        anyhow::bail!("Benchmark run not found: {}", run_id)
    }

    /// Load the latest benchmark run
    pub fn load_latest(&self) -> Result<BenchmarkRun> {
        let mut latest: Option<(PathBuf, std::time::SystemTime)> = None;

        for entry in std::fs::read_dir(&self.store_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().map(|e| e == "json").unwrap_or(false) {
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

        if let Some((path, _)) = latest {
            let content = std::fs::read_to_string(&path)?;
            serde_json::from_str(&content).context("Failed to parse benchmark file")
        } else {
            anyhow::bail!("No benchmark runs found")
        }
    }

    /// List all stored runs
    pub fn list(&self) -> Result<Vec<String>> {
        let mut runs = Vec::new();

        if !self.store_dir.exists() {
            return Ok(runs);
        }

        for entry in std::fs::read_dir(&self.store_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() && path.extension().map(|e| e == "json").unwrap_or(false) {
                if let Ok(content) = std::fs::read_to_string(&path) {
                    if let Ok(run) = serde_json::from_str::<BenchmarkRun>(&content) {
                        runs.push(run.run_id);
                    }
                }
            }
        }

        Ok(runs)
    }

    /// Parse Google Benchmark JSON output
    pub fn parse_gbench_json(&self, path: &Path) -> Result<BenchmarkRun> {
        let content = std::fs::read_to_string(path)?;
        let json: serde_json::Value = serde_json::from_str(&content)?;

        let mut run = BenchmarkRun::new("current");

        if let Some(benchmarks) = json.get("benchmarks").and_then(|b| b.as_array()) {
            for bench in benchmarks {
                let name = bench.get("name").and_then(|n| n.as_str()).unwrap_or("");
                let iterations = bench.get("iterations").and_then(|i| i.as_u64()).unwrap_or(0);
                let real_time = bench.get("real_time").and_then(|t| t.as_f64()).unwrap_or(0.0);
                let cpu_time = bench.get("cpu_time").and_then(|t| t.as_f64()).unwrap_or(0.0);
                let time_unit = bench
                    .get("time_unit")
                    .and_then(|u| u.as_str())
                    .unwrap_or("ns")
                    .to_string();

                run.add_result(BenchmarkResult {
                    name: name.to_string(),
                    iterations,
                    real_time,
                    cpu_time,
                    time_unit,
                    bytes_per_second: bench.get("bytes_per_second").and_then(|b| b.as_f64()),
                    items_per_second: bench.get("items_per_second").and_then(|i| i.as_f64()),
                    counters: HashMap::new(),
                });
            }
        }

        Ok(run)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_benchmark_result_format_time() {
        let result = BenchmarkResult {
            name: "test".to_string(),
            iterations: 1000,
            real_time: 1500.0,
            cpu_time: 1500.0,
            time_unit: "ns".to_string(),
            bytes_per_second: None,
            items_per_second: None,
            counters: HashMap::new(),
        };
        assert_eq!(result.format_time(), "1.50 us");

        let result2 = BenchmarkResult {
            real_time: 1_500_000.0,
            ..result.clone()
        };
        assert_eq!(result2.format_time(), "1.50 ms");
    }

    #[test]
    fn test_comparison() {
        let baseline = BenchmarkResult {
            name: "test".to_string(),
            iterations: 1000,
            real_time: 100.0,
            cpu_time: 100.0,
            time_unit: "ns".to_string(),
            bytes_per_second: None,
            items_per_second: None,
            counters: HashMap::new(),
        };

        let current = BenchmarkResult {
            real_time: 120.0,
            ..baseline.clone()
        };

        let comp = BenchmarkComparison::compare(&baseline, &current, 10.0);
        assert_eq!(comp.time_diff_percent, 20.0);
        assert!(comp.is_regression);
        assert!(!comp.is_improvement);
    }

    #[test]
    fn test_comparison_improvement() {
        let baseline = BenchmarkResult {
            name: "test".to_string(),
            iterations: 1000,
            real_time: 100.0,
            cpu_time: 100.0,
            time_unit: "ns".to_string(),
            bytes_per_second: None,
            items_per_second: None,
            counters: HashMap::new(),
        };

        let current = BenchmarkResult {
            real_time: 80.0,
            ..baseline.clone()
        };

        let comp = BenchmarkComparison::compare(&baseline, &current, 10.0);
        assert_eq!(comp.time_diff_percent, -20.0);
        assert!(!comp.is_regression);
        assert!(comp.is_improvement);
    }
}
