//! Package collection management
//!
//! CCGO Package Collections provide a decentralized way to discover and share C++ packages.
//! Collections are curated JSON files hosted anywhere (GitHub, company servers, etc.)
//! that list packages with metadata like versions, descriptions, and keywords.

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// A package collection containing curated packages
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageCollection {
    /// Collection name (unique identifier)
    pub name: String,

    /// Human-readable description
    pub description: String,

    /// Collection version (for cache invalidation)
    pub version: String,

    /// Collection author/maintainer
    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<String>,

    /// Collection homepage URL
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,

    /// List of packages in this collection
    pub packages: Vec<PackageMetadata>,
}

/// Metadata for a package in a collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageMetadata {
    /// Package name
    pub name: String,

    /// Latest version
    pub version: String,

    /// Short description
    pub summary: String,

    /// Git repository URL
    pub repository: String,

    /// Supported platforms
    pub platforms: Vec<String>,

    /// Search keywords
    pub keywords: Vec<String>,

    /// License (SPDX identifier)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<String>,

    /// Package homepage
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
}

/// Stored information about a subscribed collection
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionInfo {
    /// Collection URL
    pub url: String,

    /// Local name (derived from URL or collection name)
    pub name: String,

    /// When it was added
    pub added_at: String,

    /// Last refresh timestamp
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_refresh: Option<String>,

    /// Cached collection data
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cached_at: Option<String>,
}

/// Collection manager for loading and searching collections
pub struct CollectionManager {
    /// CCGO home directory (~/.ccgo)
    ccgo_home: PathBuf,
}

impl CollectionManager {
    /// Create a new collection manager
    pub fn new() -> Self {
        Self {
            ccgo_home: Self::get_ccgo_home(),
        }
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

    /// Get collections directory
    fn collections_dir(&self) -> PathBuf {
        self.ccgo_home.join("collections")
    }

    /// Get collections index file path
    fn index_file(&self) -> PathBuf {
        self.collections_dir().join("index.json")
    }

    /// Get cache file path for a collection
    fn cache_file(&self, collection_name: &str) -> PathBuf {
        self.collections_dir()
            .join("cache")
            .join(format!("{}.json", collection_name))
    }

    /// Ensure collections directory exists
    fn ensure_dirs(&self) -> Result<()> {
        let collections_dir = self.collections_dir();
        if !collections_dir.exists() {
            fs::create_dir_all(&collections_dir)
                .context("Failed to create collections directory")?;
        }

        let cache_dir = collections_dir.join("cache");
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir).context("Failed to create cache directory")?;
        }

        Ok(())
    }

    /// Load collections index
    pub fn load_index(&self) -> Result<Vec<CollectionInfo>> {
        let index_file = self.index_file();
        if !index_file.exists() {
            return Ok(Vec::new());
        }

        let content = fs::read_to_string(&index_file).context("Failed to read index file")?;
        let collections: Vec<CollectionInfo> =
            serde_json::from_str(&content).context("Failed to parse index file")?;

        Ok(collections)
    }

    /// Save collections index
    pub fn save_index(&self, collections: &[CollectionInfo]) -> Result<()> {
        self.ensure_dirs()?;

        let content =
            serde_json::to_string_pretty(collections).context("Failed to serialize index")?;
        fs::write(self.index_file(), content).context("Failed to write index file")?;

        Ok(())
    }

    /// Add a collection
    pub fn add_collection(&self, url: &str) -> Result<CollectionInfo> {
        self.ensure_dirs()?;

        // Load existing collections
        let mut collections = self.load_index()?;

        // Check if already added
        if collections.iter().any(|c| c.url == url) {
            bail!("Collection already exists: {}", url);
        }

        // Fetch the collection
        let collection = self.fetch_collection(url)?;

        // Create collection info
        let info = CollectionInfo {
            url: url.to_string(),
            name: collection.name.clone(),
            added_at: chrono::Local::now().to_rfc3339(),
            last_refresh: Some(chrono::Local::now().to_rfc3339()),
            cached_at: Some(chrono::Local::now().to_rfc3339()),
        };

        // Save to cache
        self.save_collection_cache(&info.name, &collection)?;

        // Add to index
        collections.push(info.clone());
        self.save_index(&collections)?;

        Ok(info)
    }

    /// Remove a collection
    pub fn remove_collection(&self, name_or_url: &str) -> Result<()> {
        let mut collections = self.load_index()?;

        // Find collection by name or URL
        let pos = collections
            .iter()
            .position(|c| c.name == name_or_url || c.url == name_or_url)
            .with_context(|| format!("Collection not found: {}", name_or_url))?;

        let removed = collections.remove(pos);

        // Delete cache file
        let cache_file = self.cache_file(&removed.name);
        if cache_file.exists() {
            fs::remove_file(&cache_file).context("Failed to remove cache file")?;
        }

        // Save updated index
        self.save_index(&collections)?;

        Ok(())
    }

    /// Refresh a collection
    pub fn refresh_collection(&self, name_or_url: &str) -> Result<CollectionInfo> {
        let mut collections = self.load_index()?;

        // Find collection
        let info = collections
            .iter_mut()
            .find(|c| c.name == name_or_url || c.url == name_or_url)
            .with_context(|| format!("Collection not found: {}", name_or_url))?;

        // Fetch updated collection
        let url = info.url.clone();
        let collection = self.fetch_collection(&url)?;

        // Update timestamps
        info.last_refresh = Some(chrono::Local::now().to_rfc3339());
        info.cached_at = Some(chrono::Local::now().to_rfc3339());

        // Save to cache
        let name = info.name.clone();
        self.save_collection_cache(&name, &collection)?;

        // Clone info before saving index
        let result = info.clone();

        // Save updated index
        self.save_index(&collections)?;

        Ok(result)
    }

    /// Refresh all collections
    pub fn refresh_all(&self) -> Result<Vec<(String, Result<()>)>> {
        let collections = self.load_index()?;
        let mut results = Vec::new();

        for info in &collections {
            let result = self.refresh_collection(&info.name).map(|_| ());
            results.push((info.name.clone(), result));
        }

        Ok(results)
    }

    /// Fetch collection from URL
    fn fetch_collection(&self, url: &str) -> Result<PackageCollection> {
        // For local file URLs
        if url.starts_with("file://")
            || (!url.starts_with("http://") && !url.starts_with("https://"))
        {
            let path = url.strip_prefix("file://").unwrap_or(url);
            let content = fs::read_to_string(path)
                .with_context(|| format!("Failed to read collection from {}", path))?;
            let collection: PackageCollection =
                serde_json::from_str(&content).context("Failed to parse collection JSON")?;
            return Ok(collection);
        }

        // For HTTP(S) URLs, use reqwest
        self.fetch_collection_http(url)
    }

    /// Fetch collection from HTTP(S) URL
    fn fetch_collection_http(&self, url: &str) -> Result<PackageCollection> {
        use std::time::Duration;

        // Build HTTP client with timeout
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .connect_timeout(Duration::from_secs(10))
            .user_agent(format!("ccgo/{}", env!("CARGO_PKG_VERSION")))
            .build()
            .context("Failed to create HTTP client")?;

        // Send GET request
        let response = client
            .get(url)
            .send()
            .with_context(|| format!("Failed to fetch collection from {}", url))?;

        // Check HTTP status
        let status = response.status();
        if !status.is_success() {
            bail!(
                "Failed to fetch collection from {}: HTTP {}",
                url,
                status.as_u16()
            );
        }

        // Parse JSON response
        let collection: PackageCollection = response
            .json()
            .with_context(|| format!("Failed to parse collection JSON from {}", url))?;

        Ok(collection)
    }

    /// Save collection to cache
    fn save_collection_cache(&self, name: &str, collection: &PackageCollection) -> Result<()> {
        let cache_file = self.cache_file(name);
        let content =
            serde_json::to_string_pretty(collection).context("Failed to serialize collection")?;
        fs::write(&cache_file, content).context("Failed to write cache file")?;
        Ok(())
    }

    /// Load collection from cache
    pub fn load_collection(&self, name: &str) -> Result<PackageCollection> {
        let cache_file = self.cache_file(name);
        if !cache_file.exists() {
            bail!("Collection cache not found: {}. Try refreshing with 'ccgo collection refresh {}'", name, name);
        }

        let content = fs::read_to_string(&cache_file).context("Failed to read cache file")?;
        let collection: PackageCollection =
            serde_json::from_str(&content).context("Failed to parse cached collection")?;

        Ok(collection)
    }

    /// Load all collections
    pub fn load_all_collections(&self) -> Result<Vec<(CollectionInfo, PackageCollection)>> {
        let index = self.load_index()?;
        let mut results = Vec::new();

        for info in index {
            match self.load_collection(&info.name) {
                Ok(collection) => results.push((info, collection)),
                Err(e) => {
                    eprintln!("⚠️  Failed to load collection '{}': {}", info.name, e);
                }
            }
        }

        Ok(results)
    }

    /// Search packages across all collections
    pub fn search(
        &self,
        query: &str,
        collection_filter: Option<&str>,
    ) -> Result<Vec<(String, PackageMetadata)>> {
        let collections = self.load_all_collections()?;
        let mut results = Vec::new();
        let query_lower = query.to_lowercase();

        for (info, collection) in collections {
            // Filter by collection if specified
            if let Some(filter) = collection_filter {
                if info.name != filter {
                    continue;
                }
            }

            // Search packages
            for package in collection.packages {
                // Match against name, summary, or keywords
                let matches = package.name.to_lowercase().contains(&query_lower)
                    || package.summary.to_lowercase().contains(&query_lower)
                    || package
                        .keywords
                        .iter()
                        .any(|k| k.to_lowercase().contains(&query_lower));

                if matches {
                    results.push((info.name.clone(), package));
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_serialization() {
        let collection = PackageCollection {
            name: "test-collection".to_string(),
            description: "Test collection".to_string(),
            version: "1.0.0".to_string(),
            author: Some("Test Author".to_string()),
            homepage: None,
            packages: vec![PackageMetadata {
                name: "json".to_string(),
                version: "3.11.0".to_string(),
                summary: "JSON for Modern C++".to_string(),
                repository: "https://github.com/nlohmann/json".to_string(),
                platforms: vec!["all".to_string()],
                keywords: vec!["json".to_string(), "parser".to_string()],
                license: Some("MIT".to_string()),
                homepage: None,
            }],
        };

        let json = serde_json::to_string_pretty(&collection).unwrap();
        let deserialized: PackageCollection = serde_json::from_str(&json).unwrap();

        assert_eq!(collection.name, deserialized.name);
        assert_eq!(collection.packages.len(), deserialized.packages.len());
    }

    #[test]
    fn test_collection_info_serialization() {
        let info = CollectionInfo {
            url: "https://example.com/collection.json".to_string(),
            name: "example".to_string(),
            added_at: "2026-01-17T00:00:00Z".to_string(),
            last_refresh: None,
            cached_at: None,
        };

        let json = serde_json::to_string(&info).unwrap();
        let deserialized: CollectionInfo = serde_json::from_str(&json).unwrap();

        assert_eq!(info.url, deserialized.url);
        assert_eq!(info.name, deserialized.name);
    }
}
