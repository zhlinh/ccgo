//! Search command - Search for packages in collections and registries
//!
//! Usage:
//!   ccgo search <keyword>                    # Search all sources
//!   ccgo search <keyword> --collection <name> # Search specific collection
//!   ccgo search <keyword> --registry <name>   # Search specific registry
//!   ccgo search <keyword> --details           # Show detailed information

use anyhow::{Context, Result};
use clap::Args;

use crate::collection::CollectionManager;
use crate::registry::PackageIndex;

/// Search for packages in collections and registries
#[derive(Args, Debug)]
pub struct SearchCommand {
    /// Search keyword or pattern
    pub query: String,

    /// Search only in specific collection
    #[arg(long, short = 'c')]
    pub collection: Option<String>,

    /// Search only in specific registry
    #[arg(long, short = 'r')]
    pub registry: Option<String>,

    /// Show detailed package information
    #[arg(long, short = 'd')]
    pub details: bool,

    /// Limit number of results
    #[arg(long, default_value = "20")]
    pub limit: usize,

    /// Search collections only (skip registries)
    #[arg(long)]
    pub collections_only: bool,

    /// Search registries only (skip collections)
    #[arg(long)]
    pub registries_only: bool,
}

impl SearchCommand {
    /// Execute the search command
    pub fn execute(self, _verbose: bool) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Search - Package Discovery");
        println!("{}", "=".repeat(80));

        let mut total_results = 0;
        let mut displayed = 0;

        // Search registries (unless collections_only)
        if !self.collections_only {
            let registry_count = self.search_registries(&mut total_results, &mut displayed)?;
            if registry_count > 0 {
                println!();
            }
        }

        // Search collections (unless registries_only)
        if !self.registries_only {
            self.search_collections(&mut total_results, &mut displayed)?;
        }

        // Summary
        if total_results == 0 {
            println!("\n✗ No packages found matching '{}'", self.query);
            println!("\n💡 Try:");
            println!("   - Different search terms");
            println!("   - Update registries: ccgo registry update");
            println!("   - Refresh collections: ccgo collection refresh");
        } else {
            println!("\n💡 Add a package with:");
            println!("   ccgo add <name> --git <repository-url>");
            println!("   ccgo add github:user/repo --latest");
            println!("   # or in CCGO.toml: fmt = \"^10.1\"");
        }

        Ok(())
    }

    /// Search in registries
    fn search_registries(&self, total: &mut usize, displayed: &mut usize) -> Result<usize> {
        let mut index = PackageIndex::new();
        index.ensure_default_registry();

        let registries = index.list_registries();
        if registries.is_empty() {
            return Ok(0);
        }

        let results = if let Some(ref reg) = self.registry {
            let packages = index.search_packages(reg, &self.query).unwrap_or_default();
            packages
                .into_iter()
                .map(|p| (reg.clone(), p))
                .collect::<Vec<_>>()
        } else {
            index.search_all(&self.query).unwrap_or_default()
        };

        if results.is_empty() {
            return Ok(0);
        }

        *total += results.len();

        println!(
            "\n📦 Registry results ({}):\n",
            results.len()
        );

        let remaining_limit = self.limit.saturating_sub(*displayed);
        for (idx, (registry_name, package)) in results.iter().take(remaining_limit).enumerate() {
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

            println!("{}. {} v{}", *displayed + idx + 1, package.name, latest_version);
            println!("   {}", package.description);

            if self.details {
                println!("   Repository: {}", package.repository);
                println!("   Registry: {}", registry_name);
                if !package.platforms.is_empty() {
                    println!("   Platforms: {}", package.platforms.join(", "));
                }
                if let Some(ref license) = package.license {
                    println!("   License: {}", license);
                }
                if !package.keywords.is_empty() {
                    println!("   Keywords: {}", package.keywords.join(", "));
                }
            } else {
                println!("   Registry: {}", registry_name);
            }
            println!();
        }

        let count = std::cmp::min(results.len(), remaining_limit);
        *displayed += count;

        if results.len() > remaining_limit {
            println!(
                "   ... and {} more registry results",
                results.len() - remaining_limit
            );
        }

        Ok(results.len())
    }

    /// Search in collections
    fn search_collections(&self, total: &mut usize, displayed: &mut usize) -> Result<()> {
        let manager = CollectionManager::new();

        let collections = manager.load_index().unwrap_or_default();
        if collections.is_empty() {
            return Ok(());
        }

        let results = manager
            .search(&self.query, self.collection.as_deref())
            .unwrap_or_default();

        if results.is_empty() {
            return Ok(());
        }

        *total += results.len();

        println!(
            "\n📚 Collection results ({}):\n",
            results.len()
        );

        let remaining_limit = self.limit.saturating_sub(*displayed);
        for (idx, (collection_name, package)) in results.iter().take(remaining_limit).enumerate() {
            println!("{}. {} v{}", *displayed + idx + 1, package.name, package.version);
            println!("   {}", package.summary);

            if self.details {
                println!("   Repository: {}", package.repository);
                println!("   Collection: {}", collection_name);
                if !package.platforms.is_empty() {
                    println!("   Platforms: {}", package.platforms.join(", "));
                }
                if let Some(ref license) = package.license {
                    println!("   License: {}", license);
                }
                if !package.keywords.is_empty() {
                    println!("   Keywords: {}", package.keywords.join(", "));
                }
            } else {
                println!("   Collection: {}", collection_name);
            }
            println!();
        }

        *displayed += std::cmp::min(results.len(), remaining_limit);

        if results.len() > remaining_limit {
            println!(
                "   ... and {} more collection results",
                results.len() - remaining_limit
            );
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_command_structure() {
        let cmd = SearchCommand {
            query: "json".to_string(),
            collection: None,
            registry: None,
            details: false,
            limit: 20,
            collections_only: false,
            registries_only: false,
        };

        assert_eq!(cmd.query, "json");
        assert_eq!(cmd.limit, 20);
    }
}
