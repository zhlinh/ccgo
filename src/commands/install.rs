//! Install command implementation - Pure Rust version
//!
//! Manages project dependencies from CCGO.toml.
//! Dependencies are cached globally in ~/.ccgo/ and linked to project's .ccgo/deps/.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Args;
use serde::{Deserialize, Serialize};

use crate::config::{CcgoConfig, DependencyConfig};
use crate::lockfile::{Lockfile, LockedPackage, LockedGitInfo, LOCKFILE_NAME};

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

    /// Require CCGO.lock and use exact versions from it
    #[arg(long)]
    pub locked: bool,
}

/// Git repository information (internal use)
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

        // Load existing lockfile
        let existing_lockfile = Lockfile::load(&project_dir)?;
        if let Some(ref _lockfile) = existing_lockfile {
            println!("📋 Found existing {}", LOCKFILE_NAME);
        }

        // In locked mode, lockfile is required
        if self.locked && existing_lockfile.is_none() {
            bail!(
                "No {} found. Run 'ccgo install' first to generate a lockfile, \
                 or remove --locked flag.",
                LOCKFILE_NAME
            );
        }

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

        // Check for outdated dependencies in locked mode
        if self.locked {
            if let Some(ref lockfile) = existing_lockfile {
                let outdated = lockfile.check_outdated(dependencies);
                if !outdated.is_empty() {
                    bail!(
                        "Dependencies have changed since lockfile was generated.\n\
                         Changed dependencies: {}\n\
                         Run 'ccgo install' to update the lockfile, or remove --locked flag.",
                        outdated.join(", ")
                    );
                }
            }
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
            // Show locked version if available
            if let Some(ref lockfile) = existing_lockfile {
                if let Some(locked) = lockfile.get_package(&dep.name) {
                    println!("  - {} (locked: {})", dep.name, locked.version);
                    continue;
                }
            }
            println!("  - {}", dep.name);
        }

        // Install each dependency
        println!("\n{}", "=".repeat(80));
        println!("Installing Dependencies");
        println!("{}", "=".repeat(80));

        let mut installed_count = 0;
        let mut failed_count = 0;
        let mut lockfile = existing_lockfile.unwrap_or_else(Lockfile::new);

        for dep in deps_to_install {
            // Get locked info if available
            let locked_pkg = lockfile.get_package(&dep.name).cloned();

            match self.install_dependency(dep, &project_dir, &ccgo_home, locked_pkg.as_ref()) {
                Ok(locked_package) => {
                    installed_count += 1;
                    lockfile.upsert_package(locked_package);
                }
                Err(e) => {
                    eprintln!("   ✗ Failed to install {}: {}", dep.name, e);
                    failed_count += 1;
                }
            }
        }

        // Save lockfile if any dependencies were installed
        if installed_count > 0 {
            lockfile.touch();
            lockfile.save(&project_dir)?;
            println!("\n📝 Updated {}", LOCKFILE_NAME);
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
        dep: &DependencyConfig,
        project_dir: &Path,
        ccgo_home: &Path,
        locked: Option<&LockedPackage>,
    ) -> Result<LockedPackage> {
        println!("\n📦 Installing {}...", dep.name);

        let deps_dir = project_dir.join(".ccgo").join("deps");
        fs::create_dir_all(&deps_dir).context("Failed to create .ccgo/deps directory")?;

        let install_path = deps_dir.join(&dep.name);

        // Check if already installed and matches lockfile
        if install_path.exists() && !self.force {
            if let Some(locked_pkg) = locked {
                println!("   {} already installed (locked: {})", dep.name, locked_pkg.version);
                return Ok(locked_pkg.clone());
            }
            println!("   {} already installed (use --force to reinstall)", dep.name);
            // Return a basic locked package for already installed deps
            return Ok(LockedPackage {
                name: dep.name.clone(),
                version: dep.version.clone(),
                source: Self::build_source_string(dep),
                checksum: None,
                dependencies: vec![],
                git: None,
                installed_at: Some(chrono::Local::now().to_rfc3339()),
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
            self.install_from_local_path(&dep.name, &dep.version, path, project_dir, &install_path)
        } else if let Some(ref git_url) = dep.git {
            // Git dependency - use locked revision if available
            let locked_rev = locked.and_then(|l| l.git_revision()).map(|s| s.to_string());
            self.install_from_git(&dep.name, &dep.version, git_url, dep.branch.as_deref(), locked_rev.as_deref(), &install_path, ccgo_home)
        } else {
            bail!("No valid source found for dependency '{}'", dep.name);
        }
    }

    /// Build source string from dependency config
    fn build_source_string(dep: &DependencyConfig) -> String {
        if let Some(ref git) = dep.git {
            format!("git+{}", git)
        } else if let Some(ref path) = dep.path {
            format!("path+{}", path)
        } else {
            format!("registry+{}@{}", dep.name, dep.version)
        }
    }

    /// Install from local path
    fn install_from_local_path(
        &self,
        dep_name: &str,
        version: &str,
        path: &str,
        project_dir: &Path,
        install_path: &Path,
    ) -> Result<LockedPackage> {
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

        // Try to get version from dependency's CCGO.toml if not specified
        let resolved_version = if version.is_empty() {
            Self::get_dep_version(&source_path).unwrap_or_else(|| "0.0.0".to_string())
        } else {
            version.to_string()
        };

        // Get git info if this is a git repo
        let git_info = Self::get_git_info(&source_path).map(|info| LockedGitInfo {
            revision: info.revision.unwrap_or_default(),
            branch: info.branch,
            tag: None,
            dirty: info.dirty.unwrap_or(false),
        });

        Ok(LockedPackage {
            name: dep_name.to_string(),
            version: resolved_version,
            source: format!("path+{}", path),
            checksum: None,
            dependencies: vec![],
            git: git_info,
            installed_at: Some(chrono::Local::now().to_rfc3339()),
        })
    }

    /// Get version from dependency's CCGO.toml
    fn get_dep_version(dep_path: &Path) -> Option<String> {
        let ccgo_toml = dep_path.join("CCGO.toml");
        if !ccgo_toml.exists() {
            return None;
        }
        CcgoConfig::load_from(&ccgo_toml)
            .ok()
            .and_then(|c| c.package.map(|p| p.version))
    }

    /// Install from git repository
    fn install_from_git(
        &self,
        dep_name: &str,
        version: &str,
        git_url: &str,
        branch: Option<&str>,
        locked_rev: Option<&str>,
        install_path: &Path,
        ccgo_home: &Path,
    ) -> Result<LockedPackage> {
        println!("   Source: {}", git_url);
        if let Some(rev) = locked_rev {
            println!("   Locked revision: {}", &rev[..8.min(rev.len())]);
        }
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
            cmd.args(["clone", git_url, registry_path.to_string_lossy().as_ref()]);

            if let Some(branch_name) = branch {
                cmd.args(["--branch", branch_name]);
            }

            let output = cmd.output().context("Failed to execute git clone")?;
            if !output.status.success() {
                bail!("Git clone failed: {}", String::from_utf8_lossy(&output.stderr));
            }
            println!("   ✓ Cloned to {}", registry_path.display());

            // Checkout specific revision if locked
            if let Some(rev) = locked_rev {
                println!("   Checking out locked revision {}...", &rev[..8.min(rev.len())]);
                let checkout = std::process::Command::new("git")
                    .args(["checkout", rev])
                    .current_dir(&registry_path)
                    .output()
                    .context("Failed to checkout revision")?;
                if !checkout.status.success() {
                    bail!("Git checkout failed: {}", String::from_utf8_lossy(&checkout.stderr));
                }
            }
        }

        // Link/copy from registry to project
        Self::create_symlink_or_copy(&registry_path, install_path, self.copy)?;

        println!("   ✓ Installed to {}", install_path.display());

        // Get git info
        let git_info = Self::get_git_info(&registry_path);
        let revision = git_info.as_ref()
            .and_then(|g| g.revision.clone())
            .unwrap_or_else(|| "unknown".to_string());

        // Try to get version from dependency's CCGO.toml if not specified
        let resolved_version = if version.is_empty() {
            Self::get_dep_version(&registry_path).unwrap_or_else(|| "0.0.0".to_string())
        } else {
            version.to_string()
        };

        Ok(LockedPackage {
            name: dep_name.to_string(),
            version: resolved_version,
            source: format!("git+{}#{}", git_url, revision),
            checksum: None,
            dependencies: vec![],
            git: Some(LockedGitInfo {
                revision,
                branch: branch.map(|s| s.to_string()),
                tag: None,
                dirty: git_info.and_then(|g| g.dirty).unwrap_or(false),
            }),
            installed_at: Some(chrono::Local::now().to_rfc3339()),
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
