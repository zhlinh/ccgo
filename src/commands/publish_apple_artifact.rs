//! Upload Apple binaries, then publish descriptors that reference immutable URLs.
use super::{
    artifact::{coordinate, PreparedUpload},
    input, AppleManager, PublishCommand, RegistryType,
};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

fn podspec(project: &Path) -> Result<Value> {
    let path = input::unique_file(project, "podspec", false)?;
    let result = Command::new("pod")
        .args(["ipc", "spec"])
        .arg(&path)
        .current_dir(project)
        .output()
        .context("Failed to execute pod ipc spec")?;
    if !result.status.success() {
        bail!("pod ipc spec failed for {}", path.display());
    }
    let value: Value = serde_json::from_slice(&result.stdout).context("Invalid podspec JSON")?;
    if !value.is_object() {
        bail!("Podspec must describe an object");
    }
    Ok(value)
}

fn swift_manifest(text: &str, url: &str, checksum: &str) -> Result<String> {
    // A binary wrapper with one remote target is unambiguous. Multiple targets
    // need separate archives/coordinates and must not all point to this ZIP.
    let targets = regex::Regex::new(r"(?s)\.binaryTarget\s*\([^)]*\)")?;
    let matches: Vec<_> = targets.find_iter(text).collect();
    if matches.len() != 1 {
        bail!("Package.swift must contain exactly one remote binaryTarget");
    }
    let target = matches[0];
    let url_re = regex::Regex::new(r#"\burl\s*:\s*"[^"\n]*""#)?;
    let checksum_re = regex::Regex::new(r#"\bchecksum\s*:\s*"[^"\n]*""#)?;
    if !url_re.is_match(target.as_str()) || !checksum_re.is_match(target.as_str()) {
        bail!("binaryTarget must use literal url and checksum strings");
    }
    let updated = url_re.replace(
        target.as_str(),
        regex::NoExpand(&format!("url: {}", serde_json::to_string(url)?)),
    );
    let updated = checksum_re.replace(
        &updated,
        regex::NoExpand(&format!("checksum: \"{checksum}\"")),
    );
    Ok(format!(
        "{}{}{}",
        &text[..target.start()],
        updated,
        &text[target.end()..]
    ))
}

fn add_apple_symlink(
    archive: &mut zip::ZipWriter<fs::File>,
    path: &Path,
    relative: &Path,
    directory: &Path,
) -> Result<()> {
    let target = fs::read_link(path)?;
    if target.is_absolute() || !fs::canonicalize(path)?.starts_with(directory) {
        bail!("Apple artifact symlinks must resolve within the input directory");
    }
    archive.add_symlink(
        relative.to_string_lossy(),
        target.to_string_lossy(),
        zip::write::SimpleFileOptions::default(),
    )?;
    Ok(())
}

fn zip_directory(directory: &Path, output: &Path) -> Result<()> {
    use std::io::Write;
    let mut archive = zip::ZipWriter::new(fs::File::create(output)?);
    let directory = fs::canonicalize(directory)?;
    let base = if directory.extension().and_then(|s| s.to_str()) == Some("xcframework") {
        directory.parent().context("XCFramework has no parent")?
    } else {
        directory.as_path()
    };
    for entry in walkdir::WalkDir::new(&directory).sort_by_file_name() {
        let entry = entry?;
        let relative = entry.path().strip_prefix(base)?;
        if relative.as_os_str().is_empty() {
            continue;
        }
        if entry.file_type().is_symlink() {
            add_apple_symlink(&mut archive, entry.path(), relative, &directory)?;
        }
        if entry.file_type().is_file() {
            archive.start_file(
                relative.to_string_lossy(),
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Deflated),
            )?;
            archive.write_all(&fs::read(entry.path())?)?;
        }
    }
    archive.finish()?;
    Ok(())
}

fn apple_artifact_source(
    command: &PublishCommand,
    project: &Path,
    name: &str,
    version: &str,
) -> Result<PathBuf> {
    Ok(if let Some(path) = &command.path {
        path.clone()
    } else {
        let platforms = match command.platform.as_deref() {
            Some("ios") => vec!["ios"],
            Some("macos") => vec!["macos"],
            None => vec!["ios", "macos"],
            Some(_) => bail!("Apple artifact --platform must be ios or macos"),
        };
        let mut candidates = Vec::new();
        for platform in platforms {
            let directory = project.join("target/release").join(platform);
            if !directory.is_dir() {
                continue;
            }
            for entry in fs::read_dir(directory)? {
                let path = entry?.path();
                let filename = path.file_name().unwrap_or_default().to_string_lossy();
                let prefix = format!(
                    "{}_{}_SDK-{version}",
                    name.to_uppercase(),
                    platform.to_uppercase()
                );
                if path.is_file()
                    && (filename == format!("{prefix}.zip")
                        || filename.starts_with(&format!("{prefix}-")))
                    && filename.ends_with(".zip")
                    && !filename.ends_with("-SYMBOLS.zip")
                {
                    candidates.push(path);
                }
            }
        }
        if candidates.len() != 1 {
            bail!(
                "Expected one release Apple SDK ZIP, found {}; use --path to select it",
                candidates.len()
            );
        }
        candidates.remove(0)
    })
}

fn validate_apple_archive(archive: &Path, spm: bool) -> Result<()> {
    // Inspect without extracting: valid XCFramework ZIPs may contain symlinks.
    let mut zip = zip::ZipArchive::new(fs::File::open(archive)?)?;
    let mut framework = false;
    let mut root_frameworks = std::collections::BTreeSet::new();
    for i in 0..zip.len() {
        let entry = zip.by_index(i)?;
        let path = entry.enclosed_name().context("Unsafe path in Apple ZIP")?;
        framework |= path
            .components()
            .any(|p| p.as_os_str().to_string_lossy().ends_with(".xcframework"));
        if let Some(first) = path.components().next() {
            if first
                .as_os_str()
                .to_string_lossy()
                .ends_with(".xcframework")
            {
                root_frameworks.insert(first.as_os_str().to_owned());
            }
        }
    }
    if !framework {
        bail!("Apple ZIP must contain an XCFramework");
    }
    if spm && root_frameworks.len() != 1 {
        bail!("SPM requires exactly one XCFramework at the ZIP root; use --manager cocoapods for a nested SDK ZIP");
    }
    Ok(())
}

fn publish_podspec(
    command: &PublishCommand,
    project: &Path,
    path: &Path,
    url: &str,
    verbose: bool,
) -> Result<()> {
    let mut pod = Command::new("pod");
    pod.current_dir(project);
    match command.registry {
        RegistryType::Local => {
            pod.args(["spec", "lint"]);
        }
        RegistryType::Official => {
            pod.args(["trunk", "push"]);
        }
        RegistryType::Private => {
            pod.args(["repo", "push", command.remote.as_ref().unwrap()]);
        }
    }
    pod.arg(path);
    if command.allow_warnings {
        pod.arg("--allow-warnings");
    }
    if verbose {
        pod.arg("--verbose");
    }
    if !pod.status()?.success() {
        bail!("CocoaPods publish failed; binary was already uploaded at {url}");
    }
    Ok(())
}

fn load_apple_manifests(
    project: &Path,
    version: &str,
    pods: bool,
    spm: bool,
) -> Result<(Option<Value>, Option<String>)> {
    let spec = if pods { Some(podspec(project)?) } else { None };
    if let Some(spec) = &spec {
        if spec.get("version").and_then(Value::as_str) != Some(version) {
            bail!("Podspec version must match CCGO.toml version {version}");
        }
    }
    let swift = if spm {
        Some(fs::read_to_string(project.join("Package.swift"))?)
    } else {
        None
    };
    Ok((spec, swift))
}

fn write_apple_manifests(
    project: &Path,
    version: &str,
    artifact: &str,
    spec: Option<&Value>,
    swift: Option<&str>,
) -> Result<(PathBuf, Option<PathBuf>)> {
    let output = project.join("target/ccgo-publish/apple").join(version);
    fs::create_dir_all(&output)?;
    let spec_path = spec
        .as_ref()
        .map(|_| output.join(format!("{artifact}.podspec.json")));
    if let (Some(spec), Some(path)) = (spec, &spec_path) {
        fs::write(path, serde_json::to_vec_pretty(spec)?)?;
    }
    if let Some(swift) = swift {
        fs::write(output.join("Package.swift"), swift)?;
    }
    Ok((output, spec_path))
}

fn prepare_apple_archive(source: &Path, archive: &Path, spm: bool) -> Result<String> {
    if source.is_dir() {
        zip_directory(source, archive)?;
    } else {
        if source.extension().and_then(|s| s.to_str()) != Some("zip") {
            bail!("Apple artifact --path must be a directory or ZIP");
        }
        fs::copy(source, archive).context("Failed to read Apple ZIP")?;
    }
    validate_apple_archive(archive, spm)?;
    let bytes = fs::read(archive)?;
    let checksum = format!("{:x}", Sha256::digest(&bytes));
    Ok(checksum)
}

struct AppleRelease<'a> {
    name: &'a str,
    version: String,
    release_tag: String,
    artifact: String,
}

fn apple_release<'a>(
    command: &PublishCommand,
    configuration: &'a toml::Value,
) -> Result<AppleRelease<'a>> {
    let package = configuration
        .get("project")
        .or_else(|| configuration.get("package"))
        .context("CCGO.toml requires [project] or [package]")?;
    let name = package
        .get("name")
        .and_then(|v| v.as_str())
        .context("Project name is missing")?;
    let configured_version = package
        .get("version")
        .and_then(|v| v.as_str())
        .context("Project version is missing")?;
    let (resolved_version, release_tag) = command.resolve_version_and_tag(configured_version)?;
    let version = resolved_version.as_str();
    let artifact = command
        .artifact_id
        .clone()
        .unwrap_or_else(|| format!("{name}-apple"));
    coordinate(&artifact)?;
    coordinate(version)?;
    Ok(AppleRelease {
        name,
        version: resolved_version,
        release_tag,
        artifact,
    })
}

fn validate_apple_registry(command: &PublishCommand) -> Result<()> {
    let pods = matches!(command.manager, AppleManager::Cocoapods | AppleManager::All);
    if pods && matches!(command.registry, RegistryType::Private) && command.remote.is_none() {
        bail!("Private CocoaPods registry requires --remote");
    }
    Ok(())
}

pub(super) fn publish(command: &PublishCommand, verbose: bool) -> Result<()> {
    let project = std::env::current_dir()?;
    let pods = matches!(command.manager, AppleManager::Cocoapods | AppleManager::All);
    let spm = matches!(command.manager, AppleManager::Spm | AppleManager::All);
    validate_apple_registry(command)?;
    let configuration: toml::Value =
        toml::from_str(&fs::read_to_string(project.join("CCGO.toml"))?)?;
    let AppleRelease {
        name,
        version,
        release_tag,
        artifact,
    } = apple_release(command, &configuration)?;
    let version = version.as_str();
    let temporary = tempfile::tempdir()?;
    let archive = temporary.path().join(format!("{artifact}-{version}.zip"));
    let source = apple_artifact_source(command, &project, name, version)?;
    let checksum = prepare_apple_archive(&source, &archive, spm)?;
    let (mut spec, swift) = load_apple_manifests(&project, version, pods, spm)?;
    let upload = PreparedUpload::prepare(
        command,
        &configuration,
        name,
        version,
        &release_tag,
        "apple",
    )?;
    let url = &upload.url;
    let swift = swift
        .map(|text| swift_manifest(&text, url, &checksum))
        .transpose()?;
    if let Some(spec) = &mut spec {
        spec["source"] = serde_json::json!({"http": url, "sha256": checksum});
    }
    if spm && !command.push {
        println!("Apple SPM binary validated: version {version}, tag {release_tag}. No files generated or artifacts uploaded; use --push to upload and generate the wrapper.");
        return Ok(());
    }
    let (output, spec_path) = write_apple_manifests(
        &project,
        version,
        &artifact,
        spec.as_ref(),
        swift.as_deref(),
    )?;
    upload.upload(&archive)?;
    println!(
        "Apple binary URL: {url}\nSHA-256: {checksum}\nGenerated manifests: {}",
        output.display()
    );
    if let Some(path) = spec_path {
        publish_podspec(command, &project, &path, url, verbose)?;
    }
    if spm {
        println!("Commit the generated Package.swift to your SPM wrapper repository, then push its version tag.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn swift_requires_one_remote_binary() {
        let text = r#".binaryTarget(name: "SDK", url: "https://old/sdk.zip", checksum: "old")"#;
        let result = swift_manifest(text, "https://new/sdk.zip", "abc").unwrap();
        assert!(result.contains("https://new/sdk.zip"));
        assert!(result.contains("checksum: \"abc\""));
        assert!(swift_manifest(&format!("{text},{text}"), "url", "sha").is_err());
        assert!(swift_manifest(
            ".binaryTarget(name: \"SDK\", path: \"local\")",
            "url",
            "sha"
        )
        .is_err());
    }
}
