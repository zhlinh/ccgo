//! Incremental build support
//!
//! This module provides smart rebuild detection to only rebuild changed files
//! and their dependencies, significantly improving rebuild times.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

/// Build state tracking for incremental builds
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BuildState {
    /// Project name
    pub project: String,
    /// Platform name
    pub platform: String,
    /// Link type (static, shared)
    pub link_type: String,
    /// Last successful build timestamp (Unix epoch seconds)
    pub last_build_time: u64,
    /// File hashes at last build (path -> SHA256 hash)
    pub file_hashes: HashMap<String, String>,
    /// CMake cache hash
    pub cmake_cache_hash: Option<String>,
    /// CCGO.toml hash
    pub config_hash: String,
    /// Build options hash (captures flags, platform config, etc.)
    pub options_hash: String,
}

impl BuildState {
    /// Create new build state
    pub fn new(
        project: String,
        platform: String,
        link_type: String,
        config_hash: String,
        options_hash: String,
    ) -> Self {
        Self {
            project,
            platform,
            link_type,
            last_build_time: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            file_hashes: HashMap::new(),
            cmake_cache_hash: None,
            config_hash,
            options_hash,
        }
    }

    /// Get the build state file path
    pub fn state_file(build_dir: &Path) -> PathBuf {
        build_dir.join(".ccgo_build_state.json")
    }

    /// Load build state from file
    pub fn load(build_dir: &Path) -> Result<Option<Self>> {
        let state_file = Self::state_file(build_dir);

        if !state_file.exists() {
            return Ok(None);
        }

        let content = std::fs::read_to_string(&state_file)
            .context("Failed to read build state file")?;

        let state: BuildState = serde_json::from_str(&content)
            .context("Failed to parse build state")?;

        Ok(Some(state))
    }

    /// Save build state to file
    pub fn save(&self, build_dir: &Path) -> Result<()> {
        let state_file = Self::state_file(build_dir);

        // Ensure build directory exists
        std::fs::create_dir_all(build_dir)
            .context("Failed to create build directory")?;

        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize build state")?;

        std::fs::write(&state_file, json)
            .context("Failed to write build state file")?;

        Ok(())
    }

    /// Check if incremental build is possible
    pub fn can_incremental_build(
        &self,
        config_hash: &str,
        options_hash: &str,
        cmake_cache_path: &Path,
    ) -> bool {
        // Config or options changed - need full rebuild
        if self.config_hash != config_hash || self.options_hash != options_hash {
            return false;
        }

        // CMake cache changed - need full rebuild
        if let Some(cached_hash) = &self.cmake_cache_hash {
            if let Ok(current_hash) = Self::hash_file(cmake_cache_path) {
                if cached_hash != &current_hash {
                    return false;
                }
            }
        }

        // CMake cache exists and state looks good
        cmake_cache_path.exists()
    }

    /// Scan source files and update hashes
    pub fn scan_sources(&mut self, src_dir: &Path) -> Result<()> {
        self.file_hashes.clear();

        if !src_dir.exists() {
            return Ok(());
        }

        for entry in WalkDir::new(src_dir)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();

            if path.is_file() {
                // Only track source and header files
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_string_lossy();
                    if matches!(
                        ext_str.as_ref(),
                        "c" | "cc" | "cpp" | "cxx" | "h" | "hh" | "hpp" | "hxx"
                    ) {
                        let relative_path = path.strip_prefix(src_dir)
                            .unwrap_or(path)
                            .to_string_lossy()
                            .to_string();

                        if let Ok(hash) = Self::hash_file(path) {
                            self.file_hashes.insert(relative_path, hash);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    /// Update CMake cache hash
    pub fn update_cmake_cache_hash(&mut self, cmake_cache_path: &Path) -> Result<()> {
        if cmake_cache_path.exists() {
            self.cmake_cache_hash = Some(Self::hash_file(cmake_cache_path)?);
        }
        Ok(())
    }

    /// Calculate SHA256 hash of a file
    fn hash_file(path: &Path) -> Result<String> {
        let content = std::fs::read(path)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;

        let mut hasher = Sha256::new();
        hasher.update(&content);
        let hash = hasher.finalize();

        Ok(format!("{:x}", hash))
    }

    /// Calculate hash of CCGO.toml config
    pub fn hash_config(config_path: &Path) -> Result<String> {
        Self::hash_file(config_path)
    }

    /// Calculate hash of build options (for detecting option changes)
    pub fn hash_options(options_str: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(options_str.as_bytes());
        let hash = hasher.finalize();
        format!("{:x}", hash)
    }
}

/// Incremental build analyzer
pub struct IncrementalAnalyzer {
    old_state: Option<BuildState>,
    new_state: BuildState,
}

impl IncrementalAnalyzer {
    /// Create new analyzer
    pub fn new(
        build_dir: &Path,
        project: String,
        platform: String,
        link_type: String,
        config_hash: String,
        options_hash: String,
    ) -> Result<Self> {
        let old_state = BuildState::load(build_dir)?;
        let new_state = BuildState::new(project, platform, link_type, config_hash, options_hash);

        Ok(Self { old_state, new_state })
    }

    /// Check if incremental build is possible
    pub fn can_incremental_build(&self, cmake_cache_path: &Path) -> bool {
        if let Some(old) = &self.old_state {
            old.can_incremental_build(
                &self.new_state.config_hash,
                &self.new_state.options_hash,
                cmake_cache_path,
            )
        } else {
            false
        }
    }

    /// Analyze what changed since last build
    pub fn analyze_changes(&self, src_dir: &Path) -> Result<ChangeAnalysis> {
        let mut analysis = ChangeAnalysis::default();

        // No previous state - full build required
        let old_state = match &self.old_state {
            Some(state) => state,
            None => {
                analysis.full_rebuild_required = true;
                analysis.reason = Some("No previous build state found".to_string());
                return Ok(analysis);
            }
        };

        // Check if config changed
        if old_state.config_hash != self.new_state.config_hash {
            analysis.full_rebuild_required = true;
            analysis.reason = Some("CCGO.toml configuration changed".to_string());
            return Ok(analysis);
        }

        // Check if build options changed
        if old_state.options_hash != self.new_state.options_hash {
            analysis.full_rebuild_required = true;
            analysis.reason = Some("Build options changed".to_string());
            return Ok(analysis);
        }

        // Scan current source files
        let mut current_hashes = HashMap::new();
        if src_dir.exists() {
            for entry in WalkDir::new(src_dir)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if path.is_file() {
                    if let Some(ext) = path.extension() {
                        let ext_str = ext.to_string_lossy();
                        if matches!(
                            ext_str.as_ref(),
                            "c" | "cc" | "cpp" | "cxx" | "h" | "hh" | "hpp" | "hxx"
                        ) {
                            let relative_path = path.strip_prefix(src_dir)
                                .unwrap_or(path)
                                .to_string_lossy()
                                .to_string();

                            if let Ok(hash) = BuildState::hash_file(path) {
                                current_hashes.insert(relative_path, hash);
                            }
                        }
                    }
                }
            }
        }

        // Compare hashes to detect changes
        for (path, current_hash) in &current_hashes {
            match old_state.file_hashes.get(path) {
                Some(old_hash) if old_hash != current_hash => {
                    // File modified
                    analysis.modified_files.push(path.clone());
                }
                None => {
                    // New file added
                    analysis.added_files.push(path.clone());
                }
                _ => {}
            }
        }

        // Detect removed files
        for path in old_state.file_hashes.keys() {
            if !current_hashes.contains_key(path) {
                analysis.removed_files.push(path.clone());
            }
        }

        // If files were added or removed, may need CMake reconfiguration
        if !analysis.added_files.is_empty() || !analysis.removed_files.is_empty() {
            analysis.cmake_reconfigure_needed = true;
        }

        Ok(analysis)
    }

    /// Finalize build state after successful build
    pub fn finalize(mut self, build_dir: &Path, src_dir: &Path) -> Result<BuildState> {
        // Scan source files
        self.new_state.scan_sources(src_dir)?;

        // Update CMake cache hash
        let cmake_cache = build_dir.join("CMakeCache.txt");
        self.new_state.update_cmake_cache_hash(&cmake_cache)?;

        // Update timestamp
        self.new_state.last_build_time = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Save state
        self.new_state.save(build_dir)?;

        Ok(self.new_state)
    }
}

/// Change analysis result
#[derive(Debug, Default)]
pub struct ChangeAnalysis {
    /// Full rebuild is required
    pub full_rebuild_required: bool,
    /// Reason for full rebuild
    pub reason: Option<String>,
    /// List of modified files (relative paths)
    pub modified_files: Vec<String>,
    /// List of added files (relative paths)
    pub added_files: Vec<String>,
    /// List of removed files (relative paths)
    pub removed_files: Vec<String>,
    /// Whether CMake reconfiguration is needed
    pub cmake_reconfigure_needed: bool,
}

impl ChangeAnalysis {
    /// Check if any changes detected
    pub fn has_changes(&self) -> bool {
        self.full_rebuild_required
            || !self.modified_files.is_empty()
            || !self.added_files.is_empty()
            || !self.removed_files.is_empty()
    }

    /// Get total number of changed files
    pub fn total_changes(&self) -> usize {
        self.modified_files.len() + self.added_files.len() + self.removed_files.len()
    }

    /// Print change summary
    pub fn print_summary(&self) {
        if self.full_rebuild_required {
            if let Some(reason) = &self.reason {
                println!("   🔄 Full rebuild required: {}", reason);
            } else {
                println!("   🔄 Full rebuild required");
            }
            return;
        }

        if !self.has_changes() {
            println!("   ✨ No source changes detected, using cached build");
            return;
        }

        println!("   📊 Incremental build - {} files changed:", self.total_changes());

        if !self.modified_files.is_empty() {
            println!("      Modified: {}", self.modified_files.len());
        }
        if !self.added_files.is_empty() {
            println!("      Added:    {}", self.added_files.len());
        }
        if !self.removed_files.is_empty() {
            println!("      Removed:  {}", self.removed_files.len());
        }

        if self.cmake_reconfigure_needed {
            println!("   🔧 CMake reconfiguration needed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_hash_options() {
        let hash1 = BuildState::hash_options("--release");
        let hash2 = BuildState::hash_options("--release");
        let hash3 = BuildState::hash_options("--debug");

        assert_eq!(hash1, hash2);
        assert_ne!(hash1, hash3);
    }

    #[test]
    fn test_build_state_save_load() {
        let temp_dir = tempfile::tempdir().unwrap();
        let build_dir = temp_dir.path();

        let state = BuildState::new(
            "test".to_string(),
            "linux".to_string(),
            "static".to_string(),
            "config_hash".to_string(),
            "options_hash".to_string(),
        );

        state.save(build_dir).unwrap();

        let loaded = BuildState::load(build_dir).unwrap();
        assert!(loaded.is_some());

        let loaded = loaded.unwrap();
        assert_eq!(loaded.project, "test");
        assert_eq!(loaded.platform, "linux");
    }

    #[test]
    fn test_hash_file() {
        let temp_dir = tempfile::tempdir().unwrap();
        let file_path = temp_dir.path().join("test.cpp");

        let mut file = std::fs::File::create(&file_path).unwrap();
        file.write_all(b"int main() { return 0; }").unwrap();
        drop(file);

        let hash1 = BuildState::hash_file(&file_path).unwrap();
        let hash2 = BuildState::hash_file(&file_path).unwrap();

        assert_eq!(hash1, hash2);
        assert!(!hash1.is_empty());
    }
}
