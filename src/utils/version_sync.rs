//! Cross-manifest version synchronization.
//!
//! Keeps auxiliary platform manifests (Android Gradle version catalog, OHOS
//! oh-package.json5) in lockstep with CCGO.toml's canonical `package.version`.
//!
//! Each sync function is silently tolerant: if the target file is missing or
//! the expected key is absent, it returns without error. That way projects
//! without a Gradle catalog or OHOS manifest aren't penalized.

use std::path::Path;

use regex::Regex;

/// Relative path (from the CCGO project root) to the Android Gradle version
/// catalog that gets kept in sync.
pub const ANDROID_VERSION_CATALOG: &str = "android/gradle/libs.versions.toml";

/// Relative path (from the CCGO project root) to the OHOS package manifest
/// that gets kept in sync.
pub const OHOS_PACKAGE_MANIFEST: &str = "ohos/main_ohos_sdk/oh-package.json5";

/// Sync CCGO.toml's project.version into the Android Gradle version catalog.
///
/// Updates `commMainProject = "<version>"` in-place, preserving indentation
/// and quoting. Silently no-ops when the catalog is missing or the key is
/// absent — this keeps the function safe for projects that don't use a Gradle
/// version catalog.
///
/// `catalog` is the full path to `libs.versions.toml` (callers typically
/// resolve it as `<project_root>/android/gradle/libs.versions.toml`).
pub fn sync_gradle_version_catalog(catalog: &Path, version: &str) {
    // (?m) = multiline so ^ matches line starts. Capture group 1 preserves the
    // indentation and `commMainProject = "` prefix, group 2 preserves the closing quote.
    let pattern = r#"(?m)^(\s*commMainProject\s*=\s*")[^"]*(")"#;
    sync_regex_value(catalog, pattern, version, "commMainProject");
}

/// Sync CCGO.toml's project.version into the OHOS oh-package.json5 manifest.
///
/// Updates the top-level `"version": "<version>"` field. Silently no-ops when
/// the file is missing or the field is absent — safe for projects without an
/// OHOS manifest.
///
/// `manifest` is the full path to `oh-package.json5` (callers typically
/// resolve it as `<project_root>/ohos/main_ohos_sdk/oh-package.json5`).
pub fn sync_oh_package_version(manifest: &Path, version: &str) {
    let pattern = r#"(?m)^(\s*"version"\s*:\s*")[^"]*(")"#;
    sync_regex_value(manifest, pattern, version, "version");
}

/// Generic helper: find a line matching `pattern` (which must have two capture
/// groups surrounding the value), replace the value with `new_value`, and
/// write the file back. Silently skips when the file or pattern is absent.
///
/// `label` is the field name used in the informational message printed on
/// success ("Synced <path> <label> -> <value>").
fn sync_regex_value(path: &Path, pattern: &str, new_value: &str, label: &str) {
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let Ok(re) = Regex::new(pattern) else { return };
    if !re.is_match(&content) {
        return;
    }

    let new_content = re
        .replace(&content, format!("${{1}}{new_value}${{2}}"))
        .to_string();
    if new_content == content {
        return;
    }

    if let Err(e) = std::fs::write(path, new_content) {
        eprintln!(
            "Warning: failed to sync {} -> {}: {}",
            path.display(),
            new_value,
            e
        );
        return;
    }
    eprintln!("Synced {} {label} -> {}", path.display(), new_value);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn sync_gradle_catalog_updates_value() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "[versions]").unwrap();
        writeln!(f, "commMainProject = \"1.0.0\"").unwrap();
        writeln!(f, "kotlin = \"1.9.0\"").unwrap();
        f.flush().unwrap();

        sync_gradle_version_catalog(f.path(), "2.0.0");

        let updated = std::fs::read_to_string(f.path()).unwrap();
        assert!(updated.contains("commMainProject = \"2.0.0\""));
        assert!(updated.contains("kotlin = \"1.9.0\"")); // unrelated lines untouched
    }

    #[test]
    fn sync_oh_package_updates_value() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "{{").unwrap();
        writeln!(f, "  \"name\": \"my-lib\",").unwrap();
        writeln!(f, "  \"version\": \"1.0.0\",").unwrap();
        writeln!(f, "}}").unwrap();
        f.flush().unwrap();

        sync_oh_package_version(f.path(), "2.3.4");

        let updated = std::fs::read_to_string(f.path()).unwrap();
        assert!(updated.contains("\"version\": \"2.3.4\""));
        assert!(updated.contains("\"name\": \"my-lib\""));
    }

    #[test]
    fn missing_file_is_silent() {
        // Should not panic or error — silently no-ops
        sync_gradle_version_catalog(Path::new("/nonexistent/path/libs.versions.toml"), "1.0.0");
        sync_oh_package_version(Path::new("/nonexistent/path/oh-package.json5"), "1.0.0");
    }

    #[test]
    fn missing_key_is_silent() {
        let mut f = NamedTempFile::new().unwrap();
        writeln!(f, "[versions]").unwrap();
        writeln!(f, "kotlin = \"1.9.0\"").unwrap();
        f.flush().unwrap();
        let before = std::fs::read_to_string(f.path()).unwrap();

        sync_gradle_version_catalog(f.path(), "2.0.0");

        let after = std::fs::read_to_string(f.path()).unwrap();
        assert_eq!(before, after, "file should be unchanged when key is absent");
    }
}
