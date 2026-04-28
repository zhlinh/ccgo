//! CHANGELOG.md generation from Conventional Commits.
//!
//! Rust port of the bash logic in `scripts/release.sh`. Commits are
//! categorized by their conventional-commit type prefix:
//!
//! | prefix     | section        |
//! | ---------- | -------------- |
//! | `feat:`    | Added          |
//! | `fix:`     | Fixed          |
//! | `perf:`    | Performance    |
//! | `refactor:`| Changed        |
//! | `docs:`    | Documentation  |
//! | anything that isn't `chore`/`style`/`test`/`build`/`ci` and doesn't match the above | Other |
//!
//! `chore: release v…` commits are always skipped — they're version boundaries, not content.

use std::path::Path;

use anyhow::{Context, Result};
use regex::Regex;

use crate::utils::git;

/// Default header for a freshly-created CHANGELOG.md (Keep a Changelog 1.1.0).
const CHANGELOG_HEADER: &str = "\
# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

";

/// Generate a CHANGELOG section for `version` covering the git `range` (e.g.
/// `v1.0.0..HEAD` or `<sha>..<sha>`). Returns the formatted section, or an
/// empty string when the range has no interesting commits.
pub fn generate_section(cwd: &Path, version: &str, date: &str, range: &str) -> Result<String> {
    let subjects = git::log_subjects(cwd, range)?;

    // Drop release commits — they delimit versions, they don't describe changes.
    let relevant: Vec<&str> = subjects
        .iter()
        .map(|s| s.as_str())
        .filter(|s| !s.starts_with("chore: release "))
        .collect();

    if relevant.is_empty() {
        return Ok(String::new());
    }

    let feat = filter(&relevant, "feat");
    let fix = filter(&relevant, "fix");
    let perf = filter(&relevant, "perf");
    let refactor = filter(&relevant, "refactor");
    let docs = filter(&relevant, "docs");
    let other = filter_other(&relevant);

    let mut out = String::new();
    out.push_str(&format!("## [{version}] - {date}\n\n"));
    append_bullets(&mut out, "Added", &feat, Some("feat"));
    append_bullets(&mut out, "Fixed", &fix, Some("fix"));
    append_bullets(&mut out, "Performance", &perf, Some("perf"));
    append_bullets(&mut out, "Changed", &refactor, Some("refactor"));
    append_bullets(&mut out, "Documentation", &docs, Some("docs"));
    append_bullets(&mut out, "Other", &other, None);
    Ok(out)
}

/// Filter subjects that start with the given conventional-commit `prefix`,
/// with an optional `(scope)` between the prefix and the colon.
fn filter<'a>(subjects: &[&'a str], prefix: &str) -> Vec<&'a str> {
    let re = Regex::new(&format!(r"^{prefix}(\([^)]+\))?:")).expect("static regex");
    subjects
        .iter()
        .copied()
        .filter(|s| re.is_match(s))
        .collect()
}

/// Filter "uncategorized" commits — anything that isn't one of the known
/// conventional-commit prefixes.
fn filter_other<'a>(subjects: &[&'a str]) -> Vec<&'a str> {
    // Must mirror the bash rule: excludes feat/fix/perf/refactor/docs (shown elsewhere)
    // AND chore/style/test/build/ci (silently dropped).
    let re = Regex::new(r"^(feat|fix|perf|refactor|docs|chore|style|test|build|ci)(\([^)]+\))?:")
        .expect("static regex");
    subjects
        .iter()
        .copied()
        .filter(|s| !re.is_match(s))
        .collect()
}

/// Append a `### Heading` block with one bullet per subject. When `strip_prefix`
/// is set, the "feat:", "fix(scope):" etc. prefix is rewritten to `- `; when
/// `None`, the entire subject is prefixed with `- ` verbatim.
fn append_bullets(out: &mut String, heading: &str, items: &[&str], strip_prefix: Option<&str>) {
    if items.is_empty() {
        return;
    }
    out.push_str(&format!("### {heading}\n\n"));
    for item in items {
        if let Some(prefix) = strip_prefix {
            let re = Regex::new(&format!(r"^{prefix}(\([^)]+\))?:\s*")).expect("static regex");
            out.push_str(&format!("- {}\n", re.replace(item, "")));
        } else {
            out.push_str(&format!("- {item}\n"));
        }
    }
    out.push('\n');
}

/// Insert `section` into `changelog_path` immediately after the
/// `## [Unreleased]` line, creating the file with a default header if it
/// doesn't yet exist.
pub fn insert_section(changelog_path: &Path, section: &str) -> Result<()> {
    if section.is_empty() {
        return Ok(());
    }

    let content = if changelog_path.exists() {
        std::fs::read_to_string(changelog_path)
            .with_context(|| format!("failed to read {}", changelog_path.display()))?
    } else {
        CHANGELOG_HEADER.to_string()
    };

    let new_content = splice_after_unreleased(&content, section);
    std::fs::write(changelog_path, new_content)
        .with_context(|| format!("failed to write {}", changelog_path.display()))?;
    Ok(())
}

/// Regenerate the entire CHANGELOG.md from git history.
///
/// Walks every `chore: release vX.Y.Z` commit, treating consecutive pairs as
/// version boundaries, and builds sections newest-first after `[Unreleased]`.
/// The first release's range starts at the repo's root commit.
pub fn regenerate(cwd: &Path, changelog_path: &Path) -> Result<()> {
    // Collect release commits oldest-first.
    let output = std::process::Command::new("git")
        .args([
            "log",
            "--reverse",
            "--pretty=format:%H%x09%ad%x09%s",
            "--date=short",
            "--grep=^chore: release v[0-9]",
        ])
        .current_dir(cwd)
        .output()
        .context("failed to run git log for changelog regeneration")?;
    if !output.status.success() {
        anyhow::bail!("git log failed during changelog regeneration");
    }
    let raw = String::from_utf8_lossy(&output.stdout);
    let mut releases: Vec<(String, String, String)> = Vec::new(); // (sha, date, version)
    for line in raw.lines() {
        let parts: Vec<&str> = line.splitn(3, '\t').collect();
        if parts.len() != 3 {
            continue;
        }
        let version = parts[2].trim_start_matches("chore: release v").to_string();
        releases.push((parts[0].to_string(), parts[1].to_string(), version));
    }

    if releases.is_empty() {
        // No history yet — write just the header.
        std::fs::write(changelog_path, CHANGELOG_HEADER)
            .with_context(|| format!("failed to write {}", changelog_path.display()))?;
        return Ok(());
    }

    let first_sha = git::first_commit_sha(cwd)?;

    // Build sections newest-first (i.e. iterate from the end of the vec).
    let mut sections = String::new();
    for i in (0..releases.len()).rev() {
        let (sha, date, version) = &releases[i];
        let range = if i == 0 {
            format!("{first_sha}..{sha}")
        } else {
            let prev_sha = &releases[i - 1].0;
            format!("{prev_sha}..{sha}")
        };
        let sec = generate_section(cwd, version, date, &range)?;
        sections.push_str(&sec);
    }

    let mut content = String::from(CHANGELOG_HEADER);
    content.push_str(&sections);
    std::fs::write(changelog_path, content)
        .with_context(|| format!("failed to write {}", changelog_path.display()))?;
    Ok(())
}

/// Splice `section` into `content` right after the `## [Unreleased]` line.
/// If there is no such line, prepend the section to the file (keeping the
/// existing content below).
fn splice_after_unreleased(content: &str, section: &str) -> String {
    let mut out = String::new();
    let mut inserted = false;
    for line in content.lines() {
        out.push_str(line);
        out.push('\n');
        if !inserted && line.trim_start().starts_with("## [Unreleased]") {
            out.push('\n');
            out.push_str(section);
            inserted = true;
        }
    }
    if !inserted {
        // No [Unreleased] marker — prepend a freshly-formed header block plus
        // the section, then the pre-existing content.
        let mut prefix = String::from(CHANGELOG_HEADER);
        prefix.push_str(section);
        prefix.push_str(&out);
        return prefix;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_matches_prefix_with_and_without_scope() {
        let items = vec![
            "feat: add thing",
            "feat(doc): add doxygen",
            "fix: typo",
            "chore: bump",
        ];
        let out = filter(&items, "feat");
        assert_eq!(out, vec!["feat: add thing", "feat(doc): add doxygen"]);
    }

    #[test]
    fn filter_other_drops_standard_types() {
        let items = vec![
            "feat: a",
            "chore: b",
            "refactor(x): c",
            "something else",
            "WIP hack",
        ];
        let out = filter_other(&items);
        assert_eq!(out, vec!["something else", "WIP hack"]);
    }

    #[test]
    fn append_bullets_strips_prefix() {
        let mut out = String::new();
        let items = vec!["feat: add thing", "feat(doc): more"];
        append_bullets(&mut out, "Added", &items, Some("feat"));
        assert_eq!(out, "### Added\n\n- add thing\n- more\n\n");
    }

    #[test]
    fn append_bullets_keeps_other_verbatim() {
        let mut out = String::new();
        let items = vec!["random commit", "WIP"];
        append_bullets(&mut out, "Other", &items, None);
        assert_eq!(out, "### Other\n\n- random commit\n- WIP\n\n");
    }

    #[test]
    fn splice_inserts_after_unreleased() {
        let existing = "\
# Changelog

## [Unreleased]

## [1.0.0] - 2025-01-01

### Added
- old stuff
";
        let section = "## [1.1.0] - 2025-02-01\n\n### Added\n\n- new stuff\n\n";
        let out = splice_after_unreleased(existing, section);
        let unrel_idx = out.find("## [Unreleased]").unwrap();
        let new_idx = out.find("## [1.1.0]").unwrap();
        let old_idx = out.find("## [1.0.0]").unwrap();
        assert!(unrel_idx < new_idx);
        assert!(
            new_idx < old_idx,
            "new section must land before the older one"
        );
    }

    #[test]
    fn splice_handles_missing_unreleased() {
        let existing = "no header here\n";
        let section = "## [1.1.0] - 2025-02-01\n\n";
        let out = splice_after_unreleased(existing, section);
        assert!(out.contains("## [Unreleased]"));
        assert!(out.contains("## [1.1.0]"));
        assert!(out.contains("no header here"));
    }
}
