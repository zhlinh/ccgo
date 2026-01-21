//! Dependency management module
//!
//! This module provides dependency resolution, graph management, version
//! conflict resolution, and transitive dependency handling.

pub mod graph;
pub mod resolver;
pub mod version_resolver;

pub use graph::{DependencyGraph, DependencyNode};
pub use resolver::DependencyResolver;
pub use version_resolver::{VersionConflict, VersionRequirement, VersionResolver};
