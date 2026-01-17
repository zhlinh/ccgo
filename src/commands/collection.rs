//! Collection command - Manage package collections
//!
//! Usage:
//!   ccgo collection add <url>        # Add a collection
//!   ccgo collection list              # List all collections
//!   ccgo collection remove <name>     # Remove a collection
//!   ccgo collection refresh [name]    # Refresh collection(s)

use anyhow::{Context, Result};
use clap::{Args, Subcommand};

use crate::collection::CollectionManager;

/// Manage package collections
#[derive(Args, Debug)]
pub struct CollectionCommand {
    #[command(subcommand)]
    pub action: CollectionAction,
}

/// Collection subcommands
#[derive(Subcommand, Debug)]
pub enum CollectionAction {
    /// Add a new collection
    Add {
        /// Collection URL (supports file://, http://, https://)
        url: String,
    },

    /// List all subscribed collections
    List {
        /// Show detailed information
        #[arg(long, short = 'd')]
        details: bool,
    },

    /// Remove a collection
    Remove {
        /// Collection name or URL
        name_or_url: String,
    },

    /// Refresh collection(s)
    Refresh {
        /// Collection name to refresh (default: all)
        name: Option<String>,
    },
}

impl CollectionCommand {
    /// Execute the collection command
    pub fn execute(self, _verbose: bool) -> Result<()> {
        match self.action {
            CollectionAction::Add { ref url } => self.add_collection(url),
            CollectionAction::List { details } => self.list_collections(details),
            CollectionAction::Remove { ref name_or_url } => self.remove_collection(name_or_url),
            CollectionAction::Refresh { ref name } => self.refresh_collections(name.as_deref()),
        }
    }

    /// Add a collection
    fn add_collection(&self, url: &str) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Collection - Add Collection");
        println!("{}", "=".repeat(80));

        println!("\n📥 Fetching collection from: {}", url);

        let manager = CollectionManager::new();
        let info = manager
            .add_collection(url)
            .context("Failed to add collection")?;

        println!("\n✓ Collection added successfully!");
        println!("  Name: {}", info.name);
        println!("  URL: {}", info.url);
        println!("  Added: {}", info.added_at);

        // Load and display package count
        if let Ok(collection) = manager.load_collection(&info.name) {
            println!("  Packages: {}", collection.packages.len());
            println!("\n💡 Use 'ccgo search <keyword>' to discover packages");
        }

        Ok(())
    }

    /// List collections
    fn list_collections(&self, details: bool) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Collection - Subscribed Collections");
        println!("{}", "=".repeat(80));

        let manager = CollectionManager::new();
        let collections = manager.load_index().context("Failed to load collections")?;

        if collections.is_empty() {
            println!("\n✓ No collections subscribed yet");
            println!("\n💡 Add a collection with: ccgo collection add <url>");
            println!("   Example collections:");
            println!("     - Official: https://ccgo.dev/collections/official.json");
            println!("     - Community: https://ccgo.dev/collections/community.json");
            println!("     - Local: file:///path/to/my-collection.json");
            return Ok(());
        }

        println!("\nFound {} collection(s):\n", collections.len());

        for (idx, info) in collections.iter().enumerate() {
            println!("{}. {} ({})", idx + 1, info.name, info.url);

            if details {
                println!("   Added: {}", info.added_at);
                if let Some(ref refreshed) = info.last_refresh {
                    println!("   Last refreshed: {}", refreshed);
                }

                // Load collection to show package count
                if let Ok(collection) = manager.load_collection(&info.name) {
                    println!("   Packages: {}", collection.packages.len());
                    if let Some(ref desc) = Some(&collection.description) {
                        println!("   Description: {}", desc);
                    }
                }
                println!();
            }
        }

        if !details {
            println!("\n💡 Use --details to see more information");
        }

        Ok(())
    }

    /// Remove a collection
    fn remove_collection(&self, name_or_url: &str) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Collection - Remove Collection");
        println!("{}", "=".repeat(80));

        println!("\n🗑️  Removing collection: {}", name_or_url);

        let manager = CollectionManager::new();
        manager
            .remove_collection(name_or_url)
            .context("Failed to remove collection")?;

        println!("\n✓ Collection removed successfully");

        Ok(())
    }

    /// Refresh collections
    fn refresh_collections(&self, name: Option<&str>) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Collection - Refresh Collections");
        println!("{}", "=".repeat(80));

        let manager = CollectionManager::new();

        if let Some(name) = name {
            // Refresh specific collection
            println!("\n🔄 Refreshing collection: {}", name);

            let info = manager
                .refresh_collection(name)
                .context("Failed to refresh collection")?;

            println!("\n✓ Collection refreshed successfully!");
            println!("  Name: {}", info.name);
            if let Some(ref refreshed) = info.last_refresh {
                println!("  Refreshed: {}", refreshed);
            }

            // Show package count
            if let Ok(collection) = manager.load_collection(&info.name) {
                println!("  Packages: {}", collection.packages.len());
            }
        } else {
            // Refresh all collections
            println!("\n🔄 Refreshing all collections...");

            let results = manager.refresh_all().context("Failed to refresh collections")?;

            if results.is_empty() {
                println!("\n✓ No collections to refresh");
                return Ok(());
            }

            println!();
            let mut success_count = 0;
            let mut fail_count = 0;

            for (name, result) in results {
                match result {
                    Ok(()) => {
                        println!("  ✓ {}", name);
                        success_count += 1;
                    }
                    Err(e) => {
                        println!("  ✗ {}: {}", name, e);
                        fail_count += 1;
                    }
                }
            }

            println!("\n✓ Refreshed {} collection(s)", success_count);
            if fail_count > 0 {
                println!("⚠️  Failed to refresh {} collection(s)", fail_count);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_collection_command_structure() {
        // Just verify the command structure compiles
        let _cmd = CollectionCommand {
            action: CollectionAction::List { details: false },
        };
    }
}
