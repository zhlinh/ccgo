//! Track installations whose lock metadata has not yet been saved.
use anyhow::Result;
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

fn path(project: &Path, name: &str) -> PathBuf {
    project
        .join(".ccgo/pending-installs")
        .join(format!("{:x}", md5::compute(name)))
}

pub(super) fn exists(project: &Path, name: &str) -> Result<bool> {
    match fs::symlink_metadata(path(project, name)) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error.into()),
    }
}

pub(super) fn begin(project: &Path, name: &str) -> Result<()> {
    let marker = path(project, name);
    let parent = marker.parent().unwrap();
    fs::create_dir_all(parent)?;
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    temporary.write_all(name.as_bytes())?;
    temporary.as_file().sync_all()?;
    temporary.persist(marker)?;
    Ok(())
}

pub(super) fn complete(project: &Path, names: &[String]) -> Result<()> {
    for name in names {
        match fs::remove_file(path(project, name)) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_only_clears_selected_dependencies() {
        let project = tempfile::tempdir().unwrap();
        begin(project.path(), "leaf").unwrap();
        begin(project.path(), "twig").unwrap();
        complete(project.path(), &["leaf".into()]).unwrap();
        assert!(!exists(project.path(), "leaf").unwrap());
        assert!(exists(project.path(), "twig").unwrap());
        complete(project.path(), &["leaf".into()]).unwrap();
    }
}
