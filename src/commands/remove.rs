//! Remove command - Remove a dependency from CCGO.toml
//!
//! Usage:
//!   ccgo remove <name>

use anyhow::{bail, Context, Result};
use clap::Args;
use std::fs;
use std::path::Path;

use crate::config::CcgoConfig;

/// Remove a dependency from CCGO.toml
#[derive(Args, Debug)]
pub struct RemoveCommand {
    /// Dependency name to remove
    pub name: String,

    /// Also remove the installed dependency from .ccgo/deps/
    #[arg(long)]
    pub purge: bool,
}

impl RemoveCommand {
    /// Execute the remove command
    pub fn execute(self, _verbose: bool) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Remove - Remove Dependency from CCGO.toml");
        println!("{}", "=".repeat(80));

        // Load existing CCGO.toml
        let config_path = Path::new("CCGO.toml");
        if !config_path.exists() {
            bail!("CCGO.toml not found in current directory.");
        }

        let content = fs::read_to_string(config_path)
            .context("Failed to read CCGO.toml")?;

        // Parse to validate and check existence
        let config = CcgoConfig::parse(&content)?;
        if !config.dependencies.iter().any(|d| d.name == self.name) {
            bail!("Dependency '{}' not found in CCGO.toml", self.name);
        }

        println!("\n📦 Removing dependency: {}", self.name);

        // Remove dependency from TOML
        let new_content = self.remove_dependency_from_toml(&content)?;
        fs::write(config_path, new_content)
            .context("Failed to write CCGO.toml")?;

        println!("   ✓ Removed '{}' from CCGO.toml", self.name);

        // Remove installed files if --purge
        if self.purge {
            let dep_path = Path::new(".ccgo").join("deps").join(&self.name);
            if dep_path.exists() {
                println!("\n🗑  Purging installed files...");
                if dep_path.is_symlink() {
                    fs::remove_file(&dep_path)
                        .with_context(|| format!("Failed to remove symlink: {}", dep_path.display()))?;
                } else {
                    fs::remove_dir_all(&dep_path)
                        .with_context(|| format!("Failed to remove directory: {}", dep_path.display()))?;
                }
                println!("   ✓ Removed {}", dep_path.display());
            } else {
                println!("\n   ℹ️  No installed files found at {}", dep_path.display());
            }

            // Update lock file
            self.update_lock_file()?;
        } else {
            println!("\n💡 Run 'ccgo install --force' to update installed dependencies");
            println!("   Or use 'ccgo remove {} --purge' to also delete installed files", self.name);
        }

        println!("\n✓ Dependency '{}' removed successfully", self.name);

        Ok(())
    }

    /// Remove dependency entry from TOML content
    fn remove_dependency_from_toml(&self, content: &str) -> Result<String> {
        let lines: Vec<&str> = content.lines().collect();
        let mut new_lines = Vec::new();
        let mut i = 0;
        let mut found = false;

        while i < lines.len() {
            let line = lines[i];

            // Check if this is the start of a dependency section
            if line.trim() == "[[dependencies]]" {
                // Look ahead to find the name
                let mut j = i + 1;
                let mut is_target_dep = false;

                while j < lines.len() {
                    let next_line = lines[j].trim();

                    // Stop at next section
                    if next_line.starts_with("[[") || next_line.starts_with('[') && !next_line.starts_with("[[dependencies]]") {
                        break;
                    }

                    // Check if this dependency matches our target
                    if next_line.starts_with("name = ") {
                        let name_value = next_line.trim_start_matches("name = ").trim_matches('"');
                        if name_value == self.name {
                            is_target_dep = true;
                            found = true;
                        }
                        break;
                    }

                    j += 1;
                }

                if is_target_dep {
                    // Skip this entire dependency block
                    i = j + 1;
                    while i < lines.len() {
                        let skip_line = lines[i].trim();
                        if skip_line.starts_with("[[") || (skip_line.starts_with('[') && !skip_line.is_empty()) {
                            break;
                        }
                        if skip_line.is_empty() && i + 1 < lines.len() {
                            let next = lines[i + 1].trim();
                            if next.starts_with("[[") || next.starts_with('[') {
                                i += 1;
                                break;
                            }
                        }
                        i += 1;
                    }
                    continue;
                }
            }

            new_lines.push(line);
            i += 1;
        }

        if !found {
            bail!("Failed to locate dependency '{}' in CCGO.toml", self.name);
        }

        // Join lines and clean up multiple blank lines
        let result = new_lines.join("\n");
        Ok(Self::clean_blank_lines(&result))
    }

    /// Clean up multiple consecutive blank lines
    fn clean_blank_lines(content: &str) -> String {
        let lines: Vec<&str> = content.lines().collect();
        let mut result = Vec::new();
        let mut prev_blank = false;

        for line in lines {
            let is_blank = line.trim().is_empty();
            if is_blank && prev_blank {
                continue; // Skip consecutive blank lines
            }
            result.push(line);
            prev_blank = is_blank;
        }

        result.join("\n").trim_end().to_string() + "\n"
    }

    /// Update lock file to remove the dependency
    fn update_lock_file(&self) -> Result<()> {
        let lock_path = Path::new("CCGO.toml.lock");
        if !lock_path.exists() {
            return Ok(());
        }

        let content = fs::read_to_string(lock_path)?;
        let lines: Vec<&str> = content.lines().collect();
        let mut new_lines = Vec::new();
        let mut i = 0;

        while i < lines.len() {
            let line = lines[i];

            // Check for [dependencies.NAME] section
            if line.starts_with("[dependencies.") {
                let section_name = line
                    .trim_start_matches("[dependencies.")
                    .trim_end_matches(']');

                if section_name == self.name || section_name.starts_with(&format!("{}.", self.name)) {
                    // Skip this section
                    i += 1;
                    while i < lines.len() {
                        let skip_line = lines[i].trim();
                        if skip_line.starts_with('[') && !skip_line.is_empty() {
                            break;
                        }
                        i += 1;
                    }
                    continue;
                }
            }

            new_lines.push(line);
            i += 1;
        }

        let result = new_lines.join("\n");
        fs::write(lock_path, Self::clean_blank_lines(&result))?;
        println!("   ✓ Updated CCGO.toml.lock");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_dependency() {
        let content = r#"[package]
name = "myproject"
version = "1.0.0"

[[dependencies]]
name = "dep1"
version = "^1.0"

[[dependencies]]
name = "dep2"
version = "^2.0"

[[dependencies]]
name = "dep3"
version = "^3.0"
"#;

        let cmd = RemoveCommand {
            name: "dep2".to_string(),
            purge: false,
        };

        let result = cmd.remove_dependency_from_toml(content).unwrap();
        assert!(result.contains("dep1"));
        assert!(!result.contains("dep2"));
        assert!(result.contains("dep3"));
    }

    #[test]
    fn test_clean_blank_lines() {
        let content = "line1\n\n\nline2\n\n\n\nline3";
        let cleaned = RemoveCommand::clean_blank_lines(content);
        assert_eq!(cleaned, "line1\n\nline2\n\nline3\n");
    }
}
