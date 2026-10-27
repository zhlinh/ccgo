//! `ccgo yank` — flip a published version's `yanked` flag in a registry
//! index repo. Mirrors `cargo yank`.
//!
//! The default target is the package described by the local CCGO.toml in
//! the current directory; pass `--package` to operate on someone else's
//! package from any cwd. The version must already exist in the index;
//! re-yanking an already-yanked version is a clear error rather than a
//! silent no-op so a CI pipeline that loops on this command surfaces the
//! mistake.
//!
//! Yanking is purely a flag flip — the `VersionEntry` and its
//! `archive_url` / `checksum` are preserved, so consumers who already
//! locked this version in their `CCGO.lock` continue to fetch the same
//! bytes. New `ccgo fetch` runs (without `--locked`) skip yanked entries
//! during resolution (`resolve_dep`), so subsequent fresh resolutions
//! pick a different version.

use std::path::Path;

use anyhow::{bail, Context, Result};
use clap::Args;

use crate::config::CcgoConfig;
use crate::registry::{
    index_writer, PackageEntry, PackageIndex,
};

/// Yank (or unyank) a published package version in a registry index.
///
/// Mirrors `cargo yank --vers <V> [--undo] [<crate>]`. The package name
/// defaults to the current project's CCGO.toml `[project].name`; pass
/// `--package` to override.
#[derive(Args, Debug)]
pub struct YankCommand {
    /// Override the package name (default: read CCGO.toml in cwd).
    #[arg(long)]
    pub package: Option<String>,

    /// The version to yank or unyank. The flag is `--vers` (and the
    /// field name is `vers`) rather than `--version` to avoid colliding
    /// with clap's auto-generated `--version` flag — same reason
    /// `cargo yank --vers` does it.
    #[arg(long)]
    pub vers: String,

    /// Index repository URL or local path. Required.
    #[arg(long)]
    pub index_repo: Option<String>,

    /// Index registry name (used as the local clone subdir).
    #[arg(long)]
    pub index_name: Option<String>,

    /// Why this version is being yanked. Required when yanking; rejected
    /// when `--undo` is set.
    #[arg(long)]
    pub reason: Option<String>,

    /// Undo a previous yank — flip `yanked: false` and clear the reason.
    #[arg(long)]
    pub undo: bool,

    /// Override the auto-generated commit message.
    #[arg(long)]
    pub message: Option<String>,

    /// Push to remote after committing. Without this flag the commit is
    /// made in the local clone of the index repo; the operator can `cd`
    /// in and push manually.
    #[arg(long)]
    pub push: bool,
}

impl YankCommand {
    pub fn execute(self, verbose: bool) -> Result<()> {
        // 1. Resolve package name — explicit > CCGO.toml in cwd.
        let package_name = match &self.package {
            Some(name) => name.clone(),
            None => Self::read_package_name_from_cwd()?,
        };

        // 2. Validate flag combinations.
        let index_repo = self
            .index_repo
            .clone()
            .ok_or_else(|| anyhow::anyhow!("--index-repo is required"))?;
        let index_name = self
            .index_name
            .clone()
            .unwrap_or_else(|| "custom-index".to_string());

        if self.undo && self.reason.is_some() {
            bail!("--reason is not accepted with --undo (unyank clears the reason)");
        }
        if !self.undo && self.reason.is_none() {
            bail!(
                "--reason is required when yanking. Pass --reason \"<why>\" to leave \
                 an audit trail; or pass --undo to unyank."
            );
        }

        let action = if self.undo { "Unyanking" } else { "Yanking" };
        println!(
            "🔖 {} {} {} (registry: {})",
            action, package_name, self.vers, index_name
        );

        // 3. Clone/pull the index repo, locate the package's JSON entry.
        let index_path = index_writer::prepare_index_repo(&index_repo, &index_name, verbose)?;
        let package_rel_path = PackageIndex::package_index_path(&package_name);
        let package_file = index_path.join(&package_rel_path);

        if !package_file.exists() {
            bail!(
                "package '{}' is not in the index ({} does not exist). \
                 Did you mean to publish it first?",
                package_name,
                package_file.display()
            );
        }

        // 4. Mutate the matching VersionEntry.
        let json = std::fs::read_to_string(&package_file)
            .with_context(|| format!("failed to read {}", package_file.display()))?;
        let mut entry: PackageEntry = serde_json::from_str(&json)
            .with_context(|| format!("failed to parse {}", package_file.display()))?;

        // Snapshot the available versions BEFORE the mutable lookup so
        // the not-found error message can include them without borrowing
        // `entry.versions` twice.
        let available: Vec<String> = entry
            .versions
            .iter()
            .take(5)
            .map(|v| v.version.clone())
            .collect();

        let v = entry
            .versions
            .iter_mut()
            .find(|v| v.version == self.vers)
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "version '{}' is not in the index entry for '{}'. \
                     Available: {}",
                    self.vers,
                    package_name,
                    available.join(", ")
                )
            })?;

        if self.undo {
            if !v.yanked {
                bail!(
                    "version '{}' is not currently yanked; nothing to undo",
                    self.vers
                );
            }
            v.yanked = false;
            v.yanked_reason = None;
            println!("   ✓ Cleared yanked flag");
        } else {
            if v.yanked {
                bail!(
                    "version '{}' is already yanked (reason: {}). Pass --undo to unyank \
                     first if you want to re-yank with a different reason.",
                    self.vers,
                    v.yanked_reason.as_deref().unwrap_or("<unset>")
                );
            }
            v.yanked = true;
            v.yanked_reason = self.reason.clone();
            println!(
                "   ✓ Yanked (reason: {})",
                self.reason.as_deref().unwrap_or("")
            );
        }

        // 5. Write back, refresh metadata, commit, optionally push.
        let json = serde_json::to_string_pretty(&entry)
            .context("failed to serialize updated package entry")?;
        std::fs::write(&package_file, json)
            .with_context(|| format!("failed to write {}", package_file.display()))?;

        index_writer::update_index_metadata(&index_path, &index_name)?;

        let commit_message = self.message.clone().unwrap_or_else(|| {
            if self.undo {
                format!("unyank: {} {}", package_name, self.vers)
            } else {
                let reason = self.reason.as_deref().unwrap_or("");
                format!(
                    "yank: {} {}\n\nreason: {}",
                    package_name, self.vers, reason
                )
            }
        });

        index_writer::commit_changes(&index_path, &commit_message, verbose)?;

        if self.push {
            println!("\n📤 Pushing to remote...");
            index_writer::push_changes(&index_path, verbose)?;
            println!("✅ Pushed successfully!");
        } else {
            println!("\n💡 Changes committed locally. Use --push to push to remote.");
        }

        Ok(())
    }

    fn read_package_name_from_cwd() -> Result<String> {
        let cwd = std::env::current_dir().context("failed to read current directory")?;
        let toml_path = Self::find_ccgo_toml(&cwd)?;
        let config = CcgoConfig::load_from_path(&toml_path).with_context(|| {
            format!(
                "failed to load CCGO.toml at {} (use --package to skip CCGO.toml lookup)",
                toml_path.display()
            )
        })?;
        let package = config.package.ok_or_else(|| {
            anyhow::anyhow!(
                "no [package] section in CCGO.toml at {}; pass --package <name> explicitly",
                toml_path.display()
            )
        })?;
        Ok(package.name)
    }

    fn find_ccgo_toml(start: &Path) -> Result<std::path::PathBuf> {
        let direct = start.join("CCGO.toml");
        if direct.is_file() {
            return Ok(direct);
        }
        // Look one level down (mirrors the existing pattern in publish.rs).
        if let Ok(entries) = std::fs::read_dir(start) {
            for entry in entries.flatten() {
                if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                    let candidate = entry.path().join("CCGO.toml");
                    if candidate.is_file() {
                        return Ok(candidate);
                    }
                }
            }
        }
        bail!(
            "no CCGO.toml found in {} or its immediate subdirectories. \
             Run `ccgo yank` from the project root, or pass --package <name>.",
            start.display()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::VersionEntry;

    fn entry_with_versions(versions: Vec<VersionEntry>) -> PackageEntry {
        PackageEntry {
            name: "leaf".into(),
            description: "x".into(),
            repository: "x".into(),
            homepage: None,
            license: None,
            keywords: Vec::new(),
            platforms: Vec::new(),
            versions,
        }
    }

    fn version(v: &str, tag: &str, yanked: bool) -> VersionEntry {
        VersionEntry {
            version: v.into(),
            tag: tag.into(),
            checksum: None,
            archive_url: None,
            archive_format: None,
            released_at: None,
            yanked,
            yanked_reason: None,
        }
    }

    /// Run a single yank/unyank pass against an in-memory PackageEntry,
    /// returning the resulting (yanked, reason) so tests can pin the
    /// state machine without spawning git or touching disk.
    fn apply_yank_in_memory(
        mut entry: PackageEntry,
        target_version: &str,
        undo: bool,
        reason: Option<&str>,
    ) -> Result<(bool, Option<String>)> {
        let v = entry
            .versions
            .iter_mut()
            .find(|v| v.version == target_version)
            .ok_or_else(|| anyhow::anyhow!("version '{}' not found", target_version))?;

        if undo {
            if !v.yanked {
                bail!("not currently yanked");
            }
            v.yanked = false;
            v.yanked_reason = None;
        } else {
            if v.yanked {
                bail!("already yanked");
            }
            v.yanked = true;
            v.yanked_reason = reason.map(str::to_string);
        }
        Ok((v.yanked, v.yanked_reason.clone()))
    }

    #[test]
    fn yank_flips_flag_and_records_reason() {
        let entry = entry_with_versions(vec![version("1.0.0", "v1.0.0", false)]);
        let (yanked, reason) =
            apply_yank_in_memory(entry, "1.0.0", false, Some("broken bytes")).unwrap();
        assert!(yanked);
        assert_eq!(reason.as_deref(), Some("broken bytes"));
    }

    #[test]
    fn yank_rejects_already_yanked() {
        let entry = entry_with_versions(vec![version("1.0.0", "v1.0.0", true)]);
        let err = apply_yank_in_memory(entry, "1.0.0", false, Some("oops")).unwrap_err();
        assert!(err.to_string().contains("already yanked"), "got: {err}");
    }

    #[test]
    fn unyank_clears_flag_and_reason() {
        let mut entry = entry_with_versions(vec![version("1.0.0", "v1.0.0", true)]);
        entry.versions[0].yanked_reason = Some("broken".into());
        let (yanked, reason) = apply_yank_in_memory(entry, "1.0.0", true, None).unwrap();
        assert!(!yanked);
        assert!(reason.is_none());
    }

    #[test]
    fn unyank_rejects_when_not_yanked() {
        let entry = entry_with_versions(vec![version("1.0.0", "v1.0.0", false)]);
        let err = apply_yank_in_memory(entry, "1.0.0", true, None).unwrap_err();
        assert!(
            err.to_string().contains("not currently yanked"),
            "got: {err}"
        );
    }

    #[test]
    fn yank_unknown_version_errors() {
        let entry = entry_with_versions(vec![version("1.0.0", "v1.0.0", false)]);
        let err = apply_yank_in_memory(entry, "9.9.9", false, Some("x")).unwrap_err();
        assert!(err.to_string().contains("not found"), "got: {err}");
    }
}
