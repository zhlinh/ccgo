//! Testing framework enhancements
//!
//! This module provides advanced testing features including:
//! - Test discovery and listing
//! - Code coverage reporting
//! - Test result aggregation
//! - Benchmark result comparison
//! - CI service integration

pub mod coverage;
pub mod discovery;
pub mod results;
pub mod benchmark;
pub mod ci;

pub use coverage::{CoverageConfig, CoverageFormat, CoverageReport};
pub use discovery::{DiscoveredTest, TestDiscovery};
pub use results::{TestResult, TestResultAggregator, TestSummary};
pub use benchmark::{BenchmarkComparison, BenchmarkResult, BenchmarkStore};
pub use ci::{CiFormat, CiReporter};
