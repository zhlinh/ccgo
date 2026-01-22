//! Test result aggregation module
//!
//! Parses and aggregates test results from GoogleTest XML output.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// Test result status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestStatus {
    Passed,
    Failed,
    Skipped,
    Error,
}

impl std::fmt::Display for TestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestStatus::Passed => write!(f, "PASSED"),
            TestStatus::Failed => write!(f, "FAILED"),
            TestStatus::Skipped => write!(f, "SKIPPED"),
            TestStatus::Error => write!(f, "ERROR"),
        }
    }
}

/// Individual test result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    /// Test name
    pub name: String,
    /// Test suite name
    pub suite: String,
    /// Test status
    pub status: TestStatus,
    /// Duration in seconds
    pub duration: f64,
    /// Failure message (if failed)
    pub message: Option<String>,
    /// Stack trace (if available)
    pub stack_trace: Option<String>,
    /// Output/stdout
    pub output: Option<String>,
    /// Timestamp
    pub timestamp: String,
}

impl TestResult {
    /// Create a new passed test result
    pub fn passed(suite: &str, name: &str, duration: f64) -> Self {
        Self {
            name: name.to_string(),
            suite: suite.to_string(),
            status: TestStatus::Passed,
            duration,
            message: None,
            stack_trace: None,
            output: None,
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }

    /// Create a new failed test result
    pub fn failed(suite: &str, name: &str, duration: f64, message: &str) -> Self {
        Self {
            name: name.to_string(),
            suite: suite.to_string(),
            status: TestStatus::Failed,
            duration,
            message: Some(message.to_string()),
            stack_trace: None,
            output: None,
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }

    /// Full test name (suite.name)
    pub fn full_name(&self) -> String {
        format!("{}.{}", self.suite, self.name)
    }
}

/// Test suite results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSuiteResult {
    /// Suite name
    pub name: String,
    /// Tests in this suite
    pub tests: Vec<TestResult>,
    /// Total duration
    pub duration: f64,
    /// Timestamp
    pub timestamp: String,
}

impl TestSuiteResult {
    /// Create a new test suite result
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            tests: Vec::new(),
            duration: 0.0,
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }

    /// Add a test result
    pub fn add_test(&mut self, test: TestResult) {
        self.duration += test.duration;
        self.tests.push(test);
    }

    /// Count passed tests
    pub fn passed_count(&self) -> usize {
        self.tests.iter().filter(|t| t.status == TestStatus::Passed).count()
    }

    /// Count failed tests
    pub fn failed_count(&self) -> usize {
        self.tests.iter().filter(|t| t.status == TestStatus::Failed).count()
    }

    /// Count skipped tests
    pub fn skipped_count(&self) -> usize {
        self.tests.iter().filter(|t| t.status == TestStatus::Skipped).count()
    }

    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.tests.iter().all(|t| t.status == TestStatus::Passed)
    }
}

/// Aggregated test summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestSummary {
    /// Total tests
    pub total: usize,
    /// Passed tests
    pub passed: usize,
    /// Failed tests
    pub failed: usize,
    /// Skipped tests
    pub skipped: usize,
    /// Error tests
    pub errors: usize,
    /// Total duration in seconds
    pub duration: f64,
    /// Pass rate (0.0 - 1.0)
    pub pass_rate: f64,
    /// Timestamp
    pub timestamp: String,
    /// Test suites
    pub suites: Vec<TestSuiteResult>,
}

impl TestSummary {
    /// Create an empty summary
    pub fn new() -> Self {
        Self {
            total: 0,
            passed: 0,
            failed: 0,
            skipped: 0,
            errors: 0,
            duration: 0.0,
            pass_rate: 0.0,
            timestamp: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            suites: Vec::new(),
        }
    }

    /// Add a test suite result
    pub fn add_suite(&mut self, suite: TestSuiteResult) {
        for test in &suite.tests {
            self.total += 1;
            self.duration += test.duration;
            match test.status {
                TestStatus::Passed => self.passed += 1,
                TestStatus::Failed => self.failed += 1,
                TestStatus::Skipped => self.skipped += 1,
                TestStatus::Error => self.errors += 1,
            }
        }
        self.pass_rate = if self.total > 0 {
            self.passed as f64 / self.total as f64
        } else {
            0.0
        };
        self.suites.push(suite);
    }

    /// Check if all tests passed
    pub fn all_passed(&self) -> bool {
        self.failed == 0 && self.errors == 0
    }

    /// Get failed tests
    pub fn failed_tests(&self) -> Vec<&TestResult> {
        self.suites
            .iter()
            .flat_map(|s| s.tests.iter())
            .filter(|t| t.status == TestStatus::Failed)
            .collect()
    }

    /// Print summary to console
    pub fn print_summary(&self) {
        println!("\n{}", "═".repeat(60));
        println!("TEST RESULTS SUMMARY");
        println!("{}", "═".repeat(60));
        println!("Timestamp: {}", self.timestamp);
        println!("Duration:  {:.2}s", self.duration);
        println!();

        // Status bar
        let bar_width: usize = 40;
        let passed_width = (self.passed as f64 / self.total.max(1) as f64 * bar_width as f64) as usize;
        let failed_width = (self.failed as f64 / self.total.max(1) as f64 * bar_width as f64) as usize;
        let skipped_width = bar_width.saturating_sub(passed_width).saturating_sub(failed_width);

        print!("[");
        print!("{}", "█".repeat(passed_width));
        print!("{}", "▓".repeat(failed_width));
        print!("{}", "░".repeat(skipped_width));
        println!("]");

        println!();
        println!("  ✓ Passed:  {:>4} ({:.1}%)", self.passed, self.pass_rate * 100.0);
        println!("  ✗ Failed:  {:>4}", self.failed);
        println!("  ○ Skipped: {:>4}", self.skipped);
        if self.errors > 0 {
            println!("  ! Errors:  {:>4}", self.errors);
        }
        println!("  ─────────────────");
        println!("  Total:     {:>4}", self.total);

        // Show failed tests
        if self.failed > 0 {
            println!();
            println!("{}", "─".repeat(60));
            println!("FAILED TESTS:");
            for test in self.failed_tests() {
                println!();
                println!("  ✗ {}", test.full_name());
                if let Some(ref msg) = test.message {
                    for line in msg.lines().take(5) {
                        println!("    {}", line);
                    }
                }
            }
        }

        println!("{}", "═".repeat(60));

        if self.all_passed() {
            println!("✓ All tests passed!");
        } else {
            println!("✗ {} test(s) failed", self.failed + self.errors);
        }
    }

    /// Export to JSON
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).context("Failed to serialize test summary")
    }

    /// Export to JUnit XML format
    pub fn to_junit_xml(&self) -> String {
        let mut xml = String::new();
        xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        xml.push_str(&format!(
            "<testsuites tests=\"{}\" failures=\"{}\" errors=\"{}\" skipped=\"{}\" time=\"{:.3}\">\n",
            self.total, self.failed, self.errors, self.skipped, self.duration
        ));

        for suite in &self.suites {
            xml.push_str(&format!(
                "  <testsuite name=\"{}\" tests=\"{}\" failures=\"{}\" errors=\"0\" skipped=\"{}\" time=\"{:.3}\">\n",
                escape_xml(&suite.name),
                suite.tests.len(),
                suite.failed_count(),
                suite.skipped_count(),
                suite.duration
            ));

            for test in &suite.tests {
                xml.push_str(&format!(
                    "    <testcase name=\"{}\" classname=\"{}\" time=\"{:.3}\"",
                    escape_xml(&test.name),
                    escape_xml(&test.suite),
                    test.duration
                ));

                match test.status {
                    TestStatus::Passed => {
                        xml.push_str("/>\n");
                    }
                    TestStatus::Failed => {
                        xml.push_str(">\n");
                        xml.push_str(&format!(
                            "      <failure message=\"{}\">{}</failure>\n",
                            escape_xml(test.message.as_deref().unwrap_or("")),
                            escape_xml(test.stack_trace.as_deref().unwrap_or(""))
                        ));
                        xml.push_str("    </testcase>\n");
                    }
                    TestStatus::Skipped => {
                        xml.push_str(">\n");
                        xml.push_str("      <skipped/>\n");
                        xml.push_str("    </testcase>\n");
                    }
                    TestStatus::Error => {
                        xml.push_str(">\n");
                        xml.push_str(&format!(
                            "      <error message=\"{}\">{}</error>\n",
                            escape_xml(test.message.as_deref().unwrap_or("")),
                            escape_xml(test.stack_trace.as_deref().unwrap_or(""))
                        ));
                        xml.push_str("    </testcase>\n");
                    }
                }
            }

            xml.push_str("  </testsuite>\n");
        }

        xml.push_str("</testsuites>\n");
        xml
    }
}

impl Default for TestSummary {
    fn default() -> Self {
        Self::new()
    }
}

/// Escape XML special characters
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Test result aggregator
pub struct TestResultAggregator {
    /// Collected results
    results: Vec<PathBuf>,
    /// Verbose output
    verbose: bool,
}

impl TestResultAggregator {
    /// Create a new aggregator
    pub fn new(verbose: bool) -> Self {
        Self {
            results: Vec::new(),
            verbose,
        }
    }

    /// Add a result file (XML)
    pub fn add_result_file(&mut self, path: PathBuf) {
        self.results.push(path);
    }

    /// Find all result files in a directory
    pub fn find_results(&mut self, dir: &Path) -> Result<()> {
        for entry in walkdir::WalkDir::new(dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                let name = path.file_name().unwrap().to_string_lossy();
                if name.ends_with("_result.xml") || name.ends_with("_results.xml") {
                    self.results.push(path.to_path_buf());
                }
            }
        }
        Ok(())
    }

    /// Aggregate all results
    pub fn aggregate(&self) -> Result<TestSummary> {
        let mut summary = TestSummary::new();

        for result_file in &self.results {
            if self.verbose {
                eprintln!("Parsing: {}", result_file.display());
            }

            match self.parse_gtest_xml(result_file) {
                Ok(suites) => {
                    for suite in suites {
                        summary.add_suite(suite);
                    }
                }
                Err(e) => {
                    if self.verbose {
                        eprintln!("Warning: Failed to parse {}: {}", result_file.display(), e);
                    }
                }
            }
        }

        Ok(summary)
    }

    /// Parse GoogleTest XML output
    fn parse_gtest_xml(&self, path: &Path) -> Result<Vec<TestSuiteResult>> {
        let content = std::fs::read_to_string(path)
            .context("Failed to read test result file")?;

        let mut suites = Vec::new();
        let mut current_suite: Option<TestSuiteResult> = None;
        let mut in_failure = false;
        let mut failure_message = String::new();
        let mut current_test: Option<TestResult> = None;

        for line in content.lines() {
            let line = line.trim();

            // Parse testsuite
            if line.starts_with("<testsuite ") || line.starts_with("<testsuite>") {
                if let Some(suite) = current_suite.take() {
                    suites.push(suite);
                }

                let name = extract_attr(line, "name").unwrap_or_else(|| "Unknown".to_string());
                current_suite = Some(TestSuiteResult::new(&name));
            }
            // Parse testcase
            else if line.starts_with("<testcase ") {
                let name = extract_attr(line, "name").unwrap_or_else(|| "Unknown".to_string());
                let classname = extract_attr(line, "classname").unwrap_or_else(|| "Unknown".to_string());
                let time = extract_attr(line, "time")
                    .and_then(|t| t.parse::<f64>().ok())
                    .unwrap_or(0.0);

                let mut test = TestResult::passed(&classname, &name, time);

                // Check if self-closing (passed)
                if line.ends_with("/>") {
                    if let Some(ref mut suite) = current_suite {
                        suite.add_test(test);
                    }
                } else {
                    current_test = Some(test);
                }
            }
            // Parse failure
            else if line.starts_with("<failure") {
                in_failure = true;
                failure_message.clear();

                if let Some(msg) = extract_attr(line, "message") {
                    failure_message = msg;
                }

                if let Some(ref mut test) = current_test {
                    test.status = TestStatus::Failed;
                }
            }
            // End failure
            else if line.contains("</failure>") {
                in_failure = false;
                if let Some(ref mut test) = current_test {
                    test.message = Some(failure_message.clone());
                }
            }
            // Parse skipped
            else if line.contains("<skipped") {
                if let Some(ref mut test) = current_test {
                    test.status = TestStatus::Skipped;
                }
            }
            // End testcase
            else if line == "</testcase>" {
                if let Some(test) = current_test.take() {
                    if let Some(ref mut suite) = current_suite {
                        suite.add_test(test);
                    }
                }
            }
            // End testsuite
            else if line == "</testsuite>" {
                if let Some(suite) = current_suite.take() {
                    suites.push(suite);
                }
            }
            // Collect failure content
            else if in_failure {
                if !failure_message.is_empty() {
                    failure_message.push('\n');
                }
                failure_message.push_str(line);
            }
        }

        // Don't forget last suite
        if let Some(suite) = current_suite {
            suites.push(suite);
        }

        Ok(suites)
    }
}

/// Extract attribute value from XML element
fn extract_attr(line: &str, attr: &str) -> Option<String> {
    let pattern = format!("{}=\"", attr);
    if let Some(start) = line.find(&pattern) {
        let value_start = start + pattern.len();
        if let Some(end) = line[value_start..].find('"') {
            return Some(line[value_start..value_start + end].to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_status_display() {
        assert_eq!(format!("{}", TestStatus::Passed), "PASSED");
        assert_eq!(format!("{}", TestStatus::Failed), "FAILED");
    }

    #[test]
    fn test_test_result_full_name() {
        let result = TestResult::passed("MySuite", "MyTest", 0.5);
        assert_eq!(result.full_name(), "MySuite.MyTest");
    }

    #[test]
    fn test_suite_counts() {
        let mut suite = TestSuiteResult::new("TestSuite");
        suite.add_test(TestResult::passed("TestSuite", "Test1", 0.1));
        suite.add_test(TestResult::passed("TestSuite", "Test2", 0.2));
        suite.add_test(TestResult::failed("TestSuite", "Test3", 0.3, "error"));

        assert_eq!(suite.passed_count(), 2);
        assert_eq!(suite.failed_count(), 1);
        assert!(!suite.all_passed());
    }

    #[test]
    fn test_summary_aggregation() {
        let mut summary = TestSummary::new();

        let mut suite = TestSuiteResult::new("Suite1");
        suite.add_test(TestResult::passed("Suite1", "Test1", 0.1));
        suite.add_test(TestResult::passed("Suite1", "Test2", 0.2));
        summary.add_suite(suite);

        let mut suite2 = TestSuiteResult::new("Suite2");
        suite2.add_test(TestResult::failed("Suite2", "Test3", 0.3, "error"));
        summary.add_suite(suite2);

        assert_eq!(summary.total, 3);
        assert_eq!(summary.passed, 2);
        assert_eq!(summary.failed, 1);
        assert!(!summary.all_passed());
    }

    #[test]
    fn test_escape_xml() {
        assert_eq!(escape_xml("<test>"), "&lt;test&gt;");
        assert_eq!(escape_xml("a & b"), "a &amp; b");
    }

    #[test]
    fn test_extract_attr() {
        let line = r#"<testcase name="MyTest" time="0.5">"#;
        assert_eq!(extract_attr(line, "name"), Some("MyTest".to_string()));
        assert_eq!(extract_attr(line, "time"), Some("0.5".to_string()));
        assert_eq!(extract_attr(line, "unknown"), None);
    }
}
