//! Symbol collection shared by native Apple platform builders.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};

pub(super) fn prepare_library(
    library: &Path,
    destination: &Path,
    shared: bool,
    release: bool,
) -> Result<(PathBuf, Option<PathBuf>)> {
    let dsym = collect_symbols(library, destination, shared)?;
    // Static DWARF must survive until the consuming application is linked.
    if !release || !shared {
        return Ok((library.to_path_buf(), dsym));
    }

    // Keep CMake outputs intact so incremental builds can collect full symbols again.
    let sdk_dir = library
        .parent()
        .context("Library has no parent")?
        .join("sdk_stripped");
    std::fs::create_dir_all(&sdk_dir)?;
    let stripped = sdk_dir.join(library.file_name().context("Library has no file name")?);
    std::fs::copy(library, &stripped)?;
    let output = std::process::Command::new("xcrun")
        .arg("strip")
        // Remove debug information while preserving symbols required by the linker/dyld.
        .arg("-S")
        .arg(&stripped)
        .output()
        .context("Failed to run strip for Apple SDK library")?;
    if !output.status.success() {
        bail!(
            "Failed to strip {}: {}",
            stripped.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    Ok((stripped, dsym))
}

pub(super) fn collect_symbols(
    library: &Path,
    destination: &Path,
    shared: bool,
) -> Result<Option<PathBuf>> {
    std::fs::create_dir_all(destination)?;
    let name = library.file_name().context("Library has no file name")?;
    if !shared {
        // Static archives retain DWARF in their object files; dSYM requires a linked image.
        std::fs::copy(library, destination.join(name))?;
        return Ok(None);
    }

    let dsym = destination.join(format!("{}.dSYM", name.to_string_lossy()));
    let output = std::process::Command::new("xcrun")
        .arg("dsymutil")
        .arg(library)
        .arg("-o")
        .arg(&dsym)
        .output()
        .context("Failed to run dsymutil for Apple symbols")?;
    if !output.status.success() {
        bail!(
            "Failed to generate dSYM for {}: {}",
            library.display(),
            String::from_utf8_lossy(&output.stderr)
        );
    }
    let dwarf = dsym.join("Contents/Resources/DWARF").join(name);
    if !dwarf.is_file() {
        bail!("dsymutil did not produce DWARF for {}", library.display());
    }
    // xcodebuild requires absolute paths for -debug-symbols.
    Ok(Some(std::fs::canonicalize(dsym)?))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::archive::ArchiveBuilder;

    #[test]
    #[cfg(target_os = "macos")]
    fn release_static_library_preserves_dwarf_and_remains_linkable() {
        use std::process::Command;
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("sample.c");
        let object = temp.path().join("sample.o");
        let library = temp.path().join("libsample.a");
        std::fs::write(&source, "int sample(void) { return 42; }\n").unwrap();
        assert!(Command::new("clang")
            .args(["-g", "-c"])
            .arg(&source)
            .arg("-o")
            .arg(&object)
            .status()
            .unwrap()
            .success());
        assert!(Command::new("ar")
            .arg("rcs")
            .arg(&library)
            .arg(&object)
            .status()
            .unwrap()
            .success());
        let original = std::fs::read(&library).unwrap();
        let symbols = temp.path().join("symbols");
        let (sdk, dsym) = prepare_library(&library, &symbols, false, true).unwrap();
        assert!(dsym.is_none());
        assert_eq!(
            std::fs::read(symbols.join("libsample.a")).unwrap(),
            original
        );
        assert_eq!(std::fs::read(&library).unwrap(), original);
        assert_eq!(sdk, library);
        assert_eq!(std::fs::read(&sdk).unwrap(), original);
        assert!(!temp.path().join("sdk_stripped").exists());
        std::fs::write(
            &source,
            "int sample(void); int main(void) { return sample() == 42 ? 0 : 1; }\n",
        )
        .unwrap();
        let executable = temp.path().join("consumer");
        assert!(Command::new("clang")
            .arg(&source)
            .arg(&sdk)
            .arg("-o")
            .arg(&executable)
            .status()
            .unwrap()
            .success());
        assert!(Command::new(executable).status().unwrap().success());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn release_shared_library_preserves_symbols_uuid_and_incremental_input() {
        use std::process::Command;
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("sample.c");
        let library = temp.path().join("libsample.dylib");
        std::fs::write(&source, "int sample(int value) { return value + 42; }\n").unwrap();
        assert!(Command::new("clang")
            .args(["-g", "-O2", "-dynamiclib"])
            .arg(&source)
            .arg("-o")
            .arg(&library)
            .status()
            .unwrap()
            .success());
        let original = std::fs::read(&library).unwrap();
        let destination = temp.path().join("symbols");
        let (sdk, dsym) = prepare_library(&library, &destination, true, true).unwrap();
        assert_ne!(sdk, library);
        assert_eq!(std::fs::read(&library).unwrap(), original);
        assert!(std::fs::metadata(&sdk).unwrap().len() < original.len() as u64);
        let uuid = |path: &Path| {
            let output = Command::new("xcrun")
                .args(["dwarfdump", "--uuid"])
                .arg(path)
                .output()
                .unwrap();
            assert!(output.status.success());
            String::from_utf8(output.stdout)
                .unwrap()
                .split_whitespace()
                .nth(1)
                .unwrap()
                .to_string()
        };
        assert_eq!(uuid(&sdk), uuid(&dsym.unwrap()));
        let exports = Command::new("nm").args(["-gU"]).arg(&sdk).output().unwrap();
        assert!(exports.status.success());
        assert!(String::from_utf8_lossy(&exports.stdout).contains("_sample"));
        // A second packaging run must still be able to generate symbols.
        prepare_library(&library, &destination, true, true).unwrap();
    }

    #[test]
    fn static_symbols_preserve_platform_target_libraries() {
        let temp = tempfile::tempdir().unwrap();
        let library = temp.path().join("libsample.a");
        for platform in ["ios", "macos", "tvos", "watchos"] {
            let staging = temp.path().join(platform).join("staging");
            let targets: &[&str] = if platform == "macos" {
                &["universal"]
            } else {
                &["device", "simulator"]
            };
            for target in targets {
                std::fs::write(&library, target.as_bytes()).unwrap();
                let destination = staging
                    .join("symbols")
                    .join(platform)
                    .join("static")
                    .join(target);
                assert!(collect_symbols(&library, &destination, false)
                    .unwrap()
                    .is_none());
                assert_eq!(
                    std::fs::read(destination.join("libsample.a")).unwrap(),
                    target.as_bytes()
                );
            }
            let archive = ArchiveBuilder::new(
                "sample",
                "1.0.0",
                "",
                true,
                platform,
                temp.path().join("output"),
            )
            .unwrap();
            let path = archive.create_symbols_archive(&staging).unwrap();
            let zip = zip::ZipArchive::new(std::fs::File::open(path).unwrap()).unwrap();
            let names: Vec<_> = zip.file_names().collect();
            for target in targets {
                assert!(names
                    .contains(&format!("symbols/{platform}/static/{target}/libsample.a").as_str()));
            }
        }
    }
}
