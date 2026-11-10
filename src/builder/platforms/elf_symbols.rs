//! Strip only distributable ELF shared libraries, after saving full symbols.

use std::path::Path;

use anyhow::{bail, Context, Result};

pub(super) fn strip_staged_shared_libraries(directory: &Path, strip: &Path) -> Result<()> {
    for entry in walkdir::WalkDir::new(directory) {
        let entry = entry?;
        // Symlink targets are processed once, through their regular file entry.
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy();
        if !name.ends_with(".so") && !name.contains(".so.") {
            continue;
        }
        let output = std::process::Command::new(strip)
            .arg("--strip-unneeded")
            .arg(entry.path())
            .output()
            .with_context(|| format!("Failed to run {}", strip.display()))?;
        if !output.status.success() {
            bail!(
                "Failed to strip {}: {}",
                entry.path().display(),
                String::from_utf8_lossy(&output.stderr)
            );
        }
    }
    Ok(())
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::fs::PermissionsExt;

    #[test]
    fn strips_shared_copies_including_sonames_but_preserves_static_and_symbols() {
        let temp = tempfile::tempdir().unwrap();
        let staging = temp.path().join("sdk");
        let symbols = temp.path().join("symbols");
        std::fs::create_dir_all(&staging).unwrap();
        std::fs::create_dir_all(&symbols).unwrap();
        for name in ["libdemo.so", "libdemo.so.1", "libdemo.a"] {
            std::fs::write(staging.join(name), b"full debug info").unwrap();
            std::fs::write(symbols.join(name), b"full debug info").unwrap();
        }
        let strip = temp.path().join("strip");
        std::fs::write(
            &strip,
            "#!/bin/sh\n[ \"$1\" = --strip-unneeded ] || exit 2\nprintf stripped > \"$2\"\n",
        )
        .unwrap();
        std::fs::set_permissions(&strip, std::fs::Permissions::from_mode(0o755)).unwrap();
        strip_staged_shared_libraries(&staging, &strip).unwrap();
        for name in ["libdemo.so", "libdemo.so.1"] {
            assert_eq!(std::fs::read(staging.join(name)).unwrap(), b"stripped");
            assert_eq!(
                std::fs::read(symbols.join(name)).unwrap(),
                b"full debug info"
            );
        }
        assert_eq!(
            std::fs::read(staging.join("libdemo.a")).unwrap(),
            b"full debug info"
        );
        std::fs::write(&strip, "#!/bin/sh\nexit 1\n").unwrap();
        assert!(strip_staged_shared_libraries(&staging, &strip).is_err());
    }
}
