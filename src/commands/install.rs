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
use crate::dependency::resolver::resolve_dependencies_with_strategy;
use crate::dependency::version_resolver::ConflictStrategy as VersionConflictStrategy;
use crate::lockfile::{Lockfile, LockedPackage, LockedGitInfo, LOCKFILE_NAME};
use crate::workspace::{find_workspace_root, Workspace};

/// Version conflict resolution strategy (CLI wrapper)
#[derive(Debug, Clone, Copy, Default, clap::ValueEnum)]
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

impl From<ConflictStrategy> for VersionConflictStrategy {
    fn from(s: ConflictStrategy) -> Self {
        match s {
            ConflictStrategy::First => VersionConflictStrategy::First,
            ConflictStrategy::Highest => VersionConflictStrategy::Highest,
            ConflictStrategy::Lowest => VersionConflictStrategy::Lowest,
            ConflictStrategy::Strict => VersionConflictStrategy::Strict,
        }
    }
}

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

    /// Version conflict resolution strategy
    #[arg(long, value_enum, default_value = "first")]
    pub conflict_strategy: ConflictStrategy,

    /// Install dependencies for all workspace members
    #[arg(long)]
    pub workspace: bool,

    /// Install dependencies for a specific package (in a workspace)
    #[arg(long, short = 'p')]
    pub package: Option<String>,
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
    pub fn execute(self, verbose: bool) -> Result<()> {
        let current_dir = std::env::current_dir().context("Failed to get current directory")?;

        // Check for workspace context
        if self.workspace || self.package.is_some() {
            return self.execute_workspace_install(&current_dir, verbose);
        }

        // Check if we're in a workspace root but --workspace not specified
        if Workspace::is_workspace(&current_dir) {
            eprintln!(
                "ℹ️  In workspace root. Use --workspace to install for all members, \
                 or --package <name> for a specific member."
            );
        }

        println!("{}", "=".repeat(80));
        println!("CCGO Install - Install Project Dependencies");
        println!("{}", "=".repeat(80));

        let project_dir = current_dir;
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

        // Resolve transitive dependencies with conflict strategy
        let strategy: VersionConflictStrategy = self.conflict_strategy.into();
        let dependency_graph = match resolve_dependencies_with_strategy(dependencies, &project_dir, &ccgo_home, strategy) {
            Ok(graph) => {
                // Show dependency tree
                println!("\nDependency tree:");
                println!("{}", graph.format_tree(2));

                // Show statistics
                let stats = graph.stats();
                println!(
                    "{} unique dependencies found, {} total ({} shared)",
                    stats.unique_count,
                    stats.total_count,
                    stats.shared_count
                );

                graph
            }
            Err(e) => {
                eprintln!("\n⚠️  Warning: Failed to resolve transitive dependencies: {}", e);
                eprintln!("   Continuing with direct dependencies only...");
                // Continue with just the direct dependencies
                crate::dependency::graph::DependencyGraph::new()
            }
        };

        // Determine installation order using topological sort
        let install_order = if dependency_graph.nodes().is_empty() {
            // No transitive dependencies, use direct order
            deps_to_install.iter().map(|d| d.name.clone()).collect()
        } else {
            match dependency_graph.topological_sort() {
                Ok(order) => {
                    println!("\n📦 Installing in dependency order:");
                    for (i, dep_name) in order.iter().enumerate() {
                        println!("  {}. {}", i + 1, dep_name);
                    }
                    order
                }
                Err(e) => {
                    eprintln!("\n⚠️  Warning: Failed to determine build order: {}", e);
                    eprintln!("   Installing in declaration order...");
                    deps_to_install.iter().map(|d| d.name.clone()).collect()
                }
            }
        };

        // Install each dependency
        println!("\n{}", "=".repeat(80));
        println!("Installing Dependencies");
        println!("{}", "=".repeat(80));

        let mut installed_count = 0;
        let mut failed_count = 0;
        let mut lockfile = existing_lockfile.unwrap_or_else(Lockfile::new);

        // Create a map for quick lookup of dependency configs
        let dep_map: std::collections::HashMap<String, &DependencyConfig> =
            dependencies.iter().map(|d| (d.name.clone(), d)).collect();

        // Get dependencies from the graph if available, otherwise use original list
        let deps_to_process: Vec<&DependencyConfig> = if !dependency_graph.nodes().is_empty() {
            // Install in topological order
            install_order
                .iter()
                .filter_map(|name| {
                    // Get from dependency graph first (includes transitive deps)
                    if let Some(node) = dependency_graph.get_node(name) {
                        Some(&node.config)
                    } else {
                        // Fall back to direct dependencies
                        dep_map.get(name).copied()
                    }
                })
                .collect()
        } else {
            // No graph, use original order
            deps_to_install
        };

        for dep in deps_to_process {
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

        // Check vendor/ directory first for offline builds
        let vendor_dir = project_dir.join("vendor");
        let vendor_path = vendor_dir.join(&dep.name);
        if vendor_path.exists() && !self.force {
            println!("   📦 Found in vendor/ directory (offline mode)");
            println!("   Source: {}", vendor_path.display());

            // Remove existing installation if present
            if install_path.exists() {
                if install_path.is_symlink() {
                    fs::remove_file(&install_path)?;
                } else {
                    fs::remove_dir_all(&install_path)?;
                }
            }

            // Create symlink or copy from vendor
            Self::create_symlink_or_copy(&vendor_path, &install_path, self.copy)?;
            println!("   ✓ Installed from vendor to {}", install_path.display());

            // Return a locked package entry for vendored dependency
            return Ok(LockedPackage {
                name: dep.name.clone(),
                version: dep.version.clone(),
                source: format!("vendor+{}", dep.name),
                checksum: None,
                dependencies: vec![],
                git: None,
                installed_at: Some(chrono::Local::now().to_rfc3339()),
                patch: None,
            });
        }

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
                patch: None,
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

        // Load config to check for patches
        let config = CcgoConfig::load().context("Failed to load CCGO.toml")?;

        // Check if there's a patch for this dependency
        let original_source = Self::build_source_string(dep);
        let patch_info = if let Some(patch) = config.patch.find_patch(&dep.name, Some(&original_source)) {
            println!("   🔧 Applying patch for {}...", dep.name);

            let patched_source = if let Some(ref git) = patch.git {
                format!("git+{}", git)
            } else if let Some(ref path) = patch.path {
                format!("path+{}", path)
            } else {
                original_source.clone()
            };

            println!("   Original: {}", original_source);
            println!("   Patched:  {}", patched_source);

            Some((patch, patched_source))
        } else {
            None
        };

        // Determine effective source for installation
        let (effective_path, effective_git, effective_branch, effective_rev) = if let Some((patch, _)) = &patch_info {
            // Use patched source
            let rev = if !self.locked {
                // In non-locked mode, use patch's rev if specified
                patch.rev.clone()
            } else {
                // In locked mode, prefer locked revision
                locked.and_then(|l| l.git_revision()).map(|s| s.to_string())
            };

            (
                patch.path.as_deref(),
                patch.git.as_deref(),
                patch.branch.as_deref().or(patch.tag.as_deref()),
                rev,
            )
        } else {
            // Use original source
            let locked_rev = locked.and_then(|l| l.git_revision()).map(|s| s.to_string());
            (
                dep.path.as_deref(),
                dep.git.as_deref(),
                dep.branch.as_deref(),
                locked_rev,
            )
        };

        // Install based on effective source type
        let mut locked_pkg = if let Some(path) = effective_path {
            // Local path dependency
            self.install_from_local_path(&dep.name, &dep.version, path, project_dir, &install_path)?
        } else if let Some(git_url) = effective_git {
            // Git dependency
            self.install_from_git(&dep.name, &dep.version, git_url, effective_branch, effective_rev.as_deref(), &install_path, ccgo_home)?
        } else if let Some(ref zip_url) = dep.zip {
            // Archive dependency (zip or tar.gz, https:// URL or local path)
            self.install_from_archive(&dep.name, &dep.version, zip_url, project_dir, &install_path)?
        } else {
            bail!("No valid source found for dependency '{}'", dep.name);
        };

        // Add patch information to locked package if patched
        if let Some((_, patched_source)) = patch_info {
            locked_pkg.patch = Some(crate::lockfile::PatchInfo {
                patched_source: original_source,
                replacement_source: patched_source.clone(),
                is_path_patch: effective_path.is_some(),
            });
        }

        Ok(locked_pkg)
    }

    /// Build source string from dependency config
    fn build_source_string(dep: &DependencyConfig) -> String {
        if let Some(ref git) = dep.git {
            format!("git+{}", git)
        } else if let Some(ref path) = dep.path {
            format!("path+{}", path)
        } else if let Some(ref zip) = dep.zip {
            format!("zip+{}", zip)
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
            patch: None,
        })
    }

    /// Install from a ZIP archive (https:// URL or local path)
    /// Returns true if the source URL/path refers to a tar.gz archive
    fn is_tar_gz(source: &str) -> bool {
        source.ends_with(".tar.gz") || source.ends_with(".tgz")
    }

    fn install_from_archive(
        &self,
        dep_name: &str,
        version: &str,
        zip_source: &str,
        project_dir: &Path,
        install_path: &Path,
    ) -> Result<LockedPackage> {
        println!("   Source: {}", zip_source);

        let is_remote = zip_source.starts_with("https://") || zip_source.starts_with("http://");
        let fmt = if Self::is_tar_gz(zip_source) { "tar.gz" } else { "zip" };

        let bytes = if is_remote {
            println!("   Downloading {} archive...", fmt);
            Self::download_zip(zip_source)?
        } else {
            let local_path = if Path::new(zip_source).is_absolute() {
                PathBuf::from(zip_source)
            } else {
                project_dir.join(zip_source)
            };
            if !local_path.exists() {
                anyhow::bail!("Archive file not found: {}", local_path.display());
            }
            println!("   Reading local {}: {}", fmt, local_path.display());
            fs::read(&local_path).context("Failed to read archive file")?
        };

        println!("   Extracting to {}...", install_path.display());
        fs::create_dir_all(install_path).context("Failed to create install directory")?;
        if Self::is_tar_gz(zip_source) {
            Self::extract_tar_gz(&bytes, install_path)?;
        } else {
            Self::extract_zip(&bytes, install_path)?;
        }

        println!("   ✓ Installed to {}", install_path.display());

        Ok(LockedPackage {
            name: dep_name.to_string(),
            version: version.to_string(),
            source: format!("zip+{}", zip_source),
            checksum: None,
            dependencies: vec![],
            git: None,
            installed_at: Some(chrono::Local::now().to_rfc3339()),
            patch: None,
        })
    }

    /// Download a ZIP from a URL, returning its bytes
    fn download_zip(url: &str) -> Result<Vec<u8>> {
        let response = reqwest::blocking::get(url)
            .with_context(|| format!("Failed to download ZIP from {}", url))?;
        if !response.status().is_success() {
            anyhow::bail!("HTTP {} downloading ZIP from {}", response.status(), url);
        }
        let bytes = response.bytes().context("Failed to read download response")?;
        Ok(bytes.to_vec())
    }

    /// Extract a ZIP archive (bytes) to the target directory, preserving paths
    fn extract_zip(zip_bytes: &[u8], target_dir: &Path) -> Result<()> {
        use std::io::Cursor;
        let cursor = Cursor::new(zip_bytes);
        let mut archive = zip::ZipArchive::new(cursor).context("Failed to open ZIP archive")?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let entry_name = file.name().to_string();
            // Reject absolute paths and entries containing ".." components
            let relative = std::path::Path::new(&entry_name);
            if relative.is_absolute()
                || relative
                    .components()
                    .any(|c| c == std::path::Component::ParentDir)
            {
                anyhow::bail!("ZIP entry contains unsafe path: {}", entry_name);
            }
            let out_path = target_dir.join(relative);

            if file.name().ends_with('/') {
                fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                let mut out_file = fs::File::create(&out_path)
                    .with_context(|| format!("Failed to create file: {}", out_path.display()))?;
                std::io::copy(&mut file, &mut out_file)?;
            }
        }
        Ok(())
    }

    /// Extract a tar.gz archive (bytes) to the target directory, preserving paths
    fn extract_tar_gz(bytes: &[u8], target_dir: &Path) -> Result<()> {
        use flate2::read::GzDecoder;
        use tar::Archive;

        let decoder = GzDecoder::new(bytes);
        let mut archive = Archive::new(decoder);

        for entry in archive.entries().context("Failed to read tar.gz entries")? {
            let mut entry = entry.context("Failed to read tar.gz entry")?;
            let entry_path = entry.path().context("Failed to get tar.gz entry path")?;

            // Reject absolute paths and entries containing ".." components
            if entry_path.is_absolute()
                || entry_path
                    .components()
                    .any(|c| c == std::path::Component::ParentDir)
            {
                anyhow::bail!(
                    "tar.gz entry contains unsafe path: {}",
                    entry_path.display()
                );
            }

            let out_path = target_dir.join(&entry_path);
            if entry.header().entry_type().is_dir() {
                fs::create_dir_all(&out_path)?;
            } else {
                if let Some(parent) = out_path.parent() {
                    fs::create_dir_all(parent)?;
                }
                entry
                    .unpack(&out_path)
                    .with_context(|| format!("Failed to unpack: {}", out_path.display()))?;
            }
        }
        Ok(())
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
    #[allow(clippy::too_many_arguments)]
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
            patch: None,
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
            .args(["rev-parse", "--is-inside-work-tree"])
            .current_dir(path)
            .output();

        if check.is_err() || !check.unwrap().status.success() {
            return None;
        }

        // Get revision
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .current_dir(path)
            .output()
        {
            if output.status.success() {
                git_info.revision = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
        }

        // Get branch
        if let Ok(output) = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .current_dir(path)
            .output()
        {
            if output.status.success() {
                git_info.branch = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
        }

        // Get remote URL
        if let Ok(output) = std::process::Command::new("git")
            .args(["config", "--get", "remote.origin.url"])
            .current_dir(path)
            .output()
        {
            if output.status.success() {
                git_info.remote_url = Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
            }
        }

        // Check if dirty
        if let Ok(output) = std::process::Command::new("git")
            .args(["status", "--porcelain"])
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

    /// Execute install for workspace members
    fn execute_workspace_install(&self, current_dir: &Path, verbose: bool) -> Result<()> {
        // Find workspace root
        let workspace_root = find_workspace_root(current_dir)?
            .ok_or_else(|| anyhow::anyhow!(
                "Not in a workspace. Use --workspace or --package only within a workspace."
            ))?;

        // Load workspace
        let workspace = Workspace::load(&workspace_root)?;

        if verbose {
            workspace.print_summary();
        }

        // Determine which members to install for
        let members_to_install = if let Some(ref package_name) = self.package {
            // Install for specific package
            let member = workspace.get_member(package_name)
                .ok_or_else(|| anyhow::anyhow!(
                    "Package '{}' not found in workspace. Available: {}",
                    package_name,
                    workspace.members.names().join(", ")
                ))?;
            vec![member]
        } else {
            // Install for default members (or all if no default_members specified)
            workspace.default_members()
        };

        if members_to_install.is_empty() {
            bail!("No workspace members to install for");
        }

        println!("{}", "=".repeat(80));
        println!("CCGO Workspace Install - Installing dependencies for {} member(s)", members_to_install.len());
        println!("{}", "=".repeat(80));

        let ccgo_home = Self::get_ccgo_home();

        // Clean global cache if requested (once for all members)
        if self.clean_cache && ccgo_home.exists() {
            println!("\n🗑  Cleaning global cache: {}", ccgo_home.display());
            fs::remove_dir_all(&ccgo_home).context("Failed to clean cache")?;
        }

        let mut success_count = 0;
        let mut failed_members: Vec<String> = Vec::new();

        for member in members_to_install {
            println!("\n📦 Installing dependencies for {} ({})...", member.name, member.version);
            println!("{}", "-".repeat(60));

            // Execute install in member's directory
            let member_path = workspace_root.join(&member.name);

            match self.install_for_member(&member_path, &ccgo_home, verbose) {
                Ok(count) => {
                    success_count += count;
                    println!("   ✓ Installed {} dependencies for {}", count, member.name);
                }
                Err(e) => {
                    eprintln!("   ✗ Failed to install for {}: {}", member.name, e);
                    failed_members.push(member.name.clone());
                }
            }
        }

        // Print summary
        println!("\n{}", "=".repeat(80));
        println!("Workspace Install Summary");
        println!("{}", "=".repeat(80));

        println!("\n✓ Total dependencies installed: {}", success_count);

        if !failed_members.is_empty() {
            println!("\n✗ Failed members: {}", failed_members.len());
            for name in &failed_members {
                println!("  - {}", name);
            }
            bail!("{} workspace member(s) failed to install", failed_members.len());
        }

        Ok(())
    }

    /// Install dependencies for a single workspace member
    fn install_for_member(
        &self,
        member_path: &Path,
        ccgo_home: &Path,
        _verbose: bool,
    ) -> Result<usize> {
        // Load member's CCGO.toml
        let config_path = member_path.join("CCGO.toml");
        if !config_path.exists() {
            bail!("CCGO.toml not found in {}", member_path.display());
        }

        let config = CcgoConfig::load_from(&config_path)?;
        let dependencies = &config.dependencies;

        if dependencies.is_empty() {
            return Ok(0);
        }

        // Load existing lockfile
        let existing_lockfile = Lockfile::load(member_path)?;

        // In locked mode, lockfile is required
        if self.locked && existing_lockfile.is_none() {
            bail!(
                "No {} found for {}. Run 'ccgo install' first.",
                LOCKFILE_NAME,
                member_path.display()
            );
        }

        let deps_dir = member_path.join(".ccgo").join("deps");
        fs::create_dir_all(&deps_dir).context("Failed to create .ccgo/deps directory")?;

        let mut lockfile = existing_lockfile.unwrap_or_else(Lockfile::new);
        let mut installed_count = 0;

        // Resolve transitive dependencies
        let strategy: VersionConflictStrategy = self.conflict_strategy.into();
        let dependency_graph = match resolve_dependencies_with_strategy(dependencies, member_path, ccgo_home, strategy) {
            Ok(graph) => graph,
            Err(e) => {
                eprintln!("   ⚠️  Warning: Failed to resolve transitive dependencies: {}", e);
                crate::dependency::graph::DependencyGraph::new()
            }
        };

        // Determine installation order
        let install_order = if dependency_graph.nodes().is_empty() {
            dependencies.iter().map(|d| d.name.clone()).collect()
        } else {
            dependency_graph.topological_sort().unwrap_or_else(|_| {
                dependencies.iter().map(|d| d.name.clone()).collect()
            })
        };

        // Create a map for quick lookup
        let dep_map: std::collections::HashMap<String, &DependencyConfig> =
            dependencies.iter().map(|d| (d.name.clone(), d)).collect();

        // Get dependencies to process
        let deps_to_process: Vec<&DependencyConfig> = if !dependency_graph.nodes().is_empty() {
            install_order
                .iter()
                .filter_map(|name| {
                    if let Some(node) = dependency_graph.get_node(name) {
                        Some(&node.config)
                    } else {
                        dep_map.get(name).copied()
                    }
                })
                .collect()
        } else {
            dependencies.iter().collect()
        };

        for dep in deps_to_process {
            let locked_pkg = lockfile.get_package(&dep.name).cloned();
            let _install_path = deps_dir.join(&dep.name);

            match self.install_dependency(dep, member_path, ccgo_home, locked_pkg.as_ref()) {
                Ok(locked_package) => {
                    installed_count += 1;
                    lockfile.upsert_package(locked_package);
                }
                Err(e) => {
                    eprintln!("   ⚠️  Failed to install {}: {}", dep.name, e);
                }
            }
        }

        // Save lockfile
        if installed_count > 0 {
            lockfile.touch();
            lockfile.save(member_path)?;
            Self::update_gitignore(member_path)?;
        }

        Ok(installed_count)
    }

    /// Get CCGO home directory
    fn get_ccgo_home() -> PathBuf {
        directories::BaseDirs::new()
            .map(|dirs| dirs.home_dir().to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".ccgo")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_extract_zip_creates_files() {
        use zip::write::SimpleFileOptions;

        let tmp_dir = tempfile::tempdir().unwrap();
        let extract_dir = tmp_dir.path().join("extracted");

        // Build an in-memory ZIP with two entries
        let mut buf: Vec<u8> = Vec::new();
        {
            let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = SimpleFileOptions::default();
            w.start_file("include/mylib/mylib.h", opts).unwrap();
            w.write_all(b"// header").unwrap();
            w.start_file("CCGO.toml", opts).unwrap();
            w.write_all(b"[package]\nname = \"mylib\"\nversion = \"1.0.0\"\n").unwrap();
            w.finish().unwrap();
        }

        InstallCommand::extract_zip(&buf, &extract_dir).unwrap();

        assert!(extract_dir.join("include/mylib/mylib.h").exists());
        assert!(extract_dir.join("CCGO.toml").exists());
    }

    #[test]
    fn test_extract_zip_missing_zip_returns_error() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let extract_dir = tmp_dir.path().join("extracted");
        let result = InstallCommand::extract_zip(b"not a zip", &extract_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_tar_gz() {
        assert!(InstallCommand::is_tar_gz("foo.tar.gz"));
        assert!(InstallCommand::is_tar_gz("foo.tgz"));
        assert!(InstallCommand::is_tar_gz("https://cdn.example.com/sdk-1.0.0.tar.gz"));
        assert!(!InstallCommand::is_tar_gz("foo.zip"));
        assert!(!InstallCommand::is_tar_gz("foo.tar"));
    }

    #[test]
    fn test_extract_tar_gz_creates_files() {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let tmp_dir = tempfile::tempdir().unwrap();
        let extract_dir = tmp_dir.path().join("extracted");

        // Build an in-memory tar.gz with two entries
        let mut buf: Vec<u8> = Vec::new();
        {
            let enc = GzEncoder::new(&mut buf, Compression::default());
            let mut tar = tar::Builder::new(enc);

            let header_content = b"// header";
            let mut header = tar::Header::new_gnu();
            header.set_size(header_content.len() as u64);
            header.set_mode(0o644);
            header.set_cksum();
            tar.append_data(&mut header, "include/mylib/mylib.h", header_content.as_ref())
                .unwrap();

            let toml_content = b"[package]\nname = \"mylib\"\nversion = \"1.0.0\"\n";
            let mut header2 = tar::Header::new_gnu();
            header2.set_size(toml_content.len() as u64);
            header2.set_mode(0o644);
            header2.set_cksum();
            tar.append_data(&mut header2, "CCGO.toml", toml_content.as_ref())
                .unwrap();

            tar.finish().unwrap();
        }

        InstallCommand::extract_tar_gz(&buf, &extract_dir).unwrap();

        assert!(extract_dir.join("include/mylib/mylib.h").exists());
        assert!(extract_dir.join("CCGO.toml").exists());
    }

    #[test]
    fn test_extract_tar_gz_rejects_path_traversal() {
        use flate2::write::GzEncoder;
        use flate2::Compression;

        let tmp_dir = tempfile::tempdir().unwrap();
        let extract_dir = tmp_dir.path().join("extracted");

        let mut buf: Vec<u8> = Vec::new();
        {
            let enc = GzEncoder::new(&mut buf, Compression::default());
            let mut tar = tar::Builder::new(enc);
            let content = b"evil";
            // Use append() with manually constructed header to bypass tar crate's
            // own path validation, so we can test our extract_tar_gz guard.
            let mut header = tar::Header::new_gnu();
            let gnu = header.as_gnu_mut().unwrap();
            let path = b"../../escape.txt";
            gnu.name[..path.len()].copy_from_slice(path);
            header.set_size(content.len() as u64);
            header.set_mode(0o644);
            header.set_entry_type(tar::EntryType::Regular);
            header.set_cksum();
            tar.append(&header, std::io::Cursor::new(content)).unwrap();
            tar.finish().unwrap();
        }

        let result = InstallCommand::extract_tar_gz(&buf, &extract_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unsafe path"));
    }

    #[test]
    fn test_extract_zip_rejects_path_traversal() {
        use zip::write::SimpleFileOptions;

        let tmp_dir = tempfile::tempdir().unwrap();
        let extract_dir = tmp_dir.path().join("extracted");

        let mut buf: Vec<u8> = Vec::new();
        {
            let mut w = zip::ZipWriter::new(std::io::Cursor::new(&mut buf));
            let opts = SimpleFileOptions::default();
            w.start_file("../../escape.txt", opts).unwrap();
            w.write_all(b"should not be created").unwrap();
            w.finish().unwrap();
        }

        let result = InstallCommand::extract_zip(&buf, &extract_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unsafe path"));
    }
}
