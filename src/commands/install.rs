//! Install command implementation - Pure Rust version
//!
//! Manages project dependencies from CCGO.toml.
//! Dependencies are cached globally in ~/.ccgo/ and linked to project's .ccgo/deps/.

use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Args;
use serde::{Deserialize, Serialize};

use crate::config::CcgoConfig;

/// Install project dependencies from CCGO.toml
#[derive(Args, Debug)]
pub struct InstallCommand {
    /// Specific dependency to install (default: install all)
    pub dependency: Option<String>,

    /// Force reinstall even if already installed
    #[arg(long)]
    pub force: bool,

    /// Install only platform-specific dependencies
    #[arg(long)]
    pub platform: Option<String>,

    /// Clean global cache before installing
    #[arg(long)]
    pub clean_cache: bool,

    /// Copy files instead of using symlinks
    #[arg(long)]
    pub copy: bool,
}

/// Dependency source type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
enum SourceType {
    LocalDir,
    LocalArchive,
    RemoteUrl,
}

/// Dependency installation information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct InstallInfo {
    source_type: String,
    source: String,
    install_path: String,
    installed_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    checksum: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    git_info: Option<GitInfo>,
}

/// Git repository information
#[derive(Debug, Clone, Serialize, Deserialize)]
struct GitInfo {
    #[serde(skip_serializing_if = "Option::is_none")]
    revision: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    remote_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    dirty: Option<bool>,
}

impl InstallCommand {
    /// Execute the install command
    pub fn execute(self, _verbose: bool) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Install - Install Project Dependencies");
        println!("{}", "=".repeat(80));

        let project_dir = std::env::current_dir().context("Failed to get current directory")?;
        let ccgo_home = Self::get_ccgo_home();

        println!("\nProject directory: {}", project_dir.display());
        println!("Global CCGO home: {}", ccgo_home.display());

        // Clean global cache if requested
        if self.clean_cache && ccgo_home.exists() {
            println!("\n🗑  Cleaning global cache: {}", ccgo_home.display());
            fs::remove_dir_all(&ccgo_home).context("Failed to clean cache")?;
        }

        // Load CCGO.toml
        println!("\n📖 Reading dependencies from CCGO.toml...");
        let config = CcgoConfig::load().context("Failed to load CCGO.toml")?;

        let dependencies = &config.dependencies;
        if dependencies.is_empty() {
            println!("   ℹ️  No dependencies defined in CCGO.toml");
            println!("\n💡 To add dependencies, edit CCGO.toml:");
            println!("   [[dependencies]]");
            println!("   name = \"my_lib\"");
            println!("   version = \"1.0.0\"");
            println!("   path = \"../my_lib\"  # or git = \"https://github.com/...\"");
            println!("\n✓ Install completed successfully (no dependencies to install)");
            return Ok(());
        }

        // Filter dependencies to install
        let mut deps_to_install = Vec::new();
        for dep in dependencies {
            // If specific dependency requested, filter
            if let Some(ref dep_name) = self.dependency {
                if &dep.name != dep_name {
                    continue;
                }
            }
            deps_to_install.push(dep);
        }

        if deps_to_install.is_empty() {
            if let Some(ref dep_name) = self.dependency {
                println!("   ⚠️  Dependency '{}' not found in CCGO.toml", dep_name);
            } else {
                println!("   ⚠️  No dependencies to install");
            }
            return Ok(());
        }

        println!("\nFound {} dependency(ies) to install:", deps_to_install.len());
        for dep in &deps_to_install {
            println!("  - {}", dep.name);
        }

        // Install each dependency
        println!("\n{}", "=".repeat(80));
        println!("Installing Dependencies");
        println!("{}", "=".repeat(80));

        let mut installed_count = 0;
        let mut failed_count = 0;
        let mut installed_deps = HashMap::new();

        for dep in deps_to_install {
            match self.install_dependency(dep, &project_dir, &ccgo_home) {
                Ok(install_info) => {
                    installed_count += 1;
                    installed_deps.insert(dep.name.clone(), install_info);
                }
                Err(e) => {
                    eprintln!("   ✗ Failed to install {}: {}", dep.name, e);
                    failed_count += 1;
                }
            }
        }

        // Generate lock file if any dependencies were installed
        if !installed_deps.is_empty() {
            Self::generate_lock_file(&project_dir, &installed_deps)?;
            Self::update_gitignore(&project_dir)?;
        }

        // Summary
        println!("\n{}", "=".repeat(80));
        println!("Installation Summary");
        println!("{}", "=".repeat(80));
        println!("\n✓ Successfully installed: {}", installed_count);
        println!("  Dependencies installed to: .ccgo/deps/");
        if failed_count > 0 {
            println!("✗ Failed: {}", failed_count);
        }
        println!();

        if failed_count > 0 {
            bail!("Some dependencies failed to install");
        }

        Ok(())
    }

    /// Install a single dependency
    fn install_dependency(
        &self,
        dep: &crate::config::DependencyConfig,
        project_dir: &Path,
        ccgo_home: &Path,
    ) -> Result<InstallInfo> {
        println!("\n📦 Installing {}...", dep.name);

        let deps_dir = project_dir.join(".ccgo").join("deps");
        fs::create_dir_all(&deps_dir).context("Failed to create .ccgo/deps directory")?;

        let install_path = deps_dir.join(&dep.name);

        // Check if already installed
        if install_path.exists() && !self.force {
            println!("   {} already installed (use --force to reinstall)", dep.name);
            return Ok(InstallInfo {
                source_type: "local_dir".to_string(),
                source: dep.path.as_ref().unwrap_or(&"".to_string()).clone(),
                install_path: install_path.to_string_lossy().to_string(),
                installed_at: chrono::Local::now().to_rfc3339(),
                version: Some(dep.version.clone()),
                checksum: None,
                git_info: None,
            });
        }

        // Remove existing installation if force
        if install_path.exists() {
            println!("   Removing existing installation...");
            if install_path.is_symlink() {
                fs::remove_file(&install_path)?;
            } else {
                fs::remove_dir_all(&install_path)?;
            }
        }

        // Handle based on source type
        if let Some(ref path) = dep.path {
            // Local path dependency
            self.install_from_local_path(&dep.name, path, project_dir, &install_path)
        } else if let Some(ref git_url) = dep.git {
            // Git dependency
            self.install_from_git(&dep.name, git_url, dep.branch.as_deref(), &install_path, ccgo_home)
        } else {
            bail!("No valid source found for dependency '{}'", dep.name);
        }
    }

    /// Install from local path
    fn install_from_local_path(
        &self,
        _dep_name: &str,
        path: &str,
        project_dir: &Path,
        install_path: &Path,
    ) -> Result<InstallInfo> {
        let source_path = if Path::new(path).is_absolute() {
            PathBuf::from(path)
        } else {
            project_dir.join(path)
        };

        if !source_path.exists() {
            bail!("Path does not exist: {}", source_path.display());
        }

        println!("   Source: {}", source_path.display());
        println!("   Installing from local directory...");

        // Create symlink or copy
        Self::create_symlink_or_copy(&source_path, install_path, self.copy)?;

        println!("   ✓ Installed to {}", install_path.display());

        Ok(InstallInfo {
            source_type: "local_dir".to_string(),
            source: source_path.to_string_lossy().to_string(),
            install_path: install_path.to_string_lossy().to_string(),
            installed_at: chrono::Local::now().to_rfc3339(),
            version: None,
            checksum: None,
            git_info: Self::get_git_info(&source_path),
        })
    }

    /// Install from git repository
    fn install_from_git(
        &self,
        dep_name: &str,
        git_url: &str,
        branch: Option<&str>,
        install_path: &Path,
        ccgo_home: &Path,
    ) -> Result<InstallInfo> {
        println!("   Source: {}", git_url);
        println!("   Installing from git repository...");

        // Create registry directory
        let registry_dir = ccgo_home.join("registry");
        fs::create_dir_all(&registry_dir)?;

        // Create unique hash for this git dependency
        let hash_input = format!("{}:{}", dep_name, git_url);
        let hash = format!("{:x}", md5::compute(hash_input.as_bytes()));
        let registry_name = format!("{}-{}", dep_name, &hash[..16]);
        let registry_path = registry_dir.join(&registry_name);

        // Clone or update if not exists or force
        if !registry_path.exists() || self.force {
            if registry_path.exists() {
                fs::remove_dir_all(&registry_path)?;
            }

            println!("   Cloning repository...");
            let mut cmd = std::process::Command::new("git");
            cmd.args(&["clone", git_url, registry_path.to_string_lossy().as_ref()]);

            if let Some(branch_name) = branch {
                cmd.args(&["--branch", branch_name]);
            }

            let output = cmd.output().context("Failed to execute git clone")?;
            if !output.status.success() {
                bail!("Git clone failed: {}", String::from_utf8_lossy(&output.stderr));
            }
            println!("   ✓ Cloned to {}", registry_path.display());
        }

        // Link/copy from registry to project
        Self::create_symlink_or_copy(&registry_path, install_path, self.copy)?;

        println!("   ✓ Installed to {}", install_path.display());

        Ok(InstallInfo {
            source_type: "git".to_string(),
            source: git_url.to_string(),
            install_path: install_path.to_string_lossy().to_string(),
            installed_at: chrono::Local::now().to_rfc3339(),
            version: None,
            checksum: None,
            git_info: Self::get_git_info(&registry_path),
        })
    }

    /// Create symlink or copy based on settings
    fn create_symlink_or_copy(source: &Path, target: &Path, use_copy: bool) -> Result<()> {
        if use_copy {
            println!("   Copying to {}...", target.display());
            Self::copy_dir_all(source, target)?;
        } else {
            // Try to create symlink
            #[cfg(unix)]
            {
                std::os::unix::fs::symlink(source, target).or_else(|_| {
                    println!("   ⚠️  Symlink failed, falling back to copy...");
                    Self::copy_dir_all(source, target)
                })?;
                println!("   Linked to {}", target.display());
            }

            #[cfg(windows)]
            {
                std::os::windows::fs::symlink_dir(source, target).or_else(|_| {
                    println!("   ⚠️  Symlink failed, falling back to copy...");
                    Self::copy_dir_all(source, target)
                })?;
                println!("   Linked to {}", target.display());
            }
        }
        Ok(())
    }

    /// Recursively copy directory
    fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
        fs::create_dir_all(dst)?;
        for entry in fs::read_dir(src)? {
            let entry = entry?;
            let ty = entry.file_type()?;
            if ty.is_dir() {
                Self::copy_dir_all(&entry.path(), &dst.join(entry.file_name()))?;
            } else {
                fs::copy(entry.path(), dst.join(entry.file_name()))?;
            }
        }
        Ok(())
    }

    /// Get git information for a local path
    fn get_git_info(path: &Path) -> Option<GitInfo> {
        if !path.is_dir() {
            return None;
        }

        let mut git_info = GitInfo {
            revision: None,
            branch: None,
            remote_url: None,
            dirty: None,
        };

        // Check if inside a git repository
        let check = std::process::Command::new("git")
            .args(&["rev-parse", "--is-inside-work-tree"])
            .current_dir(path)
            .output();

        if check.is_err() || !check.unwrap().status.success() {
            return None;
        }

        // Get revision
        if let Ok(output) = std::process::Command::new("git")
            .args(&["rev-parse", "HEAD"])
            .current_dir(path)
            .output()
        {
            if output.status.success() {
                git_info.revision = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
        }

        // Get branch
        if let Ok(output) = std::process::Command::new("git")
            .args(&["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(path)
            .output()
        {
            if output.status.success() {
                git_info.branch = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
        }

        // Get remote URL
        if let Ok(output) = std::process::Command::new("git")
            .args(&["config", "--get", "remote.origin.url"])
            .current_dir(path)
            .output()
        {
            if output.status.success() {
                git_info.remote_url = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
        }

        // Check if dirty
        if let Ok(output) = std::process::Command::new("git")
            .args(&["status", "--porcelain"])
            .current_dir(path)
            .output()
        {
            if output.status.success() {
                git_info.dirty = Some(!String::from_utf8_lossy(&output.stdout).trim().is_empty());
            }
        }

        Some(git_info)
    }

    /// Generate CCGO.toml.lock file
    fn generate_lock_file(project_dir: &Path, installed_deps: &HashMap<String, InstallInfo>) -> Result<()> {
        let lock_file_path = project_dir.join("CCGO.toml.lock");

        let mut content = String::new();
        content.push_str("# CCGO.toml.lock - Auto-generated lock file\n");
        content.push_str("# Do not edit this file manually\n");
        content.push_str("# Regenerate with: ccgo install --force\n\n");

        content.push_str("[metadata]\n");
        content.push_str("version = \"1.0\"\n");
        content.push_str(&format!("generated_at = \"{}\"\n", chrono::Local::now().to_rfc3339()));
        content.push_str("generator = \"ccgo install\"\n\n");

        for (dep_name, dep_info) in installed_deps {
            content.push_str(&format!("[dependencies.{}]\n", dep_name));
            content.push_str(&format!("source_type = \"{}\"\n", dep_info.source_type));
            content.push_str(&format!("source = \"{}\"\n", dep_info.source));
            content.push_str(&format!("installed_at = \"{}\"\n", dep_info.installed_at));
            content.push_str(&format!("install_path = \"{}\"\n", dep_info.install_path));

            if let Some(ref version) = dep_info.version {
                content.push_str(&format!("version = \"{}\"\n", version));
            }

            if let Some(ref checksum) = dep_info.checksum {
                content.push_str(&format!("checksum = \"{}\"\n", checksum));
            }

            if let Some(ref git_info) = dep_info.git_info {
                content.push_str(&format!("\n[dependencies.{}.git]\n", dep_name));
                if let Some(ref revision) = git_info.revision {
                    content.push_str(&format!("revision = \"{}\"\n", revision));
                }
                if let Some(ref branch) = git_info.branch {
                    content.push_str(&format!("branch = \"{}\"\n", branch));
                }
                if let Some(ref remote_url) = git_info.remote_url {
                    content.push_str(&format!("remote_url = \"{}\"\n", remote_url));
                }
                if let Some(dirty) = git_info.dirty {
                    content.push_str(&format!("dirty = {}\n", dirty));
                }
            }

            content.push_str("\n");
        }

        fs::write(&lock_file_path, content).context("Failed to write lock file")?;
        println!("\n📝 Generated lock file: {}", lock_file_path.display());

        Ok(())
    }

    /// Update .gitignore to exclude .ccgo/
    fn update_gitignore(project_dir: &Path) -> Result<()> {
        let gitignore_path = project_dir.join(".gitignore");
        let ccgo_pattern = ".ccgo/";

        if gitignore_path.exists() {
            let content = fs::read_to_string(&gitignore_path)?;
            if content.contains(ccgo_pattern) || content.contains(".ccgo") {
                return Ok(()); // Already ignored
            }

            // Append .ccgo/ to existing .gitignore
            let mut file = fs::OpenOptions::new()
                .append(true)
                .open(&gitignore_path)?;
            writeln!(file, "\n# CCGO dependencies (auto-generated)")?;
            writeln!(file, "{}", ccgo_pattern)?;
            println!("   Added {} to .gitignore", ccgo_pattern);
        } else {
            // Create new .gitignore
            let mut file = fs::File::create(&gitignore_path)?;
            writeln!(file, "# CCGO dependencies")?;
            writeln!(file, "{}", ccgo_pattern)?;
            println!("   Created .gitignore with {}", ccgo_pattern);
        }

        Ok(())
    }

    /// Get CCGO home directory
    fn get_ccgo_home() -> PathBuf {
        directories::BaseDirs::new()
            .and_then(|dirs| Some(dirs.home_dir().to_path_buf()))
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".ccgo")
    }
}
