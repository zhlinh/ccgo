//! Tree command - Display dependency tree
//!
//! Usage:
//!   ccgo tree                    # Show full dependency tree
//!   ccgo tree --depth 2          # Limit depth
//!   ccgo tree <package>          # Show specific package's dependencies
//!   ccgo tree --no-dedupe        # Show all occurrences (don't deduplicate)

use anyhow::{bail, Context, Result};
use clap::Args;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::{CcgoConfig, DependencyConfig};

/// Display dependency tree
#[derive(Args, Debug)]
pub struct TreeCommand {
    /// Show dependencies of a specific package
    pub package: Option<String>,

    /// Maximum depth to display (default: unlimited)
    #[arg(long, short = 'd')]
    pub depth: Option<usize>,

    /// Don't deduplicate repeated dependencies
    #[arg(long)]
    pub no_dedupe: bool,

    /// Show dependency versions from lock file
    #[arg(long, short = 'l')]
    pub locked: bool,
}

// Note: DepNode is reserved for future use when implementing transitive dependency resolution
#[allow(dead_code)]
#[derive(Debug, Clone)]
struct DepNode {
    name: String,
    version: String,
    source: String,
    dependencies: Vec<DependencyConfig>,
}

/// Lock file information
#[derive(Debug)]
struct LockInfo {
    dependencies: HashMap<String, LockedDep>,
}

#[derive(Debug, Clone)]
struct LockedDep {
    version: String,
    #[allow(dead_code)]
    source: String,
    install_path: String,
}

impl TreeCommand {
    /// Execute the tree command
    pub fn execute(self, _verbose: bool) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Tree - Dependency Tree Viewer");
        println!("{}", "=".repeat(80));

        let project_dir = std::env::current_dir().context("Failed to get current directory")?;

        // Load CCGO.toml
        let config = CcgoConfig::load().context("Failed to load CCGO.toml")?;

        if config.dependencies.is_empty() {
            println!("\n✓ No dependencies defined in CCGO.toml");
            return Ok(());
        }

        // Load lock file if requested or available
        let lock_info = if self.locked {
            Some(Self::load_lock_file(&project_dir).context("Failed to load CCGO.toml.lock")?)
        } else {
            Self::load_lock_file(&project_dir).ok()
        };

        println!("\n{} v{}", config.package.name, config.package.version);

        // Filter dependencies if specific package requested
        let deps_to_show: Vec<_> = if let Some(ref pkg) = self.package {
            config
                .dependencies
                .iter()
                .filter(|d| &d.name == pkg)
                .collect()
        } else {
            config.dependencies.iter().collect()
        };

        if deps_to_show.is_empty() {
            if let Some(ref pkg) = self.package {
                bail!("Package '{}' not found in dependencies", pkg);
            }
            println!("\n✓ No dependencies to display");
            return Ok(());
        }

        // Track shown dependencies for deduplication
        let mut shown = if self.no_dedupe {
            None
        } else {
            Some(HashSet::new())
        };

        // Display dependency tree
        for (idx, dep) in deps_to_show.iter().enumerate() {
            let is_last = idx == deps_to_show.len() - 1;
            let prefix = if is_last { "└── " } else { "├── " };
            let continue_prefix = if is_last { "    " } else { "│   " };

            self.print_dependency(
                dep,
                prefix,
                continue_prefix,
                0,
                &lock_info,
                &mut shown,
                &project_dir,
            )?;
        }

        println!();
        Ok(())
    }

    /// Print a dependency and its children
    fn print_dependency(
        &self,
        dep: &DependencyConfig,
        prefix: &str,
        continue_prefix: &str,
        current_depth: usize,
        lock_info: &Option<LockInfo>,
        shown: &mut Option<HashSet<String>>,
        project_dir: &Path,
    ) -> Result<()> {
        // Check depth limit
        if let Some(max_depth) = self.depth {
            if current_depth >= max_depth {
                return Ok(());
            }
        }

        // Get version from lock file or CCGO.toml
        let version_info = if let Some(ref lock) = lock_info {
            if let Some(locked_dep) = lock.dependencies.get(&dep.name) {
                format!(" v{}", locked_dep.version)
            } else {
                format!(" v{}", dep.version)
            }
        } else {
            format!(" v{}", dep.version)
        };

        // Get source info
        let source_info = self.format_source(dep);

        // Check if already shown (for deduplication)
        let dep_key = format!("{}{}", dep.name, version_info);
        let already_shown = if let Some(ref mut set) = shown {
            !set.insert(dep_key.clone())
        } else {
            false
        };

        // Print this dependency
        if already_shown {
            println!("{}{}{}{}  (*)", prefix, dep.name, version_info, source_info);
            return Ok(());
        } else {
            println!("{}{}{}{}", prefix, dep.name, version_info, source_info);
        }

        // Try to load this dependency's CCGO.toml to find its dependencies
        let sub_deps = self.load_sub_dependencies(dep, lock_info, project_dir)?;

        if !sub_deps.is_empty() {
            for (idx, sub_dep) in sub_deps.iter().enumerate() {
                let is_last = idx == sub_deps.len() - 1;
                let sub_prefix = format!(
                    "{}{}",
                    continue_prefix,
                    if is_last { "└── " } else { "├── " }
                );
                let sub_continue = format!(
                    "{}{}",
                    continue_prefix,
                    if is_last { "    " } else { "│   " }
                );

                self.print_dependency(
                    sub_dep,
                    &sub_prefix,
                    &sub_continue,
                    current_depth + 1,
                    lock_info,
                    shown,
                    project_dir,
                )?;
            }
        }

        Ok(())
    }

    /// Format source information for display
    fn format_source(&self, dep: &DependencyConfig) -> String {
        if let Some(ref path) = dep.path {
            format!("  (path: {})", path)
        } else if let Some(ref git) = dep.git {
            if let Some(ref branch) = dep.branch {
                format!("  (git: {}, branch: {})", git, branch)
            } else {
                format!("  (git: {})", git)
            }
        } else {
            String::new()
        }
    }

    /// Load sub-dependencies from a dependency's CCGO.toml
    fn load_sub_dependencies(
        &self,
        dep: &DependencyConfig,
        lock_info: &Option<LockInfo>,
        project_dir: &Path,
    ) -> Result<Vec<DependencyConfig>> {
        // Determine where to look for the dependency's CCGO.toml
        let dep_path = if let Some(ref path) = dep.path {
            // Local path dependency
            let mut p = PathBuf::from(path);
            if p.is_relative() {
                p = project_dir.join(p);
            }
            p
        } else if let Some(ref lock) = lock_info {
            // Try to get from lock file install path
            if let Some(locked_dep) = lock.dependencies.get(&dep.name) {
                PathBuf::from(&locked_dep.install_path)
            } else {
                // Not installed yet
                return Ok(Vec::new());
            }
        } else {
            // No lock file and not a path dependency
            return Ok(Vec::new());
        };

        // Try to load CCGO.toml from dependency
        let ccgo_toml_path = dep_path.join("CCGO.toml");
        if !ccgo_toml_path.exists() {
            return Ok(Vec::new());
        }

        match CcgoConfig::load_from(&ccgo_toml_path) {
            Ok(config) => Ok(config.dependencies),
            Err(_) => Ok(Vec::new()), // Ignore parse errors for sub-dependencies
        }
    }

    /// Load CCGO.toml.lock file
    fn load_lock_file(project_dir: &Path) -> Result<LockInfo> {
        let lock_path = project_dir.join("CCGO.toml.lock");
        if !lock_path.exists() {
            bail!("CCGO.toml.lock not found. Run 'ccgo install' first.");
        }

        let content = fs::read_to_string(&lock_path).context("Failed to read CCGO.toml.lock")?;

        // Parse the lock file
        let mut dependencies = HashMap::new();

        // Simple TOML parsing - look for [dependencies.xxx] sections
        let lines: Vec<&str> = content.lines().collect();
        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();

            // Look for [dependencies.xxx]
            if line.starts_with("[dependencies.") && line.ends_with(']') {
                let dep_name = line
                    .trim_start_matches("[dependencies.")
                    .trim_end_matches(']')
                    .split('.')
                    .next()
                    .unwrap_or("")
                    .to_string();

                if dep_name.is_empty() || dep_name == "git" {
                    i += 1;
                    continue;
                }

                // Parse fields for this dependency
                let mut version = String::new();
                let mut source = String::new();
                let mut install_path = String::new();

                i += 1;
                while i < lines.len() {
                    let field_line = lines[i].trim();

                    if field_line.is_empty() || field_line.starts_with('#') {
                        i += 1;
                        continue;
                    }

                    if field_line.starts_with('[') {
                        // Next section
                        break;
                    }

                    if let Some((key, value)) = field_line.split_once('=') {
                        let key = key.trim();
                        let value = value.trim().trim_matches('"');

                        match key {
                            "version" => version = value.to_string(),
                            "source" => source = value.to_string(),
                            "install_path" => install_path = value.to_string(),
                            _ => {}
                        }
                    }

                    i += 1;
                }

                if !version.is_empty() && !install_path.is_empty() {
                    dependencies.insert(
                        dep_name,
                        LockedDep {
                            version,
                            source,
                            install_path,
                        },
                    );
                }
            } else {
                i += 1;
            }
        }

        Ok(LockInfo { dependencies })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_source_path() {
        let cmd = TreeCommand {
            package: None,
            depth: None,
            no_dedupe: false,
            locked: false,
        };

        let dep = DependencyConfig {
            name: "mylib".to_string(),
            version: "1.0.0".to_string(),
            path: Some("../mylib".to_string()),
            git: None,
            branch: None,
        };

        assert_eq!(cmd.format_source(&dep), "  (path: ../mylib)");
    }

    #[test]
    fn test_format_source_git() {
        let cmd = TreeCommand {
            package: None,
            depth: None,
            no_dedupe: false,
            locked: false,
        };

        let dep = DependencyConfig {
            name: "mylib".to_string(),
            version: "1.0.0".to_string(),
            path: None,
            git: Some("https://github.com/user/repo.git".to_string()),
            branch: Some("main".to_string()),
        };

        assert_eq!(
            cmd.format_source(&dep),
            "  (git: https://github.com/user/repo.git, branch: main)"
        );
    }
}
