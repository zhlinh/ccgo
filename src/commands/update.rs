//! Update command - Update dependencies to latest compatible versions
//!
//! Usage:
//!   ccgo update              # Update all dependencies
//!   ccgo update <name>       # Update specific dependency

use anyhow::{bail, Context, Result};
use clap::Args;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{CcgoConfig, DependencyConfig};
use crate::version::VersionReq;

/// Update dependencies to latest compatible versions
#[derive(Args, Debug)]
pub struct UpdateCommand {
    /// Specific dependency to update (default: update all)
    pub dependency: Option<String>,

    /// Force update even if already at latest version
    #[arg(long)]
    pub force: bool,

    /// Dry run - show what would be updated without making changes
    #[arg(long)]
    pub dry_run: bool,

    /// Update to exact version (ignores version constraints)
    #[arg(long, requires = "dependency")]
    pub exact: Option<String>,
}

impl UpdateCommand {
    /// Execute the update command
    pub fn execute(self, verbose: bool) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Update - Update Dependencies");
        println!("{}", "=".repeat(80));

        // Load CCGO.toml
        let config_path = Path::new("CCGO.toml");
        if !config_path.exists() {
            bail!("CCGO.toml not found in current directory.");
        }

        let config = CcgoConfig::load()
            .context("Failed to load CCGO.toml")?;

        if config.dependencies.is_empty() {
            println!("\n   ℹ️  No dependencies to update");
            return Ok(());
        }

        // Filter dependencies to update
        let deps_to_update: Vec<&DependencyConfig> = if let Some(ref dep_name) = self.dependency {
            config.dependencies.iter()
                .filter(|d| &d.name == dep_name)
                .collect()
        } else {
            config.dependencies.iter().collect()
        };

        if deps_to_update.is_empty() {
            if let Some(ref dep_name) = self.dependency {
                bail!("Dependency '{}' not found in CCGO.toml", dep_name);
            }
            println!("\n   ℹ️  No dependencies to update");
            return Ok(());
        }

        println!("\n📦 Checking {} dependency(ies) for updates...\n", deps_to_update.len());

        let mut updates = Vec::new();
        let mut up_to_date = Vec::new();
        let mut errors = Vec::new();

        for dep in deps_to_update {
            match self.check_dependency_update(dep, verbose) {
                Ok(Some(update_info)) => {
                    println!("   ⬆  {} {} → {}",
                        update_info.name,
                        update_info.current_version,
                        update_info.new_version);
                    updates.push(update_info);
                }
                Ok(None) => {
                    println!("   ✓ {} (up to date)", dep.name);
                    up_to_date.push(dep.name.clone());
                }
                Err(e) => {
                    eprintln!("   ✗ {}: {}", dep.name, e);
                    errors.push((dep.name.clone(), e));
                }
            }
        }

        // Summary
        println!("\n{}", "=".repeat(80));
        println!("Update Summary");
        println!("{}", "=".repeat(80));
        println!("\n  Updates available: {}", updates.len());
        println!("  Up to date: {}", up_to_date.len());
        if !errors.is_empty() {
            println!("  Errors: {}", errors.len());
        }

        if updates.is_empty() {
            println!("\n✓ All dependencies are up to date");
            return Ok(());
        }

        // Apply updates
        if self.dry_run {
            println!("\n🔍 Dry run mode - no changes made");
            println!("\nRun without --dry-run to apply these updates");
        } else {
            println!("\n📝 Applying updates...");
            self.apply_updates(&updates)?;
            println!("\n✓ Updated {} dependency(ies)", updates.len());
            println!("\n💡 Run 'ccgo install --force' to install updated dependencies");
        }

        if !errors.is_empty() {
            println!("\n⚠️  Some dependencies could not be checked for updates");
        }

        Ok(())
    }

    /// Check if a dependency has updates available
    fn check_dependency_update(
        &self,
        dep: &DependencyConfig,
        _verbose: bool,
    ) -> Result<Option<UpdateInfo>> {
        // Handle exact version override
        if let Some(ref exact_version) = self.exact {
            if Some(&dep.name) == self.dependency.as_ref() {
                return Ok(Some(UpdateInfo {
                    name: dep.name.clone(),
                    current_version: dep.version.clone(),
                    new_version: exact_version.clone(),
                    source: UpdateSource::Exact,
                }));
            }
        }

        // For git dependencies, check for new commits
        if dep.git.is_some() {
            return self.check_git_update(dep);
        }

        // For path dependencies, check local version
        if dep.path.is_some() {
            return self.check_path_update(dep);
        }

        // For version dependencies, check version constraints
        // In the future, this would query a package registry
        // For now, we can only update if there's a lock file with newer versions
        self.check_version_update(dep)
    }

    /// Check for git dependency updates
    fn check_git_update(&self, dep: &DependencyConfig) -> Result<Option<UpdateInfo>> {
        let git_url = dep.git.as_ref().unwrap();
        let deps_dir = Path::new(".ccgo").join("deps").join(&dep.name);

        if !deps_dir.exists() {
            return Ok(None); // Not installed yet
        }

        // Get current commit
        let current_commit = self.get_git_revision(&deps_dir)?;

        // Fetch latest from remote
        let output = std::process::Command::new("git")
            .args(&["fetch", "origin"])
            .current_dir(&deps_dir)
            .output()
            .context("Failed to fetch from git remote")?;

        if !output.status.success() {
            bail!("Git fetch failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        // Get remote commit
        let branch = dep.branch.as_deref().unwrap_or("main");
        let remote_commit = self.get_git_revision_of_branch(&deps_dir, branch)?;

        if current_commit != remote_commit {
            Ok(Some(UpdateInfo {
                name: dep.name.clone(),
                current_version: format!("git+{}@{}", git_url, &current_commit[..8]),
                new_version: format!("git+{}@{}", git_url, &remote_commit[..8]),
                source: UpdateSource::Git,
            }))
        } else {
            Ok(None)
        }
    }

    /// Check for path dependency updates
    fn check_path_update(&self, dep: &DependencyConfig) -> Result<Option<UpdateInfo>> {
        let path = dep.path.as_ref().unwrap();
        let source_path = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            std::env::current_dir()?.join(path)
        };

        if !source_path.exists() {
            bail!("Path does not exist: {}", source_path.display());
        }

        // Check if path dependency has a CCGO.toml with version
        let dep_config_path = source_path.join("CCGO.toml");
        if dep_config_path.exists() {
            let dep_config = CcgoConfig::load_from_path(&dep_config_path)?;
            if let Some(package) = &dep_config.package {
                let new_version = package.version.clone();

                if new_version != dep.version {
                    return Ok(Some(UpdateInfo {
                        name: dep.name.clone(),
                        current_version: dep.version.clone(),
                        new_version,
                        source: UpdateSource::Path,
                    }));
                }
            }
        }

        Ok(None)
    }

    /// Check for version dependency updates (stub for future registry support)
    fn check_version_update(&self, dep: &DependencyConfig) -> Result<Option<UpdateInfo>> {
        // Parse version requirement
        let _req = VersionReq::parse(&dep.version)
            .with_context(|| format!("Invalid version requirement: '{}'", dep.version))?;

        // In the future, this would:
        // 1. Query a package registry for available versions
        // 2. Find the latest version matching the requirement
        // 3. Compare with current version

        // For now, we can't check for updates without a registry
        Ok(None)
    }

    /// Get git revision (commit hash)
    fn get_git_revision(&self, repo_path: &Path) -> Result<String> {
        let output = std::process::Command::new("git")
            .args(&["rev-parse", "HEAD"])
            .current_dir(repo_path)
            .output()
            .context("Failed to get git revision")?;

        if !output.status.success() {
            bail!("Git rev-parse failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get git revision of a specific branch
    fn get_git_revision_of_branch(&self, repo_path: &Path, branch: &str) -> Result<String> {
        let output = std::process::Command::new("git")
            .args(&["rev-parse", &format!("origin/{}", branch)])
            .current_dir(repo_path)
            .output()
            .context("Failed to get git revision")?;

        if !output.status.success() {
            bail!("Git rev-parse failed: {}", String::from_utf8_lossy(&output.stderr));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Apply updates to CCGO.toml
    fn apply_updates(&self, updates: &[UpdateInfo]) -> Result<()> {
        let config_path = Path::new("CCGO.toml");
        let content = fs::read_to_string(config_path)?;

        let mut update_map = HashMap::new();
        for update in updates {
            update_map.insert(update.name.clone(), update);
        }

        // Parse and update versions
        let lines: Vec<&str> = content.lines().collect();
        let mut new_lines = Vec::new();
        let mut current_dep: Option<String> = None;

        for line in lines {
            let trimmed = line.trim();

            // Track which dependency we're in
            if trimmed.starts_with("name = ") {
                let name = trimmed
                    .trim_start_matches("name = ")
                    .trim_matches('"');
                current_dep = Some(name.to_string());
            }

            // Update version line if this dependency needs updating
            if trimmed.starts_with("version = ") {
                if let Some(ref dep_name) = current_dep {
                    if let Some(update) = update_map.get(dep_name) {
                        // Replace version line
                        let indent = line.len() - line.trim_start().len();
                        let new_line = format!("{}version = \"{}\"", " ".repeat(indent), update.new_version);
                        new_lines.push(new_line);
                        continue;
                    }
                }
            }

            new_lines.push(line.to_string());
        }

        let new_content = new_lines.join("\n") + "\n";
        fs::write(config_path, new_content)?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct UpdateInfo {
    name: String,
    current_version: String,
    new_version: String,
    #[allow(dead_code)]
    source: UpdateSource,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum UpdateSource {
    Git,
    Path,
    Exact,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_info_creation() {
        let info = UpdateInfo {
            name: "mylib".to_string(),
            current_version: "1.0.0".to_string(),
            new_version: "1.1.0".to_string(),
            source: UpdateSource::Path,
        };

        assert_eq!(info.name, "mylib");
        assert_eq!(info.current_version, "1.0.0");
        assert_eq!(info.new_version, "1.1.0");
    }
}
