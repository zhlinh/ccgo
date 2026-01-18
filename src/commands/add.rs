//! Add command - Add a dependency to CCGO.toml
//!
//! Usage:
//!   ccgo add <name> --version <version>
//!   ccgo add <name> --git <url> [--branch <branch>]
//!   ccgo add <name> --path <path>

use anyhow::{bail, Context, Result};
use clap::Args;
use std::fs;
use std::path::Path;

use crate::config::CcgoConfig;
use crate::version::VersionReq;

/// Add a dependency to CCGO.toml
#[derive(Args, Debug)]
pub struct AddCommand {
    /// Dependency name
    pub name: String,

    /// Version requirement (e.g., "^1.0", "~1.2.3", ">=1.0,<2.0")
    #[arg(long, short = 'V', conflicts_with_all = &["git", "path"])]
    pub version: Option<String>,

    /// Git repository URL
    #[arg(long, conflicts_with_all = &["version", "path"])]
    pub git: Option<String>,

    /// Git branch name (use with --git)
    #[arg(long, requires = "git")]
    pub branch: Option<String>,

    /// Local path to dependency
    #[arg(long, conflicts_with_all = &["version", "git"])]
    pub path: Option<String>,

    /// Don't run install after adding
    #[arg(long)]
    pub no_install: bool,
}

impl AddCommand {
    /// Execute the add command
    pub fn execute(self, _verbose: bool) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Add - Add Dependency to CCGO.toml");
        println!("{}", "=".repeat(80));

        // Validate inputs
        if self.version.is_none() && self.git.is_none() && self.path.is_none() {
            bail!("Must specify one of: --version, --git, or --path");
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
        if config.dependencies.iter().any(|d| d.name == self.name) {
            bail!("Dependency '{}' already exists in CCGO.toml", self.name);
        }

        println!("\n📦 Adding dependency: {}", self.name);

        // Build dependency entry
        let dep_entry = self.build_dependency_entry();
        println!("{}", dep_entry);

        // Append to CCGO.toml
        content = self.append_dependency(&content, &dep_entry);
        fs::write(config_path, content)
            .context("Failed to write CCGO.toml")?;

        println!("\n✓ Added '{}' to CCGO.toml", self.name);

        // Run install unless disabled
        if !self.no_install {
            println!("\n{}", "=".repeat(80));
            println!("Installing dependency...");
            println!("{}", "=".repeat(80));

            let install_cmd = crate::commands::install::InstallCommand {
                dependency: Some(self.name.clone()),
                force: false,
                platform: None,
                clean_cache: false,
                copy: false,
                locked: false,
            };

            if let Err(e) = install_cmd.execute(_verbose) {
                eprintln!("\n⚠️  Failed to install '{}': {}", self.name, e);
                eprintln!("   You can install manually with: ccgo install {}", self.name);
            }
        } else {
            println!("\n💡 Run 'ccgo install' to install the dependency");
        }

        Ok(())
    }

    /// Build the TOML entry for this dependency
    fn build_dependency_entry(&self) -> String {
        let mut entry = format!("[[dependencies]]\nname = \"{}\"", self.name);

        if let Some(ref version) = self.version {
            entry.push_str(&format!("\nversion = \"{}\"", version));
        }

        if let Some(ref git) = self.git {
            entry.push_str(&format!("\ngit = \"{}\"", git));
            if let Some(ref branch) = self.branch {
                entry.push_str(&format!("\nbranch = \"{}\"", branch));
            }
            // Add a default version for git dependencies
            entry.push_str("\nversion = \"0.0.0\"");
        }

        if let Some(ref path) = self.path {
            entry.push_str(&format!("\npath = \"{}\"", path));
            // Add a default version for path dependencies
            entry.push_str("\nversion = \"0.0.0\"");
        }

        entry.push_str("\n");
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
            path: None,
            no_install: true,
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
            path: None,
            no_install: true,
        };

        let entry = cmd.build_dependency_entry();
        assert!(entry.contains("name = \"mylib\""));
        assert!(entry.contains("git = \"https://github.com/user/repo.git\""));
        assert!(entry.contains("branch = \"main\""));
    }

    #[test]
    fn test_build_path_dependency() {
        let cmd = AddCommand {
            name: "mylib".to_string(),
            version: None,
            git: None,
            branch: None,
            path: Some("../mylib".to_string()),
            no_install: true,
        };

        let entry = cmd.build_dependency_entry();
        assert!(entry.contains("name = \"mylib\""));
        assert!(entry.contains("path = \"../mylib\""));
    }
}
