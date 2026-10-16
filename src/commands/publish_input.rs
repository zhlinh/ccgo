//! Shared resolution rules for explicit publish inputs.
use anyhow::{bail, Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Select exactly one artifact; ambiguity must never choose a different build.
pub(super) fn unique_file(directory: &Path, extension: &str, recursive: bool) -> Result<PathBuf> {
    let mut files = Vec::new();
    let depth = if recursive { usize::MAX } else { 1 };
    for entry in walkdir::WalkDir::new(directory).max_depth(depth) {
        let entry = entry?;
        if entry.file_type().is_file()
            && entry.path().extension().and_then(|s| s.to_str()) == Some(extension)
        {
            files.push(entry.into_path());
        }
    }
    files.sort();
    match files.as_slice() {
        [file] => Ok(file.clone()),
        [] => bail!("No .{extension} file found in {}", directory.display()),
        _ => bail!(
            "Multiple .{extension} files found in {}; specify the exact file with --path",
            directory.display()
        ),
    }
}

pub(super) fn mkdocs_config(directory: &Path) -> Result<PathBuf> {
    let files: Vec<_> = ["mkdocs.yml", "mkdocs.yaml"]
        .iter()
        .map(|name| directory.join(name))
        .filter(|file| file.is_file())
        .collect();
    match files.as_slice() {
        [file] => Ok(file.clone()),
        [] => bail!(
            "No mkdocs.yml or mkdocs.yaml found in {}",
            directory.display()
        ),
        _ => bail!(
            "Both mkdocs.yml and mkdocs.yaml exist in {}; specify the config file with --path",
            directory.display()
        ),
    }
}

/// Archives can wrap an otherwise valid project in enclosing directories.
pub(super) fn archive_descriptor(directory: &Path, names: &[&str]) -> Result<PathBuf> {
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(directory) {
        let entry = entry?;
        if entry.file_type().is_file() && names.iter().any(|name| entry.file_name() == *name) {
            files.push(entry.into_path());
        }
    }
    files.sort();
    match files.as_slice() {
        [file] => Ok(file.clone()),
        [] => bail!("ZIP must contain one {}", names.join(" or ")),
        _ => bail!(
            "ZIP contains multiple {} files; unpack it and specify the exact file with --path",
            names.join(" / ")
        ),
    }
}

pub(super) fn extract_zip(input: &Path, destination: &Path) -> Result<()> {
    let mut zip = zip::ZipArchive::new(fs::File::open(input)?)
        .with_context(|| format!("Invalid publish ZIP: {}", input.display()))?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let relative = entry
            .enclosed_name()
            .context("Unsafe path in publish ZIP")?;
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            bail!("Symlinks are not supported in publish ZIPs");
        }
        let path = destination.join(relative);
        if entry.is_dir() {
            fs::create_dir_all(path)?;
        } else {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)?;
            }
            std::io::copy(&mut entry, &mut fs::File::create(path)?)?;
        }
    }
    Ok(())
}
