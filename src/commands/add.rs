//! Add command - Add a dependency to CCGO.toml
//!
//! Usage:
//!   ccgo add <name> --version <version>
//!   ccgo add <name> --git <url> [--branch <branch>]
//!   ccgo add <name> --path <path>
//!   ccgo add github:user/repo              # Git shorthand
//!   ccgo add github:user/repo@v1.0.0       # With version tag
//!   ccgo add <name> --git gh:user/repo     # Shorthand in --git

use anyhow::{bail, Context, Result};
use clap::Args;
use std::fs;
use std::path::Path;

use crate::config::CcgoConfig;
use crate::registry::{expand_git_shorthand, discover_latest_version};
use crate::version::VersionReq;

/// Add a dependency to CCGO.toml
///
/// # Examples
///
/// ```bash
/// # Add by Git shorthand (auto-discovers latest version)
/// ccgo add github:fmtlib/fmt
/// ccgo add gh:nlohmann/json
///
/// # Add with specific version tag
/// ccgo add github:fmtlib/fmt@v10.1.1
///
/// # Traditional methods still work
/// ccgo add mylib --git https://github.com/user/repo.git
/// ccgo add mylib --version ^1.0.0
/// ccgo add mylib --path ../mylib
/// ```
#[derive(Args, Debug)]
pub struct AddCommand {
    /// Dependency name or Git shorthand (e.g., "github:user/repo", "gh:user/repo@v1.0")
    pub name: String,

    /// Version requirement (e.g., "^1.0", "~1.2.3", ">=1.0,<2.0")
    #[arg(long, short = 'V', conflicts_with_all = &["path"])]
    pub version: Option<String>,

    /// Git repository URL or shorthand (e.g., "https://...", "github:user/repo")
    #[arg(long, conflicts_with_all = &["path"])]
    pub git: Option<String>,

    /// Git branch name (use with --git)
    #[arg(long)]
    pub branch: Option<String>,

    /// Git tag (use with --git, overrides --branch)
    #[arg(long)]
    pub tag: Option<String>,

    /// Local path to dependency
    #[arg(long, conflicts_with_all = &["version", "git"])]
    pub path: Option<String>,

    /// Don't run install after adding
    #[arg(long)]
    pub no_install: bool,

    /// Auto-discover and use the latest version tag
    #[arg(long)]
    pub latest: bool,

    /// Include pre-release versions when using --latest
    #[arg(long)]
    pub prerelease: bool,
}

impl AddCommand {
    /// Execute the add command
    pub fn execute(mut self, _verbose: bool) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Add - Add Dependency to CCGO.toml");
        println!("{}", "=".repeat(80));

        // Check if name is a Git shorthand (contains ':' or '/')
        let (dep_name, git_url, git_ref) = self.resolve_input()?;

        // Update self with resolved values
        if git_url.is_some() {
            self.git = git_url;
        }
        if git_ref.is_some() && self.branch.is_none() && self.tag.is_none() {
            self.tag = git_ref;
        }

        // Validate inputs - now accepts shorthand patterns too
        if self.version.is_none() && self.git.is_none() && self.path.is_none() {
            bail!(
                "Must specify one of: --version, --git, --path, or use Git shorthand\n\
                 Examples:\n\
                   ccgo add github:fmtlib/fmt\n\
                   ccgo add gh:user/repo@v1.0.0\n\
                   ccgo add mylib --git https://github.com/user/repo.git\n\
                   ccgo add mylib --version ^1.0.0"
            );
        }

        // Auto-discover latest version if requested
        if self.latest {
            if let Some(ref git) = self.git {
                println!("\n🔍 Discovering latest version from {}...", git);
                match discover_latest_version(git, self.prerelease) {
                    Ok(Some(tag_info)) => {
                        println!("   Found: {} ({})", tag_info.tag,
                            if tag_info.semver.as_ref().map_or(false, |v| v.is_stable()) {
                                "stable"
                            } else {
                                "prerelease"
                            }
                        );
                        self.tag = Some(tag_info.tag);
                    }
                    Ok(None) => {
                        println!("   ⚠️  No version tags found, using branch 'main'");
                        self.branch = Some("main".to_string());
                    }
                    Err(e) => {
                        println!("   ⚠️  Failed to discover versions: {}", e);
                        println!("   Using branch 'main' as fallback");
                        self.branch = Some("main".to_string());
                    }
                }
            }
        }

        // Validate version requirement if provided
        if let Some(ref version) = self.version {
            VersionReq::parse(version)
                .with_context(|| format!("Invalid version requirement: '{}'", version))?;
        }

        // Load existing CCGO.toml
        let config_path = Path::new("CCGO.toml");
        if !config_path.exists() {
            bail!("CCGO.toml not found in current directory. Run 'ccgo init' first.");
        }

        let mut content = fs::read_to_string(config_path)
            .context("Failed to read CCGO.toml")?;

        // Parse to validate and check for duplicates
        let config = CcgoConfig::parse(&content)?;
        if config.dependencies.iter().any(|d| d.name == dep_name) {
            bail!("Dependency '{}' already exists in CCGO.toml", dep_name);
        }

        println!("\n📦 Adding dependency: {}", dep_name);

        // Build dependency entry with resolved name
        let dep_entry = self.build_dependency_entry_with_name(&dep_name);
        println!("{}", dep_entry);

        // Append to CCGO.toml
        content = self.append_dependency(&content, &dep_entry);
        fs::write(config_path, content)
            .context("Failed to write CCGO.toml")?;

        println!("\n✓ Added '{}' to CCGO.toml", dep_name);

        // Run install unless disabled
        if !self.no_install {
            println!("\n{}", "=".repeat(80));
            println!("Installing dependency...");
            println!("{}", "=".repeat(80));

            let install_cmd = crate::commands::install::InstallCommand {
                dependency: Some(dep_name.clone()),
                force: false,
                platform: None,
                clean_cache: false,
                copy: false,
                locked: false,
                conflict_strategy: crate::commands::install::ConflictStrategy::default(),
                workspace: false,
                package: None,
            };

            if let Err(e) = install_cmd.execute(_verbose) {
                eprintln!("\n⚠️  Failed to install '{}': {}", dep_name, e);
                eprintln!("   You can install manually with: ccgo install {}", dep_name);
            }
        } else {
            println!("\n💡 Run 'ccgo install' to install the dependency");
        }

        Ok(())
    }

    /// Resolve input - check if it's a Git shorthand and extract components
    fn resolve_input(&self) -> Result<(String, Option<String>, Option<String>)> {
        // Check if name looks like a Git shorthand
        let is_shorthand = self.name.contains(':')
            || (self.name.contains('/') && !self.name.starts_with('.') && !self.name.starts_with('/'));

        if is_shorthand && self.git.is_none() && self.path.is_none() {
            // Parse as Git shorthand
            let spec = expand_git_shorthand(&self.name)?;
            let dep_name = spec.repo.clone();
            let git_url = Some(spec.url);
            let git_ref = spec.reference;
            return Ok((dep_name, git_url, git_ref));
        }

        // Check if --git is a shorthand
        if let Some(ref git) = self.git {
            if git.contains(':') && !git.starts_with("https://") && !git.starts_with("http://") && !git.starts_with("git@") {
                let spec = expand_git_shorthand(git)?;
                return Ok((self.name.clone(), Some(spec.url), spec.reference));
            }
        }

        // Use name as-is
        Ok((self.name.clone(), self.git.clone(), None))
    }

    /// Build the TOML entry for this dependency
    fn build_dependency_entry(&self) -> String {
        self.build_dependency_entry_with_name(&self.name)
    }

    /// Build the TOML entry with a specific name
    fn build_dependency_entry_with_name(&self, name: &str) -> String {
        let mut entry = format!("[[dependencies]]\nname = \"{}\"", name);

        if let Some(ref version) = self.version {
            entry.push_str(&format!("\nversion = \"{}\"", version));
        }

        if let Some(ref git) = self.git {
            entry.push_str(&format!("\ngit = \"{}\"", git));

            // Tag takes precedence over branch
            if let Some(ref tag) = self.tag {
                entry.push_str(&format!("\nbranch = \"{}\"", tag));
            } else if let Some(ref branch) = self.branch {
                entry.push_str(&format!("\nbranch = \"{}\"", branch));
            }

            // Add a default version for git dependencies if not specified
            if self.version.is_none() {
                entry.push_str("\nversion = \"0.0.0\"");
            }
        }

        if let Some(ref path) = self.path {
            entry.push_str(&format!("\npath = \"{}\"", path));
            // Add a default version for path dependencies if not specified
            if self.version.is_none() {
                entry.push_str("\nversion = \"0.0.0\"");
            }
        }

        entry.push('\n');
        entry
    }

    /// Append dependency to CCGO.toml content
    fn append_dependency(&self, content: &str, dep_entry: &str) -> String {
        // Check if there's already a [[dependencies]] section
        if content.contains("[[dependencies]]") {
            // Append after the last dependency
            format!("{}\n{}", content.trim_end(), dep_entry)
        } else {
            // Add first dependency with a blank line
            format!("{}\n\n{}", content.trim_end(), dep_entry)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_version_dependency() {
        let cmd = AddCommand {
            name: "mylib".to_string(),
            version: Some("^1.0.0".to_string()),
            git: None,
            branch: None,
            tag: None,
            path: None,
            no_install: true,
            latest: false,
            prerelease: false,
        };

        let entry = cmd.build_dependency_entry();
        assert!(entry.contains("name = \"mylib\""));
        assert!(entry.contains("version = \"^1.0.0\""));
    }

    #[test]
    fn test_build_git_dependency() {
        let cmd = AddCommand {
            name: "mylib".to_string(),
            version: None,
            git: Some("https://github.com/user/repo.git".to_string()),
            branch: Some("main".to_string()),
            tag: None,
            path: None,
            no_install: true,
            latest: false,
            prerelease: false,
        };

        let entry = cmd.build_dependency_entry();
        assert!(entry.contains("name = \"mylib\""));
        assert!(entry.contains("git = \"https://github.com/user/repo.git\""));
        assert!(entry.contains("branch = \"main\""));
    }

    #[test]
    fn test_build_git_dependency_with_tag() {
        let cmd = AddCommand {
            name: "mylib".to_string(),
            version: None,
            git: Some("https://github.com/user/repo.git".to_string()),
            branch: None,
            tag: Some("v1.0.0".to_string()),
            path: None,
            no_install: true,
            latest: false,
            prerelease: false,
        };

        let entry = cmd.build_dependency_entry();
        assert!(entry.contains("name = \"mylib\""));
        assert!(entry.contains("git = \"https://github.com/user/repo.git\""));
        assert!(entry.contains("branch = \"v1.0.0\""));
    }

    #[test]
    fn test_build_path_dependency() {
        let cmd = AddCommand {
            name: "mylib".to_string(),
            version: None,
            git: None,
            branch: None,
            tag: None,
            path: Some("../mylib".to_string()),
            no_install: true,
            latest: false,
            prerelease: false,
        };

        let entry = cmd.build_dependency_entry();
        assert!(entry.contains("name = \"mylib\""));
        assert!(entry.contains("path = \"../mylib\""));
    }

    #[test]
    fn test_resolve_github_shorthand() {
        let cmd = AddCommand {
            name: "github:fmtlib/fmt".to_string(),
            version: None,
            git: None,
            branch: None,
            tag: None,
            path: None,
            no_install: true,
            latest: false,
            prerelease: false,
        };

        let (name, git, _) = cmd.resolve_input().unwrap();
        assert_eq!(name, "fmt");
        assert_eq!(git.unwrap(), "https://github.com/fmtlib/fmt.git");
    }

    #[test]
    fn test_resolve_github_shorthand_with_ref() {
        let cmd = AddCommand {
            name: "github:fmtlib/fmt@v10.1.1".to_string(),
            version: None,
            git: None,
            branch: None,
            tag: None,
            path: None,
            no_install: true,
            latest: false,
            prerelease: false,
        };

        let (name, git, git_ref) = cmd.resolve_input().unwrap();
        assert_eq!(name, "fmt");
        assert_eq!(git.unwrap(), "https://github.com/fmtlib/fmt.git");
        assert_eq!(git_ref.unwrap(), "v10.1.1");
    }

    #[test]
    fn test_resolve_bare_repo_path() {
        let cmd = AddCommand {
            name: "fmtlib/fmt".to_string(),
            version: None,
            git: None,
            branch: None,
            tag: None,
            path: None,
            no_install: true,
            latest: false,
            prerelease: false,
        };

        let (name, git, _) = cmd.resolve_input().unwrap();
        assert_eq!(name, "fmt");
        assert_eq!(git.unwrap(), "https://github.com/fmtlib/fmt.git");
    }

    #[test]
    fn test_resolve_git_shorthand_in_option() {
        let cmd = AddCommand {
            name: "fmt".to_string(),
            version: None,
            git: Some("gh:fmtlib/fmt".to_string()),
            branch: None,
            tag: None,
            path: None,
            no_install: true,
            latest: false,
            prerelease: false,
        };

        let (name, git, _) = cmd.resolve_input().unwrap();
        assert_eq!(name, "fmt");
        assert_eq!(git.unwrap(), "https://github.com/fmtlib/fmt.git");
    }
}
