//! Registry command - Manage package registries
//!
//! Usage:
//!   ccgo registry add <name> <url>    # Add a registry
//!   ccgo registry list                 # List all registries
//!   ccgo registry remove <name>        # Remove a registry
//!   ccgo registry update [name]        # Update registry/registries
//!   ccgo registry info <name>          # Show registry info
//!   ccgo registry search <query>       # Search packages across registries

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::registry::{PackageIndex, DEFAULT_REGISTRY};

/// Manage package registries
#[derive(Args, Debug)]
pub struct RegistryCommand {
    #[command(subcommand)]
    pub action: RegistryAction,
}

/// Registry subcommands
#[derive(Subcommand, Debug)]
pub enum RegistryAction {
    /// Add a new registry
    Add {
        /// Registry name (e.g., "company", "private")
        name: String,

        /// Git URL of the registry index repository
        url: String,
    },

    /// List all configured registries
    List {
        /// Show detailed information
        #[arg(long, short = 'd')]
        details: bool,
    },

    /// Remove a registry
    Remove {
        /// Registry name to remove
        name: String,
    },

    /// Update registry index (clone or pull)
    Update {
        /// Registry name to update (default: all)
        name: Option<String>,
    },

    /// Show registry information
    Info {
        /// Registry name
        name: String,
    },

    /// Search packages across registries
    Search {
        /// Search query
        query: String,

        /// Search in specific registry only
        #[arg(long, short = 'r')]
        registry: Option<String>,

        /// Limit number of results
        #[arg(long, default_value = "20")]
        limit: usize,
    },
}

impl RegistryCommand {
    /// Execute the registry command
    pub fn execute(self, verbose: bool) -> Result<()> {
        match self.action {
            RegistryAction::Add { ref name, ref url } => Self::add_registry(name, url),
            RegistryAction::List { details } => Self::list_registries(details),
            RegistryAction::Remove { ref name } => Self::remove_registry(name),
            RegistryAction::Update { ref name } => Self::update_registries(name.as_deref(), verbose),
            RegistryAction::Info { ref name } => Self::show_info(name),
            RegistryAction::Search {
                ref query,
                ref registry,
                limit,
            } => Self::search_packages(query, registry.as_deref(), limit),
        }
    }

    /// Add a registry
    fn add_registry(name: &str, url: &str) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Registry - Add Registry");
        println!("{}", "=".repeat(80));

        if name == DEFAULT_REGISTRY {
            println!("\n⚠️  Cannot override the default registry name");
            println!("   Please use a different name for custom registries");
            return Ok(());
        }

        let mut index = PackageIndex::new();

        // Check if already exists
        if index.get_registry(name).is_some() {
            println!("\n⚠️  Registry '{}' already exists", name);
            println!("   Remove it first with: ccgo registry remove {}", name);
            return Ok(());
        }

        index.add_registry(name, url);

        println!("\n✓ Registry added: {}", name);
        println!("  URL: {}", url);
        println!("\n📥 Cloning index repository...");

        match index.update_registry(name) {
            Ok(result) => {
                println!("   {}", result);
                println!("\n💡 Use 'ccgo registry search <query>' to find packages");
            }
            Err(e) => {
                println!("   ⚠️  Failed to clone: {}", e);
                println!("   The registry was added but not cached locally.");
                println!("   Try again with: ccgo registry update {}", name);
            }
        }

        Ok(())
    }

    /// List registries
    fn list_registries(details: bool) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Registry - Configured Registries");
        println!("{}", "=".repeat(80));

        let mut index = PackageIndex::new();
        index.ensure_default_registry();

        let registries = index.list_registries();

        if registries.is_empty() {
            println!("\n⚠️  No registries configured");
            println!("\n💡 Add a registry with: ccgo registry add <name> <url>");
            return Ok(());
        }

        println!("\nRegistries:");
        for registry in registries {
            let cached = if index.is_cached(&registry.name) {
                "✓"
            } else {
                "✗"
            };

            println!(
                "\n  {} {} {}",
                cached,
                registry.name,
                if registry.name == DEFAULT_REGISTRY {
                    "(default)"
                } else {
                    ""
                }
            );
            println!("    URL: {}", registry.url);

            if details {
                if let Ok(Some(metadata)) = index.load_metadata(&registry.name) {
                    println!("    Version: {}", metadata.version);
                    println!("    Packages: {}", metadata.package_count);
                    println!("    Updated: {}", metadata.updated_at);
                }
            }
        }

        println!("\n💡 Update registries with: ccgo registry update");

        Ok(())
    }

    /// Remove a registry
    fn remove_registry(name: &str) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Registry - Remove Registry");
        println!("{}", "=".repeat(80));

        let mut index = PackageIndex::new();

        match index.remove_registry(name) {
            Ok(()) => {
                println!("\n✓ Registry removed: {}", name);
            }
            Err(e) => {
                println!("\n✗ Failed to remove registry: {}", e);
            }
        }

        Ok(())
    }

    /// Update registries
    fn update_registries(name: Option<&str>, verbose: bool) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Registry - Update Registry Index");
        println!("{}", "=".repeat(80));

        let mut index = PackageIndex::new();
        index.ensure_default_registry();

        if let Some(name) = name {
            // Update specific registry
            println!("\n📥 Updating registry: {}...", name);

            match index.update_registry(name) {
                Ok(result) => {
                    println!("   ✓ {}", result);
                }
                Err(e) => {
                    println!("   ✗ Failed: {}", e);
                }
            }
        } else {
            // Update all registries
            let results = index.update_all();

            println!("\nUpdating {} registry(ies)...\n", results.len());

            for (registry_name, result) in results {
                match result {
                    Ok(update_result) => {
                        println!("  ✓ {}: {}", registry_name, update_result);
                    }
                    Err(e) => {
                        println!("  ✗ {}: {}", registry_name, e);
                        if verbose {
                            println!("    {}", e);
                        }
                    }
                }
            }
        }

        println!("\n💡 Search packages with: ccgo registry search <query>");

        Ok(())
    }

    /// Show registry info
    fn show_info(name: &str) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Registry - Registry Information");
        println!("{}", "=".repeat(80));

        let index = PackageIndex::new();

        let registry = index
            .get_registry(name)
            .with_context(|| format!("Registry not found: {}", name))?;

        println!("\nRegistry: {}", registry.name);
        println!("  URL: {}", registry.url);
        println!("  Cached: {}", index.is_cached(name));

        if let Ok(Some(metadata)) = index.load_metadata(name) {
            println!("\nIndex Metadata:");
            println!("  Name: {}", metadata.name);
            println!("  Description: {}", metadata.description);
            println!("  Version: {}", metadata.version);
            println!("  Packages: {}", metadata.package_count);
            println!("  Last Updated: {}", metadata.updated_at);
            if let Some(homepage) = metadata.homepage {
                println!("  Homepage: {}", homepage);
            }
        } else {
            println!("\n⚠️  Registry not cached locally");
            println!("   Run 'ccgo registry update {}' to fetch the index", name);
        }

        Ok(())
    }

    /// Search packages
    fn search_packages(query: &str, registry: Option<&str>, limit: usize) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Registry - Package Search");
        println!("{}", "=".repeat(80));

        let mut index = PackageIndex::new();
        index.ensure_default_registry();

        println!(
            "\n🔍 Searching for '{}' in {}...",
            query,
            registry.unwrap_or("all registries")
        );

        let results = if let Some(reg) = registry {
            let packages = index.search_packages(reg, query)?;
            packages
                .into_iter()
                .map(|p| (reg.to_string(), p))
                .collect()
        } else {
            index.search_all(query)?
        };

        if results.is_empty() {
            println!("\n✗ No packages found matching '{}'", query);
            println!("\n💡 Try:");
            println!("   - Different search terms");
            println!("   - Update registries: ccgo registry update");
            return Ok(());
        }

        let total = results.len();
        let display_count = std::cmp::min(total, limit);

        println!("\nFound {} package(s):\n", total);

        for (idx, (registry_name, package)) in results.iter().take(display_count).enumerate() {
            // Get latest version
            let latest_version = package
                .versions
                .iter()
                .filter(|v| !v.yanked)
                .max_by(|a, b| {
                    let ver_a = crate::registry::SemVer::parse(&a.version);
                    let ver_b = crate::registry::SemVer::parse(&b.version);
                    match (ver_a, ver_b) {
                        (Some(a), Some(b)) => a.cmp(&b),
                        _ => a.version.cmp(&b.version),
                    }
                })
                .map(|v| v.version.as_str())
                .unwrap_or("?");

            println!("{}. {} v{}", idx + 1, package.name, latest_version);
            println!("   {}", package.description);
            println!("   Registry: {}", registry_name);

            if !package.keywords.is_empty() {
                println!("   Keywords: {}", package.keywords.join(", "));
            }

            println!();
        }

        if total > display_count {
            println!(
                "... and {} more. Use --limit {} to see all results",
                total - display_count,
                total
            );
        }

        println!("💡 Add a package with:");
        println!("   ccgo add <name> --version <version>");
        println!("   # or in CCGO.toml: fmt = \"^10.1\"");

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_action_parsing() {
        // Just verify the enum variants exist
        let _add = RegistryAction::Add {
            name: "test".to_string(),
            url: "https://example.com".to_string(),
        };
        let _list = RegistryAction::List { details: false };
        let _remove = RegistryAction::Remove {
            name: "test".to_string(),
        };
        let _update = RegistryAction::Update { name: None };
    }
}
