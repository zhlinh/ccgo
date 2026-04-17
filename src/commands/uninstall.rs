//! Uninstall a package from the global CCGO cache.
//!
//! Inverse of `ccgo install` — removes an entry under
//! `$CCGO_HOME/packages/<name>/[<version>]/` and prunes any bin symlinks
//! in `$CCGO_HOME/bin/` that point into it.

use std::path::PathBuf;

use anyhow::{anyhow, Context, Result};
use clap::Args;
use console::style;

/// Uninstall a package previously installed with `ccgo install`.
#[derive(Args, Debug)]
pub struct UninstallCommand {
    /// Package to remove. Accepts `name` (all versions) or `name@version`.
    pub spec: String,

    /// Skip confirmation prompts (for scripts).
    #[arg(long)]
    pub yes: bool,
}

impl UninstallCommand {
    pub fn execute(self, _verbose: bool) -> Result<()> {
        let (name, version) = split_spec(&self.spec);
        let home = ccgo_home_dir()?;
        let pkg_root = home.join("packages").join(name.to_lowercase());

        if !pkg_root.exists() {
            return Err(anyhow!(
                "Package '{}' not found in {}",
                name,
                pkg_root.display()
            ));
        }

        // Gather candidate version directories.
        let versions: Vec<PathBuf> = if let Some(v) = version.as_deref() {
            let v_dir = pkg_root.join(v);
            if !v_dir.exists() {
                return Err(anyhow!(
                    "Version '{}' of '{}' not installed ({})",
                    v,
                    name,
                    v_dir.display()
                ));
            }
            vec![v_dir]
        } else {
            let mut v = Vec::new();
            for entry in std::fs::read_dir(&pkg_root)? {
                let entry = entry?;
                if entry.path().is_dir() {
                    v.push(entry.path());
                }
            }
            v
        };

        // Collect bin symlinks that point into these versions before deletion.
        let bin_root = home.join("bin");
        let mut orphaned_bins: Vec<PathBuf> = Vec::new();
        if bin_root.is_dir() {
            for entry in std::fs::read_dir(&bin_root)? {
                let entry = entry?;
                let path = entry.path();
                if !path.is_symlink() {
                    continue;
                }
                let Ok(target) = std::fs::read_link(&path) else { continue };
                // canonicalize absolutely; relative links resolve against bin_root
                let abs_target = if target.is_absolute() {
                    target.clone()
                } else {
                    bin_root.join(&target)
                };
                if versions.iter().any(|v| abs_target.starts_with(v)) {
                    orphaned_bins.push(path);
                }
            }
        }

        // Preview + confirm.
        println!("{}", style("About to remove:").yellow().bold());
        for v in &versions {
            println!("   📦 {}", v.display());
        }
        for b in &orphaned_bins {
            println!("   🔗 {} (bin symlink)", b.display());
        }
        if !self.yes {
            println!("\nRe-run with --yes to confirm.");
            return Ok(());
        }

        for v in &versions {
            std::fs::remove_dir_all(v)
                .with_context(|| format!("Failed to remove {}", v.display()))?;
        }
        for b in &orphaned_bins {
            std::fs::remove_file(b).ok();
        }

        // If the package dir is now empty, remove it too.
        if let Ok(mut rd) = std::fs::read_dir(&pkg_root) {
            if rd.next().is_none() {
                let _ = std::fs::remove_dir(&pkg_root);
            }
        }

        println!("\n{}", style("✅ Uninstalled").green().bold());
        Ok(())
    }
}

/// Parse `name` or `name@version` → (name, Some(version) | None).
fn split_spec(spec: &str) -> (String, Option<String>) {
    match spec.split_once('@') {
        Some((n, v)) => (n.to_string(), Some(v.to_string())),
        None => (spec.to_string(), None),
    }
}

fn ccgo_home_dir() -> Result<PathBuf> {
    if let Ok(custom) = std::env::var("CCGO_HOME") {
        return Ok(PathBuf::from(custom));
    }
    let home = std::env::var("HOME")
        .map_err(|_| anyhow!("HOME env not set; cannot determine global cache path"))?;
    Ok(PathBuf::from(home).join(".ccgo"))
}
