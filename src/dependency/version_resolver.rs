//! Version conflict resolution using semantic versioning
//!
//! This module provides intelligent version conflict resolution when multiple
//! dependencies require different versions of the same package.

use std::collections::HashMap;
use std::fmt;

use anyhow::{Context, Result};
use semver::{Version, VersionReq};

/// Version conflict resolution strategy
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ConflictStrategy {
    /// Use the first version encountered (default)
    #[default]
    First,
    /// Use the highest compatible version
    Highest,
    /// Use the lowest compatible version
    Lowest,
    /// Fail on any version conflict
    Strict,
}

/// Version requirement types
#[derive(Debug, Clone)]
pub enum VersionRequirement {
    /// Exact version (e.g., "1.2.3")
    Exact(Version),
    /// Version range (e.g., "^1.2", "~1.2.3", ">=1.0, <2.0")
    Range(VersionReq),
    /// Any version (e.g., "*" or empty)
    Any,
}

impl VersionRequirement {
    /// Parse version requirement from string
    pub fn parse(version_str: &str) -> Result<Self> {
        let version_str = version_str.trim();

        if version_str.is_empty() || version_str == "*" {
            return Ok(Self::Any);
        }

        // Try parsing as exact version first
        if let Ok(version) = Version::parse(version_str) {
            return Ok(Self::Exact(version));
        }

        // Try parsing as version requirement (range)
        let req = VersionReq::parse(version_str)
            .with_context(|| format!("Failed to parse version requirement '{}'", version_str))?;

        Ok(Self::Range(req))
    }

    /// Check if this requirement is satisfied by the given version
    pub fn matches(&self, version: &Version) -> bool {
        match self {
            Self::Exact(v) => v == version,
            Self::Range(req) => req.matches(version),
            Self::Any => true,
        }
    }

    /// Check if this requirement is compatible with another requirement
    pub fn is_compatible_with(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Any, _) | (_, Self::Any) => true,
            (Self::Exact(v1), Self::Exact(v2)) => v1 == v2,
            (Self::Exact(v), Self::Range(req)) | (Self::Range(req), Self::Exact(v)) => {
                req.matches(v)
            }
            (Self::Range(req1), Self::Range(req2)) => {
                // Check if ranges overlap by testing against sample versions
                // This is a heuristic - proper range intersection is complex
                self.try_find_compatible_version(other).is_some()
            }
        }
    }

    /// Try to find a version that satisfies both requirements
    fn try_find_compatible_version(&self, other: &Self) -> Option<Version> {
        // Generate candidate versions to test
        let candidates = match (self, other) {
            (Self::Exact(v), _) if other.matches(v) => return Some(v.clone()),
            (_, Self::Exact(v)) if self.matches(v) => return Some(v.clone()),
            (Self::Range(req1), Self::Range(req2)) => {
                // Try common versions
                let test_versions = vec![
                    "1.0.0", "1.1.0", "1.2.0", "2.0.0", "2.1.0", "3.0.0",
                ];
                test_versions
                    .into_iter()
                    .filter_map(|v| Version::parse(v).ok())
                    .find(|v| req1.matches(v) && req2.matches(v))
            }
            _ => None,
        };

        candidates
    }
}

impl fmt::Display for VersionRequirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exact(v) => write!(f, "{}", v),
            Self::Range(req) => write!(f, "{}", req),
            Self::Any => write!(f, "*"),
        }
    }
}

/// Version conflict information
#[derive(Debug, Clone)]
pub struct VersionConflict {
    /// Package name with conflict
    pub package: String,
    /// List of (dependent, required version) pairs
    pub requirements: Vec<(String, VersionRequirement)>,
}

impl VersionConflict {
    /// Create a new version conflict
    pub fn new(package: String) -> Self {
        Self {
            package,
            requirements: Vec::new(),
        }
    }

    /// Add a requirement from a dependent
    pub fn add_requirement(&mut self, dependent: String, requirement: VersionRequirement) {
        self.requirements.push((dependent, requirement));
    }

    /// Check if this is an actual conflict (incompatible requirements)
    pub fn is_conflicting(&self) -> bool {
        if self.requirements.len() < 2 {
            return false;
        }

        // Check if any pair of requirements is incompatible
        for i in 0..self.requirements.len() {
            for j in (i + 1)..self.requirements.len() {
                let (_, req1) = &self.requirements[i];
                let (_, req2) = &self.requirements[j];

                if !req1.is_compatible_with(req2) {
                    return true;
                }
            }
        }

        false
    }

    /// Try to resolve the conflict by finding a compatible version
    pub fn resolve(&self) -> Result<Version> {
        self.resolve_with_strategy(ConflictStrategy::default())
    }

    /// Resolve the conflict using a specific strategy
    pub fn resolve_with_strategy(&self, strategy: ConflictStrategy) -> Result<Version> {
        if self.requirements.is_empty() {
            anyhow::bail!("No requirements to resolve for '{}'", self.package);
        }

        if self.requirements.len() == 1 {
            // Single requirement - use it
            let (_, req) = &self.requirements[0];
            return self.extract_version_from_requirement(req);
        }

        // Check for strict mode first
        if strategy == ConflictStrategy::Strict && self.is_conflicting() {
            anyhow::bail!(
                "Version conflict for '{}' (strict mode enabled):\n{}",
                self.package,
                self.format_requirements()
            );
        }

        // Collect all concrete versions from requirements
        let mut versions: Vec<Version> = Vec::new();
        for (_, req) in &self.requirements {
            if let Ok(v) = self.extract_version_from_requirement(req) {
                versions.push(v);
            }
        }

        if versions.is_empty() {
            anyhow::bail!(
                "Cannot resolve version conflict for '{}': no concrete versions found\n{}",
                self.package,
                self.format_requirements()
            );
        }

        // Apply strategy
        match strategy {
            ConflictStrategy::First => {
                // Use the first version encountered
                Ok(versions[0].clone())
            }
            ConflictStrategy::Highest => {
                // Sort and use highest version
                versions.sort();
                Ok(versions.last().unwrap().clone())
            }
            ConflictStrategy::Lowest => {
                // Sort and use lowest version
                versions.sort();
                Ok(versions.first().unwrap().clone())
            }
            ConflictStrategy::Strict => {
                // Already checked for conflicts above, use first version
                Ok(versions[0].clone())
            }
        }
    }

    /// Extract a concrete version from a requirement
    fn extract_version_from_requirement(&self, req: &VersionRequirement) -> Result<Version> {
        match req {
            VersionRequirement::Exact(v) => Ok(v.clone()),
            VersionRequirement::Range(range) => {
                // Try to extract version from range
                // For now, use a simple heuristic: if range is like "^1.2.3", use 1.2.3
                let range_str = range.to_string();

                // Strip common prefixes
                let version_str = range_str
                    .trim_start_matches('^')
                    .trim_start_matches('~')
                    .trim_start_matches(">=")
                    .trim_start_matches('>')
                    .split(',')
                    .next()
                    .unwrap_or(&range_str)
                    .trim();

                Version::parse(version_str).with_context(|| {
                    format!("Cannot extract version from range '{}'", range)
                })
            }
            VersionRequirement::Any => {
                // Default to 1.0.0 for "any" requirement
                Ok(Version::new(1, 0, 0))
            }
        }
    }

    /// Format requirements for display
    fn format_requirements(&self) -> String {
        let mut output = String::new();
        for (dependent, req) in &self.requirements {
            output.push_str(&format!("  - {} requires {}\n", dependent, req));
        }
        output
    }
}

/// Version conflict resolver
pub struct VersionResolver {
    /// Map of package name to conflict info
    conflicts: HashMap<String, VersionConflict>,
    /// Conflict resolution strategy
    strategy: ConflictStrategy,
}

impl VersionResolver {
    /// Create a new version resolver
    pub fn new() -> Self {
        Self {
            conflicts: HashMap::new(),
            strategy: ConflictStrategy::default(),
        }
    }

    /// Create a new version resolver with a specific strategy
    pub fn with_strategy(strategy: ConflictStrategy) -> Self {
        Self {
            conflicts: HashMap::new(),
            strategy,
        }
    }

    /// Set the conflict resolution strategy
    pub fn set_strategy(&mut self, strategy: ConflictStrategy) {
        self.strategy = strategy;
    }

    /// Get the current conflict resolution strategy
    pub fn strategy(&self) -> ConflictStrategy {
        self.strategy
    }

    /// Record a dependency requirement
    pub fn add_requirement(
        &mut self,
        package: String,
        dependent: String,
        version: String,
    ) -> Result<()> {
        let requirement = VersionRequirement::parse(&version)?;

        self.conflicts
            .entry(package.clone())
            .or_insert_with(|| VersionConflict::new(package))
            .add_requirement(dependent, requirement);

        Ok(())
    }

    /// Check if there are any version conflicts
    pub fn has_conflicts(&self) -> bool {
        self.conflicts.values().any(|c| c.is_conflicting())
    }

    /// Get all detected conflicts
    pub fn get_conflicts(&self) -> Vec<&VersionConflict> {
        self.conflicts
            .values()
            .filter(|c| c.is_conflicting())
            .collect()
    }

    /// Resolve all conflicts and return chosen versions
    pub fn resolve_all(&self) -> Result<HashMap<String, Version>> {
        let mut resolved = HashMap::new();

        for (package, conflict) in &self.conflicts {
            let version = conflict.resolve_with_strategy(self.strategy).with_context(|| {
                format!("Failed to resolve version conflict for '{}'", package)
            })?;

            resolved.insert(package.clone(), version);
        }

        Ok(resolved)
    }

    /// Get resolved version for a specific package
    pub fn resolve_package(&self, package: &str) -> Result<Version> {
        if let Some(conflict) = self.conflicts.get(package) {
            conflict.resolve_with_strategy(self.strategy)
        } else {
            anyhow::bail!("No requirements recorded for package '{}'", package);
        }
    }

    /// Check if in strict mode
    pub fn is_strict(&self) -> bool {
        self.strategy == ConflictStrategy::Strict
    }

    /// Get the strategy name for display
    pub fn strategy_name(&self) -> &'static str {
        match self.strategy {
            ConflictStrategy::First => "first",
            ConflictStrategy::Highest => "highest",
            ConflictStrategy::Lowest => "lowest",
            ConflictStrategy::Strict => "strict",
        }
    }

    /// Print conflict summary
    pub fn print_conflicts(&self) {
        let conflicts = self.get_conflicts();
        if conflicts.is_empty() {
            return;
        }

        println!("\n⚠️  Detected {} version conflicts:", conflicts.len());
        for conflict in conflicts {
            println!("\n   Package: {}", conflict.package);
            for (dependent, req) in &conflict.requirements {
                println!("      {} requires {}", dependent, req);
            }
        }
    }
}

impl Default for VersionResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_exact_version() {
        let req = VersionRequirement::parse("1.2.3").unwrap();
        match req {
            VersionRequirement::Exact(v) => {
                assert_eq!(v, Version::new(1, 2, 3));
            }
            _ => panic!("Expected Exact version"),
        }
    }

    #[test]
    fn test_parse_caret_range() {
        let req = VersionRequirement::parse("^1.2.3").unwrap();
        match req {
            VersionRequirement::Range(_) => {}
            _ => panic!("Expected Range"),
        }
    }

    #[test]
    fn test_parse_any() {
        let req1 = VersionRequirement::parse("*").unwrap();
        let req2 = VersionRequirement::parse("").unwrap();

        assert!(matches!(req1, VersionRequirement::Any));
        assert!(matches!(req2, VersionRequirement::Any));
    }

    #[test]
    fn test_requirement_matches() {
        let req = VersionRequirement::parse("^1.2.0").unwrap();
        let v1 = Version::new(1, 2, 5);
        let v2 = Version::new(2, 0, 0);

        assert!(req.matches(&v1));
        assert!(!req.matches(&v2));
    }

    #[test]
    fn test_exact_version_compatibility() {
        let req1 = VersionRequirement::parse("1.2.3").unwrap();
        let req2 = VersionRequirement::parse("1.2.3").unwrap();
        let req3 = VersionRequirement::parse("1.2.4").unwrap();

        assert!(req1.is_compatible_with(&req2));
        assert!(!req1.is_compatible_with(&req3));
    }

    #[test]
    fn test_resolve_single_requirement() {
        let mut conflict = VersionConflict::new("package".to_string());
        conflict.add_requirement(
            "dep1".to_string(),
            VersionRequirement::parse("1.2.3").unwrap(),
        );

        let version = conflict.resolve().unwrap();
        assert_eq!(version, Version::new(1, 2, 3));
    }

    #[test]
    fn test_resolve_compatible_requirements() {
        let mut conflict = VersionConflict::new("package".to_string());
        conflict.add_requirement(
            "dep1".to_string(),
            VersionRequirement::parse("^1.2.0").unwrap(),
        );
        conflict.add_requirement(
            "dep2".to_string(),
            VersionRequirement::parse("1.2.5").unwrap(),
        );

        let version = conflict.resolve().unwrap();
        assert_eq!(version.major, 1);
        assert_eq!(version.minor, 2);
    }

    #[test]
    fn test_detect_conflict() {
        let mut conflict = VersionConflict::new("package".to_string());
        conflict.add_requirement(
            "dep1".to_string(),
            VersionRequirement::parse("1.0.0").unwrap(),
        );
        conflict.add_requirement(
            "dep2".to_string(),
            VersionRequirement::parse("2.0.0").unwrap(),
        );

        assert!(conflict.is_conflicting());
    }

    #[test]
    fn test_version_resolver() {
        let mut resolver = VersionResolver::new();

        resolver
            .add_requirement("fmt".to_string(), "project".to_string(), "^10.0.0".to_string())
            .unwrap();
        resolver
            .add_requirement("fmt".to_string(), "dep1".to_string(), "10.1.0".to_string())
            .unwrap();

        assert!(!resolver.has_conflicts());

        let version = resolver.resolve_package("fmt").unwrap();
        assert_eq!(version.major, 10);
    }

    #[test]
    fn test_version_resolver_conflict() {
        let mut resolver = VersionResolver::new();

        resolver
            .add_requirement("fmt".to_string(), "dep1".to_string(), "1.0.0".to_string())
            .unwrap();
        resolver
            .add_requirement("fmt".to_string(), "dep2".to_string(), "2.0.0".to_string())
            .unwrap();

        assert!(resolver.has_conflicts());

        // Should succeed with "first" strategy (uses first version)
        let result = resolver.resolve_package("fmt");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Version::new(1, 0, 0));
    }

    #[test]
    fn test_conflict_strategy_first() {
        let mut resolver = VersionResolver::with_strategy(ConflictStrategy::First);

        resolver
            .add_requirement("fmt".to_string(), "dep1".to_string(), "1.0.0".to_string())
            .unwrap();
        resolver
            .add_requirement("fmt".to_string(), "dep2".to_string(), "2.0.0".to_string())
            .unwrap();

        let version = resolver.resolve_package("fmt").unwrap();
        assert_eq!(version, Version::new(1, 0, 0)); // First version
    }

    #[test]
    fn test_conflict_strategy_highest() {
        let mut resolver = VersionResolver::with_strategy(ConflictStrategy::Highest);

        resolver
            .add_requirement("fmt".to_string(), "dep1".to_string(), "1.0.0".to_string())
            .unwrap();
        resolver
            .add_requirement("fmt".to_string(), "dep2".to_string(), "2.0.0".to_string())
            .unwrap();

        let version = resolver.resolve_package("fmt").unwrap();
        assert_eq!(version, Version::new(2, 0, 0)); // Highest version
    }

    #[test]
    fn test_conflict_strategy_lowest() {
        let mut resolver = VersionResolver::with_strategy(ConflictStrategy::Lowest);

        resolver
            .add_requirement("fmt".to_string(), "dep1".to_string(), "3.0.0".to_string())
            .unwrap();
        resolver
            .add_requirement("fmt".to_string(), "dep2".to_string(), "2.0.0".to_string())
            .unwrap();
        resolver
            .add_requirement("fmt".to_string(), "dep3".to_string(), "1.0.0".to_string())
            .unwrap();

        let version = resolver.resolve_package("fmt").unwrap();
        assert_eq!(version, Version::new(1, 0, 0)); // Lowest version
    }

    #[test]
    fn test_conflict_strategy_strict() {
        let mut resolver = VersionResolver::with_strategy(ConflictStrategy::Strict);

        resolver
            .add_requirement("fmt".to_string(), "dep1".to_string(), "1.0.0".to_string())
            .unwrap();
        resolver
            .add_requirement("fmt".to_string(), "dep2".to_string(), "2.0.0".to_string())
            .unwrap();

        // Should fail in strict mode
        let result = resolver.resolve_package("fmt");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("strict mode"));
    }

    #[test]
    fn test_conflict_strategy_strict_no_conflict() {
        let mut resolver = VersionResolver::with_strategy(ConflictStrategy::Strict);

        resolver
            .add_requirement("fmt".to_string(), "dep1".to_string(), "^1.0.0".to_string())
            .unwrap();
        resolver
            .add_requirement("fmt".to_string(), "dep2".to_string(), "1.2.0".to_string())
            .unwrap();

        // Should succeed when versions are compatible
        let result = resolver.resolve_package("fmt");
        assert!(result.is_ok());
    }

    #[test]
    fn test_set_strategy() {
        let mut resolver = VersionResolver::new();
        assert_eq!(resolver.strategy(), ConflictStrategy::First);

        resolver.set_strategy(ConflictStrategy::Highest);
        assert_eq!(resolver.strategy(), ConflictStrategy::Highest);
        assert_eq!(resolver.strategy_name(), "highest");
    }
}
