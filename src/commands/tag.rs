//! Tag command implementation - Pure Rust version
//!
//! Creates and manages Git tags for versioning.
//! Supports auto-versioning from CCGO.toml, custom messages, and remote operations.

use std::process::Command;

use anyhow::{bail, Context, Result};
use clap::Args;

use crate::config::CcgoConfig;

/// Create version tag from CCGO.toml
#[derive(Args, Debug)]
pub struct TagCommand {
    /// Tag version (e.g., v1.0.0). If not provided, auto-generated from CCGO.toml
    pub version: Option<String>,

    /// Custom tag message (default: auto-generated with version info)
    #[arg(short, long)]
    pub message: Option<String>,

    /// Create lightweight tag instead of annotated tag
    #[arg(long)]
    pub lightweight: bool,

    /// Push tag to remote after creation (default: no push)
    #[arg(long)]
    pub push: bool,

    /// Force create tag (replace if exists)
    #[arg(short, long)]
    pub force: bool,

    /// Delete specified tag (local and remote)
    #[arg(short, long)]
    pub delete: bool,

    /// When deleting, only delete local tag (used with --delete)
    #[arg(long)]
    pub local_only: bool,

    /// List all tags
    #[arg(short, long)]
    pub list: bool,

    /// When listing, show remote tags (used with --list)
    #[arg(long)]
    pub remote: bool,
}

impl TagCommand {
    /// Execute the tag command
    pub fn execute(self, _verbose: bool) -> Result<()> {
        // Handle list operation
        if self.list {
            return self.list_tags();
        }

        // Handle delete operation
        if self.delete {
            if self.version.is_none() {
                eprintln!("✗ {}", "Version required for delete operation");
                eprintln!("Usage: ccgo tag --delete v1.0.0");
                std::process::exit(1);
            }
            return self.delete_tag();
        }

        // Handle create operation
        self.create_tag()
    }

    /// Create a Git tag
    fn create_tag(self) -> Result<()> {
        eprintln!("=== {} ===", "Creating Git tag");

        // Load project configuration
        let config = CcgoConfig::load()
            .context("Failed to load CCGO.toml. Please run from project root directory")?;

        let project_dir = std::env::current_dir().context("Failed to get current directory")?;

        // Determine tag version
        let tag_version = if let Some(v) = self.version {
            v
        } else {
            // Auto-generate from CCGO.toml
            let config_version = &config.package.version;
            if config_version.starts_with('v') {
                config_version.clone()
            } else {
                format!("v{}", config_version)
            }
        };

        eprintln!("{}", &format!("Tag version: {}", tag_version));

        // Get git information
        let git_info = get_git_info(&project_dir)?;

        // Generate tag message
        let tag_message = if let Some(msg) = self.message {
            msg
        } else {
            generate_tag_message(&tag_version, &config.package.name, &git_info)
        };

        if !self.lightweight {
            println!("\nTag message:");
            println!("{}", "-".repeat(60));
            println!("{}", tag_message);
            println!("{}\n", "-".repeat(60));
        }

        // Check if tag already exists
        if tag_exists(&project_dir, &tag_version)? {
            if !self.force {
                eprintln!("✗ {}", &format!("Tag '{}' already exists", tag_version));
                eprintln!("Use --force to replace it");
                std::process::exit(1);
            } else {
                eprintln!("⚠ {}", &format!(
                    "Tag '{}' already exists, will be replaced (--force)",
                    tag_version
                ));
                // Delete existing tag
                let _ = Command::new("git")
                    .args(&["tag", "-d", &tag_version])
                    .current_dir(&project_dir)
                    .output();
            }
        }

        // Create tag
        let result = if self.lightweight {
            // Lightweight tag
            let mut cmd = Command::new("git");
            cmd.args(&["tag", &tag_version]);
            if self.force {
                cmd.arg("-f");
            }
            cmd.current_dir(&project_dir)
                .status()
                .context("Failed to create lightweight tag")?
        } else {
            // Annotated tag
            let mut cmd = Command::new("git");
            cmd.args(&["tag", "-a", &tag_version, "-m", &tag_message]);
            if self.force {
                cmd.arg("-f");
            }
            cmd.current_dir(&project_dir)
                .status()
                .context("Failed to create annotated tag")?
        };

        if !result.success() {
            bail!("Failed to create tag");
        }

        eprintln!("✓ {}", &format!(
            "Created {} tag: {}",
            if self.lightweight {
                "lightweight"
            } else {
                "annotated"
            },
            tag_version
        ));

        // Show tag info
        let output = Command::new("git")
            .args(&["show", &tag_version, "--no-patch"])
            .current_dir(&project_dir)
            .output()
            .context("Failed to show tag info")?;

        if output.status.success() {
            println!("\nTag info:");
            println!("{}", String::from_utf8_lossy(&output.stdout));
        }

        // Push to remote
        if self.push {
            eprintln!("{}", "Pushing tag to remote...");
            let mut push_cmd = Command::new("git");
            push_cmd.args(&["push", "origin", &tag_version]);
            if self.force {
                push_cmd.arg("-f");
            }

            let result = push_cmd.current_dir(&project_dir).status();

            match result {
                Ok(status) if status.success() => {
                    eprintln!("✓ {}", &format!("Pushed tag to origin/{}", tag_version));
                }
                _ => {
                    eprintln!("⚠ {}", "Failed to push tag to remote");
                    println!("Tag created locally. You can push it manually:");
                    println!("  git push origin {}", tag_version);
                }
            }
        } else {
            println!("\n📝 Tag created locally");
            println!("To push it to remote, run:");
            println!("  git push origin {}", tag_version);
        }

        println!("\n{}", "=".repeat(60));
        eprintln!("✓ {}", "Tag operation completed successfully!");
        println!("{}\n", "=".repeat(60));

        Ok(())
    }

    /// Delete a Git tag
    fn delete_tag(self) -> Result<()> {
        let tag_version = self.version.as_ref().unwrap();

        eprintln!("=== {} ===", &format!("Deleting tag: {}", tag_version));

        let project_dir = std::env::current_dir().context("Failed to get current directory")?;

        // Delete local tag
        let result = Command::new("git")
            .args(&["tag", "-d", tag_version])
            .current_dir(&project_dir)
            .status();

        match result {
            Ok(status) if status.success() => {
                eprintln!("✓ {}", &format!("Deleted local tag: {}", tag_version));
            }
            _ => {
                eprintln!("⚠ {}", &format!(
                    "Local tag '{}' not found or already deleted",
                    tag_version
                ));
            }
        }

        // Delete remote tag
        if !self.local_only {
            let result = Command::new("git")
                .args(&["push", "origin", "--delete", tag_version])
                .current_dir(&project_dir)
                .status();

            match result {
                Ok(status) if status.success() => {
                    eprintln!("✓ {}", &format!("Deleted remote tag: origin/{}", tag_version));
                }
                _ => {
                    eprintln!("⚠ {}", "Failed to delete remote tag (may not exist)");
                }
            }
        }

        eprintln!("✓ {}", "Tag deletion completed");

        Ok(())
    }

    /// List Git tags
    fn list_tags(&self) -> Result<()> {
        let project_dir = std::env::current_dir().context("Failed to get current directory")?;

        if self.remote {
            eprintln!("=== {} ===", "Remote tags");

            let output = Command::new("git")
                .args(&["ls-remote", "--tags", "origin"])
                .current_dir(&project_dir)
                .output()
                .context("Failed to list remote tags")?;

            if !output.status.success() {
                bail!("Failed to list remote tags");
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.trim().is_empty() {
                println!("  No remote tags found");
            } else {
                for line in stdout.lines() {
                    let parts: Vec<&str> = line.split('\t').collect();
                    if parts.len() >= 2 {
                        let tag = parts[1].replace("refs/tags/", "");
                        // Skip dereferenced tags (ending with ^{})
                        if !tag.ends_with("^{}") {
                            println!("  {}", tag);
                        }
                    }
                }
            }
        } else {
            eprintln!("=== {} ===", "Local tags");

            let output = Command::new("git")
                .args(&["tag", "-l"])
                .current_dir(&project_dir)
                .output()
                .context("Failed to list tags")?;

            if !output.status.success() {
                bail!("Failed to list tags");
            }

            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.trim().is_empty() {
                println!("  No local tags found");
            } else {
                let mut tags: Vec<&str> = stdout.lines().filter(|s| !s.is_empty()).collect();
                tags.sort();
                for tag in tags {
                    println!("  {}", tag);
                }
            }
        }

        println!();

        Ok(())
    }
}

/// Git information for tag message
#[derive(Debug)]
struct GitInfo {
    branch: String,
    version_code: String,
    revision: String,
    datetime: String,
}

/// Get git information
fn get_git_info(project_dir: &std::path::Path) -> Result<GitInfo> {
    // Get branch name
    let branch = Command::new("git")
        .args(&["symbolic-ref", "--short", "-q", "HEAD"])
        .current_dir(project_dir)
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    // Get version code (commit count)
    let version_code = Command::new("git")
        .args(&["rev-list", "HEAD", "--count"])
        .current_dir(project_dir)
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "0".to_string());

    // Get revision (short commit hash)
    let revision = Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .current_dir(project_dir)
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
            } else {
                None
            }
        })
        .unwrap_or_else(|| "unknown".to_string());

    // Get datetime from last commit
    let datetime = Command::new("git")
        .args(&["log", "-n1", "--format=%at"])
        .current_dir(project_dir)
        .output()
        .ok()
        .and_then(|out| {
            if out.status.success() {
                let timestamp = String::from_utf8_lossy(&out.stdout).trim().to_string();
                timestamp.parse::<i64>().ok().and_then(|ts| {
                    chrono::DateTime::from_timestamp(ts, 0)
                        .map(|dt| dt.format("%Y-%m-%d %H:%M:%S").to_string())
                })
            } else {
                None
            }
        })
        .unwrap_or_else(|| chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string());

    Ok(GitInfo {
        branch,
        version_code,
        revision,
        datetime,
    })
}

/// Generate default tag message
fn generate_tag_message(version: &str, project_name: &str, info: &GitInfo) -> String {
    format!(
        "{}\n\nPROJECT: {}\nVERSION: {}\nVERSION_CODE: {}\nREVISION: {}\nBRANCH: {}\nDATETIME: {}",
        version, project_name, version, info.version_code, info.revision, info.branch, info.datetime
    )
}

/// Check if a tag exists
fn tag_exists(project_dir: &std::path::Path, tag: &str) -> Result<bool> {
    let output = Command::new("git")
        .args(&["rev-parse", tag])
        .current_dir(project_dir)
        .output()
        .context("Failed to check tag existence")?;

    Ok(output.status.success())
}
