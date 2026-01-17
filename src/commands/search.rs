//! Search command - Search for packages in collections
//!
//! Usage:
//!   ccgo search <keyword>                    # Search all collections
//!   ccgo search <keyword> --collection <name> # Search specific collection
//!   ccgo search <keyword> --details           # Show detailed information

use anyhow::{Context, Result};
use clap::Args;

use crate::collection::CollectionManager;

/// Search for packages in collections
#[derive(Args, Debug)]
pub struct SearchCommand {
    /// Search keyword or pattern
    pub query: String,

    /// Search only in specific collection
    #[arg(long, short = 'c')]
    pub collection: Option<String>,

    /// Show detailed package information
    #[arg(long, short = 'd')]
    pub details: bool,

    /// Limit number of results
    #[arg(long, default_value = "20")]
    pub limit: usize,
}

impl SearchCommand {
    /// Execute the search command
    pub fn execute(self, _verbose: bool) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Search - Package Discovery");
        println!("{}", "=".repeat(80));

        let manager = CollectionManager::new();

        // Check if any collections are subscribed
        let collections = manager.load_index().context("Failed to load collections")?;
        if collections.is_empty() {
            println!("\n⚠️  No collections subscribed yet");
            println!("\n💡 Add a collection first:");
            println!("   ccgo collection add <url>");
            println!("\n   Example:");
            println!("   ccgo collection add file:///path/to/collection.json");
            return Ok(());
        }

        // Perform search
        println!(
            "\n🔍 Searching for '{}' in {}{}...",
            self.query,
            if let Some(ref coll) = self.collection {
                format!("collection '{}'", coll)
            } else {
                format!("{} collection(s)", collections.len())
            },
            ""
        );

        let results = manager
            .search(&self.query, self.collection.as_deref())
            .context("Failed to search packages")?;

        if results.is_empty() {
            println!("\n✗ No packages found matching '{}'", self.query);
            println!("\n💡 Try:");
            println!("   - Using different keywords");
            println!("   - Refreshing collections: ccgo collection refresh");
            println!("   - Adding more collections: ccgo collection add <url>");
            return Ok(());
        }

        // Display results
        let total = results.len();
        let display_count = std::cmp::min(total, self.limit);

        println!("\nFound {} package(s):\n", total);

        for (idx, (collection_name, package)) in results.iter().take(display_count).enumerate() {
            println!("{}. {} v{}", idx + 1, package.name, package.version);
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

        if total > display_count {
            println!(
                "... and {} more. Use --limit {} to see all results",
                total - display_count,
                total
            );
        }

        println!("💡 Add a package with:");
        println!("   ccgo add <name> --git <repository-url>");
        println!("   ccgo add <name> --version <version>");

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
            details: false,
            limit: 20,
        };

        assert_eq!(cmd.query, "json");
        assert_eq!(cmd.limit, 20);
    }
}
