//! Test discovery module
//!
//! Provides automatic discovery of tests from executables and CMake CTest.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

/// A discovered test
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredTest {
    /// Test name (e.g., "TestSuite.TestCase")
    pub name: String,
    /// Test suite name
    pub suite: String,
    /// Test case name
    pub case: String,
    /// Source file (if available)
    pub file: Option<String>,
    /// Line number (if available)
    pub line: Option<u32>,
    /// Executable containing this test
    pub executable: PathBuf,
    /// Test type (GoogleTest, Catch2, CTest)
    pub test_type: TestType,
}

/// Type of test framework
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TestType {
    GoogleTest,
    Catch2,
    CTest,
    Unknown,
}

impl std::fmt::Display for TestType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestType::GoogleTest => write!(f, "GoogleTest"),
            TestType::Catch2 => write!(f, "Catch2"),
            TestType::CTest => write!(f, "CTest"),
            TestType::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Test discovery engine
pub struct TestDiscovery {
    /// Build directory containing test executables
    build_dir: PathBuf,
    /// Whether to use verbose output
    verbose: bool,
}

impl TestDiscovery {
    /// Create a new test discovery instance
    pub fn new(build_dir: PathBuf, verbose: bool) -> Self {
        Self { build_dir, verbose }
    }

    /// Discover all tests from executables and CTest
    pub fn discover_all(&self) -> Result<Vec<DiscoveredTest>> {
        let mut tests = Vec::new();

        // Discover from test executables
        let executables = self.find_test_executables()?;
        for exe in executables {
            if let Ok(exe_tests) = self.discover_from_executable(&exe) {
                tests.extend(exe_tests);
            }
        }

        // Discover from CTest if available
        if let Ok(ctest_tests) = self.discover_from_ctest() {
            // Merge CTest tests, avoiding duplicates
            for ctest_test in ctest_tests {
                if !tests.iter().any(|t| t.name == ctest_test.name) {
                    tests.push(ctest_test);
                }
            }
        }

        Ok(tests)
    }

    /// Find test executables in build directory
    pub fn find_test_executables(&self) -> Result<Vec<PathBuf>> {
        let install_dir = self.build_dir.join("out");
        let mut executables = Vec::new();

        if !install_dir.exists() {
            // Try alternate locations
            let alt_dirs = [
                self.build_dir.clone(),
                self.build_dir.join("bin"),
                self.build_dir.join("Debug"),
                self.build_dir.join("Release"),
            ];

            for alt_dir in alt_dirs {
                if alt_dir.exists() {
                    self.scan_for_executables(&alt_dir, &mut executables)?;
                }
            }
        } else {
            self.scan_for_executables(&install_dir, &mut executables)?;
        }

        Ok(executables)
    }

    /// Scan directory for test executables
    fn scan_for_executables(&self, dir: &Path, executables: &mut Vec<PathBuf>) -> Result<()> {
        if !dir.is_dir() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let file_name = path.file_name().unwrap().to_string_lossy();
                // Look for test-related names
                if file_name.contains("test")
                    || file_name.contains("Test")
                    || file_name.contains("_googletest")
                    || file_name.contains("_catch2")
                {
                    if self.is_executable(&path) {
                        executables.push(path);
                    }
                }
            }
        }

        Ok(())
    }

    /// Check if a file is executable
    fn is_executable(&self, path: &Path) -> bool {
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            if let Ok(metadata) = std::fs::metadata(path) {
                return metadata.permissions().mode() & 0o111 != 0;
            }
            false
        }

        #[cfg(windows)]
        {
            path.extension()
                .map(|ext| ext == "exe")
                .unwrap_or(false)
        }
    }

    /// Discover tests from a GoogleTest executable
    pub fn discover_from_executable(&self, executable: &Path) -> Result<Vec<DiscoveredTest>> {
        let mut tests = Vec::new();

        // Try GoogleTest first
        if let Ok(gtest_tests) = self.discover_googletest(executable) {
            tests.extend(gtest_tests);
        }
        // Try Catch2
        else if let Ok(catch2_tests) = self.discover_catch2(executable) {
            tests.extend(catch2_tests);
        }

        Ok(tests)
    }

    /// Discover GoogleTest tests using --gtest_list_tests
    fn discover_googletest(&self, executable: &Path) -> Result<Vec<DiscoveredTest>> {
        let output = Command::new(executable)
            .arg("--gtest_list_tests")
            .output()
            .context("Failed to run --gtest_list_tests")?;

        if !output.status.success() {
            anyhow::bail!("Not a GoogleTest executable");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tests = Vec::new();
        let mut current_suite = String::new();

        for line in stdout.lines() {
            if line.is_empty() {
                continue;
            }

            // Suite names end with a period and are not indented
            if !line.starts_with(' ') && line.ends_with('.') {
                current_suite = line.trim_end_matches('.').to_string();
            }
            // Test case names are indented
            else if line.starts_with("  ") && !current_suite.is_empty() {
                let case_name = line.trim();
                // Skip parameterized test info (lines starting with #)
                if case_name.starts_with('#') {
                    continue;
                }
                // Handle parameterized tests (case_name may contain additional info)
                let case_name = case_name.split_whitespace().next().unwrap_or(case_name);

                tests.push(DiscoveredTest {
                    name: format!("{}.{}", current_suite, case_name),
                    suite: current_suite.clone(),
                    case: case_name.to_string(),
                    file: None,
                    line: None,
                    executable: executable.to_path_buf(),
                    test_type: TestType::GoogleTest,
                });
            }
        }

        if tests.is_empty() {
            anyhow::bail!("No GoogleTest tests found");
        }

        Ok(tests)
    }

    /// Discover Catch2 tests using --list-tests
    fn discover_catch2(&self, executable: &Path) -> Result<Vec<DiscoveredTest>> {
        let output = Command::new(executable)
            .arg("--list-tests")
            .output()
            .context("Failed to run --list-tests")?;

        if !output.status.success() {
            anyhow::bail!("Not a Catch2 executable");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tests = Vec::new();

        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with("All available") {
                continue;
            }

            // Catch2 test names are listed one per line
            let (suite, case) = if line.contains("::") {
                let parts: Vec<&str> = line.splitn(2, "::").collect();
                (parts[0].to_string(), parts.get(1).unwrap_or(&"").to_string())
            } else {
                ("Default".to_string(), line.to_string())
            };

            tests.push(DiscoveredTest {
                name: line.to_string(),
                suite,
                case,
                file: None,
                line: None,
                executable: executable.to_path_buf(),
                test_type: TestType::Catch2,
            });
        }

        Ok(tests)
    }

    /// Discover tests from CTest
    fn discover_from_ctest(&self) -> Result<Vec<DiscoveredTest>> {
        let output = Command::new("ctest")
            .arg("-N")
            .current_dir(&self.build_dir)
            .output()
            .context("Failed to run ctest -N")?;

        if !output.status.success() {
            anyhow::bail!("CTest not available");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let mut tests = Vec::new();

        for line in stdout.lines() {
            // CTest output format: "  Test #N: TestName"
            if line.contains("Test #") {
                if let Some(colon_pos) = line.find(':') {
                    let test_name = line[colon_pos + 1..].trim().to_string();
                    tests.push(DiscoveredTest {
                        name: test_name.clone(),
                        suite: "CTest".to_string(),
                        case: test_name,
                        file: None,
                        line: None,
                        executable: PathBuf::new(),
                        test_type: TestType::CTest,
                    });
                }
            }
        }

        Ok(tests)
    }

    /// List tests grouped by suite
    pub fn list_by_suite(&self) -> Result<HashMap<String, Vec<DiscoveredTest>>> {
        let tests = self.discover_all()?;
        let mut by_suite: HashMap<String, Vec<DiscoveredTest>> = HashMap::new();

        for test in tests {
            by_suite
                .entry(test.suite.clone())
                .or_default()
                .push(test);
        }

        Ok(by_suite)
    }

    /// Filter tests by pattern (supports glob-like matching)
    pub fn filter(&self, pattern: &str) -> Result<Vec<DiscoveredTest>> {
        let tests = self.discover_all()?;

        // Convert glob pattern to simple matching
        let pattern = pattern.replace('*', "");

        Ok(tests
            .into_iter()
            .filter(|t| {
                t.name.contains(&pattern)
                    || t.suite.contains(&pattern)
                    || t.case.contains(&pattern)
            })
            .collect())
    }

    /// Print discovered tests in a formatted table
    pub fn print_tests(&self, tests: &[DiscoveredTest]) {
        if tests.is_empty() {
            println!("No tests discovered.");
            return;
        }

        println!("\n{} tests discovered:", tests.len());
        println!("{}", "─".repeat(80));
        println!(
            "{:<40} {:<15} {:<20}",
            "Test Name", "Type", "Executable"
        );
        println!("{}", "─".repeat(80));

        for test in tests {
            let exe_name = test
                .executable
                .file_name()
                .map(|n| n.to_string_lossy())
                .unwrap_or_default();
            println!(
                "{:<40} {:<15} {:<20}",
                if test.name.len() > 38 {
                    format!("{}...", &test.name[..35])
                } else {
                    test.name.clone()
                },
                test.test_type,
                if exe_name.len() > 18 {
                    format!("{}...", &exe_name[..15])
                } else {
                    exe_name.to_string()
                }
            );
        }

        println!("{}", "─".repeat(80));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_test_type_display() {
        assert_eq!(format!("{}", TestType::GoogleTest), "GoogleTest");
        assert_eq!(format!("{}", TestType::Catch2), "Catch2");
        assert_eq!(format!("{}", TestType::CTest), "CTest");
    }

    #[test]
    fn test_discovery_new() {
        let discovery = TestDiscovery::new(PathBuf::from("/tmp/build"), false);
        assert_eq!(discovery.build_dir, PathBuf::from("/tmp/build"));
    }
}
