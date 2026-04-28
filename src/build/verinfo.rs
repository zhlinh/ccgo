//! Build-time verinfo generator.
//!
//! When a project declares `[build].verinfo_path = "<proj>/base/"` in its
//! `CCGO.toml`, ccgo writes two auto-generated files into the current
//! project's `cmake_build/ccgo_generated/` tree before each build:
//!
//! * `cmake_build/ccgo_generated/include/<verinfo_path>/verinfo_ccgo_gen.h`
//!   — C header with `#define <PROJECT>_CCGO_PROJECT_VERIDENTITY "..."`.
//! * `cmake_build/ccgo_generated/src/verinfo_ccgo_gen.cc`
//!   — translation unit that defines a `const char[]` symbol with
//!   `__attribute__((used))`, plus an optional ELF `.note.ccgo.project`
//!   note for structured readout.
//!
//! The generated tree lives entirely under `cmake_build/`, which is
//! gitignored by every ccgo-managed project. Nothing is written into the
//! source tree — so the working copy stays clean across builds, even as the
//! embedded identity changes on every `ccgo build`.
//!
//! The resulting artifact (`.so`/`.a`/`.dylib`/`.dll`) carries the build
//! identity as an embedded string. Recover it from a shipped library with:
//!
//! ```text
//! strings libfoo.so | grep VERIDENTITY=
//! # → FOO_CCGO_PROJECT_VERIDENTITY=25.2.9519653-20260414202500-a1b2c3d
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
//!
//! The CMake template (`cmake/template/Root.CMakeLists.txt.in`) adds
//! `cmake_build/ccgo_generated/include` to the global include path and
//! appends `cmake_build/ccgo_generated/src/verinfo_ccgo_gen.cc` to each
//! platform's `SELF_SRC_FILES`, guarded by `if(EXISTS ...)`.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

/// Subdirectory under a project's `cmake_build/` where ccgo writes its
/// generated sources. Kept in one place so the CMake template and the Rust
/// generator stay in sync.
const GENERATED_SUBDIR: &str = "cmake_build/ccgo_generated";

/// Handle returned after generating the verinfo files. Callers that want to
/// feed the generated `.cc` into CMake can read `.source`.
#[derive(Debug, Clone)]
pub struct GeneratedVerinfo {
    /// Absolute path to the generated C header
    /// (`cmake_build/ccgo_generated/include/<verinfo_path>/verinfo_ccgo_gen.h`).
    pub header: PathBuf,
    /// Absolute path to the generated C++ source
    /// (`cmake_build/ccgo_generated/src/verinfo_ccgo_gen.cc`).
    pub source: PathBuf,
    /// The full VERIDENTITY string that was embedded (for logging).
    pub identity: String,
}

/// Generate `verinfo_ccgo_gen.h` + `verinfo_ccgo_gen.cc` under
/// `<project_root>/cmake_build/ccgo_generated/`. The CMake template picks
/// the generated `.cc` up via `list(APPEND SELF_SRC_FILES ...)` guarded by
/// `if(EXISTS ...)`.
///
/// * `project_root` — absolute project directory.
/// * `header_include_subdir` — value of `[build].verinfo_path`. Controls
///   the `#include` namespace of the generated header, e.g. `"stdcomm/base/"`
///   produces `cmake_build/ccgo_generated/include/stdcomm/base/verinfo_ccgo_gen.h`
///   which project code can pull in via
///   `#include "stdcomm/base/verinfo_ccgo_gen.h"`.
/// * `project_name` — used for the `<PROJECT>_CCGO_PROJECT_VERIDENTITY`
///   macro / symbol name.
/// * `identity` — pre-computed `<ver>-<ts>-<sha>[-dirty]` string.
pub fn generate(
    project_root: &Path,
    header_include_subdir: &str,
    project_name: &str,
    identity: &str,
) -> Result<GeneratedVerinfo> {
    let generated_root = project_root.join(GENERATED_SUBDIR);
    let header_dir = generated_root.join("include").join(header_include_subdir);
    let source_dir = generated_root.join("src");

    fs::create_dir_all(&header_dir).with_context(|| {
        format!(
            "Failed to create verinfo header dir {}",
            header_dir.display()
        )
    })?;
    fs::create_dir_all(&source_dir).with_context(|| {
        format!(
            "Failed to create verinfo source dir {}",
            source_dir.display()
        )
    })?;

    // `<PROJECT>_CCGO_PROJECT_VERIDENTITY` — namespaced to avoid colliding
    // with any `<PROJECT>_VERIDENTITY` the project may already define.
    let macro_name = format!("{}_CCGO_PROJECT_VERIDENTITY", project_name.to_uppercase());
    let symbol_name = format!("{}_ccgo_project_veridentity", project_name.to_lowercase());

    let header_path = header_dir.join("verinfo_ccgo_gen.h");
    let header = render_header(&macro_name, &symbol_name, identity);
    write_if_changed(&header_path, &header)?;

    let source_path = source_dir.join("verinfo_ccgo_gen.cc");
    let source = render_source(&macro_name, &symbol_name, identity, project_name);
    write_if_changed(&source_path, &source)?;

    Ok(GeneratedVerinfo {
        header: header_path,
        source: source_path,
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
#ifndef CCGO_VERINFO_CCGO_GEN_H_
#define CCGO_VERINFO_CCGO_GEN_H_

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

#endif  /* CCGO_VERINFO_CCGO_GEN_H_ */
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
 *
 * Compiled as C++ on every platform (Apple included) — the code body is
 * pure C, so the .cc extension is safe everywhere and lets the CMake
 * template pick up a single translation unit without per-platform fan-out.
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
    fs::write(path, content).with_context(|| format!("Failed to write {}", path.display()))?;
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
            "foo/base/",
            "foo",
            "1.2.3-20260101000000-abc1234",
        )
        .unwrap();

        // Files land under cmake_build/ccgo_generated/, not in the source tree.
        let expected_header = tmp
            .path()
            .join("cmake_build/ccgo_generated/include/foo/base/verinfo_ccgo_gen.h");
        let expected_source = tmp
            .path()
            .join("cmake_build/ccgo_generated/src/verinfo_ccgo_gen.cc");
        assert_eq!(out.header, expected_header);
        assert_eq!(out.source, expected_source);
        assert!(out.header.is_file());
        assert!(out.source.is_file());

        let header = fs::read_to_string(&out.header).unwrap();
        assert!(header.contains("FOO_CCGO_PROJECT_VERIDENTITY_STRING"));
        assert!(header.contains("1.2.3-20260101000000-abc1234"));
        assert!(header.contains("CCGO_VERINFO_CCGO_GEN_H_"));

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
