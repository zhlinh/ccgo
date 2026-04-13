//! `ccgo release` — bump the project version, sync platform manifests,
//! update CHANGELOG.md, and (by default) commit + tag.
//!
//! Companion to `scripts/release.sh`, which releases the ccgo CLI itself.
//! This command releases a *user* CCGO project that has a `CCGO.toml` at its
//! root. The flow is modeled after `npm version`:
//!
//! ```text
//! ccgo release patch              # bump + sync + changelog + commit + tag
//! ccgo release 1.2.3              # pin to an explicit version
//! ccgo release patch --push       # also push commit and tag to origin
//! ccgo release --changelog-only   # just regenerate CHANGELOG.md
//! ```
//!
//! Defaults to aborting on a dirty working tree so we never stage someone's
//! half-finished work; `--allow-dirty` escapes that check for scripted use.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use chrono::Local;
use clap::Args;
use console::style;
use regex::Regex;
use semver::Version;

use crate::config::CcgoConfig;
use crate::utils::{changelog, git, version_sync};

/// Bump the project version, sync platform manifests, update CHANGELOG.md, and commit.
#[derive(Args, Debug)]
#[command(disable_version_flag = true)]
pub struct ReleaseCommand {
    /// Version bump type or explicit X.Y.Z (required unless --changelog-only).
    ///
    /// Accepted forms: `patch`, `minor`, `major`, or a literal semver like `1.2.3`.
    #[arg(value_name = "BUMP-OR-VERSION")]
    pub version: Option<String>,

    /// Also push commit and tag to the `origin` remote after committing.
    #[arg(long)]
    pub push: bool,

    /// Skip creating the git commit and tag (leaves modified files staged-ready).
    #[arg(long)]
    pub no_commit: bool,

    /// Skip updating CHANGELOG.md.
    #[arg(long)]
    pub no_changelog: bool,

    /// Skip syncing the Android/OHOS platform manifests.
    #[arg(long)]
    pub no_sync: bool,

    /// Print the planned changes without writing anything.
    #[arg(long)]
    pub dry_run: bool,

    /// Proceed even if the working tree has uncommitted changes.
    ///
    /// Use with care — a dirty tree means `git commit` will pick up files you
    /// didn't mean to release together with the version bump.
    #[arg(long)]
    pub allow_dirty: bool,

    /// Tag prefix applied to the version (e.g. `v` produces `v1.2.3`).
    #[arg(long, default_value = "v")]
    pub tag_prefix: String,

    /// Commit message template. `{version}` is replaced with the new version.
    #[arg(long, default_value = "chore: release v{version}")]
    pub message: String,

    /// Regenerate the entire CHANGELOG.md from git history and exit.
    ///
    /// Mutually exclusive with a version argument — there's nothing to bump.
    #[arg(long, conflicts_with_all = ["version", "push", "no_commit", "no_sync"])]
    pub changelog_only: bool,
}

impl ReleaseCommand {
    pub fn execute(self, _verbose: bool) -> Result<()> {
        let project_root = git::find_ancestor_with(
            &std::env::current_dir().context("failed to get current directory")?,
            "CCGO.toml",
        )
        .context("CCGO.toml not found in this directory or any parent")?;

        if self.changelog_only {
            return self.run_changelog_only(&project_root);
        }

        let (current, new_version) = self.resolve_versions(&project_root)?;
        if new_version == current {
            println!(
                "{} current version is already {}",
                style("Nothing to do:").yellow(),
                current
            );
            return Ok(());
        }

        self.print_header(&project_root, &current, &new_version);
        self.check_clean_tree(&project_root)?;

        let staged = self.write_project_files(&project_root, &current, &new_version)?;

        if self.dry_run {
            println!();
            println!("{}", style("Dry run complete. No changes written.").bold());
            return Ok(());
        }

        if self.no_commit {
            println!();
            println!(
                "{} version files updated. Commit + tag skipped (--no-commit).",
                style("Done:").green()
            );
            return Ok(());
        }

        self.commit_tag_and_push(&project_root, &new_version, &staged)?;
        println!();
        println!("{} {}", style("Released").green().bold(), new_version);
        Ok(())
    }

    /// Resolve the current version from CCGO.toml and compute the target
    /// version per the user's bump argument.
    fn resolve_versions(&self, project_root: &Path) -> Result<(String, String)> {
        let bump = self.version.as_deref().ok_or_else(|| {
            anyhow::anyhow!(
                "version argument is required (patch|minor|major|X.Y.Z) unless --changelog-only"
            )
        })?;

        let ccgo_toml = project_root.join("CCGO.toml");
        let config = CcgoConfig::load_from_path(&ccgo_toml)?;
        let current = config
            .package
            .as_ref()
            .map(|p| p.version.clone())
            .ok_or_else(|| anyhow::anyhow!("CCGO.toml has no [package] section with a version"))?;
        let new_version = bump_version(&current, bump)?;
        Ok((current, new_version))
    }

    fn print_header(&self, project_root: &Path, current: &str, new_version: &str) {
        println!("{}", style("Releasing CCGO project").bold());
        println!("  Project root:    {}", project_root.display());
        println!("  Current version: {}", style(current).yellow());
        println!("  New version:     {}", style(new_version).green());
        if self.dry_run {
            println!("  {}", style("(dry run — no files will be modified)").dim());
        }
        println!();
    }

    fn check_clean_tree(&self, project_root: &Path) -> Result<()> {
        if self.allow_dirty || self.dry_run {
            return Ok(());
        }
        if !git::is_working_tree_clean(project_root)? {
            bail!(
                "working tree has uncommitted changes; commit or stash them first, \
                 or pass --allow-dirty to override"
            );
        }
        Ok(())
    }

    /// Write (or print, under --dry-run) every file the release modifies and
    /// return the list that should be staged for the release commit.
    fn write_project_files(
        &self,
        project_root: &Path,
        current: &str,
        new_version: &str,
    ) -> Result<Vec<PathBuf>> {
        let mut staged: Vec<PathBuf> = Vec::new();

        let ccgo_toml = project_root.join("CCGO.toml");
        if self.dry_run {
            println!(
                "  would update {}: version -> {}",
                ccgo_toml.display(),
                new_version
            );
        } else {
            update_ccgo_toml_version(&ccgo_toml, current, new_version)?;
            println!("  {} {}", style("Updated").green(), ccgo_toml.display());
        }
        staged.push(ccgo_toml);

        if !self.no_sync {
            self.sync_platform_manifests(project_root, new_version, &mut staged);
        }

        if !self.no_changelog {
            self.update_changelog(project_root, new_version, &mut staged)?;
        }

        Ok(staged)
    }

    fn sync_platform_manifests(
        &self,
        project_root: &Path,
        new_version: &str,
        staged: &mut Vec<PathBuf>,
    ) {
        let android_catalog = project_root.join(version_sync::ANDROID_VERSION_CATALOG);
        if android_catalog.is_file() {
            if self.dry_run {
                println!(
                    "  would update {}: commMainProject -> {}",
                    android_catalog.display(),
                    new_version
                );
            } else {
                version_sync::sync_gradle_version_catalog(&android_catalog, new_version);
            }
            staged.push(android_catalog);
        }

        let ohos_manifest = project_root.join(version_sync::OHOS_PACKAGE_MANIFEST);
        if ohos_manifest.is_file() {
            if self.dry_run {
                println!(
                    "  would update {}: version -> {}",
                    ohos_manifest.display(),
                    new_version
                );
            } else {
                version_sync::sync_oh_package_version(&ohos_manifest, new_version);
            }
            staged.push(ohos_manifest);
        }
    }

    fn update_changelog(
        &self,
        project_root: &Path,
        new_version: &str,
        staged: &mut Vec<PathBuf>,
    ) -> Result<()> {
        let changelog_path = project_root.join("CHANGELOG.md");
        let range = compute_changelog_range(project_root)?;
        let date = Local::now().format("%Y-%m-%d").to_string();
        let section = changelog::generate_section(project_root, new_version, &date, &range)?;

        if section.is_empty() {
            println!(
                "  {} no changelog entries for {} (no relevant commits in {})",
                style("Skipped").yellow(),
                new_version,
                range
            );
            return Ok(());
        }

        if self.dry_run {
            println!("  would insert section into {}:", changelog_path.display());
            for line in section.lines().take(10) {
                println!("    {line}");
            }
            if section.lines().count() > 10 {
                println!("    …");
            }
        } else {
            changelog::insert_section(&changelog_path, &section)?;
            println!(
                "  {} {}",
                style("Updated").green(),
                changelog_path.display()
            );
        }
        staged.push(changelog_path);
        Ok(())
    }

    fn commit_tag_and_push(
        &self,
        project_root: &Path,
        new_version: &str,
        staged: &[PathBuf],
    ) -> Result<()> {
        let staged_refs: Vec<&Path> = staged.iter().map(|p| p.as_path()).collect();
        git::add_paths(project_root, &staged_refs)?;

        let commit_message = self.message.replace("{version}", new_version);
        git::commit(project_root, &commit_message)?;
        println!("  {} {}", style("Committed").green(), commit_message);

        let tag = format!("{}{}", self.tag_prefix, new_version);
        git::create_annotated_tag(project_root, &tag, &commit_message)?;
        println!("  {} tag {}", style("Created").green(), tag);

        if self.push {
            let branch = git::current_branch(project_root)?;
            git::push(project_root, "origin", &branch)?;
            git::push(project_root, "origin", &tag)?;
            println!(
                "  {} origin {} and {}",
                style("Pushed").green(),
                branch,
                tag
            );
        } else {
            println!();
            println!("To publish:");
            println!(
                "  git push origin $(git rev-parse --abbrev-ref HEAD) && git push origin {tag}"
            );
        }
        Ok(())
    }

    fn run_changelog_only(self, project_root: &Path) -> Result<()> {
        let changelog_path = project_root.join("CHANGELOG.md");
        if self.dry_run {
            println!(
                "would regenerate {} from git history",
                changelog_path.display()
            );
            return Ok(());
        }
        changelog::regenerate(project_root, &changelog_path)?;
        println!(
            "{} {}",
            style("Regenerated").green(),
            changelog_path.display()
        );
        Ok(())
    }
}

/// Compute the `<sha>..HEAD` range used to generate a new CHANGELOG section.
///
/// Uses the last `chore: release v…` commit as the lower bound, or the repo's
/// root commit when there's no prior release.
fn compute_changelog_range(cwd: &Path) -> Result<String> {
    if let Some(sha) = git::last_release_sha(cwd)? {
        Ok(format!("{sha}..HEAD"))
    } else {
        let first = git::first_commit_sha(cwd)?;
        Ok(format!("{first}..HEAD"))
    }
}

/// Bump `current` per a keyword (`patch|minor|major`) or return `bump`
/// verbatim when it already looks like a semver version.
pub fn bump_version(current: &str, bump: &str) -> Result<String> {
    let parsed = Version::parse(current)
        .with_context(|| format!("CCGO.toml version {current:?} is not valid semver"))?;
    match bump {
        "patch" => Ok(format!(
            "{}.{}.{}",
            parsed.major,
            parsed.minor,
            parsed.patch + 1
        )),
        "minor" => Ok(format!("{}.{}.0", parsed.major, parsed.minor + 1)),
        "major" => Ok(format!("{}.0.0", parsed.major + 1)),
        explicit => {
            // Accept explicit X.Y.Z (+ optional pre/build).
            Version::parse(explicit).with_context(|| {
                format!("invalid version {explicit:?}; expected patch|minor|major|X.Y.Z")
            })?;
            Ok(explicit.to_string())
        }
    }
}

/// Rewrite the `version = "<old>"` line under `[package]`/`[project]` in
/// `CCGO.toml`. We substitute by regex to preserve comments, formatting, and
/// any other keys verbatim — matches the approach used for the Android and
/// OHOS manifest syncs.
fn update_ccgo_toml_version(path: &Path, current: &str, new_version: &str) -> Result<()> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("failed to read {}", path.display()))?;

    // Match `version = "<current>"` at the start of a line. Escape `current`
    // in case it contains regex metacharacters (e.g. `1.0.0-rc.1`).
    let pattern = format!(
        r#"(?m)^(\s*version\s*=\s*")({})(")"#,
        regex::escape(current)
    );
    let re = Regex::new(&pattern).expect("static regex");
    if !re.is_match(&content) {
        bail!(
            "could not find `version = \"{current}\"` in {} — is CCGO.toml malformed?",
            path.display()
        );
    }
    let new_content = re
        .replace(&content, format!("${{1}}{new_version}${{3}}"))
        .to_string();
    std::fs::write(path, new_content)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bump_patch_minor_major() {
        assert_eq!(bump_version("1.2.3", "patch").unwrap(), "1.2.4");
        assert_eq!(bump_version("1.2.3", "minor").unwrap(), "1.3.0");
        assert_eq!(bump_version("1.2.3", "major").unwrap(), "2.0.0");
    }

    #[test]
    fn bump_explicit_version_passes_through() {
        assert_eq!(bump_version("1.2.3", "9.9.9").unwrap(), "9.9.9");
        assert_eq!(bump_version("1.0.0", "2.0.0-rc.1").unwrap(), "2.0.0-rc.1");
    }

    #[test]
    fn bump_rejects_garbage() {
        assert!(bump_version("1.2.3", "foo").is_err());
        assert!(bump_version("not-semver", "patch").is_err());
    }

    #[test]
    fn update_ccgo_toml_rewrites_version_in_place() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(
            tmp.path(),
            "[package]\nname = \"foo\"\nversion = \"1.0.0\"\ndescription = \"x\"\n",
        )
        .unwrap();
        update_ccgo_toml_version(tmp.path(), "1.0.0", "1.1.0").unwrap();
        let after = std::fs::read_to_string(tmp.path()).unwrap();
        assert!(after.contains("version = \"1.1.0\""));
        assert!(after.contains("name = \"foo\""));
        assert!(after.contains("description = \"x\""));
    }

    #[test]
    fn update_ccgo_toml_errors_when_version_mismatch() {
        let tmp = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(tmp.path(), "[package]\nversion = \"2.0.0\"\n").unwrap();
        let err = update_ccgo_toml_version(tmp.path(), "1.0.0", "1.1.0").unwrap_err();
        assert!(err.to_string().contains("could not find"));
    }
}
