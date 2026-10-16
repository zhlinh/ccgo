//! Stage Windows debug artifacts separately from distributable libraries.
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::builder::archive::ArchiveBuilder;

pub(super) struct WindowsSymbols {
    staging: tempfile::TempDir,
    has_files: bool,
}

impl WindowsSymbols {
    pub fn new() -> Result<Self> {
        Ok(Self {
            staging: tempfile::tempdir()?,
            has_files: false,
        })
    }

    fn directory(&self, arch: &str, toolchain: &str, link_type: &str) -> PathBuf {
        self.staging
            .path()
            .join("symbols/windows/obj")
            .join(arch)
            .join(toolchain)
            .join(link_type)
    }

    pub fn collect_pdbs(&mut self, build_dir: &Path, arch: &str, link_type: &str) -> Result<()> {
        let destination = self.directory(arch, "msvc", link_type);
        // Preserve relative paths: compiler and linker PDBs can share filenames.
        for entry in walkdir::WalkDir::new(build_dir) {
            let entry = entry?;
            if !entry.file_type().is_file()
                || !entry
                    .path()
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("pdb"))
            {
                continue;
            }
            let relative = entry.path().strip_prefix(build_dir)?;
            self.copy(entry.path(), &destination.join(relative))?;
        }
        Ok(())
    }

    pub fn collect_dlls(&mut self, libraries: &[PathBuf], arch: &str) -> Result<()> {
        let destination = self.directory(arch, "mingw", "shared");
        for library in libraries {
            if library
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("dll"))
            {
                self.copy(library, &destination.join(library.file_name().unwrap()))?;
            }
        }
        Ok(())
    }

    fn copy(&mut self, source: &Path, destination: &Path) -> Result<()> {
        std::fs::create_dir_all(destination.parent().unwrap())?;
        std::fs::copy(source, destination)
            .with_context(|| format!("Failed to save Windows symbols from {}", source.display()))?;
        self.has_files = true;
        Ok(())
    }

    pub fn finish(self, archive: &ArchiveBuilder) -> Result<Option<PathBuf>> {
        if self.has_files {
            Ok(Some(archive.create_symbols_archive(self.staging.path())?))
        } else {
            Ok(None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{fs, io::Read};

    fn archive(root: &Path) -> ArchiveBuilder {
        ArchiveBuilder::new("demo", "1.2.3", "", true, "windows", root.to_path_buf()).unwrap()
    }

    #[test]
    fn archives_pdbs_without_collisions_and_preserves_unstripped_dlls() {
        let root = tempfile::tempdir().unwrap();
        let build = root.path().join("build");
        fs::create_dir_all(build.join("nested")).unwrap();
        fs::write(build.join("demo.pdb"), b"linker pdb").unwrap();
        fs::write(build.join("nested/demo.PDB"), b"compiler pdb").unwrap();
        fs::write(build.join("demo.dll"), b"unstripped dll").unwrap();
        fs::write(build.join("demo.lib"), b"import library").unwrap();
        let mut symbols = WindowsSymbols::new().unwrap();
        let staging = symbols.staging.path().to_path_buf();
        symbols.collect_pdbs(&build, "x86_64", "shared").unwrap();
        symbols.collect_pdbs(&build, "aarch64", "static").unwrap();
        symbols
            .collect_dlls(&[build.join("demo.dll"), build.join("demo.lib")], "x86_64")
            .unwrap();
        // Simulate strip modifying the SDK DLL after collecting its debug copy.
        fs::write(build.join("demo.dll"), b"stripped dll").unwrap();
        let path = symbols
            .finish(&archive(&root.path().join("output")))
            .unwrap()
            .unwrap();
        assert!(path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .ends_with("-SYMBOLS.zip"));
        assert!(!staging.exists());
        let mut zip = zip::ZipArchive::new(fs::File::open(path).unwrap()).unwrap();
        for (name, expected) in [
            (
                "symbols/windows/obj/x86_64/msvc/shared/demo.pdb",
                "linker pdb",
            ),
            (
                "symbols/windows/obj/x86_64/msvc/shared/nested/demo.PDB",
                "compiler pdb",
            ),
            (
                "symbols/windows/obj/aarch64/msvc/static/demo.pdb",
                "linker pdb",
            ),
            (
                "symbols/windows/obj/x86_64/mingw/shared/demo.dll",
                "unstripped dll",
            ),
        ] {
            let mut contents = String::new();
            zip.by_name(name)
                .unwrap()
                .read_to_string(&mut contents)
                .unwrap();
            assert_eq!(contents, expected);
        }
        assert!(zip.file_names().all(|name| !name.ends_with(".lib")));
        assert_eq!(fs::read(build.join("demo.dll")).unwrap(), b"stripped dll");
    }

    #[test]
    fn empty_symbols_do_not_create_an_archive_and_staging_is_cleaned_up() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("demo.lib"), b"static library").unwrap();
        let mut symbols = WindowsSymbols::new().unwrap();
        let staging = symbols.staging.path().to_path_buf();
        symbols
            .collect_pdbs(root.path(), "x86_64", "static")
            .unwrap();
        symbols
            .collect_dlls(&[root.path().join("demo.lib")], "x86_64")
            .unwrap();
        assert!(symbols
            .finish(&archive(&root.path().join("output")))
            .unwrap()
            .is_none());
        assert!(!staging.exists());
        let symbols = WindowsSymbols::new().unwrap();
        let staging = symbols.staging.path().to_path_buf();
        drop(symbols);
        assert!(!staging.exists());
    }
}
