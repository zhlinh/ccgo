//! Build analytics and performance tracking
//!
//! This module provides build performance metrics collection and reporting.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Build phase for timing breakdown
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum BuildPhase {
    /// Dependency resolution
    DependencyResolution,
    /// CMake configuration
    CmakeConfiguration,
    /// Compilation
    Compilation,
    /// Linking
    Linking,
    /// Archive creation
    Archiving,
    /// Post-processing
    PostProcessing,
}

impl std::fmt::Display for BuildPhase {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildPhase::DependencyResolution => write!(f, "Dependency Resolution"),
            BuildPhase::CmakeConfiguration => write!(f, "CMake Configuration"),
            BuildPhase::Compilation => write!(f, "Compilation"),
            BuildPhase::Linking => write!(f, "Linking"),
            BuildPhase::Archiving => write!(f, "Archiving"),
            BuildPhase::PostProcessing => write!(f, "Post-Processing"),
        }
    }
}

/// Phase timing information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhaseMetrics {
    /// Phase name
    pub phase: BuildPhase,
    /// Duration in seconds
    pub duration_secs: f64,
    /// Percentage of total build time
    pub percentage: f64,
}

/// Cache statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CacheStats {
    /// Cache tool used (ccache, sccache, none)
    pub tool: Option<String>,
    /// Cache hits
    pub hits: u64,
    /// Cache misses
    pub misses: u64,
    /// Hit rate percentage (0-100)
    pub hit_rate: f64,
}

/// File statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FileStats {
    /// Number of source files compiled
    pub source_files: usize,
    /// Number of header files
    pub header_files: usize,
    /// Total source lines of code
    pub total_lines: usize,
    /// Output artifact size in bytes
    pub artifact_size_bytes: u64,
}

/// Build analytics data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildAnalytics {
    /// Project name
    pub project: String,
    /// Platform name
    pub platform: String,
    /// Build timestamp (ISO 8601)
    pub timestamp: String,
    /// Total build duration in seconds
    pub total_duration_secs: f64,
    /// Phase timings
    pub phases: Vec<PhaseMetrics>,
    /// Cache statistics
    pub cache_stats: CacheStats,
    /// File statistics
    pub file_stats: FileStats,
    /// Number of parallel jobs used
    pub parallel_jobs: usize,
    /// Peak memory usage in MB
    pub peak_memory_mb: Option<u64>,
    /// Build success status
    pub success: bool,
    /// Error/warning counts
    pub error_count: usize,
    pub warning_count: usize,
}

impl BuildAnalytics {
    /// Get the analytics storage directory
    pub fn analytics_dir() -> Result<PathBuf> {
        let base_dirs = directories::BaseDirs::new()
            .context("Failed to get base directories")?;
        let home = base_dirs.home_dir();
        let dir = home.join(".ccgo").join("analytics");
        std::fs::create_dir_all(&dir)?;
        Ok(dir)
    }

    /// Get the analytics file path for a project
    pub fn analytics_file(project: &str) -> Result<PathBuf> {
        Ok(Self::analytics_dir()?.join(format!("{}.json", project)))
    }

    /// Save analytics to file
    pub fn save(&self) -> Result<()> {
        let file_path = Self::analytics_file(&self.project)?;

        // Load existing analytics
        let mut history = Self::load_history(&self.project)?;

        // Add this build
        history.push(self.clone());

        // Keep only last 100 builds
        if history.len() > 100 {
            history.drain(0..history.len() - 100);
        }

        // Save to file
        let json = serde_json::to_string_pretty(&history)?;
        std::fs::write(&file_path, json)?;

        Ok(())
    }

    /// Load analytics history for a project
    pub fn load_history(project: &str) -> Result<Vec<BuildAnalytics>> {
        let file_path = Self::analytics_file(project)?;

        if !file_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(&file_path)?;
        let history: Vec<BuildAnalytics> = serde_json::from_str(&content)?;

        Ok(history)
    }

    /// Clear analytics history for a project
    pub fn clear_history(project: &str) -> Result<()> {
        let file_path = Self::analytics_file(project)?;

        if file_path.exists() {
            std::fs::remove_file(&file_path)?;
        }

        Ok(())
    }

    /// Get average build time from history
    pub fn average_build_time(project: &str) -> Result<Option<f64>> {
        let history = Self::load_history(project)?;

        if history.is_empty() {
            return Ok(None);
        }

        let total: f64 = history.iter().map(|a| a.total_duration_secs).sum();
        Ok(Some(total / history.len() as f64))
    }

    /// Print analytics summary
    pub fn print_summary(&self) {
        println!("\n{}", "=".repeat(80));
        println!("Build Analytics Summary");
        println!("{}", "=".repeat(80));
        println!();
        println!("Project:        {}", self.project);
        println!("Platform:       {}", self.platform);
        println!("Timestamp:      {}", self.timestamp);
        println!("Total Duration: {:.2}s", self.total_duration_secs);
        println!("Parallel Jobs:  {}", self.parallel_jobs);
        println!("Success:        {}", if self.success { "✓" } else { "✗" });
        println!();

        // Phase breakdown
        if !self.phases.is_empty() {
            println!("Phase Breakdown:");
            println!("{}", "-".repeat(80));
            for phase in &self.phases {
                println!(
                    "  {:.<40} {:>8.2}s ({:>5.1}%)",
                    format!("{}", phase.phase),
                    phase.duration_secs,
                    phase.percentage
                );
            }
            println!();
        }

        // Cache statistics
        if let Some(tool) = &self.cache_stats.tool {
            println!("Cache Statistics (using {}):", tool);
            println!("{}", "-".repeat(80));
            println!("  Hits:     {}", self.cache_stats.hits);
            println!("  Misses:   {}", self.cache_stats.misses);
            println!("  Hit Rate: {:.1}%", self.cache_stats.hit_rate);
            println!();
        }

        // File statistics
        println!("File Statistics:");
        println!("{}", "-".repeat(80));
        println!("  Source Files:  {}", self.file_stats.source_files);
        println!("  Header Files:  {}", self.file_stats.header_files);
        println!("  Total Lines:   {}", self.file_stats.total_lines);
        println!(
            "  Artifact Size: {:.2} MB",
            self.file_stats.artifact_size_bytes as f64 / 1024.0 / 1024.0
        );
        println!();

        // Errors/Warnings
        if self.error_count > 0 || self.warning_count > 0 {
            println!("Diagnostics:");
            println!("{}", "-".repeat(80));
            if self.error_count > 0 {
                println!("  Errors:   {}", self.error_count);
            }
            if self.warning_count > 0 {
                println!("  Warnings: {}", self.warning_count);
            }
            println!();
        }

        println!("{}", "=".repeat(80));
    }
}

/// Build analytics collector
pub struct AnalyticsCollector {
    project: String,
    platform: String,
    start_time: Instant,
    phase_timers: HashMap<BuildPhase, Instant>,
    phase_durations: HashMap<BuildPhase, Duration>,
    parallel_jobs: usize,
    success: bool,
    error_count: usize,
    warning_count: usize,
}

impl AnalyticsCollector {
    /// Create a new analytics collector
    pub fn new(project: String, platform: String, parallel_jobs: usize) -> Self {
        Self {
            project,
            platform,
            start_time: Instant::now(),
            phase_timers: HashMap::new(),
            phase_durations: HashMap::new(),
            parallel_jobs,
            success: false,
            error_count: 0,
            warning_count: 0,
        }
    }

    /// Start timing a phase
    pub fn start_phase(&mut self, phase: BuildPhase) {
        self.phase_timers.insert(phase, Instant::now());
    }

    /// End timing a phase
    pub fn end_phase(&mut self, phase: BuildPhase) {
        if let Some(start) = self.phase_timers.remove(&phase) {
            let duration = start.elapsed();
            self.phase_durations.insert(phase, duration);
        }
    }

    /// Set build success status
    pub fn set_success(&mut self, success: bool) {
        self.success = success;
    }

    /// Increment error count
    pub fn add_error(&mut self) {
        self.error_count += 1;
    }

    /// Increment warning count
    pub fn add_warning(&mut self) {
        self.warning_count += 1;
    }

    /// Add errors/warnings from count
    pub fn add_diagnostics(&mut self, errors: usize, warnings: usize) {
        self.error_count += errors;
        self.warning_count += warnings;
    }

    /// Finalize and create build analytics
    pub fn finalize(
        self,
        cache_stats: Option<CacheStats>,
        file_stats: Option<FileStats>,
    ) -> BuildAnalytics {
        let total_duration = self.start_time.elapsed();
        let total_secs = total_duration.as_secs_f64();

        // Calculate phase metrics
        let mut phases: Vec<PhaseMetrics> = self
            .phase_durations
            .iter()
            .map(|(phase, duration)| {
                let duration_secs = duration.as_secs_f64();
                let percentage = (duration_secs / total_secs) * 100.0;
                PhaseMetrics {
                    phase: *phase,
                    duration_secs,
                    percentage,
                }
            })
            .collect();

        // Sort by duration (longest first)
        phases.sort_by(|a, b| {
            b.duration_secs
                .partial_cmp(&a.duration_secs)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        BuildAnalytics {
            project: self.project,
            platform: self.platform,
            timestamp: chrono::Local::now().to_rfc3339(),
            total_duration_secs: total_secs,
            phases,
            cache_stats: cache_stats.unwrap_or_default(),
            file_stats: file_stats.unwrap_or_default(),
            parallel_jobs: self.parallel_jobs,
            peak_memory_mb: None, // TODO: Implement memory tracking
            success: self.success,
            error_count: self.error_count,
            warning_count: self.warning_count,
        }
    }
}

/// Get cache statistics from cache tool
pub fn get_cache_stats(cache_tool: Option<&str>) -> Option<CacheStats> {
    let tool = cache_tool?;

    match tool {
        "ccache" => get_ccache_stats(),
        "sccache" => get_sccache_stats(),
        _ => None,
    }
}

/// Get ccache statistics
fn get_ccache_stats() -> Option<CacheStats> {
    use std::process::Command;

    let output = Command::new("ccache")
        .arg("-s")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse ccache -s output
    // Example:
    //   cache hit (direct)             12345
    //   cache miss                      6789
    let mut hits = 0u64;
    let mut misses = 0u64;

    for line in stdout.lines() {
        if line.contains("cache hit") {
            if let Some(count) = line.split_whitespace().last() {
                hits += count.parse::<u64>().unwrap_or(0);
            }
        } else if line.contains("cache miss") {
            if let Some(count) = line.split_whitespace().last() {
                misses = count.parse::<u64>().unwrap_or(0);
            }
        }
    }

    let total = hits + misses;
    let hit_rate = if total > 0 {
        (hits as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    Some(CacheStats {
        tool: Some("ccache".to_string()),
        hits,
        misses,
        hit_rate,
    })
}

/// Get sccache statistics
fn get_sccache_stats() -> Option<CacheStats> {
    use std::process::Command;

    let output = Command::new("sccache")
        .arg("--show-stats")
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Parse sccache --show-stats output
    // Example:
    //   Compile requests                    12345
    //   Cache hits                           9876
    //   Cache misses                         2469
    let mut hits = 0u64;
    let mut misses = 0u64;

    for line in stdout.lines() {
        let line = line.trim();
        if line.starts_with("Cache hits") {
            if let Some(count) = line.split_whitespace().last() {
                hits = count.parse::<u64>().unwrap_or(0);
            }
        } else if line.starts_with("Cache misses") {
            if let Some(count) = line.split_whitespace().last() {
                misses = count.parse::<u64>().unwrap_or(0);
            }
        }
    }

    let total = hits + misses;
    let hit_rate = if total > 0 {
        (hits as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    Some(CacheStats {
        tool: Some("sccache".to_string()),
        hits,
        misses,
        hit_rate,
    })
}

/// Count source and header files in a directory
pub fn count_files(dir: &Path) -> Result<(usize, usize)> {
    let mut source_files = 0;
    let mut header_files = 0;

    if !dir.exists() {
        return Ok((0, 0));
    }

    for entry in walkdir::WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .filter_map(|e| e.ok())
    {
        let path = entry.path();
        if path.is_file() {
            if let Some(ext) = path.extension() {
                match ext.to_str() {
                    Some("c") | Some("cc") | Some("cpp") | Some("cxx") => {
                        source_files += 1;
                    }
                    Some("h") | Some("hh") | Some("hpp") | Some("hxx") => {
                        header_files += 1;
                    }
                    _ => {}
                }
            }
        }
    }

    Ok((source_files, header_files))
}

/// Get artifact size
pub fn get_artifact_size(path: &Path) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }

    let metadata = std::fs::metadata(path)?;
    Ok(metadata.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_phase_display() {
        assert_eq!(
            BuildPhase::Compilation.to_string(),
            "Compilation"
        );
    }

    #[test]
    fn test_analytics_collector() {
        let mut collector = AnalyticsCollector::new(
            "test".to_string(),
            "linux".to_string(),
            4,
        );

        collector.start_phase(BuildPhase::Compilation);
        std::thread::sleep(std::time::Duration::from_millis(10));
        collector.end_phase(BuildPhase::Compilation);

        collector.set_success(true);

        let analytics = collector.finalize(None, None);

        assert_eq!(analytics.project, "test");
        assert_eq!(analytics.platform, "linux");
        assert_eq!(analytics.parallel_jobs, 4);
        assert!(analytics.success);
        assert!(analytics.total_duration_secs > 0.0);
    }
}
