//! Build-time verinfo generator.
//!
//! When a project declares `[build].verinfo_path = "include/foo/base/"` in
//! its `CCGO.toml`, ccgo writes two auto-generated files into that directory
//! before each build:
//!
//! * `verinfo_gen.h` — C header with `#define <PROJECT>_VERIDENTITY "..."`.
//! * `verinfo_gen.c` — translation unit that defines a `const char[]` symbol
//!                     with `__attribute__((used))`, plus an optional ELF
//!                     `.note.ccgo.project` note for structured readout.
//!
//! The resulting artifact (`.so`/`.a`/`.dylib`/`.dll`) carries the build
//! identity as an embedded string. Recover it from a shipped library with:
//!
//! ```text
//! strings libfoo.so | grep VERIDENTITY=
//! # → FOO_VERIDENTITY=25.2.9519653-20260414202500-a1b2c3d
//! ```
//!
//! Or, when ELF tooling is available:
//!
//! ```text
//! readelf -n libfoo.so | grep ccgo.project
//! ```
//!
//! The version string itself (used for filenames, dependency resolution)
//! stays strictly Cargo-style — sourced only from `[package].version`.
//! VERIDENTITY is a separate build fingerprint and never affects filenames.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Handle returned after generating the verinfo files. Callers that want to
/// feed the generated `.c` into CMake can read `.source_c()`.
#[derive(Debug, Clone)]
pub struct GeneratedVerinfo {
    /// Absolute path to the generated C header (`verinfo_gen.h`).
    pub header: PathBuf,
    /// Absolute path to the generated C source (`verinfo_gen.c`).
    pub source: PathBuf,
    /// The full VERIDENTITY string that was embedded (for logging).
    pub identity: String,
}

/// Generate `verinfo_gen.h` plus the platform-appropriate `verinfo_gen.{cc,mm}`
/// translation units that ccgo's CMake template will pick up directly into
/// the final shared library target (avoiding dead-strip when the symbol is
/// linked through a static sub-archive).
///
/// * `project_root` — absolute project directory.
/// * `header_dir_rel` — value of `[build].verinfo_path` (e.g.
///   `"include/stdcomm/base/"`). Header lands here.
/// * `source_dir_rel_override` — value of `[build].verinfo_source_path`. When
///   set, that single path is used (no per-platform fan-out). When `None`,
///   ccgo writes one source per supported `src/api/<platform>/` directory:
///   * `src/api/apple/verinfo_gen.mm`   (matches Apple platforms' .mm glob)
///   * `src/api/native/verinfo_gen.cc`  (matches Android/Linux/Windows/OHOS)
///   * `src/api/windows/verinfo_gen.cc` (Windows-specific glob)
/// * `project_name` — used for the `<PROJECT>_CCGO_PROJECT_VERIDENTITY`
///   macro / symbol name.
/// * `identity` — pre-computed `<ver>-<ts>-<sha>[-dirty]` string.
pub fn generate(
    project_root: &Path,
    header_dir_rel: &str,
    source_dir_rel_override: Option<&str>,
    project_name: &str,
    identity: &str,
) -> Result<GeneratedVerinfo> {
    let header_dir = project_root.join(header_dir_rel);
    fs::create_dir_all(&header_dir).with_context(|| {
        format!("Failed to create verinfo header dir {}", header_dir.display())
    })?;

    // `<PROJECT>_CCGO_PROJECT_VERIDENTITY` — namespaced to avoid colliding
    // with any `<PROJECT>_VERIDENTITY` the project may already define.
    let macro_name = format!(
        "{}_CCGO_PROJECT_VERIDENTITY",
        project_name.to_uppercase()
    );
    let symbol_name = format!(
        "{}_ccgo_project_veridentity",
        project_name.to_lowercase()
    );

    let header_path = header_dir.join("verinfo_gen.h");
    let header = render_header(&macro_name, &symbol_name, identity);
    write_if_changed(&header_path, &header)?;

    let source = render_source(&macro_name, &symbol_name, identity, project_name);

    // Pick destination(s).
    let mut source_paths: Vec<PathBuf> = Vec::new();
    if let Some(rel) = source_dir_rel_override {
        let dir = project_root.join(rel);
        fs::create_dir_all(&dir).with_context(|| {
            format!("Failed to create verinfo source dir {}", dir.display())
        })?;
        // Heuristic: if the user-chosen dir matches an Apple convention,
        // emit .mm; otherwise default to .cc.
        let ext = if rel.contains("apple") || rel.contains("ios")
            || rel.contains("macos") || rel.contains("tvos")
            || rel.contains("watchos")
        {
            "mm"
        } else {
            "cc"
        };
        source_paths.push(dir.join(format!("verinfo_gen.{ext}")));
    } else {
        // Default: fan out to each platform-specific glob path so every
        // `ccgo build <platform>` invocation can pick up the right TU.
        for (rel, ext) in [
            ("src/api/apple/", "mm"),
            ("src/api/native/", "cc"),
            ("src/api/windows/", "cc"),
        ] {
            let dir = project_root.join(rel);
            fs::create_dir_all(&dir).with_context(|| {
                format!("Failed to create verinfo source dir {}", dir.display())
            })?;
            source_paths.push(dir.join(format!("verinfo_gen.{ext}")));
        }
    }

    for path in &source_paths {
        write_if_changed(path, &source)?;
    }

    Ok(GeneratedVerinfo {
        header: header_path,
        source: source_paths.into_iter().next().unwrap(),
        identity: identity.to_string(),
    })
}

fn render_header(macro_name: &str, symbol_name: &str, identity: &str) -> String {
    format!(
        r#"/*
 * Auto-generated by ccgo — do not edit.
 * Regenerated on every `ccgo build`.
 *
 * Build identity macro + symbol declaration.
 */
#ifndef CCGO_VERINFO_GEN_H_
#define CCGO_VERINFO_GEN_H_

#define {macro_name}_STRING "{identity}"
#define {macro_name} "{macro_name}=" {macro_name}_STRING

#ifdef __cplusplus
extern "C" {{
#endif

/* Symbol always present in the compiled artifact; `strings` can find it. */
extern const char {symbol_name}[];

#ifdef __cplusplus
}}
#endif

#endif  /* CCGO_VERINFO_GEN_H_ */
"#
    )
}

fn render_source(
    macro_name: &str,
    symbol_name: &str,
    identity: &str,
    project_name: &str,
) -> String {
    // ELF note payload: "<project>\0<identity>\0". Only emitted on ELF
    // targets; other platforms still get the plain `const char` symbol,
    // which is sufficient for `strings` extraction.
    let note_name = project_name.to_lowercase();
    format!(
        r#"/*
 * Auto-generated by ccgo — do not edit.
 * Regenerated on every `ccgo build`.
 *
 * Embeds `{macro_name}` into the compiled artifact so the source state a
 * binary came from can be recovered post-ship:
 *
 *     strings libfoo.so | grep VERIDENTITY=
 *
 * On ELF targets the identity is additionally placed in the
 * `.note.ccgo.project` section, which `readelf -n` can pretty-print.
 */
#include <stddef.h>

#ifdef __cplusplus
extern "C" {{
#endif

/* Keep the identity string in the final artifact through every stage:
 *   - `used`                          defeats compile-time DCE
 *   - `visibility("default")`         overrides `-fvisibility=hidden`
 *   - `retain`                        blocks the linker's dead-strip
 *     (Clang 13+/GCC 11+ attribute; silently ignored by older toolchains
 *      which already preserved the symbol thanks to `used`).
 * We also add a self-referencing anchor function marked `used` so that even
 * with LTO the const is proven reachable from the public symbol graph. */
#if defined(__clang__) || (defined(__GNUC__) && __GNUC__ >= 11)
#  define CCGO_USED __attribute__((used, retain, visibility("default")))
#elif defined(__clang__) || defined(__GNUC__)
#  define CCGO_USED __attribute__((used, visibility("default")))
#else
#  define CCGO_USED
#endif

/* Prefixed so `strings | grep VERIDENTITY=` returns a self-identifying line. */
CCGO_USED const char {symbol_name}[] = "{macro_name}={identity}";

/* Exported accessor — keeps the const reachable through LTO. */
CCGO_USED const char *{symbol_name}_get(void) {{
    return {symbol_name};
}}

/* Library-load constructor — forces the linker to pull this translation
 * unit into the final dylib/so, even when no other object references
 * {symbol_name} directly. Constructors are never dead-stripped by ld.
 * Made non-static + visibility("default") so the symbol is externally
 * visible and Apple's `ld` treats it as an anchor for the .o. The asm
 * volatile reference defeats LTO's attempt to fold it away. */
#if defined(__clang__) || defined(__GNUC__)
CCGO_USED __attribute__((constructor))
void {symbol_name}_register(void) {{
    __asm__ volatile ("" : : "r"({symbol_name}) : "memory");
}}
#endif

#if defined(__ELF__)
/* ELF note: name="{note_name}\0", desc="{identity}\0", type=0 (ccgo-defined). */
struct ccgo_note_{note_name} {{
    unsigned int namesz;
    unsigned int descsz;
    unsigned int type;
    char name[sizeof("{note_name}")];
    char desc[sizeof("{identity}")];
}};

CCGO_USED
__attribute__((section(".note.ccgo.project"), aligned(4)))
static const struct ccgo_note_{note_name} ccgo_note_{note_name}_instance = {{
    sizeof("{note_name}"),
    sizeof("{identity}"),
    0,
    "{note_name}",
    "{identity}"
}};
#endif

#ifdef __cplusplus
}}  /* extern "C" */
#endif
"#
    )
}

/// Write `path` only if its current contents differ — avoids touching mtime
/// and causing spurious rebuilds when the identity is stable (e.g. developer
/// rebuilds without any git state change).
fn write_if_changed(path: &Path, content: &str) -> Result<()> {
    if let Ok(existing) = fs::read_to_string(path) {
        if existing == content {
            return Ok(());
        }
    }
    fs::write(path, content)
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn generates_expected_files() {
        let tmp = TempDir::new().unwrap();
        let out = generate(
            tmp.path(),
            "include/foo/base/",
            Some("src/base/"),
            "foo",
            "1.2.3-20260101000000-abc1234",
        )
        .unwrap();
        assert!(out.header.is_file());
        assert!(out.source.is_file());
        let header = fs::read_to_string(&out.header).unwrap();
        assert!(header.contains("FOO_CCGO_PROJECT_VERIDENTITY_STRING"));
        assert!(header.contains("1.2.3-20260101000000-abc1234"));
        let source = fs::read_to_string(&out.source).unwrap();
        assert!(source.contains("foo_ccgo_project_veridentity"));
        assert!(source.contains(".note.ccgo.project"));
    }

    #[test]
    fn write_if_changed_skips_identical_content() {
        let tmp = TempDir::new().unwrap();
        let path = tmp.path().join("f.txt");
        write_if_changed(&path, "hello").unwrap();
        let mtime1 = fs::metadata(&path).unwrap().modified().unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        write_if_changed(&path, "hello").unwrap();
        let mtime2 = fs::metadata(&path).unwrap().modified().unwrap();
        assert_eq!(mtime1, mtime2);
    }
}
