//! CI service integration module
//!
//! Provides integration with various CI services including GitHub Actions,
//! GitLab CI, Jenkins, and Azure DevOps.

use std::path::Path;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use super::coverage::CoverageReport;
use super::results::TestSummary;
use super::benchmark::ComparisonReport;

/// CI output format
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CiFormat {
    /// GitHub Actions format (annotations, workflow commands)
    #[default]
    GitHubActions,
    /// GitLab CI format
    GitLabCI,
    /// Azure DevOps format
    AzureDevOps,
    /// Jenkins format
    Jenkins,
    /// TeamCity format
    TeamCity,
    /// Generic format (plain text)
    Generic,
}

impl std::str::FromStr for CiFormat {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "github" | "github-actions" | "gha" => Ok(CiFormat::GitHubActions),
            "gitlab" | "gitlab-ci" => Ok(CiFormat::GitLabCI),
            "azure" | "azure-devops" | "ado" => Ok(CiFormat::AzureDevOps),
            "jenkins" => Ok(CiFormat::Jenkins),
            "teamcity" => Ok(CiFormat::TeamCity),
            "generic" | "plain" => Ok(CiFormat::Generic),
            _ => anyhow::bail!(
                "Unknown CI format: {}. Valid formats: github, gitlab, azure, jenkins, teamcity, generic",
                s
            ),
        }
    }
}

impl CiFormat {
    /// Auto-detect CI environment
    pub fn detect() -> Self {
        if std::env::var("GITHUB_ACTIONS").is_ok() {
            CiFormat::GitHubActions
        } else if std::env::var("GITLAB_CI").is_ok() {
            CiFormat::GitLabCI
        } else if std::env::var("TF_BUILD").is_ok() {
            CiFormat::AzureDevOps
        } else if std::env::var("JENKINS_URL").is_ok() {
            CiFormat::Jenkins
        } else if std::env::var("TEAMCITY_VERSION").is_ok() {
            CiFormat::TeamCity
        } else {
            CiFormat::Generic
        }
    }
}

/// CI reporter for outputting results in CI-friendly formats
pub struct CiReporter {
    format: CiFormat,
}

impl CiReporter {
    /// Create a new CI reporter
    pub fn new(format: CiFormat) -> Self {
        Self { format }
    }

    /// Create a reporter with auto-detected format
    pub fn auto_detect() -> Self {
        Self::new(CiFormat::detect())
    }

    /// Report test results
    pub fn report_tests(&self, summary: &TestSummary) {
        match self.format {
            CiFormat::GitHubActions => self.report_tests_github(summary),
            CiFormat::GitLabCI => self.report_tests_gitlab(summary),
            CiFormat::AzureDevOps => self.report_tests_azure(summary),
            CiFormat::Jenkins => self.report_tests_jenkins(summary),
            CiFormat::TeamCity => self.report_tests_teamcity(summary),
            CiFormat::Generic => self.report_tests_generic(summary),
        }
    }

    /// Report test results for GitHub Actions
    fn report_tests_github(&self, summary: &TestSummary) {
        // Set output variables
        println!("::set-output name=tests_total::{}", summary.total);
        println!("::set-output name=tests_passed::{}", summary.passed);
        println!("::set-output name=tests_failed::{}", summary.failed);
        println!("::set-output name=tests_skipped::{}", summary.skipped);
        println!(
            "::set-output name=tests_pass_rate::{:.1}",
            summary.pass_rate * 100.0
        );

        // Create job summary
        println!("::group::Test Results");
        println!("## Test Results");
        println!();
        println!("| Metric | Value |");
        println!("|--------|-------|");
        println!("| Total | {} |", summary.total);
        println!("| ✅ Passed | {} |", summary.passed);
        println!("| ❌ Failed | {} |", summary.failed);
        println!("| ⏭️ Skipped | {} |", summary.skipped);
        println!("| Pass Rate | {:.1}% |", summary.pass_rate * 100.0);
        println!("::endgroup::");

        // Report failures as annotations
        for test in summary.failed_tests() {
            let message = test.message.as_deref().unwrap_or("Test failed");
            // Try to extract file and line from stack trace
            if let Some(ref trace) = test.stack_trace {
                if let Some((file, line)) = parse_file_line(trace) {
                    println!(
                        "::error file={},line={}::Test failed: {}",
                        file, line, test.full_name()
                    );
                    continue;
                }
            }
            println!(
                "::error::Test failed: {} - {}",
                test.full_name(),
                message.lines().next().unwrap_or("")
            );
        }

        // Set overall status
        if !summary.all_passed() {
            println!("::error::{} test(s) failed", summary.failed + summary.errors);
        }
    }

    /// Report test results for GitLab CI
    fn report_tests_gitlab(&self, summary: &TestSummary) {
        // GitLab uses JUnit XML for test reporting, so just print summary
        println!("## Test Results");
        println!();
        println!("- Total: {}", summary.total);
        println!("- Passed: {}", summary.passed);
        println!("- Failed: {}", summary.failed);
        println!("- Skipped: {}", summary.skipped);
        println!("- Pass Rate: {:.1}%", summary.pass_rate * 100.0);

        // Report failures
        if summary.failed > 0 {
            println!();
            println!("### Failed Tests");
            for test in summary.failed_tests() {
                println!("- `{}`", test.full_name());
                if let Some(ref msg) = test.message {
                    println!("  - {}", msg.lines().next().unwrap_or(""));
                }
            }
        }
    }

    /// Report test results for Azure DevOps
    fn report_tests_azure(&self, summary: &TestSummary) {
        // Azure DevOps logging commands
        println!(
            "##vso[task.setvariable variable=tests_total]{}",
            summary.total
        );
        println!(
            "##vso[task.setvariable variable=tests_passed]{}",
            summary.passed
        );
        println!(
            "##vso[task.setvariable variable=tests_failed]{}",
            summary.failed
        );

        // Log summary
        println!("##[section]Test Results");
        println!("Total: {}", summary.total);
        println!("Passed: {}", summary.passed);
        println!("Failed: {}", summary.failed);
        println!("Skipped: {}", summary.skipped);

        // Report failures as errors
        for test in summary.failed_tests() {
            let message = test.message.as_deref().unwrap_or("Test failed");
            println!(
                "##vso[task.logissue type=error]Test failed: {} - {}",
                test.full_name(),
                message.lines().next().unwrap_or("")
            );
        }

        if !summary.all_passed() {
            println!(
                "##vso[task.complete result=Failed;]{} test(s) failed",
                summary.failed
            );
        }
    }

    /// Report test results for Jenkins
    fn report_tests_jenkins(&self, summary: &TestSummary) {
        // Jenkins uses JUnit XML, just print summary
        println!("Test Results Summary");
        println!("====================");
        println!("Total:   {}", summary.total);
        println!("Passed:  {}", summary.passed);
        println!("Failed:  {}", summary.failed);
        println!("Skipped: {}", summary.skipped);
        println!("Pass Rate: {:.1}%", summary.pass_rate * 100.0);

        if summary.failed > 0 {
            println!();
            println!("FAILED TESTS:");
            for test in summary.failed_tests() {
                println!("  - {}", test.full_name());
            }
        }
    }

    /// Report test results for TeamCity
    fn report_tests_teamcity(&self, summary: &TestSummary) {
        // TeamCity service messages
        println!(
            "##teamcity[buildStatisticValue key='tests.total' value='{}']",
            summary.total
        );
        println!(
            "##teamcity[buildStatisticValue key='tests.passed' value='{}']",
            summary.passed
        );
        println!(
            "##teamcity[buildStatisticValue key='tests.failed' value='{}']",
            summary.failed
        );
        println!(
            "##teamcity[buildStatisticValue key='tests.skipped' value='{}']",
            summary.skipped
        );

        // Report each test
        for suite in &summary.suites {
            println!(
                "##teamcity[testSuiteStarted name='{}']",
                escape_teamcity(&suite.name)
            );

            for test in &suite.tests {
                println!(
                    "##teamcity[testStarted name='{}']",
                    escape_teamcity(&test.name)
                );

                match test.status {
                    super::results::TestStatus::Failed => {
                        println!(
                            "##teamcity[testFailed name='{}' message='{}']",
                            escape_teamcity(&test.name),
                            escape_teamcity(test.message.as_deref().unwrap_or(""))
                        );
                    }
                    super::results::TestStatus::Skipped => {
                        println!(
                            "##teamcity[testIgnored name='{}']",
                            escape_teamcity(&test.name)
                        );
                    }
                    _ => {}
                }

                println!(
                    "##teamcity[testFinished name='{}' duration='{}']",
                    escape_teamcity(&test.name),
                    (test.duration * 1000.0) as u64
                );
            }

            println!(
                "##teamcity[testSuiteFinished name='{}']",
                escape_teamcity(&suite.name)
            );
        }
    }

    /// Report test results in generic format
    fn report_tests_generic(&self, summary: &TestSummary) {
        summary.print_summary();
    }

    /// Report coverage results
    pub fn report_coverage(&self, report: &CoverageReport) {
        match self.format {
            CiFormat::GitHubActions => self.report_coverage_github(report),
            CiFormat::GitLabCI => self.report_coverage_gitlab(report),
            CiFormat::AzureDevOps => self.report_coverage_azure(report),
            _ => self.report_coverage_generic(report),
        }
    }

    fn report_coverage_github(&self, report: &CoverageReport) {
        println!(
            "::set-output name=coverage_line::{:.1}",
            report.line_coverage_percent()
        );
        println!(
            "::set-output name=coverage_branch::{:.1}",
            report.branch_coverage_percent()
        );
        println!(
            "::set-output name=coverage_function::{:.1}",
            report.function_coverage_percent()
        );

        println!("::group::Coverage Report");
        println!("## Code Coverage");
        println!();
        println!("| Metric | Coverage |");
        println!("|--------|----------|");
        println!("| Lines | {:.1}% |", report.line_coverage_percent());
        println!("| Branches | {:.1}% |", report.branch_coverage_percent());
        println!("| Functions | {:.1}% |", report.function_coverage_percent());
        println!("::endgroup::");
    }

    fn report_coverage_gitlab(&self, report: &CoverageReport) {
        // GitLab parses coverage from regex in .gitlab-ci.yml
        println!("Coverage: {:.1}%", report.line_coverage_percent());
    }

    fn report_coverage_azure(&self, report: &CoverageReport) {
        println!(
            "##vso[task.setvariable variable=coverage_line]{:.1}",
            report.line_coverage_percent()
        );
        println!(
            "##vso[task.setvariable variable=coverage_branch]{:.1}",
            report.branch_coverage_percent()
        );
        println!("##[section]Code Coverage");
        println!("Line Coverage: {:.1}%", report.line_coverage_percent());
        println!("Branch Coverage: {:.1}%", report.branch_coverage_percent());
        println!(
            "Function Coverage: {:.1}%",
            report.function_coverage_percent()
        );
    }

    fn report_coverage_generic(&self, report: &CoverageReport) {
        report.print_summary();
    }

    /// Report benchmark comparison results
    pub fn report_benchmarks(&self, report: &ComparisonReport) {
        match self.format {
            CiFormat::GitHubActions => self.report_benchmarks_github(report),
            CiFormat::GitLabCI => self.report_benchmarks_gitlab(report),
            CiFormat::AzureDevOps => self.report_benchmarks_azure(report),
            _ => self.report_benchmarks_generic(report),
        }
    }

    fn report_benchmarks_github(&self, report: &ComparisonReport) {
        println!(
            "::set-output name=benchmark_regressions::{}",
            report.regressions
        );
        println!(
            "::set-output name=benchmark_improvements::{}",
            report.improvements
        );

        println!("::group::Benchmark Comparison");
        println!("{}", report.to_markdown());
        println!("::endgroup::");

        // Report regressions as warnings
        for comp in report.comparisons.iter().filter(|c| c.is_regression) {
            println!(
                "::warning::Benchmark regression: {} ({:+.1}%)",
                comp.name, comp.time_diff_percent
            );
        }
    }

    fn report_benchmarks_gitlab(&self, report: &ComparisonReport) {
        println!("{}", report.to_markdown());
    }

    fn report_benchmarks_azure(&self, report: &ComparisonReport) {
        println!(
            "##vso[task.setvariable variable=benchmark_regressions]{}",
            report.regressions
        );
        println!("##[section]Benchmark Comparison");
        report.print_report();

        if report.has_regressions() {
            println!(
                "##vso[task.logissue type=warning]{} benchmark regression(s) detected",
                report.regressions
            );
        }
    }

    fn report_benchmarks_generic(&self, report: &ComparisonReport) {
        report.print_report();
    }

    /// Write JUnit XML report
    pub fn write_junit_xml(&self, summary: &TestSummary, path: &Path) -> Result<()> {
        let xml = summary.to_junit_xml();
        std::fs::write(path, xml)?;
        Ok(())
    }
}

/// Parse file and line number from stack trace
fn parse_file_line(trace: &str) -> Option<(String, u32)> {
    // Try common patterns:
    // file.cpp:123
    // file.cpp(123)
    // at file.cpp:123
    for line in trace.lines() {
        let line = line.trim();
        // Strip common prefixes like "at "
        let line = line.strip_prefix("at ").unwrap_or(line);

        // Pattern: file.cpp:123
        if let Some(colon_pos) = line.rfind(':') {
            let file = &line[..colon_pos];
            if let Ok(line_num) = line[colon_pos + 1..].trim().parse::<u32>() {
                if file.ends_with(".cpp")
                    || file.ends_with(".cc")
                    || file.ends_with(".c")
                    || file.ends_with(".h")
                    || file.ends_with(".hpp")
                {
                    return Some((file.to_string(), line_num));
                }
            }
        }

        // Pattern: file.cpp(123)
        if let Some(paren_pos) = line.rfind('(') {
            let file = &line[..paren_pos];
            if let Some(close_pos) = line.rfind(')') {
                if let Ok(line_num) = line[paren_pos + 1..close_pos].parse::<u32>() {
                    if file.ends_with(".cpp")
                        || file.ends_with(".cc")
                        || file.ends_with(".c")
                        || file.ends_with(".h")
                        || file.ends_with(".hpp")
                    {
                        return Some((file.to_string(), line_num));
                    }
                }
            }
        }
    }

    None
}

/// Escape special characters for TeamCity
fn escape_teamcity(s: &str) -> String {
    s.replace('|', "||")
        .replace('\'', "|'")
        .replace('\n', "|n")
        .replace('\r', "|r")
        .replace('[', "|[")
        .replace(']', "|]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ci_format_parse() {
        assert_eq!(
            "github".parse::<CiFormat>().unwrap(),
            CiFormat::GitHubActions
        );
        assert_eq!("gitlab".parse::<CiFormat>().unwrap(), CiFormat::GitLabCI);
        assert_eq!("azure".parse::<CiFormat>().unwrap(), CiFormat::AzureDevOps);
    }

    #[test]
    fn test_parse_file_line() {
        assert_eq!(
            parse_file_line("test.cpp:42"),
            Some(("test.cpp".to_string(), 42))
        );
        assert_eq!(
            parse_file_line("  at /path/to/test.cpp:123"),
            Some(("/path/to/test.cpp".to_string(), 123))
        );
        assert_eq!(parse_file_line("no file here"), None);
    }

    #[test]
    fn test_escape_teamcity() {
        assert_eq!(escape_teamcity("test|name"), "test||name");
        assert_eq!(escape_teamcity("test\nname"), "test|nname");
        assert_eq!(escape_teamcity("test[0]"), "test|[0|]");
    }
}
