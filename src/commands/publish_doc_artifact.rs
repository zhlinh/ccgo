//! Versioned static documentation archives stored in Maven.
use super::{artifact::PreparedUpload, input, ArtifactProvider, PublishCommand};
use anyhow::{bail, Context, Result};
use std::{fs, path::Path, process::Command};

fn archive_site(site: &Path, output: &Path) -> Result<()> {
    use std::io::Write;
    if !site.join("index.html").is_file() {
        bail!("Built documentation must contain index.html");
    }
    let mut zip = zip::ZipWriter::new(fs::File::create(output)?);
    for entry in walkdir::WalkDir::new(site).sort_by_file_name() {
        let entry = entry?;
        if entry.file_type().is_symlink() {
            bail!("Documentation archive cannot contain symlinks");
        }
        if entry.file_type().is_file() {
            let relative = entry.path().strip_prefix(site)?;
            zip.start_file(
                relative.to_string_lossy().replace('\\', "/"),
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated),
            )?;
            zip.write_all(&fs::read(entry.path())?)?;
        }
    }
    zip.finish()?;
    Ok(())
}

pub(super) fn publish(command: &PublishCommand, verbose: bool) -> Result<()> {
    if !matches!(command.artifact_provider, Some(ArtifactProvider::Maven)) {
        bail!("Doc artifact publishing supports --artifact-provider maven only");
    }
    if command.skip_build || command.doc_open || command.doc_force {
        bail!("Doc Maven publishing does not support --skip-build, --doc-open or --doc-force");
    }
    let cwd = std::env::current_dir()?;
    let configuration: toml::Value = toml::from_str(&fs::read_to_string(cwd.join("CCGO.toml"))?)?;
    let project = configuration
        .get("project")
        .or_else(|| configuration.get("package"))
        .context("CCGO.toml requires [project] or [package]")?;
    let name = project
        .get("name")
        .and_then(|v| v.as_str())
        .context("Missing project name")?;
    let version = project
        .get("version")
        .and_then(|v| v.as_str())
        .context("Missing project version")?;
    let (version, tag) = command.resolve_version_and_tag(version)?;
    let upload = PreparedUpload::prepare(command, &configuration, name, &version, &tag, "doc")?;
    let config = match &command.path {
        Some(path) => path.clone(),
        None => input::mkdocs_config(&cwd)?,
    };
    let temporary = tempfile::tempdir()?;
    let site = temporary.path().join("site");
    let mut build = Command::new("mkdocs");
    build
        .current_dir(config.parent().context("MkDocs config has no parent")?)
        .args(["build", "--clean", "--config-file"])
        .arg(&config)
        .arg("--site-dir")
        .arg(&site);
    if verbose {
        build.arg("--verbose");
    }
    if !build
        .status()
        .context("Failed to execute mkdocs build")?
        .success()
    {
        bail!("Documentation build failed");
    }
    let archive = temporary.path().join("docs.zip");
    archive_site(&site, &archive)?;
    if command.push {
        upload.upload(&archive)?;
        println!("Documentation archive published: {}", upload.url);
    } else {
        println!(
            "Documentation archive validated. Use --push to upload to {}",
            upload.url
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_contains_site_at_root_and_requires_index() {
        let temporary = tempfile::tempdir().unwrap();
        let site = temporary.path().join("site");
        fs::create_dir_all(site.join("assets")).unwrap();
        let output = temporary.path().join("docs.zip");
        assert!(archive_site(&site, &output).is_err());
        fs::write(site.join("index.html"), "hello").unwrap();
        fs::write(site.join("assets/style.css"), "body {}").unwrap();
        archive_site(&site, &output).unwrap();
        let mut zip = zip::ZipArchive::new(fs::File::open(output).unwrap()).unwrap();
        assert_eq!(zip.len(), 2);
        assert!(zip.by_name("index.html").is_ok());
        assert!(zip.by_name("assets/style.css").is_ok());
    }
}
