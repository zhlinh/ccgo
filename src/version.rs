//! Semantic version parsing and constraint matching
//!
//! Supports version requirements similar to Cargo:
//! - Exact: "1.2.3" or "=1.2.3"
//! - Caret: "^1.2.3" (compatible with 1.x.x, >= 1.2.3, < 2.0.0)
//! - Tilde: "~1.2.3" (compatible with 1.2.x, >= 1.2.3, < 1.3.0)
//! - Wildcard: "1.*" or "1.2.*"
//! - Greater/Less: ">=1.0.0", "<2.0.0", ">1.0,<2.0"
//! - Range: ">=1.2.0, <1.8.0"

use anyhow::{bail, Context, Result};
use std::cmp::Ordering;
use std::fmt;

/// A semantic version (major.minor.patch)
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Version {
    pub major: u64,
    pub minor: u64,
    pub patch: u64,
    pub pre: Vec<String>,
    pub build: Vec<String>,
}

impl Version {
    /// Parse a version string like "1.2.3"
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();

        // Split build metadata (e.g., "1.2.3+build.123")
        let (version_part, build) = if let Some(pos) = s.find('+') {
            let (v, b) = s.split_at(pos);
            (v, b[1..].split('.').map(|s| s.to_string()).collect())
        } else {
            (s, vec![])
        };

        // Split pre-release (e.g., "1.2.3-alpha.1")
        let (numeric_part, pre) = if let Some(pos) = version_part.find('-') {
            let (v, p) = version_part.split_at(pos);
            (v, p[1..].split('.').map(|s| s.to_string()).collect())
        } else {
            (version_part, vec![])
        };

        // Parse major.minor.patch
        let parts: Vec<&str> = numeric_part.split('.').collect();
        if parts.len() != 3 {
            bail!("Invalid version format: '{}'. Expected 'major.minor.patch'", s);
        }

        let major = parts[0].parse::<u64>()
            .with_context(|| format!("Invalid major version: '{}'", parts[0]))?;
        let minor = parts[1].parse::<u64>()
            .with_context(|| format!("Invalid minor version: '{}'", parts[1]))?;
        let patch = parts[2].parse::<u64>()
            .with_context(|| format!("Invalid patch version: '{}'", parts[2]))?;

        Ok(Version {
            major,
            minor,
            patch,
            pre,
            build,
        })
    }

    /// Check if this version matches a requirement
    pub fn matches(&self, req: &VersionReq) -> bool {
        req.matches(self)
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}.{}", self.major, self.minor, self.patch)?;
        if !self.pre.is_empty() {
            write!(f, "-{}", self.pre.join("."))?;
        }
        if !self.build.is_empty() {
            write!(f, "+{}", self.build.join("."))?;
        }
        Ok(())
    }
}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.major.cmp(&other.major) {
            Ordering::Equal => {}
            ord => return ord,
        }
        match self.minor.cmp(&other.minor) {
            Ordering::Equal => {}
            ord => return ord,
        }
        match self.patch.cmp(&other.patch) {
            Ordering::Equal => {}
            ord => return ord,
        }

        // Pre-release versions have lower precedence
        match (self.pre.is_empty(), other.pre.is_empty()) {
            (true, false) => return Ordering::Greater,
            (false, true) => return Ordering::Less,
            (true, true) => return Ordering::Equal,
            (false, false) => {}
        }

        // Compare pre-release identifiers
        self.pre.cmp(&other.pre)
    }
}

/// A version requirement/constraint
#[derive(Debug, Clone, PartialEq)]
pub struct VersionReq {
    comparators: Vec<Comparator>,
}

#[derive(Debug, Clone, PartialEq)]
struct Comparator {
    op: Op,
    major: u64,
    minor: Option<u64>,
    patch: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Op {
    Exact,      // =1.2.3 or 1.2.3
    Greater,    // >1.2.3
    GreaterEq,  // >=1.2.3
    Less,       // <1.2.3
    LessEq,     // <=1.2.3
    Tilde,      // ~1.2.3 (allow patch changes)
    Caret,      // ^1.2.3 (allow minor/patch changes)
    Wildcard,   // 1.* or 1.2.*
}

impl VersionReq {
    /// Parse a version requirement string
    pub fn parse(s: &str) -> Result<Self> {
        let s = s.trim();

        if s.is_empty() {
            bail!("Empty version requirement");
        }

        // Handle multiple comma-separated requirements
        if s.contains(',') {
            let comparators: Result<Vec<_>> = s
                .split(',')
                .map(|part| Self::parse_single(part.trim()))
                .collect();
            return Ok(VersionReq {
                comparators: comparators?,
            });
        }

        // Single requirement
        Ok(VersionReq {
            comparators: vec![Self::parse_single(s)?],
        })
    }

    /// Parse a single comparator (no commas)
    fn parse_single(s: &str) -> Result<Comparator> {
        let s = s.trim();

        // Check for operator prefix
        let (op, version_str) = if let Some(rest) = s.strip_prefix(">=") {
            (Op::GreaterEq, rest.trim())
        } else if let Some(rest) = s.strip_prefix("<=") {
            (Op::LessEq, rest.trim())
        } else if let Some(rest) = s.strip_prefix('>') {
            (Op::Greater, rest.trim())
        } else if let Some(rest) = s.strip_prefix('<') {
            (Op::Less, rest.trim())
        } else if let Some(rest) = s.strip_prefix('=') {
            (Op::Exact, rest.trim())
        } else if let Some(rest) = s.strip_prefix('~') {
            (Op::Tilde, rest.trim())
        } else if let Some(rest) = s.strip_prefix('^') {
            (Op::Caret, rest.trim())
        } else if s.contains('*') {
            (Op::Wildcard, s)
        } else {
            // Default to caret (Cargo-style)
            (Op::Caret, s)
        };

        Self::parse_version_parts(op, version_str)
    }

    /// Parse version parts for a comparator
    fn parse_version_parts(op: Op, s: &str) -> Result<Comparator> {
        let parts: Vec<&str> = s.split('.').collect();

        if parts.is_empty() || parts.len() > 3 {
            bail!("Invalid version format: '{}'", s);
        }

        // Parse major
        let major = if parts[0] == "*" {
            0
        } else {
            parts[0].parse::<u64>()
                .with_context(|| format!("Invalid major version: '{}'", parts[0]))?
        };

        // Parse minor (optional)
        let minor = if parts.len() > 1 {
            if parts[1] == "*" {
                None
            } else {
                Some(parts[1].parse::<u64>()
                    .with_context(|| format!("Invalid minor version: '{}'", parts[1]))?)
            }
        } else {
            None
        };

        // Parse patch (optional)
        let patch = if parts.len() > 2 {
            if parts[2] == "*" {
                None
            } else {
                Some(parts[2].parse::<u64>()
                    .with_context(|| format!("Invalid patch version: '{}'", parts[2]))?)
            }
        } else {
            None
        };

        Ok(Comparator {
            op,
            major,
            minor,
            patch,
        })
    }

    /// Check if a version matches this requirement
    pub fn matches(&self, version: &Version) -> bool {
        // All comparators must match (AND logic)
        self.comparators.iter().all(|c| c.matches(version))
    }
}

impl fmt::Display for VersionReq {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let parts: Vec<String> = self.comparators.iter().map(|c| c.to_string()).collect();
        write!(f, "{}", parts.join(", "))
    }
}

impl Comparator {
    /// Check if a version matches this comparator
    fn matches(&self, version: &Version) -> bool {
        match self.op {
            Op::Exact => self.matches_exact(version),
            Op::Greater => self.matches_greater(version),
            Op::GreaterEq => self.matches_greater_eq(version),
            Op::Less => self.matches_less(version),
            Op::LessEq => self.matches_less_eq(version),
            Op::Tilde => self.matches_tilde(version),
            Op::Caret => self.matches_caret(version),
            Op::Wildcard => self.matches_wildcard(version),
        }
    }

    fn matches_exact(&self, version: &Version) -> bool {
        version.major == self.major
            && self.minor.map_or(true, |m| version.minor == m)
            && self.patch.map_or(true, |p| version.patch == p)
    }

    fn matches_greater(&self, version: &Version) -> bool {
        let req_version = self.to_version();
        version > &req_version
    }

    fn matches_greater_eq(&self, version: &Version) -> bool {
        let req_version = self.to_version();
        version >= &req_version
    }

    fn matches_less(&self, version: &Version) -> bool {
        let req_version = self.to_version();
        version < &req_version
    }

    fn matches_less_eq(&self, version: &Version) -> bool {
        let req_version = self.to_version();
        version <= &req_version
    }

    /// Tilde: ~1.2.3 allows >=1.2.3, <1.3.0 (patch-level changes)
    fn matches_tilde(&self, version: &Version) -> bool {
        if version.major != self.major {
            return false;
        }

        match self.minor {
            None => true, // ~1 allows 1.x.x
            Some(minor) => {
                if version.minor != minor {
                    return false;
                }
                match self.patch {
                    None => true, // ~1.2 allows 1.2.x
                    Some(patch) => version.patch >= patch, // ~1.2.3 allows >=1.2.3, <1.3.0
                }
            }
        }
    }

    /// Caret: ^1.2.3 allows >=1.2.3, <2.0.0 (compatible changes)
    fn matches_caret(&self, version: &Version) -> bool {
        if version.major != self.major {
            return false;
        }

        // ^0.x.y is special: only patch updates allowed
        if self.major == 0 {
            if let Some(minor) = self.minor {
                if version.minor != minor {
                    return false;
                }
                if let Some(patch) = self.patch {
                    return version.patch >= patch;
                }
            }
            return true;
        }

        // ^1.2.3 allows >=1.2.3, <2.0.0
        match self.minor {
            None => true, // ^1 allows 1.x.x
            Some(minor) => {
                if version.minor > minor {
                    return true;
                }
                if version.minor < minor {
                    return false;
                }
                // version.minor == minor
                match self.patch {
                    None => true, // ^1.2 allows 1.2.x
                    Some(patch) => version.patch >= patch,
                }
            }
        }
    }

    /// Wildcard: 1.* or 1.2.*
    fn matches_wildcard(&self, version: &Version) -> bool {
        if version.major != self.major {
            return false;
        }

        if let Some(minor) = self.minor {
            if version.minor != minor {
                return false;
            }
        }

        // patch is always wildcard if we got here
        true
    }

    /// Convert to a concrete version for comparison
    fn to_version(&self) -> Version {
        Version {
            major: self.major,
            minor: self.minor.unwrap_or(0),
            patch: self.patch.unwrap_or(0),
            pre: vec![],
            build: vec![],
        }
    }
}

impl fmt::Display for Comparator {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let op_str = match self.op {
            Op::Exact => "=",
            Op::Greater => ">",
            Op::GreaterEq => ">=",
            Op::Less => "<",
            Op::LessEq => "<=",
            Op::Tilde => "~",
            Op::Caret => "^",
            Op::Wildcard => "",
        };

        write!(f, "{}{}", op_str, self.major)?;
        if let Some(minor) = self.minor {
            write!(f, ".{}", minor)?;
        } else if self.op == Op::Wildcard {
            write!(f, ".*")?;
            return Ok(());
        }
        if let Some(patch) = self.patch {
            write!(f, ".{}", patch)?;
        } else if self.op == Op::Wildcard {
            write!(f, ".*")?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_parse() {
        let v = Version::parse("1.2.3").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.minor, 2);
        assert_eq!(v.patch, 3);

        let v = Version::parse("1.0.0-alpha.1+build.123").unwrap();
        assert_eq!(v.major, 1);
        assert_eq!(v.pre, vec!["alpha", "1"]);
        assert_eq!(v.build, vec!["build", "123"]);
    }

    #[test]
    fn test_version_comparison() {
        let v1 = Version::parse("1.2.3").unwrap();
        let v2 = Version::parse("1.2.4").unwrap();
        let v3 = Version::parse("1.3.0").unwrap();

        assert!(v1 < v2);
        assert!(v2 < v3);
        assert!(v1 < v3);
    }

    #[test]
    fn test_exact_version() {
        let req = VersionReq::parse("=1.2.3").unwrap();
        assert!(req.matches(&Version::parse("1.2.3").unwrap()));
        assert!(!req.matches(&Version::parse("1.2.4").unwrap()));
    }

    #[test]
    fn test_caret_version() {
        let req = VersionReq::parse("^1.2.3").unwrap();
        assert!(req.matches(&Version::parse("1.2.3").unwrap()));
        assert!(req.matches(&Version::parse("1.2.4").unwrap()));
        assert!(req.matches(&Version::parse("1.3.0").unwrap()));
        assert!(!req.matches(&Version::parse("2.0.0").unwrap()));
        assert!(!req.matches(&Version::parse("1.2.2").unwrap()));
    }

    #[test]
    fn test_tilde_version() {
        let req = VersionReq::parse("~1.2.3").unwrap();
        assert!(req.matches(&Version::parse("1.2.3").unwrap()));
        assert!(req.matches(&Version::parse("1.2.4").unwrap()));
        assert!(!req.matches(&Version::parse("1.3.0").unwrap()));
        assert!(!req.matches(&Version::parse("2.0.0").unwrap()));
    }

    #[test]
    fn test_wildcard_version() {
        let req = VersionReq::parse("1.*").unwrap();
        assert!(req.matches(&Version::parse("1.0.0").unwrap()));
        assert!(req.matches(&Version::parse("1.9.9").unwrap()));
        assert!(!req.matches(&Version::parse("2.0.0").unwrap()));

        let req = VersionReq::parse("1.2.*").unwrap();
        assert!(req.matches(&Version::parse("1.2.0").unwrap()));
        assert!(req.matches(&Version::parse("1.2.99").unwrap()));
        assert!(!req.matches(&Version::parse("1.3.0").unwrap()));
    }

    #[test]
    fn test_range_version() {
        let req = VersionReq::parse(">=1.2.0, <1.8.0").unwrap();
        assert!(!req.matches(&Version::parse("1.1.9").unwrap()));
        assert!(req.matches(&Version::parse("1.2.0").unwrap()));
        assert!(req.matches(&Version::parse("1.5.0").unwrap()));
        assert!(!req.matches(&Version::parse("1.8.0").unwrap()));
    }

    #[test]
    fn test_greater_less() {
        let req = VersionReq::parse(">1.0.0").unwrap();
        assert!(!req.matches(&Version::parse("1.0.0").unwrap()));
        assert!(req.matches(&Version::parse("1.0.1").unwrap()));

        let req = VersionReq::parse("<2.0.0").unwrap();
        assert!(req.matches(&Version::parse("1.9.9").unwrap()));
        assert!(!req.matches(&Version::parse("2.0.0").unwrap()));
    }
}
