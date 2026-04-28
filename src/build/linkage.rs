//! Linkage resolution: decide how each dependency is integrated into the
//! consumer's build product.
//!
//! This module has two concerns:
//! 1. Filesystem inspection (`detect_dep_artifacts`) — pure read-only walk
//!    of `.ccgo/deps/<name>/lib/<platform>/{static,shared}/` to discover
//!    what artifacts the dep ships.
//! 2. Pure decision logic (`resolve_linkage`) — given the artifacts and
//!    user hints, decide how to link. No I/O, no toolchain calls.
//!
//! Two callers use the result:
//! 1. The Rust platform builders (Android/OHOS) decide whether to invoke
//!    a static-archive merge step.
//! 2. The CMake template gets a `CCGO_DEPENDENCY_<NAME>_LINKAGE` variable
//!    so it can choose between `_SHARED_LIBRARIES` and `_STATIC_LIBRARIES`.

use std::path::Path;

use anyhow::{anyhow, Result};

use crate::commands::build::LinkType;
use crate::config::Linkage;

/// What artifacts a dependency directory provides.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepArtifacts {
    OnlyStatic,
    OnlyShared,
    Both,
    /// Dep has a `src/` directory and `CCGO.toml` but no pre-built `.a`/`.so`
    /// for the platform being queried. **Must not be passed to `resolve_linkage`** —
    /// the materialize pass (`BuildContext::materialize_source_deps`) is required
    /// to compile the dep first, after which detection re-classifies it as
    /// `Both`, `OnlyStatic`, or `OnlyShared`. Passing this variant directly to
    /// the resolver is a programmer error and returns `Err`.
    SourceOnly,
    /// Nothing usable found; the caller should fail loudly rather than
    /// silently producing an empty link line.
    None,
}

/// The concrete instruction the build pipeline acts on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedLinkage {
    SharedExternal,
    StaticEmbedded,
    StaticExternal,
}

impl std::fmt::Display for ResolvedLinkage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ResolvedLinkage::SharedExternal => "shared-external",
            ResolvedLinkage::StaticEmbedded => "static-embedded",
            ResolvedLinkage::StaticExternal => "static-external",
        };
        f.write_str(s)
    }
}

/// Decide how a dependency is linked into the consumer's build product.
///
/// Pure function — no filesystem or toolchain access.
///
/// # Parameters
/// - `consumer`: the link type of the build product that consumes the dep.
///   Must be a concrete single type (`Static` or `Shared`); `Both` is
///   rejected with an error so callers must split it explicitly.
/// - `artifacts`: what the dep directory actually provides on disk. The
///   caller owns the detection (Task 3 introduces a helper); this function
///   only decides what to do with the result.
/// - `hint`: optional explicit override from the consumer's CCGO.toml
///   (`linkage = "..."`). When `None`, the resolver picks a sensible
///   default. When set, it is honored where physically possible and
///   produces a precise error otherwise.
/// - `dep_name`: only used to compose error messages — does not affect
///   the resolution.
pub fn resolve_linkage(
    consumer: LinkType,
    artifacts: DepArtifacts,
    hint: Option<Linkage>,
    dep_name: &str,
) -> Result<ResolvedLinkage> {
    if artifacts == DepArtifacts::None {
        return Err(anyhow!(
            "dependency '{dep_name}' has no usable artifacts (no .so, no .a, no source). \
             Run `ccgo build` in the dep's source project, or check the zip/path/git source."
        ));
    }

    match consumer {
        // Static consumer: always thin. The hint can only choose between
        // referencing the dep's .so vs .a externally — never embed.
        LinkType::Static => match artifacts {
            DepArtifacts::OnlyShared => Ok(ResolvedLinkage::SharedExternal),
            DepArtifacts::OnlyStatic | DepArtifacts::Both => Ok(ResolvedLinkage::StaticExternal),
            DepArtifacts::SourceOnly => Err(anyhow!(
                "internal: dependency '{dep_name}' is still source-only at \
                 link-resolution time. `materialize_source_deps` must run \
                 before `resolve_linkage` so the dep's .a/.so are available. \
                 This is a ccgo bug — please report it."
            )),
            DepArtifacts::None => unreachable!("handled above"),
        },
        // Shared consumer: forced moves first, then honor hint, then default.
        LinkType::Shared => match artifacts {
            DepArtifacts::OnlyStatic => {
                // No .so — must embed regardless of hint.
                if matches!(hint, Some(Linkage::SharedExternal)) {
                    return Err(anyhow!(
                        "dependency '{dep_name}' provides only a static archive but \
                         linkage = \"shared-external\" was requested. Either drop the \
                         hint (ccgo will static-embed automatically), or rebuild '{dep_name}' \
                         with link_type = \"shared\" or \"both\"."
                    ));
                }
                Ok(ResolvedLinkage::StaticEmbedded)
            }
            DepArtifacts::OnlyShared => {
                // No .a — embedding is impossible.
                if matches!(hint, Some(Linkage::StaticEmbedded)) {
                    return Err(anyhow!(
                        "dependency '{dep_name}' provides only a shared library (.so) but \
                         linkage = \"static-embedded\" was requested. Either drop the hint \
                         (ccgo will use shared-external automatically), or rebuild '{dep_name}' \
                         with link_type = \"static\" or \"both\" so a .a is available."
                    ));
                }
                if matches!(hint, Some(Linkage::StaticExternal)) {
                    return Err(anyhow!(
                        "dependency '{dep_name}': linkage = \"static-external\" is not valid \
                         for a shared consumer (the .so would have unresolved external \
                         references). Use shared-external (default) or static-embedded."
                    ));
                }
                Ok(ResolvedLinkage::SharedExternal)
            }
            DepArtifacts::Both => match hint {
                None | Some(Linkage::SharedExternal) => Ok(ResolvedLinkage::SharedExternal),
                Some(Linkage::StaticEmbedded) => Ok(ResolvedLinkage::StaticEmbedded),
                Some(Linkage::StaticExternal) => Err(anyhow!(
                    "dependency '{dep_name}': linkage = \"static-external\" is not valid for \
                     a shared consumer (the .so would have unresolved external static \
                     references). Use shared-external or static-embedded."
                )),
            },
            DepArtifacts::SourceOnly => Err(anyhow!(
                "internal: dependency '{dep_name}' is still source-only at \
                 link-resolution time. `materialize_source_deps` must run \
                 before `resolve_linkage` so the dep's .a/.so are available. \
                 This is a ccgo bug — please report it."
            )),
            DepArtifacts::None => unreachable!("handled above"),
        },
        LinkType::Both => Err(anyhow!(
            "resolve_linkage requires a concrete consumer link type \
             (static or shared); caller must collapse LinkType::Both \
             into two separate calls. Use LinkType::preferred_single() \
             once that helper lands in Task 5."
        )),
    }
}

/// Inspect a directory like `.ccgo/deps/<name>/` to discover what artifacts
/// the dep already provides for a given platform. Source-only deps (the
/// dir contains `src/` and a `CCGO.toml` but no `lib/<platform>/`) are
/// reported as `SourceOnly` so the caller can build them on demand.
pub fn detect_dep_artifacts(dep_root: &Path, platform: &str) -> DepArtifacts {
    let lib_root = dep_root.join("lib").join(platform);
    // xcframework bundles appear in both static/ and shared/ on Apple platforms:
    // static/foo.xcframework internally contains a .a, shared/foo.xcframework
    // contains a .dylib. Both are detected by the same "xcframework" extension.
    let has_static = dir_contains_any_ext(&lib_root.join("static"), &["a", "lib", "xcframework"]);
    let has_shared = dir_contains_any_ext(
        &lib_root.join("shared"),
        &["so", "dylib", "dll", "framework", "xcframework"],
    );

    match (has_shared, has_static) {
        (true, true) => DepArtifacts::Both,
        (true, false) => DepArtifacts::OnlyShared,
        (false, true) => DepArtifacts::OnlyStatic,
        (false, false) => {
            if dep_root.join("src").is_dir() && dep_root.join("CCGO.toml").is_file() {
                DepArtifacts::SourceOnly
            } else {
                DepArtifacts::None
            }
        }
    }
}

/// Walks `dir` recursively and returns true if any entry — file OR
/// directory — has an extension in `exts`. Directory bundles like Apple's
/// `Foo.framework` and `Foo.xcframework` are matched on their own name,
/// not by descending into them. `walkdir` is used so symlink loops can't
/// hang the scanner (`follow_links(false)`); unreadable entries are
/// silently skipped via `flatten()`.
fn dir_contains_any_ext(dir: &Path, exts: &[&str]) -> bool {
    walkdir::WalkDir::new(dir)
        .follow_links(false)
        .into_iter()
        .flatten()
        .any(|entry| {
            entry
                .path()
                .extension()
                .and_then(|s| s.to_str())
                .map(|ext| exts.iter().any(|e| e.eq_ignore_ascii_case(ext)))
                .unwrap_or(false)
        })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn touch(p: &std::path::Path) {
        if let Some(parent) = p.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(p, b"").unwrap();
    }

    #[test]
    fn static_consumer_always_external_regardless_of_hint() {
        for hint in [
            None,
            Some(Linkage::SharedExternal),
            Some(Linkage::StaticEmbedded),
            Some(Linkage::StaticExternal),
        ] {
            for arts in [
                DepArtifacts::OnlyStatic,
                DepArtifacts::OnlyShared,
                DepArtifacts::Both,
            ] {
                let resolved = resolve_linkage(LinkType::Static, arts, hint, "dep").unwrap();
                assert!(
                    matches!(
                        resolved,
                        ResolvedLinkage::StaticExternal | ResolvedLinkage::SharedExternal
                    ),
                    "static consumer should never produce StaticEmbedded; got {resolved:?} for hint={hint:?} arts={arts:?}"
                );
            }
        }
    }

    #[test]
    fn static_consumer_with_static_artifacts_resolves_static_external() {
        for arts in [
            DepArtifacts::OnlyStatic,
            DepArtifacts::Both,
        ] {
            for hint in [
                None,
                Some(Linkage::SharedExternal),
                Some(Linkage::StaticEmbedded),
                Some(Linkage::StaticExternal),
            ] {
                let r = resolve_linkage(LinkType::Static, arts, hint, "dep").unwrap();
                assert_eq!(
                    r,
                    ResolvedLinkage::StaticExternal,
                    "static consumer + arts={arts:?} hint={hint:?} should be StaticExternal"
                );
            }
        }
    }

    #[test]
    fn static_consumer_uses_shared_external_when_only_shared_available() {
        let r = resolve_linkage(LinkType::Static, DepArtifacts::OnlyShared, None, "dep").unwrap();
        assert_eq!(r, ResolvedLinkage::SharedExternal);
    }

    #[test]
    fn shared_consumer_only_static_must_embed() {
        let r = resolve_linkage(LinkType::Shared, DepArtifacts::OnlyStatic, None, "dep").unwrap();
        assert_eq!(r, ResolvedLinkage::StaticEmbedded);
    }

    #[test]
    fn shared_consumer_only_shared_must_external() {
        let r = resolve_linkage(LinkType::Shared, DepArtifacts::OnlyShared, None, "dep").unwrap();
        assert_eq!(r, ResolvedLinkage::SharedExternal);
    }

    #[test]
    fn shared_consumer_both_defaults_to_external() {
        let r = resolve_linkage(LinkType::Shared, DepArtifacts::Both, None, "dep").unwrap();
        assert_eq!(r, ResolvedLinkage::SharedExternal);
    }

    #[test]
    fn shared_consumer_both_honors_static_embedded_hint() {
        let r = resolve_linkage(
            LinkType::Shared,
            DepArtifacts::Both,
            Some(Linkage::StaticEmbedded),
            "dep",
        )
        .unwrap();
        assert_eq!(r, ResolvedLinkage::StaticEmbedded);
    }

    #[test]
    fn shared_consumer_with_static_external_hint_is_an_error() {
        // shared consumer can't have an external static dep — would leave
        // unresolved symbols in the .so.
        let err = resolve_linkage(
            LinkType::Shared,
            DepArtifacts::Both,
            Some(Linkage::StaticExternal),
            "stdcomm",
        )
        .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("stdcomm"),
            "expected dep name in error, got: {msg}"
        );
        assert!(
            msg.contains("static-external"),
            "expected hint name in error, got: {msg}"
        );
    }

    #[test]
    fn dep_with_no_artifacts_is_an_error() {
        let err = resolve_linkage(LinkType::Shared, DepArtifacts::None, None, "ghost").unwrap_err();
        assert!(err.to_string().contains("ghost"));
    }

    #[test]
    fn shared_consumer_only_shared_with_embedded_hint_is_an_error() {
        // The hint is `static-embedded` but no .a exists — the resolver must
        // surface this as an error so the user can fix CCGO.toml or rebuild
        // the dep with link_type = both.
        let err = resolve_linkage(
            LinkType::Shared,
            DepArtifacts::OnlyShared,
            Some(Linkage::StaticEmbedded),
            "stdcomm",
        )
        .unwrap_err();
        assert!(err.to_string().contains("stdcomm"));
        assert!(err.to_string().contains(".a"));
    }

    #[test]
    fn link_type_both_is_rejected_with_clear_message() {
        let err = resolve_linkage(LinkType::Both, DepArtifacts::Both, None, "dep").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("LinkType::Both") || msg.contains("static or shared"),
            "expected message about Both not being allowed, got: {msg}"
        );
    }

    #[test]
    fn detect_only_static() {
        let tmp = TempDir::new().unwrap();
        let dep = tmp.path().join("stdcomm");
        touch(&dep.join("lib/macos/static/libstdcomm.a"));
        assert_eq!(
            detect_dep_artifacts(&dep, "macos"),
            DepArtifacts::OnlyStatic
        );
    }

    #[test]
    fn detect_only_shared() {
        let tmp = TempDir::new().unwrap();
        let dep = tmp.path().join("stdcomm");
        touch(&dep.join("lib/macos/shared/libstdcomm.dylib"));
        assert_eq!(
            detect_dep_artifacts(&dep, "macos"),
            DepArtifacts::OnlyShared
        );
    }

    #[test]
    fn detect_both() {
        let tmp = TempDir::new().unwrap();
        let dep = tmp.path().join("stdcomm");
        touch(&dep.join("lib/android/static/arm64-v8a/libstdcomm.a"));
        touch(&dep.join("lib/android/shared/arm64-v8a/libstdcomm.so"));
        assert_eq!(detect_dep_artifacts(&dep, "android"), DepArtifacts::Both);
    }

    #[test]
    fn detect_none_when_dir_empty() {
        let tmp = TempDir::new().unwrap();
        let dep = tmp.path().join("ghost");
        fs::create_dir_all(&dep).unwrap();
        assert_eq!(detect_dep_artifacts(&dep, "android"), DepArtifacts::None);
    }

    #[test]
    fn detect_source_only_when_src_dir_exists_without_libs() {
        let tmp = TempDir::new().unwrap();
        let dep = tmp.path().join("stdcomm");
        touch(&dep.join("src/foo.cc"));
        touch(&dep.join("CCGO.toml"));
        assert_eq!(
            detect_dep_artifacts(&dep, "linux"),
            DepArtifacts::SourceOnly
        );
    }

    #[test]
    fn resolved_linkage_displays_as_kebab_case() {
        assert_eq!(ResolvedLinkage::SharedExternal.to_string(), "shared-external");
        assert_eq!(ResolvedLinkage::StaticEmbedded.to_string(), "static-embedded");
        assert_eq!(ResolvedLinkage::StaticExternal.to_string(), "static-external");
    }

    #[test]
    fn detect_none_when_src_exists_but_no_manifest() {
        // SourceOnly requires BOTH `src/` AND `CCGO.toml`. A loose `src/` dir
        // alone is not enough — without a manifest there is nothing to drive
        // the build pass, so the resolver must fail loudly.
        let tmp = TempDir::new().unwrap();
        let dep = tmp.path().join("partial");
        touch(&dep.join("src/foo.cc"));
        // intentionally no CCGO.toml
        assert_eq!(detect_dep_artifacts(&dep, "linux"), DepArtifacts::None);
    }

    #[test]
    fn detect_dep_artifacts_path_is_case_sensitive() {
        // Document the convention: paths under lib/<platform>/ are lowercase.
        // The lowercase normalization happens in BuildContext::resolved_dep_linkages
        // before this is called. This test pins the lowercase requirement.
        let tmp = TempDir::new().unwrap();
        let dep = tmp.path().join("leaf");
        touch(&dep.join("lib/android/static/libleaf.a"));

        // Lowercase platform → finds the .a (the documented convention).
        assert_eq!(
            detect_dep_artifacts(&dep, "android"),
            DepArtifacts::OnlyStatic
        );

        // Note: a stricter test would also assert that "Android" (capital)
        // returns DepArtifacts::None, but that depends on filesystem case-
        // sensitivity. macOS APFS and Windows NTFS are case-insensitive by
        // default, so the assertion would be flaky across hosts. The
        // resilience guarantee instead lives at the layer above —
        // BuildContext::resolved_dep_linkages always lowercases the platform
        // string before calling this helper, so callers cannot trip the
        // mismatch in practice.
    }

    #[test]
    fn detect_only_shared_apple_framework() {
        // `Foo.framework` is a directory bundle, not a file. The walker must
        // match it via the dir's own extension instead of descending into it
        // and looking for a `.dylib` that isn't there.
        let tmp = TempDir::new().unwrap();
        let dep = tmp.path().join("foundrycomm");
        touch(&dep.join("lib/macos/shared/Foo.framework/Foo"));
        assert_eq!(
            detect_dep_artifacts(&dep, "macos"),
            DepArtifacts::OnlyShared
        );
    }

    #[test]
    fn detect_both_apple_xcframework() {
        // On Apple platforms, ccgo ships both a static xcframework (containing
        // a .a) in lib/<platform>/static/ and a shared xcframework (containing
        // a .dylib) in lib/<platform>/shared/. Both use the .xcframework
        // directory extension. The detector must recognise xcframework in
        // BOTH the static and shared buckets so that BuildContext can resolve
        // linkage hints like static-embedded correctly.
        let tmp = TempDir::new().unwrap();
        let dep = tmp.path().join("leaf");
        touch(&dep.join("lib/macos/static/leaf.xcframework/Info.plist"));
        touch(&dep.join("lib/macos/shared/leaf.xcframework/Info.plist"));
        assert_eq!(detect_dep_artifacts(&dep, "macos"), DepArtifacts::Both);
    }

    #[test]
    fn detect_only_static_xcframework() {
        // static xcframework only (no shared) → OnlyStatic
        let tmp = TempDir::new().unwrap();
        let dep = tmp.path().join("leaf");
        touch(&dep.join("lib/macos/static/leaf.xcframework/Info.plist"));
        assert_eq!(
            detect_dep_artifacts(&dep, "macos"),
            DepArtifacts::OnlyStatic
        );
    }

    #[test]
    fn source_only_in_resolver_is_an_invariant_error_for_static_consumer() {
        let err = resolve_linkage(LinkType::Static, DepArtifacts::SourceOnly, None, "leaf").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("materialize_source_deps") || msg.contains("source-only"),
            "expected message pointing at materialize step, got: {msg}"
        );
        assert!(msg.contains("leaf"), "expected dep name in error, got: {msg}");
    }

    #[test]
    fn source_only_in_resolver_is_an_invariant_error_for_shared_consumer() {
        let err = resolve_linkage(LinkType::Shared, DepArtifacts::SourceOnly, None, "leaf").unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("materialize_source_deps") || msg.contains("source-only"),
            "expected message pointing at materialize step, got: {msg}"
        );
        assert!(msg.contains("leaf"), "expected dep name in error, got: {msg}");
    }
}
