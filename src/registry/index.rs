//! Lightweight package index repository support
//!
//! CCGO uses a Git-based index repository for package discovery, similar to:
//! - Rust's crates.io-index
//! - Homebrew's homebrew-core
//!
//! # Index Structure
//!
//! ```text
//! ccgo-packages/
//! ├── index.json              # Index metadata
//! ├── f/
//! │   └── fmt.json            # Package metadata for 'fmt'
//! ├── s/
//! │   └── spdlog.json         # Package metadata for 'spdlog'
//! └── n/
//!     └── nlohmann-json.json  # Package metadata for 'nlohmann-json'
//! ```
//!
//! # Usage in CCGO.toml
//!
//! ```toml
//! # Default index (simplified syntax)
//! [dependencies]
//! fmt = "^10.1"
//! spdlog = "1.12.0"
//!
//! # Custom private index
//! [registries]
//! company = "https://github.com/company/ccgo-packages.git"
//!
//! [dependencies]
//! internal-lib = { registry = "company", version = "^2.0" }
//! ```

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

/// Default registry name
pub const DEFAULT_REGISTRY: &str = "ccgo-packages";

/// Default registry URL
pub const DEFAULT_REGISTRY_URL: &str = "https://github.com/aspect-build/ccgo-packages.git";

/// Index metadata (index.json)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexMetadata {
    /// Index format version
    pub version: u32,

    /// Index name
    pub name: String,

    /// Index description
    pub description: String,

    /// Homepage URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,

    /// Total package count
    pub package_count: usize,

    /// Last updated timestamp (RFC 3339)
    pub updated_at: String,
}

/// Package entry in the index
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageEntry {
    /// Package name
    pub name: String,

    /// Short description
    pub description: String,

    /// Git repository URL
    pub repository: String,

    /// Homepage URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,

    /// License (SPDX identifier)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Keywords for search
    #[serde(default)]
    pub keywords: Vec<String>,

    /// Supported platforms
    #[serde(default)]
    pub platforms: Vec<String>,

    /// Available versions
    pub versions: Vec<VersionEntry>,
}

/// Version entry for a package
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VersionEntry {
    /// Version string (e.g., "10.1.1")
    pub version: String,

    /// Git tag for this version
    pub tag: String,

    /// SHA-256 checksum of the archive (optional)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,

    /// Release date (RFC 3339)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub released_at: Option<String>,

    /// Whether this version is yanked (deprecated)
    #[serde(default)]
    pub yanked: bool,

    /// Yanked reason
    #[serde(skip_serializing_if = "Option::is_none")]
    pub yanked_reason: Option<String>,
}

/// Registry configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryConfig {
    /// Registry name
    pub name: String,

    /// Git URL of the index repository
    pub url: String,

    /// Local cache path
    #[serde(skip)]
    pub cache_path: Option<PathBuf>,
}

/// Result of updating a registry
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UpdateResult {
    /// Registry was cloned (first time)
    Cloned,
    /// Registry was updated with new changes
    Updated,
    /// Registry was already up to date
    AlreadyUpToDate,
}

impl std::fmt::Display for UpdateResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            UpdateResult::Cloned => write!(f, "cloned"),
            UpdateResult::Updated => write!(f, "updated"),
            UpdateResult::AlreadyUpToDate => write!(f, "already up to date"),
        }
    }
}

/// Resolved package information
#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    /// Package name
    pub name: String,
    /// Git repository URL
    pub repository: String,
    /// Git tag for the resolved version
    pub tag: String,
    /// Resolved version string
    pub version: String,
    /// Registry name
    pub registry: String,
}

/// Package index manager
pub struct PackageIndex {
    /// CCGO home directory
    ccgo_home: PathBuf,

    /// Configured registries
    registries: HashMap<String, RegistryConfig>,

    /// Whether the default registry is configured
    has_default: bool,
}

impl PackageIndex {
    /// Create a new package index manager
    pub fn new() -> Self {
        let ccgo_home = Self::get_ccgo_home();
        let mut index = Self {
            ccgo_home,
            registries: HashMap::new(),
            has_default: false,
        };
        // Load saved registries
        let _ = index.load_registries();
        index
    }

    /// Create with custom CCGO home (for testing)
    pub fn with_home(ccgo_home: PathBuf) -> Self {
        let mut index = Self {
            ccgo_home,
            registries: HashMap::new(),
            has_default: false,
        };
        let _ = index.load_registries();
        index
    }

    /// Get CCGO home directory
    fn get_ccgo_home() -> PathBuf {
        if let Ok(home) = std::env::var("CCGO_HOME") {
            PathBuf::from(home)
        } else if let Ok(home) = std::env::var("HOME") {
            PathBuf::from(home).join(".ccgo")
        } else {
            PathBuf::from(".ccgo")
        }
    }

    /// Get the index cache directory
    fn index_cache_dir(&self) -> PathBuf {
        self.ccgo_home.join("registry").join("index")
    }

    /// Get cache path for a registry
    fn registry_cache_path(&self, registry_name: &str) -> PathBuf {
        self.index_cache_dir().join(registry_name)
    }

    /// Get the registries config file path
    fn registries_config_path(&self) -> PathBuf {
        self.ccgo_home.join("registry").join("registries.json")
    }

    /// Ensure the default registry is configured
    pub fn ensure_default_registry(&mut self) {
        if !self.has_default && !self.registries.contains_key(DEFAULT_REGISTRY) {
            self.add_registry(DEFAULT_REGISTRY, DEFAULT_REGISTRY_URL);
            self.has_default = true;
        }
    }

    /// Load registries from config file
    fn load_registries(&mut self) -> Result<()> {
        let config_path = self.registries_config_path();
        if !config_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(&config_path)
            .context("Failed to read registries config")?;

        let configs: Vec<RegistryConfig> = serde_json::from_str(&content)
            .context("Failed to parse registries config")?;

        for mut config in configs {
            config.cache_path = Some(self.registry_cache_path(&config.name));
            if config.name == DEFAULT_REGISTRY {
                self.has_default = true;
            }
            self.registries.insert(config.name.clone(), config);
        }

        Ok(())
    }

    /// Save registries to config file
    fn save_registries(&self) -> Result<()> {
        let config_path = self.registries_config_path();
        fs::create_dir_all(config_path.parent().unwrap())
            .context("Failed to create registry config directory")?;

        let configs: Vec<&RegistryConfig> = self.registries.values().collect();
        let content = serde_json::to_string_pretty(&configs)
            .context("Failed to serialize registries")?;

        fs::write(&config_path, content)
            .context("Failed to write registries config")?;

        Ok(())
    }

    /// Add a registry
    pub fn add_registry(&mut self, name: &str, url: &str) {
        self.registries.insert(
            name.to_string(),
            RegistryConfig {
                name: name.to_string(),
                url: url.to_string(),
                cache_path: Some(self.registry_cache_path(name)),
            },
        );
        let _ = self.save_registries();
    }

    /// Remove a registry
    pub fn remove_registry(&mut self, name: &str) -> Result<()> {
        if name == DEFAULT_REGISTRY {
            bail!("Cannot remove the default registry");
        }

        self.registries.remove(name)
            .with_context(|| format!("Registry not found: {}", name))?;

        // Remove cache directory
        let cache_path = self.registry_cache_path(name);
        if cache_path.exists() {
            fs::remove_dir_all(&cache_path)
                .context("Failed to remove registry cache")?;
        }

        self.save_registries()?;
        Ok(())
    }

    /// List all configured registries
    pub fn list_registries(&self) -> Vec<&RegistryConfig> {
        self.registries.values().collect()
    }

    /// Get a registry by name
    pub fn get_registry(&self, name: &str) -> Option<&RegistryConfig> {
        self.registries.get(name)
    }

    /// Check if a registry is cached locally
    pub fn is_cached(&self, registry_name: &str) -> bool {
        let cache_path = self.registry_cache_path(registry_name);
        cache_path.exists() && cache_path.join("index.json").exists()
    }

    /// Update a registry index (clone or pull)
    pub fn update_registry(&self, registry_name: &str) -> Result<UpdateResult> {
        let registry = self
            .registries
            .get(registry_name)
            .with_context(|| format!("Registry not found: {}", registry_name))?;

        let cache_path = self.registry_cache_path(registry_name);

        if cache_path.exists() {
            // Pull updates
            let output = Command::new("git")
                .args(["pull", "--ff-only"])
                .current_dir(&cache_path)
                .output()
                .context("Failed to update registry")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("Failed to update registry: {}", stderr);
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.contains("Already up to date") {
                Ok(UpdateResult::AlreadyUpToDate)
            } else {
                Ok(UpdateResult::Updated)
            }
        } else {
            // Clone the index
            fs::create_dir_all(cache_path.parent().unwrap())
                .context("Failed to create registry cache directory")?;

            let output = Command::new("git")
                .args(["clone", "--depth", "1", &registry.url])
                .arg(&cache_path)
                .output()
                .context("Failed to clone registry index")?;

            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                bail!("Failed to clone registry: {}", stderr);
            }

            Ok(UpdateResult::Cloned)
        }
    }

    /// Update all registries
    pub fn update_all(&self) -> Vec<(String, Result<UpdateResult>)> {
        self.registries
            .keys()
            .map(|name| (name.clone(), self.update_registry(name)))
            .collect()
    }

    /// Load index metadata for a registry
    pub fn load_metadata(&self, registry_name: &str) -> Result<Option<IndexMetadata>> {
        let cache_path = self.registry_cache_path(registry_name);
        let metadata_path = cache_path.join("index.json");

        if !metadata_path.exists() {
            return Ok(None);
        }

        let content = fs::read_to_string(&metadata_path)
            .context("Failed to read index metadata")?;

        let metadata: IndexMetadata = serde_json::from_str(&content)
            .context("Failed to parse index metadata")?;

        Ok(Some(metadata))
    }

    /// Look up a package in a registry
    pub fn lookup_package(&self, registry_name: &str, package_name: &str) -> Result<Option<PackageEntry>> {
        let cache_path = self.registry_cache_path(registry_name);
        let first_char = package_name.chars().next().unwrap_or('_').to_lowercase().to_string();
        let package_file = cache_path.join(&first_char).join(format!("{}.json", package_name));

        if !package_file.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&package_file)
            .with_context(|| format!("Failed to read package file: {}", package_file.display()))?;

        let package: PackageEntry = serde_json::from_str(&content)
            .with_context(|| format!("Failed to parse package file: {}", package_file.display()))?;

        Ok(Some(package))
    }

    /// Search packages in a registry
    pub fn search_packages(&self, registry_name: &str, query: &str) -> Result<Vec<PackageEntry>> {
        let cache_path = self.registry_cache_path(registry_name);
        let query_lower = query.to_lowercase();
        let mut results = Vec::new();

        // Walk through all subdirectories
        for entry in std::fs::read_dir(&cache_path).context("Failed to read registry cache")? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() && path.file_name().map_or(false, |n| n.len() == 1) {
                // This is a letter directory
                for package_file in std::fs::read_dir(&path)? {
                    let package_file = package_file?;
                    let file_path = package_file.path();

                    if file_path.extension().map_or(false, |e| e == "json") {
                        if let Ok(content) = std::fs::read_to_string(&file_path) {
                            if let Ok(package) = serde_json::from_str::<PackageEntry>(&content) {
                                // Match against name, description, or keywords
                                let matches = package.name.to_lowercase().contains(&query_lower)
                                    || package.description.to_lowercase().contains(&query_lower)
                                    || package.keywords.iter().any(|k| k.to_lowercase().contains(&query_lower));

                                if matches {
                                    results.push(package);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(results)
    }

    /// Get the latest version of a package
    pub fn get_latest_version(&self, registry_name: &str, package_name: &str) -> Result<Option<VersionEntry>> {
        let package = self.lookup_package(registry_name, package_name)?;

        if let Some(pkg) = package {
            // Find the latest non-yanked version
            let latest = pkg.versions.iter()
                .filter(|v| !v.yanked)
                .max_by(|a, b| {
                    // Compare by semver
                    let ver_a = crate::registry::SemVer::parse(&a.version);
                    let ver_b = crate::registry::SemVer::parse(&b.version);
                    match (ver_a, ver_b) {
                        (Some(a), Some(b)) => a.cmp(&b),
                        (Some(_), None) => std::cmp::Ordering::Greater,
                        (None, Some(_)) => std::cmp::Ordering::Less,
                        (None, None) => a.version.cmp(&b.version),
                    }
                });

            Ok(latest.cloned())
        } else {
            Ok(None)
        }
    }

    /// Resolve a package specification to a Git URL and tag
    pub fn resolve_package(
        &self,
        registry_name: &str,
        package_name: &str,
        version_req: &str,
    ) -> Result<Option<ResolvedPackage>> {
        let package = self.lookup_package(registry_name, package_name)?;

        if let Some(pkg) = package {
            let req = crate::version::VersionReq::parse(version_req)?;

            // Sort versions by semver (highest first) to get best match
            let mut versions: Vec<_> = pkg.versions.iter()
                .filter(|v| !v.yanked)
                .collect();

            versions.sort_by(|a, b| {
                let ver_a = crate::registry::SemVer::parse(&a.version);
                let ver_b = crate::registry::SemVer::parse(&b.version);
                match (ver_b, ver_a) {
                    (Some(b), Some(a)) => b.cmp(&a),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => b.version.cmp(&a.version),
                }
            });

            // Find best matching version
            for ver in versions {
                // Normalize version string for parsing
                let ver_str = normalize_version(&ver.version);
                if let Ok(parsed) = crate::version::Version::parse(&ver_str) {
                    if req.matches(&parsed) {
                        return Ok(Some(ResolvedPackage {
                            name: package_name.to_string(),
                            repository: pkg.repository.clone(),
                            tag: ver.tag.clone(),
                            version: ver.version.clone(),
                            registry: registry_name.to_string(),
                        }));
                    }
                }
            }
        }

        Ok(None)
    }

    /// Resolve a package from any registry (tries default first)
    pub fn resolve_from_any(
        &self,
        package_name: &str,
        version_req: &str,
    ) -> Result<Option<ResolvedPackage>> {
        // Try default registry first
        if let Some(resolved) = self.resolve_package(DEFAULT_REGISTRY, package_name, version_req)? {
            return Ok(Some(resolved));
        }

        // Try other registries
        for name in self.registries.keys() {
            if name == DEFAULT_REGISTRY {
                continue;
            }
            if let Some(resolved) = self.resolve_package(name, package_name, version_req)? {
                return Ok(Some(resolved));
            }
        }

        Ok(None)
    }

    /// Search packages across all registries
    pub fn search_all(&self, query: &str) -> Result<Vec<(String, PackageEntry)>> {
        let mut results = Vec::new();

        for registry_name in self.registries.keys() {
            if let Ok(packages) = self.search_packages(registry_name, query) {
                for pkg in packages {
                    results.push((registry_name.clone(), pkg));
                }
            }
        }

        Ok(results)
    }
}

impl Default for PackageIndex {
    fn default() -> Self {
        Self::new()
    }
}

/// Normalize a version string to ensure it has major.minor.patch format
fn normalize_version(version: &str) -> String {
    let parts: Vec<&str> = version.split('.').collect();
    match parts.len() {
        1 => format!("{}.0.0", parts[0]),
        2 => format!("{}.{}.0", parts[0], parts[1]),
        _ => version.to_string(),
    }
}

/// Generate a package entry JSON file
pub fn generate_package_entry(
    name: &str,
    description: &str,
    repository: &str,
    versions: Vec<(String, String)>, // (version, tag) pairs
) -> PackageEntry {
    PackageEntry {
        name: name.to_string(),
        description: description.to_string(),
        repository: repository.to_string(),
        homepage: None,
        license: None,
        keywords: Vec::new(),
        platforms: vec!["all".to_string()],
        versions: versions
            .into_iter()
            .map(|(version, tag)| VersionEntry {
                version,
                tag,
                checksum: None,
                released_at: None,
                yanked: false,
                yanked_reason: None,
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_entry_serialization() {
        let entry = generate_package_entry(
            "fmt",
            "A modern formatting library",
            "https://github.com/fmtlib/fmt.git",
            vec![
                ("10.2.1".to_string(), "10.2.1".to_string()),
                ("10.1.1".to_string(), "10.1.1".to_string()),
            ],
        );

        let json = serde_json::to_string_pretty(&entry).unwrap();
        assert!(json.contains("\"name\": \"fmt\""));
        assert!(json.contains("\"version\": \"10.2.1\""));
    }

    #[test]
    fn test_index_metadata_serialization() {
        let metadata = IndexMetadata {
            version: 1,
            name: "ccgo-packages".to_string(),
            description: "Official CCGO package index".to_string(),
            homepage: Some("https://github.com/ccgo-packages".to_string()),
            package_count: 100,
            updated_at: "2026-01-24T00:00:00Z".to_string(),
        };

        let json = serde_json::to_string_pretty(&metadata).unwrap();
        assert!(json.contains("\"version\": 1"));
        assert!(json.contains("\"package_count\": 100"));
    }

    #[test]
    fn test_version_entry_yanked() {
        let entry = VersionEntry {
            version: "1.0.0".to_string(),
            tag: "v1.0.0".to_string(),
            checksum: None,
            released_at: None,
            yanked: true,
            yanked_reason: Some("Security vulnerability".to_string()),
        };

        assert!(entry.yanked);
        assert!(entry.yanked_reason.is_some());
    }
}
