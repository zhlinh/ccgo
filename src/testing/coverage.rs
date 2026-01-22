//! Code coverage module
//!
//! Provides code coverage collection and reporting using gcov/llvm-cov.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// Coverage output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CoverageFormat {
    /// HTML report (default)
    #[default]
    Html,
    /// LCOV format
    Lcov,
    /// JSON format
    Json,
    /// Cobertura XML format
    Cobertura,
    /// Console summary
    Summary,
}

impl std::str::FromStr for CoverageFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "html" => Ok(CoverageFormat::Html),
            "lcov" => Ok(CoverageFormat::Lcov),
            "json" => Ok(CoverageFormat::Json),
            "cobertura" | "xml" => Ok(CoverageFormat::Cobertura),
            "summary" | "text" => Ok(CoverageFormat::Summary),
            _ => bail!("Unknown coverage format: {}. Valid formats: html, lcov, json, cobertura, summary", s),
        }
    }
}

impl std::fmt::Display for CoverageFormat {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CoverageFormat::Html => write!(f, "html"),
            CoverageFormat::Lcov => write!(f, "lcov"),
            CoverageFormat::Json => write!(f, "json"),
            CoverageFormat::Cobertura => write!(f, "cobertura"),
            CoverageFormat::Summary => write!(f, "summary"),
        }
    }
}

/// Coverage configuration
#[derive(Debug, Clone)]
pub struct CoverageConfig {
    /// Source directory to analyze
    pub source_dir: PathBuf,
    /// Build directory with coverage data
    pub build_dir: PathBuf,
    /// Output directory for reports
    pub output_dir: PathBuf,
    /// Output format
    pub format: CoverageFormat,
    /// Exclude patterns (glob)
    pub exclude: Vec<String>,
    /// Include only patterns (glob)
    pub include: Vec<String>,
    /// Minimum coverage threshold (0-100)
    pub threshold: Option<f64>,
    /// Whether to fail if below threshold
    pub fail_under: bool,
}

impl Default for CoverageConfig {
    fn default() -> Self {
        Self {
            source_dir: PathBuf::from("."),
            build_dir: PathBuf::from("cmake_build"),
            output_dir: PathBuf::from("coverage"),
            format: CoverageFormat::Html,
            exclude: vec![
                "*/test/*".to_string(),
                "*/tests/*".to_string(),
                "*/third_party/*".to_string(),
                "*/vendor/*".to_string(),
            ],
            include: vec![],
            threshold: None,
            fail_under: false,
        }
    }
}

/// File coverage data
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileCoverage {
    /// File path
    pub path: PathBuf,
    /// Lines covered
    pub lines_covered: u32,
    /// Total lines
    pub lines_total: u32,
    /// Branches covered
    pub branches_covered: u32,
    /// Total branches
    pub branches_total: u32,
    /// Functions covered
    pub functions_covered: u32,
    /// Total functions
    pub functions_total: u32,
    /// Line-by-line coverage (line number -> hit count)
    pub line_coverage: HashMap<u32, u32>,
}

impl FileCoverage {
    /// Calculate line coverage percentage
    pub fn line_coverage_percent(&self) -> f64 {
        if self.lines_total == 0 {
            100.0
        } else {
            (self.lines_covered as f64 / self.lines_total as f64) * 100.0
        }
    }

    /// Calculate branch coverage percentage
    pub fn branch_coverage_percent(&self) -> f64 {
        if self.branches_total == 0 {
            100.0
        } else {
            (self.branches_covered as f64 / self.branches_total as f64) * 100.0
        }
    }

    /// Calculate function coverage percentage
    pub fn function_coverage_percent(&self) -> f64 {
        if self.functions_total == 0 {
            100.0
        } else {
            (self.functions_covered as f64 / self.functions_total as f64) * 100.0
        }
    }
}

/// Coverage report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CoverageReport {
    /// Timestamp of report generation
    pub timestamp: String,
    /// Total lines covered
    pub lines_covered: u32,
    /// Total lines
    pub lines_total: u32,
    /// Total branches covered
    pub branches_covered: u32,
    /// Total branches
    pub branches_total: u32,
    /// Total functions covered
    pub functions_covered: u32,
    /// Total functions
    pub functions_total: u32,
    /// Per-file coverage
    pub files: Vec<FileCoverage>,
}

impl CoverageReport {
    /// Create a new empty report
    pub fn new() -> Self {
        Self {
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            lines_covered: 0,
            lines_total: 0,
            branches_covered: 0,
            branches_total: 0,
            functions_covered: 0,
            functions_total: 0,
            files: Vec::new(),
        }
    }

    /// Calculate overall line coverage percentage
    pub fn line_coverage_percent(&self) -> f64 {
        if self.lines_total == 0 {
            100.0
        } else {
            (self.lines_covered as f64 / self.lines_total as f64) * 100.0
        }
    }

    /// Calculate overall branch coverage percentage
    pub fn branch_coverage_percent(&self) -> f64 {
        if self.branches_total == 0 {
            100.0
        } else {
            (self.branches_covered as f64 / self.branches_total as f64) * 100.0
        }
    }

    /// Calculate overall function coverage percentage
    pub fn function_coverage_percent(&self) -> f64 {
        if self.functions_total == 0 {
            100.0
        } else {
            (self.functions_covered as f64 / self.functions_total as f64) * 100.0
        }
    }

    /// Add file coverage and update totals
    pub fn add_file(&mut self, file: FileCoverage) {
        self.lines_covered += file.lines_covered;
        self.lines_total += file.lines_total;
        self.branches_covered += file.branches_covered;
        self.branches_total += file.branches_total;
        self.functions_covered += file.functions_covered;
        self.functions_total += file.functions_total;
        self.files.push(file);
    }

    /// Check if coverage meets threshold
    pub fn meets_threshold(&self, threshold: f64) -> bool {
        self.line_coverage_percent() >= threshold
    }

    /// Print summary to console
    pub fn print_summary(&self) {
        println!("\n{}", "═".repeat(60));
        println!("CODE COVERAGE REPORT");
        println!("{}", "═".repeat(60));
        println!("Generated: {}", self.timestamp);
        println!();
        println!("Overall Coverage:");
        println!(
            "  Lines:     {:>6}/{:<6} ({:.1}%)",
            self.lines_covered,
            self.lines_total,
            self.line_coverage_percent()
        );
        println!(
            "  Branches:  {:>6}/{:<6} ({:.1}%)",
            self.branches_covered,
            self.branches_total,
            self.branch_coverage_percent()
        );
        println!(
            "  Functions: {:>6}/{:<6} ({:.1}%)",
            self.functions_covered,
            self.functions_total,
            self.function_coverage_percent()
        );
        println!();
        println!("Files: {}", self.files.len());
        println!("{}", "─".repeat(60));

        // Show top 10 files with lowest coverage
        let mut sorted_files: Vec<_> = self.files.iter().collect();
        sorted_files.sort_by(|a, b| {
            a.line_coverage_percent()
                .partial_cmp(&b.line_coverage_percent())
                .unwrap()
        });

        println!("\nLowest Coverage Files:");
        for file in sorted_files.iter().take(10) {
            let path_str = file.path.to_string_lossy();
            let display_path = if path_str.len() > 45 {
                format!("...{}", &path_str[path_str.len() - 42..])
            } else {
                path_str.to_string()
            };
            println!(
                "  {:>5.1}%  {}",
                file.line_coverage_percent(),
                display_path
            );
        }

        println!("{}", "═".repeat(60));
    }

    /// Export to JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize coverage report")
    }

    /// Export to LCOV format
    pub fn to_lcov(&self) -> String {
        let mut output = String::new();

        for file in &self.files {
            output.push_str(&format!("SF:{}\n", file.path.display()));

            for (line, hits) in &file.line_coverage {
                output.push_str(&format!("DA:{},{}\n", line, hits));
            }

            output.push_str(&format!("LF:{}\n", file.lines_total));
            output.push_str(&format!("LH:{}\n", file.lines_covered));
            output.push_str("end_of_record\n");
        }

        output
    }
}

impl Default for CoverageReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Coverage collector
pub struct CoverageCollector {
    config: CoverageConfig,
    verbose: bool,
}

impl CoverageCollector {
    /// Create a new coverage collector
    pub fn new(config: CoverageConfig, verbose: bool) -> Self {
        Self { config, verbose }
    }

    /// Detect available coverage tool
    pub fn detect_tool(&self) -> Result<CoverageTool> {
        // Try llvm-cov first (better for Clang)
        if Command::new("llvm-cov").arg("--version").output().is_ok() {
            return Ok(CoverageTool::LlvmCov);
        }

        // Try gcov (for GCC)
        if Command::new("gcov").arg("--version").output().is_ok() {
            return Ok(CoverageTool::Gcov);
        }

        // Try lcov (wrapper for gcov)
        if Command::new("lcov").arg("--version").output().is_ok() {
            return Ok(CoverageTool::Lcov);
        }

        bail!("No coverage tool found. Install llvm-cov, gcov, or lcov.")
    }

    /// Collect coverage data
    pub fn collect(&self) -> Result<CoverageReport> {
        let tool = self.detect_tool()?;

        if self.verbose {
            eprintln!("Using coverage tool: {:?}", tool);
        }

        match tool {
            CoverageTool::LlvmCov => self.collect_llvm_cov(),
            CoverageTool::Gcov => self.collect_gcov(),
            CoverageTool::Lcov => self.collect_lcov(),
        }
    }

    /// Collect using llvm-cov
    fn collect_llvm_cov(&self) -> Result<CoverageReport> {
        // Find profraw files
        let profdata = self.config.build_dir.join("default.profdata");

        // Merge profraw files if needed
        let profraw_files: Vec<_> = walkdir::WalkDir::new(&self.config.build_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "profraw").unwrap_or(false))
            .map(|e| e.path().to_path_buf())
            .collect();

        if !profraw_files.is_empty() {
            let mut cmd = Command::new("llvm-profdata");
            cmd.arg("merge")
                .arg("-sparse")
                .arg("-o")
                .arg(&profdata);

            for file in &profraw_files {
                cmd.arg(file);
            }

            let status = cmd.status().context("Failed to run llvm-profdata")?;
            if !status.success() {
                bail!("llvm-profdata merge failed");
            }
        }

        // Find test executable
        let test_exe = self.find_instrumented_executable()?;

        // Generate coverage report
        let output = Command::new("llvm-cov")
            .arg("export")
            .arg("-format=text")
            .arg("-instr-profile")
            .arg(&profdata)
            .arg(&test_exe)
            .output()
            .context("Failed to run llvm-cov export")?;

        if !output.status.success() {
            bail!("llvm-cov export failed");
        }

        // Parse output
        self.parse_llvm_cov_output(&String::from_utf8_lossy(&output.stdout))
    }

    /// Collect using gcov
    fn collect_gcov(&self) -> Result<CoverageReport> {
        // Find .gcda files
        let gcda_files: Vec<_> = walkdir::WalkDir::new(&self.config.build_dir)
            .into_iter()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "gcda").unwrap_or(false))
            .map(|e| e.path().to_path_buf())
            .collect();

        if gcda_files.is_empty() {
            bail!("No .gcda files found. Did you run tests with coverage enabled?");
        }

        let mut report = CoverageReport::new();

        for gcda in gcda_files {
            let output = Command::new("gcov")
                .arg("-b")
                .arg("-c")
                .arg(&gcda)
                .current_dir(gcda.parent().unwrap_or(Path::new(".")))
                .output()
                .context("Failed to run gcov")?;

            if output.status.success() {
                if let Ok(file_coverage) = self.parse_gcov_output(
                    &gcda,
                    &String::from_utf8_lossy(&output.stdout),
                ) {
                    report.add_file(file_coverage);
                }
            }
        }

        Ok(report)
    }

    /// Collect using lcov
    fn collect_lcov(&self) -> Result<CoverageReport> {
        let info_file = self.config.output_dir.join("coverage.info");

        // Create output directory
        std::fs::create_dir_all(&self.config.output_dir)?;

        // Capture coverage
        let status = Command::new("lcov")
            .arg("--capture")
            .arg("--directory")
            .arg(&self.config.build_dir)
            .arg("--output-file")
            .arg(&info_file)
            .status()
            .context("Failed to run lcov")?;

        if !status.success() {
            bail!("lcov capture failed");
        }

        // Remove unwanted files
        if !self.config.exclude.is_empty() {
            let mut cmd = Command::new("lcov");
            cmd.arg("--remove")
                .arg(&info_file)
                .arg("--output-file")
                .arg(&info_file);

            for pattern in &self.config.exclude {
                cmd.arg(pattern);
            }

            cmd.status().context("Failed to filter lcov data")?;
        }

        // Parse info file
        self.parse_lcov_info(&info_file)
    }

    /// Find instrumented test executable
    fn find_instrumented_executable(&self) -> Result<PathBuf> {
        let install_dir = self.config.build_dir.join("out");

        for entry in std::fs::read_dir(&install_dir).unwrap_or_else(|_| {
            std::fs::read_dir(&self.config.build_dir).expect("Build dir not found")
        }) {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let name = path.file_name().unwrap().to_string_lossy();
                if name.contains("test") || name.contains("Test") {
                    return Ok(path);
                }
            }
        }

        bail!("No instrumented test executable found")
    }

    /// Parse llvm-cov export output
    fn parse_llvm_cov_output(&self, _output: &str) -> Result<CoverageReport> {
        // Simplified parsing - in production, parse JSON output
        Ok(CoverageReport::new())
    }

    /// Parse gcov output for a file
    fn parse_gcov_output(&self, gcda_path: &Path, _output: &str) -> Result<FileCoverage> {
        // Find corresponding .gcov file
        let gcov_file = gcda_path.with_extension("gcov");

        let mut coverage = FileCoverage {
            path: gcda_path.with_extension("cpp"),
            lines_covered: 0,
            lines_total: 0,
            branches_covered: 0,
            branches_total: 0,
            functions_covered: 0,
            functions_total: 0,
            line_coverage: HashMap::new(),
        };

        if gcov_file.exists() {
            let content = std::fs::read_to_string(&gcov_file)?;

            for line in content.lines() {
                // Format: hits:line:source
                let parts: Vec<&str> = line.splitn(3, ':').collect();
                if parts.len() >= 2 {
                    let hits_str = parts[0].trim();
                    if let Ok(line_num) = parts[1].trim().parse::<u32>() {
                        if hits_str == "-" {
                            // Non-executable line
                            continue;
                        } else if hits_str == "#####" {
                            coverage.lines_total += 1;
                            coverage.line_coverage.insert(line_num, 0);
                        } else if let Ok(hits) = hits_str.parse::<u32>() {
                            coverage.lines_total += 1;
                            coverage.lines_covered += 1;
                            coverage.line_coverage.insert(line_num, hits);
                        }
                    }
                }
            }
        }

        Ok(coverage)
    }

    /// Parse lcov info file
    fn parse_lcov_info(&self, info_file: &Path) -> Result<CoverageReport> {
        let content = std::fs::read_to_string(info_file)?;
        let mut report = CoverageReport::new();
        let mut current_file: Option<FileCoverage> = None;

        for line in content.lines() {
            if line.starts_with("SF:") {
                // New source file
                if let Some(file) = current_file.take() {
                    report.add_file(file);
                }
                current_file = Some(FileCoverage {
                    path: PathBuf::from(&line[3..]),
                    lines_covered: 0,
                    lines_total: 0,
                    branches_covered: 0,
                    branches_total: 0,
                    functions_covered: 0,
                    functions_total: 0,
                    line_coverage: HashMap::new(),
                });
            } else if line.starts_with("DA:") {
                // Line data: DA:line,hits
                if let Some(ref mut file) = current_file {
                    let parts: Vec<&str> = line[3..].split(',').collect();
                    if parts.len() >= 2 {
                        if let (Ok(line_num), Ok(hits)) =
                            (parts[0].parse::<u32>(), parts[1].parse::<u32>())
                        {
                            file.lines_total += 1;
                            if hits > 0 {
                                file.lines_covered += 1;
                            }
                            file.line_coverage.insert(line_num, hits);
                        }
                    }
                }
            } else if line.starts_with("BRDA:") {
                // Branch data
                if let Some(ref mut file) = current_file {
                    file.branches_total += 1;
                    if !line.ends_with("-") && !line.ends_with(",0") {
                        file.branches_covered += 1;
                    }
                }
            } else if line.starts_with("FNF:") {
                // Function count
                if let Some(ref mut file) = current_file {
                    if let Ok(count) = line[4..].parse::<u32>() {
                        file.functions_total = count;
                    }
                }
            } else if line.starts_with("FNH:") {
                // Functions hit
                if let Some(ref mut file) = current_file {
                    if let Ok(count) = line[4..].parse::<u32>() {
                        file.functions_covered = count;
                    }
                }
            } else if line == "end_of_record" {
                if let Some(file) = current_file.take() {
                    report.add_file(file);
                }
            }
        }

        // Don't forget the last file
        if let Some(file) = current_file {
            report.add_file(file);
        }

        Ok(report)
    }

    /// Generate HTML report using genhtml
    pub fn generate_html(&self, report: &CoverageReport) -> Result<PathBuf> {
        let html_dir = self.config.output_dir.join("html");
        std::fs::create_dir_all(&html_dir)?;

        // Write LCOV data
        let lcov_file = self.config.output_dir.join("coverage.info");
        std::fs::write(&lcov_file, report.to_lcov())?;

        // Check if genhtml is available
        if Command::new("genhtml").arg("--version").output().is_ok() {
            let status = Command::new("genhtml")
                .arg(&lcov_file)
                .arg("--output-directory")
                .arg(&html_dir)
                .arg("--title")
                .arg("CCGO Coverage Report")
                .status()
                .context("Failed to run genhtml")?;

            if !status.success() {
                bail!("genhtml failed");
            }

            Ok(html_dir.join("index.html"))
        } else {
            // Generate simple HTML manually
            self.generate_simple_html(report, &html_dir)
        }
    }

    /// Generate simple HTML report without genhtml
    fn generate_simple_html(&self, report: &CoverageReport, html_dir: &Path) -> Result<PathBuf> {
        let index_path = html_dir.join("index.html");

        let html = format!(
            r#"<!DOCTYPE html>
<html>
<head>
    <title>Coverage Report</title>
    <style>
        body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif; margin: 20px; }}
        h1 {{ color: #333; }}
        table {{ border-collapse: collapse; width: 100%; }}
        th, td {{ border: 1px solid #ddd; padding: 8px; text-align: left; }}
        th {{ background-color: #4CAF50; color: white; }}
        tr:nth-child(even) {{ background-color: #f2f2f2; }}
        .high {{ color: green; }}
        .medium {{ color: orange; }}
        .low {{ color: red; }}
        .summary {{ background-color: #e7f3fe; padding: 15px; border-radius: 5px; margin-bottom: 20px; }}
    </style>
</head>
<body>
    <h1>Coverage Report</h1>
    <div class="summary">
        <p><strong>Generated:</strong> {}</p>
        <p><strong>Line Coverage:</strong> <span class="{}">{:.1}%</span> ({}/{})</p>
        <p><strong>Branch Coverage:</strong> {:.1}% ({}/{})</p>
        <p><strong>Function Coverage:</strong> {:.1}% ({}/{})</p>
    </div>
    <h2>Files</h2>
    <table>
        <tr>
            <th>File</th>
            <th>Line Coverage</th>
            <th>Lines</th>
        </tr>
        {}
    </table>
</body>
</html>"#,
            report.timestamp,
            if report.line_coverage_percent() >= 80.0 {
                "high"
            } else if report.line_coverage_percent() >= 50.0 {
                "medium"
            } else {
                "low"
            },
            report.line_coverage_percent(),
            report.lines_covered,
            report.lines_total,
            report.branch_coverage_percent(),
            report.branches_covered,
            report.branches_total,
            report.function_coverage_percent(),
            report.functions_covered,
            report.functions_total,
            report
                .files
                .iter()
                .map(|f| format!(
                    "<tr><td>{}</td><td class=\"{}\">{:.1}%</td><td>{}/{}</td></tr>",
                    f.path.display(),
                    if f.line_coverage_percent() >= 80.0 {
                        "high"
                    } else if f.line_coverage_percent() >= 50.0 {
                        "medium"
                    } else {
                        "low"
                    },
                    f.line_coverage_percent(),
                    f.lines_covered,
                    f.lines_total
                ))
                .collect::<Vec<_>>()
                .join("\n")
        );

        std::fs::write(&index_path, html)?;
        Ok(index_path)
    }
}

/// Available coverage tools
#[derive(Debug, Clone, Copy)]
pub enum CoverageTool {
    LlvmCov,
    Gcov,
    Lcov,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coverage_format_parse() {
        assert_eq!(
            "html".parse::<CoverageFormat>().unwrap(),
            CoverageFormat::Html
        );
        assert_eq!(
            "lcov".parse::<CoverageFormat>().unwrap(),
            CoverageFormat::Lcov
        );
        assert_eq!(
            "json".parse::<CoverageFormat>().unwrap(),
            CoverageFormat::Json
        );
    }

    #[test]
    fn test_coverage_report_new() {
        let report = CoverageReport::new();
        assert_eq!(report.lines_covered, 0);
        assert_eq!(report.lines_total, 0);
        assert_eq!(report.line_coverage_percent(), 100.0);
    }

    #[test]
    fn test_file_coverage_percent() {
        let file = FileCoverage {
            path: PathBuf::from("test.cpp"),
            lines_covered: 80,
            lines_total: 100,
            branches_covered: 0,
            branches_total: 0,
            functions_covered: 0,
            functions_total: 0,
            line_coverage: HashMap::new(),
        };
        assert_eq!(file.line_coverage_percent(), 80.0);
    }

    #[test]
    fn test_coverage_threshold() {
        let mut report = CoverageReport::new();
        report.lines_covered = 75;
        report.lines_total = 100;

        assert!(report.meets_threshold(70.0));
        assert!(!report.meets_threshold(80.0));
    }
}
