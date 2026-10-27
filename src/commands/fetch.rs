//! Install command implementation - Pure Rust version
//!
//! Manages project dependencies from CCGO.toml.
//! Dependencies are cached globally in ~/.ccgo/ and linked to project's .ccgo/deps/.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use clap::Args;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::config::{CcgoConfig, DependencyConfig};
use crate::dependency::graph::DependencyGraph;
use crate::dependency::resolver::resolve_dependencies_with_strategy;
use crate::dependency::version_resolver::ConflictStrategy as VersionConflictStrategy;
use crate::lockfile::{LockedGitInfo, LockedPackage, Lockfile, LOCKFILE_NAME};
use crate::registry::RegistrySession;
use crate::workspace::{find_workspace_root, Workspace};

#[path = "fetch_registry_locked.rs"]
mod registry_locked;

#[path = "fetch_install.rs"]
mod installation;

#[path = "fetch_pending.rs"]
mod pending;

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
pub struct FetchCommand {
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

struct EffectiveSource<'a> {
    path: Option<&'a str>,
    git: Option<&'a str>,
    branch: Option<&'a str>,
    revision: Option<String>,
}

struct FetchContext<'a> {
    project_dir: &'a Path,
    ccgo_home: &'a Path,
    config: &'a CcgoConfig,
    registries: &'a mut RegistrySession,
    completed: Vec<String>,
}

impl FetchCommand {
    /// Execute the install command
    pub fn execute(self, verbose: bool) -> Result<()> {
        let current_dir = std::env::current_dir().context("Failed to get current directory")?;

        // Check for workspace context
        if self.workspace || self.package.is_some() {
            return self.execute_workspace_install(&current_dir, verbose);
        }

        Self::print_workspace_notice(&current_dir);

        println!("{}", "=".repeat(80));
        println!("CCGO Install - Install Project Dependencies");
        println!("{}", "=".repeat(80));

        let project_dir = current_dir;
        let ccgo_home = Self::get_ccgo_home();

        println!("\nProject directory: {}", project_dir.display());
        println!("Global CCGO home: {}", ccgo_home.display());

        self.clean_global_cache(&ccgo_home)?;
        let _home_guard = crate::registry::cache_access::home_lock(&ccgo_home, false)?;

        let _project_guard = installation::lock_project(&project_dir)?;

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
                "No {} found. Run 'ccgo fetch' first to generate a lockfile, \
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

        self.validate_locked_dependencies(existing_lockfile.as_ref(), dependencies)?;

        let deps_to_install = self.selected_dependencies(dependencies);

        if deps_to_install.is_empty() {
            if let Some(ref dep_name) = self.dependency {
                println!("   ⚠️  Dependency '{}' not found in CCGO.toml", dep_name);
            } else {
                println!("   ⚠️  No dependencies to install");
            }
            return Ok(());
        }

        Self::print_dependencies(&deps_to_install, existing_lockfile.as_ref());

        let dependency_graph = self.resolve_install_graph(dependencies, &project_dir, &ccgo_home);

        let install_order = Self::installation_order(&dependency_graph, &deps_to_install);

        // Install each dependency
        println!("\n{}", "=".repeat(80));
        println!("Installing Dependencies");
        println!("{}", "=".repeat(80));

        let mut lockfile = existing_lockfile.unwrap_or_else(Lockfile::new);
        let packages_before = lockfile.packages.clone();

        let deps_to_process = Self::ordered_dependencies(
            &dependency_graph,
            dependencies,
            deps_to_install,
            &install_order,
        );

        let mut registries = RegistrySession::default();
        let context = &mut FetchContext {
            project_dir: &project_dir,
            ccgo_home: &ccgo_home,
            config: &config,
            registries: &mut registries,
            completed: Vec::new(),
        };
        let (installed_count, failed_count) =
            self.install_batch(&deps_to_process, &mut lockfile, context);

        self.save_fetch_lockfile(context, &mut lockfile, &packages_before, installed_count)?;

        Self::print_install_summary(installed_count, failed_count)
    }

    fn print_workspace_notice(current_dir: &Path) {
        // Check if we're in a workspace root but --workspace not specified
        if Workspace::is_workspace(current_dir) {
            eprintln!(
                "ℹ️  In workspace root. Use --workspace to install for all members, \
                 or --package <name> for a specific member."
            );
        }
    }

    fn clean_global_cache(&self, ccgo_home: &Path) -> Result<()> {
        if self.clean_cache && ccgo_home.exists() {
            println!("\n🗑  Cleaning global cache: {}", ccgo_home.display());
            crate::registry::cache_access::clean_home(ccgo_home)?;
        }
        Ok(())
    }

    fn selected_dependencies<'a>(
        &self,
        dependencies: &'a [DependencyConfig],
    ) -> Vec<&'a DependencyConfig> {
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

        deps_to_install
    }

    fn print_dependencies(
        deps_to_install: &[&DependencyConfig],
        existing_lockfile: Option<&Lockfile>,
    ) {
        println!(
            "\nFound {} dependency(ies) to install:",
            deps_to_install.len()
        );
        for dep in deps_to_install {
            // Show locked version if available
            if let Some(lockfile) = existing_lockfile {
                if let Some(locked) = lockfile.get_package(&dep.name) {
                    println!("  - {} (locked: {})", dep.name, locked.version);
                    continue;
                }
            }
            println!("  - {}", dep.name);
        }
    }

    fn resolve_install_graph(
        &self,
        dependencies: &[DependencyConfig],
        project_dir: &Path,
        ccgo_home: &Path,
    ) -> DependencyGraph {
        // Resolve transitive dependencies with conflict strategy
        let strategy: VersionConflictStrategy = self.conflict_strategy.into();
        let dependency_graph = match resolve_dependencies_with_strategy(
            dependencies,
            project_dir,
            ccgo_home,
            strategy,
        ) {
            Ok(graph) => {
                // Show dependency tree
                println!("\nDependency tree:");
                println!("{}", graph.format_tree(2));

                // Show statistics
                let stats = graph.stats();
                println!(
                    "{} unique dependencies found, {} total ({} shared)",
                    stats.unique_count, stats.total_count, stats.shared_count
                );

                graph
            }
            Err(e) => {
                eprintln!(
                    "\n⚠️  Warning: Failed to resolve transitive dependencies: {}",
                    e
                );
                eprintln!("   Continuing with direct dependencies only...");
                // Continue with just the direct dependencies
                crate::dependency::graph::DependencyGraph::new()
            }
        };

        dependency_graph
    }

    fn installation_order(
        dependency_graph: &DependencyGraph,
        deps_to_install: &[&DependencyConfig],
    ) -> Vec<String> {
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

        install_order
    }

    fn ordered_dependencies<'a>(
        dependency_graph: &'a DependencyGraph,
        dependencies: &'a [DependencyConfig],
        deps_to_install: Vec<&'a DependencyConfig>,
        install_order: &[String],
    ) -> Vec<&'a DependencyConfig> {
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

        deps_to_process
    }

    fn validate_locked_dependencies(
        &self,
        existing_lockfile: Option<&Lockfile>,
        dependencies: &[DependencyConfig],
    ) -> Result<()> {
        // Check for outdated dependencies in locked mode
        if self.locked {
            if let Some(lockfile) = existing_lockfile {
                let outdated = lockfile.check_outdated(dependencies);
                if !outdated.is_empty() {
                    bail!(
                        "Dependencies have changed since lockfile was generated.\n\
                         Changed dependencies: {}\n\
                         Run 'ccgo fetch' to update the lockfile, or remove --locked flag.",
                        outdated.join(", ")
                    );
                }
            }
        }

        Ok(())
    }

    /// Install a single dependency
    fn install_dependency(
        &self,
        dep: &DependencyConfig,
        context: &mut FetchContext<'_>,
        locked: Option<&LockedPackage>,
    ) -> Result<LockedPackage> {
        let project_dir = context.project_dir;
        let ccgo_home = context.ccgo_home;
        let config = context.config;
        let registries = &mut *context.registries;
        println!("\n📦 Installing {}...", dep.name);

        let deps_dir = project_dir.join(".ccgo").join("deps");
        fs::create_dir_all(&deps_dir).context("Failed to create .ccgo/deps directory")?;

        let install_path = deps_dir.join(&dep.name);

        if self.locked {
            let locked = locked.ok_or_else(|| {
                anyhow::anyhow!("Dependency '{}' is missing from CCGO.lock", dep.name)
            })?;
            if dep.git.is_none()
                && dep.path.is_none()
                && dep.zip.is_none()
                && locked.source.starts_with("registry+")
            {
                pending::begin(project_dir, &dep.name)?;
                return self.install_locked_registry(
                    dep,
                    config,
                    locked,
                    &install_path,
                    ccgo_home,
                    registries,
                );
            }
        }

        if let Some(installed) =
            self.reuse_installed_dependency(dep, project_dir, &install_path, locked)?
        {
            return Ok(installed);
        }

        pending::begin(project_dir, &dep.name)?;
        self.install_patched_dependency(dep, context, locked, &install_path)
    }

    fn install_batch(
        &self,
        deps_to_process: &[&DependencyConfig],
        lockfile: &mut Lockfile,
        context: &mut FetchContext<'_>,
    ) -> (usize, usize) {
        let mut installed_count = 0;
        let mut failed_count = 0;
        for dep in deps_to_process {
            // Get locked info if available
            let locked_pkg = lockfile.get_package(&dep.name).cloned();

            match self.install_dependency(dep, context, locked_pkg.as_ref()) {
                Ok(locked_package) => {
                    context.completed.push(dep.name.clone());
                    installed_count += 1;
                    lockfile.upsert_package(locked_package);
                }
                Err(e) => {
                    eprintln!("   ✗ Failed to install {}: {}", dep.name, e);
                    failed_count += 1;
                }
            }
        }

        (installed_count, failed_count)
    }

    fn save_fetch_lockfile(
        &self,
        context: &FetchContext<'_>,
        lockfile: &mut Lockfile,
        packages_before: &[LockedPackage],
        installed_count: usize,
    ) -> Result<()> {
        // Save lockfile only when the packages actually changed, to avoid
        // spurious timestamp-only diffs when all deps were already up-to-date.
        if !self.locked && installed_count > 0 && lockfile.packages != packages_before {
            lockfile.touch();
            lockfile.save(context.project_dir)?;
            println!("\n📝 Updated {}", LOCKFILE_NAME);
            crate::utils::ide::update_ide_ignores(
                context.project_dir,
                context.config.build_dir_name(),
            )?;
        }

        pending::complete(context.project_dir, &context.completed)?;
        Ok(())
    }

    fn print_install_summary(installed_count: usize, failed_count: usize) -> Result<()> {
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

    fn reuse_installed_dependency(
        &self,
        dep: &DependencyConfig,
        project_dir: &Path,
        install_path: &Path,
        locked: Option<&LockedPackage>,
    ) -> Result<Option<LockedPackage>> {
        // Check vendor/ directory first for offline builds
        let vendor_dir = project_dir.join("vendor");
        let vendor_path = vendor_dir.join(&dep.name);
        if vendor_path.exists() && !self.force {
            println!("   📦 Found in vendor/ directory (offline mode)");
            println!("   Source: {}", vendor_path.display());

            pending::begin(project_dir, &dep.name)?;
            // Create symlink or copy from vendor
            Self::link_validated_checkout(&vendor_path, install_path, self.copy)?;
            println!("   ✓ Installed from vendor to {}", install_path.display());

            // Return a locked package entry for vendored dependency
            return Ok(Some(LockedPackage {
                name: dep.name.clone(),
                version: dep.version.clone(),
                source: format!("vendor+{}", dep.name),
                checksum: None,
                dependencies: vec![],
                git: None,
                installed_at: Some(chrono::Local::now().to_rfc3339()),
                patch: None,
            }));
        }

        // Check if already installed and matches lockfile
        if install_path.exists() && !self.force && !pending::exists(project_dir, &dep.name)? {
            if let Some(locked_pkg) = locked {
                println!(
                    "   {} already installed (locked: {})",
                    dep.name, locked_pkg.version
                );
                return Ok(Some(locked_pkg.clone()));
            }
            println!(
                "   {} already installed (use --force to reinstall)",
                dep.name
            );
            // This tree has no lock record yet, so the entry below cannot carry a
            // content checksum. Mark it pending until that entry is persisted;
            // otherwise a failed save would leave an unvalidated tree reusable.
            pending::begin(project_dir, &dep.name)?;
            // Return a basic locked package for already installed deps
            return Ok(Some(LockedPackage {
                name: dep.name.clone(),
                version: dep.version.clone(),
                source: Self::build_source_string(dep),
                checksum: None,
                dependencies: vec![],
                git: None,
                installed_at: Some(chrono::Local::now().to_rfc3339()),
                patch: None,
            }));
        }

        Ok(None)
    }

    fn install_patched_dependency(
        &self,
        dep: &DependencyConfig,
        context: &mut FetchContext<'_>,
        locked: Option<&LockedPackage>,
        install_path: &Path,
    ) -> Result<LockedPackage> {
        let config = context.config;
        // Check if there's a patch for this dependency
        let original_source = Self::build_source_string(dep);
        let patch_info =
            if let Some(patch) = config.patch.find_patch(&dep.name, Some(&original_source)) {
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

        let source =
            self.effective_source(dep, patch_info.as_ref().map(|(patch, _)| *patch), locked);
        let is_path_patch = source.path.is_some();

        let mut locked_pkg = self.install_effective_source(dep, context, &source, install_path)?;

        // Add patch information to locked package if patched
        if let Some((_, patched_source)) = patch_info {
            locked_pkg.patch = Some(crate::lockfile::PatchInfo {
                patched_source: original_source,
                replacement_source: patched_source.clone(),
                is_path_patch,
            });
        }

        Ok(locked_pkg)
    }

    fn effective_source<'a>(
        &self,
        dep: &'a DependencyConfig,
        patch: Option<&'a crate::config::PatchDependency>,
        locked: Option<&LockedPackage>,
    ) -> EffectiveSource<'a> {
        let locked_revision = locked
            .and_then(|package| package.git_revision())
            .map(str::to_owned);
        match patch {
            Some(patch) => EffectiveSource {
                path: patch.path.as_deref(),
                git: patch.git.as_deref(),
                branch: patch.branch.as_deref().or(patch.tag.as_deref()),
                revision: if self.locked {
                    locked_revision
                } else {
                    patch.rev.clone()
                },
            },
            None => EffectiveSource {
                path: dep.path.as_deref(),
                git: dep.git.as_deref(),
                branch: dep.branch.as_deref(),
                revision: locked_revision,
            },
        }
    }

    fn install_effective_source(
        &self,
        dep: &DependencyConfig,
        context: &mut FetchContext<'_>,
        source: &EffectiveSource<'_>,
        install_path: &Path,
    ) -> Result<LockedPackage> {
        let project_dir = context.project_dir;
        let ccgo_home = context.ccgo_home;
        let config = context.config;
        let registries = &mut *context.registries;
        // Install based on effective source type
        if let Some(path) = source.path {
            // Local path dependency
            self.install_from_local_path(&dep.name, &dep.version, path, project_dir, install_path)
        } else if let Some(git_url) = source.git {
            // Git dependency
            self.install_from_git(
                &dep.name,
                &dep.version,
                git_url,
                source.branch,
                source.revision.as_deref(),
                install_path,
                ccgo_home,
            )
        } else if let Some(ref zip_url) = dep.zip {
            // Archive dependency (zip or tar.gz, https:// URL or local path)
            self.install_from_archive(&dep.name, &dep.version, zip_url, project_dir, install_path)
        } else if let Some(resolved) = self.try_resolve_registry(dep, config, registries)? {
            // Resolved through a [registries] index — download + extract the
            // published archive (or fall back to git+tag clone when the index
            // entry has no archive_url). Preferred over the local-cache
            // fallback below.
            self.install_from_registry(dep, &resolved, install_path, ccgo_home)
        } else if !dep.version.is_empty() {
            // Name + version only. Resolve from the global packages cache
            // populated by `ccgo install` in the source project. This mirrors
            // the Cargo/Maven workflow where the dependency is identified by
            // package name rather than a location.
            self.install_from_local_packages(&dep.name, &dep.version, ccgo_home, install_path)
        } else {
            bail!(
                "Dependency '{}' has no source (git, path, zip, or version). \
                 Specify a source in CCGO.toml or run `ccgo install` \
                 in the source project to populate {}.",
                dep.name,
                ccgo_home.join("packages").join(&dep.name).display()
            );
        }
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
        Self::link_validated_checkout(&source_path, install_path, self.copy)?;

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

    /// Install from the global CCGO packages cache.
    ///
    /// The source project previously ran `ccgo install` to populate
    /// `$CCGO_HOME/packages/<name>/<version>/`. We link that directory into
    /// the current project's `.ccgo/deps/<name>/`. Equivalent to consuming
    /// a `mvn install`-produced artifact from `~/.m2/`.
    fn install_from_local_packages(
        &self,
        dep_name: &str,
        version: &str,
        ccgo_home: &Path,
        install_path: &Path,
    ) -> Result<LockedPackage> {
        let source_path = ccgo_home
            .join("packages")
            .join(dep_name.to_lowercase())
            .join(version);

        if !source_path.exists() {
            bail!(
                "Dependency '{}' v{} not found in local packages cache.\n\
                 Expected at: {}\n\
                 In the source project, run: ccgo install",
                dep_name,
                version,
                source_path.display()
            );
        }

        println!(
            "   Source: {} (local packages cache)",
            source_path.display()
        );
        println!("   Linking from packages cache…");

        Self::link_validated_checkout(&source_path, install_path, self.copy)?;

        println!("   ✓ Installed to {}", install_path.display());

        Ok(LockedPackage {
            name: dep_name.to_string(),
            version: version.to_string(),
            source: format!("packages+{}@{}", dep_name, version),
            checksum: None,
            dependencies: vec![],
            git: None,
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
        let fmt = if Self::is_tar_gz(zip_source) {
            "tar.gz"
        } else {
            "zip"
        };

        let bytes = if is_remote {
            println!("   Downloading {} archive...", fmt);
            Self::download_zip(zip_source)?
        } else {
            // Strip optional file:// scheme, then resolve absolute vs relative.
            let path_str = zip_source.strip_prefix("file://").unwrap_or(zip_source);
            let local_path = if Path::new(path_str).is_absolute() {
                PathBuf::from(path_str)
            } else {
                project_dir.join(path_str)
            };
            if !local_path.exists() {
                anyhow::bail!("Archive file not found: {}", local_path.display());
            }
            println!("   Reading local {}: {}", fmt, local_path.display());
            fs::read(&local_path).context("Failed to read archive file")?
        };

        println!("   Extracting to {}...", install_path.display());
        installation::replace(install_path, |candidate| {
            fs::create_dir_all(candidate)?;
            if Self::is_tar_gz(zip_source) {
                Self::extract_tar_gz(&bytes, candidate)
            } else {
                Self::extract_zip(&bytes, candidate)
            }
        })?;

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

    /// Try to resolve a `version`-only dependency through the project's
    /// `[registries]`. Returns `Ok(None)` when no registries are declared
    /// (preserving legacy behavior for projects that don't use the index),
    /// or when no declared registry has a non-yanked exact-version match.
    ///
    /// When `dep.registry` is set, lookup is restricted to that single named
    /// registry. Otherwise ambiguous matches are rejected instead of depending
    /// on HashMap iteration order. Each cache is synced once per fetch session;
    /// all resolutions in that session read the captured commit.
    fn try_resolve_registry(
        &self,
        dep: &DependencyConfig,
        config: &CcgoConfig,
        registries: &mut RegistrySession,
    ) -> Result<Option<crate::registry::ResolvedRegistryDep>> {
        use crate::registry::{resolve::resolve_entry, RegistryCache};

        if config.registries.is_empty() {
            return Ok(None);
        }

        // Build (name, cache) candidates. If dep.registry is set, restrict to
        // that one; otherwise inspect every declared registry.
        let mut candidates: Vec<(String, RegistryCache)> = if let Some(name) = &dep.registry {
            let url = config.registries.get(name).ok_or_else(|| {
                anyhow::anyhow!(
                    "dependency '{}' references unknown registry '{}' (not in [registries])",
                    dep.name,
                    name
                )
            })?;
            vec![(name.clone(), RegistryCache::new(name.clone(), url.clone()))]
        } else {
            config
                .registries
                .iter()
                .map(|(name, url)| (name.clone(), RegistryCache::new(name.clone(), url.clone())))
                .collect()
        };

        candidates.sort_by(|a, b| a.0.cmp(&b.0));

        let mut resolved = None;
        for (name, cache) in &candidates {
            let snapshot = registries.synced(cache)?;
            let Some(entry) = snapshot.lookup(&dep.name)? else {
                continue;
            };
            if let Some(found) = resolve_entry(name, cache.url(), &dep.name, &dep.version, &entry) {
                if resolved.is_some() {
                    bail!(
                        "Dependency '{} {}' matches multiple registries; set registry explicitly",
                        dep.name,
                        dep.version
                    );
                }
                resolved = Some(found);
            }
        }
        if resolved.is_none() && dep.registry.is_some() {
            bail!(
                "Dependency '{} {}' has no non-yanked match in its selected registry",
                dep.name,
                dep.version
            );
        }
        Ok(resolved)
    }

    /// Install a dependency that has been resolved through a `[registries]`
    /// index. Downloads the published archive (https or file://), verifies
    /// the SHA-256 checksum recorded in the index when present, and extracts
    /// into `install_path`.
    ///
    /// Install a registry-resolved dep. Two paths:
    ///
    /// 1. **Binary archive** — when the index records `archive_url`, download
    ///    those bytes, SHA-256-verify, and `extract_zip`. Fastest path, byte-
    ///    deterministic.
    ///
    /// 2. **Git source fallback** — when `archive_url` is absent but
    ///    `package_repository` is set, `git clone --branch <tag>` the source
    ///    repo. Useful while a project is bootstrapping its index (no zip
    ///    artifact CDN yet) or for source-only projects that prefer git as
    ///    the source of truth. The lockfile's `source` stays `registry+...`
    ///    so re-fetch goes through the registry again, but `locked.git`
    ///    carries the resolved commit SHA for `--locked` reproducibility.
    ///
    /// Errors when both fields are absent (the index publisher hasn't
    /// supplied either route to the artifact).
    ///
    /// NOTE: archive path only supports zip today; tar.gz is a future task.
    fn install_from_registry(
        &self,
        dep: &DependencyConfig,
        resolved: &crate::registry::ResolvedRegistryDep,
        install_path: &Path,
        ccgo_home: &Path,
    ) -> Result<LockedPackage> {
        if let Some(archive_url) = resolved.version_entry.archive_url.as_deref() {
            return self.install_from_registry_archive(dep, resolved, archive_url, install_path);
        }

        if !resolved.package_repository.is_empty() {
            return self.install_from_registry_git(dep, resolved, install_path, ccgo_home);
        }

        anyhow::bail!(
            "registry '{}' has neither archive_url nor repository for {} {}; \
             publisher must either upload an artifact and re-run \
             `ccgo publish package --archive-url-template ...`, or ensure the \
             package's git URL is recorded in the index entry",
            resolved.registry_name,
            resolved.package_name,
            resolved.version_entry.version
        )
    }

    fn install_from_registry_archive(
        &self,
        dep: &DependencyConfig,
        resolved: &crate::registry::ResolvedRegistryDep,
        archive_url: &str,
        install_path: &Path,
    ) -> Result<LockedPackage> {
        println!(
            "📦 Resolving {} {} via registry '{}' (archive)",
            resolved.package_name, resolved.version_entry.version, resolved.registry_name
        );
        println!("   Archive: {}", archive_url);

        if resolved.version_entry.archive_format() != "zip" {
            bail!(
                "Registry archive format '{}' is not supported; expected zip",
                resolved.version_entry.archive_format()
            );
        }

        let bytes = Self::fetch_archive_bytes(archive_url)?;
        Self::verify_archive_checksum(resolved, &bytes)?;
        installation::replace(install_path, |candidate| {
            fs::create_dir_all(candidate)?;
            Self::extract_zip(&bytes, candidate)
        })?;

        println!("   ✓ Installed to {}", install_path.display());

        Ok(LockedPackage {
            name: dep.name.clone(),
            version: resolved.version_entry.version.clone(),
            source: format!("registry+{}", resolved.registry_url),
            checksum: Some(format!("sha256:{:x}", Sha256::digest(&bytes))),
            dependencies: vec![],
            git: None,
            installed_at: Some(chrono::Local::now().to_rfc3339()),
            patch: None,
        })
    }

    fn install_from_registry_git(
        &self,
        dep: &DependencyConfig,
        resolved: &crate::registry::ResolvedRegistryDep,
        install_path: &Path,
        ccgo_home: &Path,
    ) -> Result<LockedPackage> {
        let git_url = &resolved.package_repository;
        let tag = &resolved.version_entry.tag;

        println!(
            "📦 Resolving {} {} via registry '{}' (git+tag)",
            resolved.package_name, resolved.version_entry.version, resolved.registry_name
        );
        println!("   Repository: {}", git_url);
        println!("   Tag: {}", tag);

        // install_from_git accepts `branch` — `git clone --branch <name>`
        // works for tags too (it's a ref, not specifically a branch). Pass
        // the version-entry tag through and let git handle it.
        let mut locked = self.install_from_git(
            &dep.name,
            &resolved.version_entry.version,
            git_url,
            Some(tag),
            None, // no locked rev: registry resolution writes the commit SHA below
            install_path,
            ccgo_home,
        )?;

        // Override the source to mark this as registry-resolved. The
        // `locked.git` field that install_from_git populated stays — its
        // commit SHA is what `ccgo fetch --locked` checks for byte
        // reproducibility on subsequent runs.
        locked.source = format!("registry+{}", resolved.registry_url);
        // Index-recorded checksum (if any) is over the source tarball,
        // which doesn't match what we've cloned. Drop it; the commit SHA
        // is the deterministic anchor for git-resolved deps.
        locked.checksum = None;

        Ok(locked)
    }

    /// Fetch raw archive bytes from an `archive_url`. Supports `https://`,
    /// `http://`, and `file://` schemes.
    fn fetch_archive_bytes(archive_url: &str) -> Result<Vec<u8>> {
        if archive_url.starts_with("http://") || archive_url.starts_with("https://") {
            Self::download_zip(archive_url)
        } else if let Some(path_str) = archive_url.strip_prefix("file://") {
            // Reject relative paths early: `file://relative/foo.zip` would
            // silently resolve against the process cwd and read a wrong
            // file. The index publisher must always emit absolute paths
            // (or http(s) URLs) — there is no useful base path to resolve
            // against here, since `archive_url` flows through the index
            // entry without any context about who wrote it.
            let path = Path::new(path_str);
            if !path.is_absolute() {
                anyhow::bail!(
                    "file:// archive_url must be an absolute path; got: {} \
                     (the index publisher should emit a path starting with /)",
                    archive_url
                );
            }
            std::fs::read(path).with_context(|| format!("failed to read {}", path.display()))
        } else {
            anyhow::bail!("unsupported archive_url scheme: {}", archive_url);
        }
    }

    /// Verify the SHA-256 checksum recorded in the index against the
    /// downloaded archive bytes. No-op when the index doesn't record one.
    fn verify_archive_checksum(
        resolved: &crate::registry::ResolvedRegistryDep,
        bytes: &[u8],
    ) -> Result<()> {
        let Some(expected) = &resolved.version_entry.checksum else {
            return Ok(());
        };
        let actual = format!("sha256:{:x}", Sha256::digest(bytes));
        if &actual != expected {
            anyhow::bail!(
                "checksum mismatch for {} {}: index says {}, downloaded {}",
                resolved.package_name,
                resolved.version_entry.version,
                expected,
                actual
            );
        }
        Ok(())
    }

    /// Download a ZIP from a URL, returning its bytes
    fn download_zip(url: &str) -> Result<Vec<u8>> {
        let response = reqwest::blocking::get(url)
            .with_context(|| format!("Failed to download ZIP from {}", url))?;
        if !response.status().is_success() {
            anyhow::bail!("HTTP {} downloading ZIP from {}", response.status(), url);
        }
        let bytes = response
            .bytes()
            .context("Failed to read download response")?;
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

        let checkout = crate::registry::git_cache::prepare(
            ccgo_home,
            &crate::registry::git_cache::GitCacheSpec {
                name: dep_name,
                url: git_url,
                branch,
                revision: locked_rev,
            },
            self.force,
        )?;
        let registry_path = &checkout.path;

        let git_info = Self::get_git_info(registry_path);
        if locked_rev.is_some() && git_info.as_ref().and_then(|info| info.dirty) == Some(true) {
            bail!("Locked Git checkout is dirty; use --force to retrieve the locked commit again");
        }

        Self::link_validated_checkout(registry_path, install_path, self.copy)?;

        println!("   ✓ Installed to {}", install_path.display());

        // Get git info
        let revision = git_info
            .as_ref()
            .and_then(|g| g.revision.clone())
            .unwrap_or_else(|| "unknown".to_string());

        // Try to get version from dependency's CCGO.toml if not specified
        let resolved_version = if version.is_empty() {
            Self::get_dep_version(registry_path).unwrap_or_else(|| "0.0.0".to_string())
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

    fn link_validated_checkout(
        registry_path: &Path,
        install_path: &Path,
        copy: bool,
    ) -> Result<()> {
        let source = registry_path.canonicalize()?;
        installation::replace(install_path, |candidate| {
            Self::create_symlink_or_copy(&source, candidate, copy)
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
                git_info.revision =
                    Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
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
                git_info.remote_url =
                    Some(String::from_utf8_lossy(&output.stdout).trim().to_string());
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

    /// Execute install for workspace members
    fn execute_workspace_install(&self, current_dir: &Path, verbose: bool) -> Result<()> {
        // Find workspace root
        let workspace_root = find_workspace_root(current_dir)?.ok_or_else(|| {
            anyhow::anyhow!(
                "Not in a workspace. Use --workspace or --package only within a workspace."
            )
        })?;

        // Load workspace
        let workspace = Workspace::load(&workspace_root)?;

        if verbose {
            workspace.print_summary();
        }

        let members_to_install = self.workspace_members(&workspace)?;

        if members_to_install.is_empty() {
            bail!("No workspace members to install for");
        }

        println!("{}", "=".repeat(80));
        println!(
            "CCGO Workspace Install - Installing dependencies for {} member(s)",
            members_to_install.len()
        );
        println!("{}", "=".repeat(80));

        let ccgo_home = Self::get_ccgo_home();

        self.clean_global_cache(&ccgo_home)?;
        let _home_guard = crate::registry::cache_access::home_lock(&ccgo_home, false)?;

        let mut success_count = 0;
        let mut failed_members: Vec<String> = Vec::new();

        let mut registries = RegistrySession::default();
        for member in members_to_install {
            println!(
                "\n📦 Installing dependencies for {} ({})...",
                member.name, member.version
            );
            println!("{}", "-".repeat(60));

            // Execute install in member's directory
            let member_path = workspace_root.join(&member.name);

            match self.install_for_member(&member_path, &ccgo_home, verbose, &mut registries) {
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
            bail!(
                "{} workspace member(s) failed to install",
                failed_members.len()
            );
        }

        Ok(())
    }

    fn workspace_members<'a>(
        &self,
        workspace: &'a Workspace,
    ) -> Result<Vec<&'a crate::workspace::WorkspaceMember>> {
        // Determine which members to install for
        let members_to_install = if let Some(ref package_name) = self.package {
            // Install for specific package
            let member = workspace.get_member(package_name).ok_or_else(|| {
                anyhow::anyhow!(
                    "Package '{}' not found in workspace. Available: {}",
                    package_name,
                    workspace.members.names().join(", ")
                )
            })?;
            vec![member]
        } else {
            // Install for default members (or all if no default_members specified)
            workspace.default_members()
        };

        Ok(members_to_install)
    }

    /// Install dependencies for a single workspace member
    fn install_for_member(
        &self,
        member_path: &Path,
        ccgo_home: &Path,
        _verbose: bool,
        registries: &mut RegistrySession,
    ) -> Result<usize> {
        let _project_guard = installation::lock_project(member_path)?;
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
                "No {} found for {}. Run 'ccgo fetch' first.",
                LOCKFILE_NAME,
                member_path.display()
            );
        }

        self.validate_locked_dependencies(existing_lockfile.as_ref(), dependencies)?;

        let deps_dir = member_path.join(".ccgo").join("deps");
        fs::create_dir_all(&deps_dir).context("Failed to create .ccgo/deps directory")?;

        let mut lockfile = existing_lockfile.unwrap_or_else(Lockfile::new);
        let packages_before = lockfile.packages.clone();
        let dependency_graph = self.resolve_install_graph(dependencies, member_path, ccgo_home);
        let selected: Vec<_> = dependencies.iter().collect();
        let install_order = Self::installation_order(&dependency_graph, &selected);
        let deps_to_process =
            Self::ordered_dependencies(&dependency_graph, dependencies, selected, &install_order);
        let context = &mut FetchContext {
            project_dir: member_path,
            ccgo_home,
            config: &config,
            registries,
            completed: Vec::new(),
        };
        let (installed_count, failed_count) =
            self.install_batch(&deps_to_process, &mut lockfile, context);

        self.save_fetch_lockfile(context, &mut lockfile, &packages_before, installed_count)?;

        if failed_count > 0 {
            bail!(
                "Some dependencies failed to install for {}",
                member_path.display()
            );
        }

        Ok(installed_count)
    }

    /// Get CCGO home directory
    fn get_ccgo_home() -> PathBuf {
        if let Some(home) = std::env::var_os("CCGO_HOME") {
            return PathBuf::from(home);
        }
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
            w.write_all(b"[package]\nname = \"mylib\"\nversion = \"1.0.0\"\n")
                .unwrap();
            w.finish().unwrap();
        }

        FetchCommand::extract_zip(&buf, &extract_dir).unwrap();

        assert!(extract_dir.join("include/mylib/mylib.h").exists());
        assert!(extract_dir.join("CCGO.toml").exists());
    }

    #[test]
    fn test_extract_zip_missing_zip_returns_error() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let extract_dir = tmp_dir.path().join("extracted");
        let result = FetchCommand::extract_zip(b"not a zip", &extract_dir);
        assert!(result.is_err());
    }

    #[test]
    fn test_is_tar_gz() {
        assert!(FetchCommand::is_tar_gz("foo.tar.gz"));
        assert!(FetchCommand::is_tar_gz("foo.tgz"));
        assert!(FetchCommand::is_tar_gz(
            "https://cdn.example.com/sdk-1.0.0.tar.gz"
        ));
        assert!(!FetchCommand::is_tar_gz("foo.zip"));
        assert!(!FetchCommand::is_tar_gz("foo.tar"));
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
            tar.append_data(
                &mut header,
                "include/mylib/mylib.h",
                header_content.as_ref(),
            )
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

        FetchCommand::extract_tar_gz(&buf, &extract_dir).unwrap();

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

        let result = FetchCommand::extract_tar_gz(&buf, &extract_dir);
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

        let result = FetchCommand::extract_zip(&buf, &extract_dir);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("unsafe path"));
    }
}
