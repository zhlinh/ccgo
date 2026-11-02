//! Upload Apple binaries, then publish descriptors that reference immutable URLs.
use super::{
    artifact::{coordinate, PreparedUpload},
    input, AppleLinkage, AppleManager, PublishCommand, RegistryType,
};
use anyhow::{bail, Context, Result};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{
    fs,
    io::Write,
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

fn manifest_dir(project: &Path, version: &str) -> Result<PathBuf> {
    let output = project.join("target/ccgo-publish/apple").join(version);
    fs::create_dir_all(&output)?;
    Ok(output)
}

fn write_podspec(
    project: &Path,
    version: &str,
    artifact_id: &str,
    spec: &Value,
) -> Result<PathBuf> {
    let output = manifest_dir(project, version)?;
    let path = output.join(format!("{artifact_id}.podspec.json"));
    fs::write(&path, serde_json::to_vec_pretty(spec)?)?;
    Ok(path)
}

fn strip_shared_suffix(name: &str) -> &str {
    name.strip_suffix("-shared")
        .or_else(|| name.strip_suffix("_shared"))
        .or_else(|| name.strip_suffix("Shared"))
        .unwrap_or(name)
}

/// `Demo` -> `DemoShared`; `Dep/Sub` -> `DepShared/Sub`.
fn shared_name(name: &str) -> String {
    let (root, suffix) = match name.split_once('/') {
        Some((root, subspec)) => (root, format!("/{subspec}")),
        None => (name, String::new()),
    };
    format!("{}Shared{suffix}", strip_shared_suffix(root))
}

/// Which dependencies get a `-shared` suffix in the dynamic manifests.
///
/// Defaults to every `[[dependencies]]` name (pod names are matched
/// case-insensitively before any subspec/product path). External packages such
/// as `Alamofire` are therefore left untouched unless declared as ccgo deps.
#[derive(Default)]
struct SharedSuffixPolicy {
    dependencies: BTreeSet<String>,
    exclude: BTreeSet<String>,
}

fn string_set(value: Option<&toml::Value>) -> BTreeSet<String> {
    value
        .and_then(toml::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(toml::Value::as_str)
        .map(|name| name.to_lowercase())
        .collect()
}

impl SharedSuffixPolicy {
    fn from_config(config: &toml::Value) -> Self {
        let declared: BTreeSet<String> = config
            .get("dependencies")
            .and_then(toml::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|dependency| dependency.get("name").and_then(toml::Value::as_str))
            .map(|name| name.to_lowercase())
            .collect();

        let apple = config
            .get("publish")
            .and_then(|publish| publish.get("apple"));
        let explicit = apple.and_then(|apple| apple.get("shared_suffix"));
        let dependencies = if explicit.is_some() {
            string_set(explicit)
        } else {
            declared
        };
        let exclude = string_set(apple.and_then(|apple| apple.get("no_shared_suffix")));
        Self {
            dependencies,
            exclude,
        }
    }

    fn should_suffix(&self, name: &str) -> bool {
        let root = name.split('/').next().unwrap_or(name).to_lowercase();
        !self.exclude.contains(&root) && self.dependencies.contains(&root)
    }
}

/// Point dependency declarations at the `-shared` pod for dynamic artifacts.
///
/// A shared library links against the shared form of its dependencies, so
/// publishing it under the static dependency names would make CocoaPods
/// resolve the wrong binary.
fn rewrite_shared_dependencies(spec: &mut Value, policy: &SharedSuffixPolicy) {
    if let Some(dependencies) = spec.get_mut("dependencies").and_then(Value::as_object_mut) {
        let renames: Vec<(String, String, Value)> = dependencies
            .iter()
            .filter(|(name, _)| !name.ends_with("-shared") && policy.should_suffix(name))
            .map(|(name, requirement)| (name.clone(), shared_name(name), requirement.clone()))
            .collect();
        for (name, shared, requirement) in renames {
            dependencies.remove(&name);
            dependencies.insert(shared, requirement);
        }
    }
    if let Some(subspecs) = spec.get_mut("subspecs").and_then(Value::as_array_mut) {
        for subspec in subspecs {
            rewrite_shared_dependencies(subspec, policy);
        }
    }
}

#[derive(Default)]
struct ApplePodNames {
    static_name: Option<String>,
    shared_name: Option<String>,
}

impl ApplePodNames {
    fn from_cli_and_config(command: &PublishCommand, config: &toml::Value) -> Self {
        let apple = config
            .get("publish")
            .and_then(|publish| publish.get("apple"));
        let configured = |key: &str| {
            apple
                .and_then(|apple| apple.get(key))
                .and_then(toml::Value::as_str)
                .map(str::to_string)
        };
        Self {
            static_name: command.pod_name.clone().or_else(|| configured("pod_name")),
            shared_name: command
                .shared_pod_name
                .clone()
                .or_else(|| configured("shared_pod_name")),
        }
    }
}

fn podspec_for(
    base: &Value,
    artifact: &AppleArtifact,
    policy: &SharedSuffixPolicy,
    names: &ApplePodNames,
) -> Result<Value> {
    let mut spec = base.clone();
    let original_name = spec
        .get("name")
        .and_then(Value::as_str)
        .context("podspec must have a name")?
        .to_string();
    let module_name = spec
        .get("module_name")
        .and_then(Value::as_str)
        .unwrap_or(&original_name)
        .to_string();
    let default_static = strip_shared_suffix(&original_name).to_string();
    let static_name = names
        .static_name
        .clone()
        .unwrap_or_else(|| default_static.clone());
    let shared_name = names
        .shared_name
        .clone()
        .unwrap_or_else(|| format!("{static_name}Shared"));

    match artifact.linkage {
        AppleLinkage::Static => {
            spec["name"] = serde_json::json!(static_name);
            if static_name != module_name {
                spec["module_name"] = serde_json::json!(module_name);
            }
        }
        AppleLinkage::Shared => {
            spec["name"] = serde_json::json!(shared_name);
            // The XCFramework module keeps the original name; without this,
            // CocoaPods would derive the module from the shared pod name.
            spec["module_name"] = serde_json::json!(module_name);
        }
        AppleLinkage::All => unreachable!("podspecs are generated per artifact"),
    }
    spec["source"] = serde_json::json!({
        "http": artifact.upload.url,
        "sha256": artifact.checksum,
    });
    spec["vendored_frameworks"] = serde_json::json!(artifact.xcframework);
    spec["static_framework"] = serde_json::json!(artifact.linkage == AppleLinkage::Static);
    if artifact.linkage == AppleLinkage::Shared {
        rewrite_shared_dependencies(&mut spec, policy);
    }
    Ok(spec)
}

fn linkage_dir(linkage: AppleLinkage) -> &'static str {
    match linkage {
        AppleLinkage::Static => "static",
        AppleLinkage::Shared => "shared",
        AppleLinkage::All => unreachable!("linkage_dir is only used for a concrete artifact"),
    }
}

fn linkage_from_name(name: &str) -> Option<AppleLinkage> {
    match name {
        "static" => Some(AppleLinkage::Static),
        "shared" => Some(AppleLinkage::Shared),
        _ => None,
    }
}

fn linkage_from_path(path: &Path) -> Option<AppleLinkage> {
    for component in path.components() {
        match component.as_os_str().to_string_lossy().as_ref() {
            "static" => return Some(AppleLinkage::Static),
            "shared" => return Some(AppleLinkage::Shared),
            _ => {}
        }
    }
    None
}

/// Locate `lib/<platform>/<static|shared>/<name>.xcframework` in an SDK ZIP.
///
/// Returns `None` when the ZIP is already a single-XCFramework archive.
fn nested_linkages(
    source: &Path,
    requested_platform: Option<&str>,
) -> Result<Option<(String, BTreeSet<AppleLinkage>)>> {
    let mut zip = zip::ZipArchive::new(fs::File::open(source)?)?;
    let mut found: BTreeMap<String, BTreeSet<AppleLinkage>> = BTreeMap::new();

    for index in 0..zip.len() {
        let name = zip.by_index(index)?.name().to_string();
        let mut parts = name.split('/');
        if parts.next() != Some("lib") {
            continue;
        }
        let Some(platform) = parts.next() else {
            continue;
        };
        let Some(linkage) = parts.next().and_then(linkage_from_name) else {
            continue;
        };
        let Some(framework) = parts.next() else {
            continue;
        };
        if !framework.ends_with(".xcframework") {
            continue;
        }
        found
            .entry(platform.to_string())
            .or_default()
            .insert(linkage);
    }

    if found.is_empty() {
        return Ok(None);
    }

    let platform = match requested_platform {
        Some(platform) => {
            if !found.contains_key(platform) {
                bail!(
                    "Apple SDK ZIP has no lib/{platform}/ XCFramework; found: {}",
                    found.keys().cloned().collect::<Vec<_>>().join(", ")
                );
            }
            platform.to_string()
        }
        None if found.len() == 1 => found.keys().next().unwrap().clone(),
        None => bail!(
            "Apple SDK ZIP contains multiple platforms ({}); pass --platform",
            found.keys().cloned().collect::<Vec<_>>().join(", ")
        ),
    };

    let available = found.remove(&platform).unwrap();
    Ok(Some((platform, available)))
}

fn selected_linkages(
    available: &BTreeSet<AppleLinkage>,
    requested: AppleLinkage,
) -> Result<Vec<AppleLinkage>> {
    let mut selected = Vec::new();
    match requested {
        AppleLinkage::Static => {
            if !available.contains(&AppleLinkage::Static) {
                bail!("Apple SDK ZIP has no static XCFramework; available: shared only");
            }
            selected.push(AppleLinkage::Static);
        }
        AppleLinkage::Shared => {
            if !available.contains(&AppleLinkage::Shared) {
                bail!("Apple SDK ZIP has no shared XCFramework; available: static only");
            }
            selected.push(AppleLinkage::Shared);
        }
        AppleLinkage::All => {
            for linkage in [AppleLinkage::Static, AppleLinkage::Shared] {
                if available.contains(&linkage) {
                    selected.push(linkage);
                }
            }
            if selected.is_empty() {
                bail!("Apple SDK ZIP contains no static or shared XCFramework");
            }
        }
    }
    Ok(selected)
}

fn single_linkage(source: &Path, requested: AppleLinkage) -> Result<AppleLinkage> {
    let inferred = linkage_from_path(source);
    match (inferred, requested) {
        (Some(actual), AppleLinkage::Static) if actual != AppleLinkage::Static => {
            bail!("Apple input is a shared XCFramework but --linkage is static")
        }
        (Some(actual), AppleLinkage::Shared) if actual != AppleLinkage::Shared => {
            bail!("Apple input is a static XCFramework but --linkage is shared")
        }
        (Some(actual), _) => Ok(actual),
        (None, AppleLinkage::Shared) => Ok(AppleLinkage::Shared),
        (None, _) => Ok(AppleLinkage::Static),
    }
}

/// Copy one nested XCFramework to a new ZIP with the XCFramework at its root.
fn repackage_linkage(
    source: &Path,
    platform: &str,
    linkage: &str,
    output: &Path,
) -> Result<String> {
    let mut source_zip = zip::ZipArchive::new(fs::File::open(source)?)?;
    let prefix = format!("lib/{platform}/{linkage}/");

    let mut root: Option<String> = None;
    for index in 0..source_zip.len() {
        let name = source_zip.by_index(index)?.name().to_string();
        let Some(rest) = name.strip_prefix(&prefix) else {
            continue;
        };
        if let Some(first) = rest.split('/').next() {
            if first.ends_with(".xcframework") {
                root = Some(first.to_string());
                break;
            }
        }
    }
    let root = root.with_context(|| {
        format!("Apple SDK ZIP has no {prefix}<name>.xcframework; cannot package {linkage}")
    })?;

    let mut output_zip = zip::ZipWriter::new(fs::File::create(output)?);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);
    let root_prefix = format!("{prefix}{root}/");

    for index in 0..source_zip.len() {
        let mut entry = source_zip.by_index(index)?;
        let name = entry.name().to_string();
        if !name.starts_with(&root_prefix) || entry.is_dir() {
            continue;
        }
        let relative = name.strip_prefix(&prefix).unwrap();
        if entry
            .unix_mode()
            .is_some_and(|mode| mode & 0o170000 == 0o120000)
        {
            let mut target = String::new();
            std::io::Read::read_to_string(&mut entry, &mut target)?;
            output_zip.add_symlink(relative, target, options)?;
        } else {
            output_zip.start_file(relative, options)?;
            std::io::copy(&mut entry, &mut output_zip)?;
        }
    }
    output_zip.finish()?;
    Ok(root)
}

fn xcframework_path(archive: &Path) -> Result<String> {
    let mut zip = zip::ZipArchive::new(fs::File::open(archive)?)?;
    for index in 0..zip.len() {
        let name = zip.by_index(index)?.name().to_string();
        let parts: Vec<&str> = name.split('/').collect();
        if let Some(position) = parts.iter().position(|part| part.ends_with(".xcframework")) {
            return Ok(parts[..=position].join("/"));
        }
    }
    bail!("Apple archive has no XCFramework")
}

fn artifact_id_for(base: &str, linkage: AppleLinkage) -> String {
    match linkage {
        // If the caller already named the base with `-shared`, only the shared
        // artifact keeps that name; strip it for the static variant so the two
        // uploads cannot collide.
        AppleLinkage::Static => base.strip_suffix("-shared").unwrap_or(base).to_string(),
        AppleLinkage::Shared if base.ends_with("-shared") => base.to_string(),
        AppleLinkage::Shared => format!("{base}-shared"),
        AppleLinkage::All => base.to_string(),
    }
}

struct AppleArtifact {
    linkage: AppleLinkage,
    artifact_id: String,
    xcframework: String,
    archive: PathBuf,
    checksum: String,
    upload: PreparedUpload,
}

fn prepare_artifacts(
    command: &PublishCommand,
    configuration: &toml::Value,
    source: &Path,
    temporary: &Path,
    release: &AppleRelease,
    spm: bool,
) -> Result<Vec<AppleArtifact>> {
    let nested = if source.is_dir() {
        None
    } else {
        nested_linkages(source, command.platform.as_deref())?
    };

    let linkages = match &nested {
        Some((_, available)) => selected_linkages(available, command.linkage)?,
        None => vec![single_linkage(source, command.linkage)?],
    };

    let mut artifacts = Vec::new();
    for linkage in linkages {
        let artifact_id = artifact_id_for(&release.artifact, linkage);
        let archive = temporary.join(format!("{artifact_id}-{}.zip", release.version));
        let xcframework = match &nested {
            Some((platform, _)) => Some(repackage_linkage(
                source,
                platform,
                linkage_dir(linkage),
                &archive,
            )?),
            None if source.is_dir() => {
                zip_directory(source, &archive)?;
                None
            }
            None => {
                fs::copy(source, &archive).context("Failed to read Apple ZIP")?;
                None
            }
        };
        validate_apple_archive(&archive, spm)?;
        let xcframework = match xcframework {
            Some(path) => path,
            None => xcframework_path(&archive)?,
        };
        let bytes = fs::read(&archive)?;
        let checksum = format!("{:x}", Sha256::digest(&bytes));
        let upload = PreparedUpload::prepare_with_artifact(
            command,
            configuration,
            &artifact_id,
            &release.version,
            &release.release_tag,
            "apple",
        )?;
        artifacts.push(AppleArtifact {
            linkage,
            artifact_id,
            xcframework,
            archive,
            checksum,
            upload,
        });
    }
    Ok(artifacts)
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

fn update_binary_target(block: &str, url: &str, checksum: &str) -> Result<String> {
    let url_re = regex::Regex::new(r#"\burl\s*:\s*"[^"\n]*""#)?;
    let checksum_re = regex::Regex::new(r#"\bchecksum\s*:\s*"[^"\n]*""#)?;
    if !url_re.is_match(block) || !checksum_re.is_match(block) {
        bail!("binaryTarget must use literal url and checksum strings");
    }
    let updated = url_re.replace(
        block,
        regex::NoExpand(&format!("url: {}", serde_json::to_string(url)?)),
    );
    Ok(checksum_re
        .replace(
            &updated,
            regex::NoExpand(&format!("checksum: \"{checksum}\"")),
        )
        .to_string())
}

/// Point external product dependencies of `*shared*` targets at their
/// `-shared` variants. Binary targets themselves cannot declare dependencies,
/// so this only affects wrapper targets with a `shared` in the name.
fn swift_call_ranges(text: &str) -> Result<Vec<(usize, usize)>> {
    let start_re =
        regex::Regex::new(r"\.(?:binaryTarget|target|executableTarget|testTarget)\s*\(")?;
    let mut ranges = Vec::new();
    for start in start_re.find_iter(text) {
        let open = text[..start.end()].rfind('(').unwrap();
        if let Some(close) = matching_paren(text, open) {
            ranges.push((start.start(), close + 1));
        }
    }
    Ok(ranges)
}

fn matching_paren(text: &str, open: usize) -> Option<usize> {
    let mut depth = 0usize;
    let mut in_string = false;
    let mut escape = false;
    for (offset, character) in text[open..].char_indices() {
        let index = open + offset;
        if in_string {
            if escape {
                escape = false;
            } else if character == '\\' {
                escape = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }
        match character {
            '"' => in_string = true,
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(index);
                }
            }
            _ => {}
        }
    }
    None
}

fn rewrite_shared_spm_dependencies(text: &str, policy: &SharedSuffixPolicy) -> Result<String> {
    let name_re = regex::Regex::new(r#"\bname\s*:\s*"([^"]+)""#)?;
    let product_re = regex::Regex::new(r#"(\.product\s*\(\s*name\s*:\s*")([^"]+)(")"#)?;

    let mut replacements = Vec::new();
    for (start, end) in swift_call_ranges(text)? {
        let block = &text[start..end];
        let name = name_re
            .captures(block)
            .and_then(|captures| captures.get(1))
            .map(|name| name.as_str().to_lowercase())
            .unwrap_or_default();
        if !name.contains("shared") {
            continue;
        }
        let rewritten = product_re.replace_all(block, |captures: &regex::Captures| {
            let dependency = &captures[2];
            let shared = if !policy.should_suffix(dependency) {
                dependency.to_string()
            } else {
                shared_name(dependency)
            };
            format!("{}{}{}", &captures[1], shared, &captures[3])
        });
        if rewritten != block {
            replacements.push((start, end, rewritten.to_string()));
        }
    }

    let mut output = text.to_string();
    for (start, end, replacement) in replacements.into_iter().rev() {
        output.replace_range(start..end, &replacement);
    }
    Ok(output)
}

fn binary_target_count(text: &str) -> Result<usize> {
    Ok(regex::Regex::new(r"(?s)\.binaryTarget\s*\([^)]*\)")?
        .find_iter(text)
        .count())
}

fn swift_manifest_matching(text: &str, artifact: &AppleArtifact) -> Result<String> {
    let block_re = regex::Regex::new(r"(?s)\.binaryTarget\s*\([^)]*\)")?;
    let name_re = regex::Regex::new(r#"\bname\s*:\s*"([^"]+)""#)?;
    let linkage = linkage_dir(artifact.linkage);
    for block in block_re.find_iter(text) {
        let name = name_re
            .captures(block.as_str())
            .and_then(|captures| captures.get(1))
            .map(|name| name.as_str().to_lowercase())
            .unwrap_or_default();
        // Only the shared target is marked; the static one may keep the
        // package's original target name.
        let matches = if linkage == "shared" {
            name.contains("shared")
        } else {
            !name.contains("shared")
        };
        if matches {
            let updated =
                update_binary_target(block.as_str(), &artifact.upload.url, &artifact.checksum)?;
            let mut output = text.to_string();
            output.replace_range(block.start()..block.end(), &updated);
            return Ok(output);
        }
    }
    bail!("Package.swift has no binaryTarget for --linkage {linkage}")
}

fn swift_manifest_pair(text: &str, artifacts: &[AppleArtifact]) -> Result<String> {
    let block_re = regex::Regex::new(r"(?s)\.binaryTarget\s*\([^)]*\)")?;
    let name_re = regex::Regex::new(r#"\bname\s*:\s*"([^"]+)""#)?;
    let blocks: Vec<_> = block_re.find_iter(text).collect();
    if blocks.len() != artifacts.len() {
        bail!(
            "Package.swift must contain exactly {} remote binaryTarget(s) named static/shared for --linkage all",
            artifacts.len()
        );
    }

    let mut replacements = Vec::new();
    let mut seen = BTreeSet::new();
    for block in &blocks {
        let name = name_re
            .captures(block.as_str())
            .and_then(|captures| captures.get(1))
            .map(|name| name.as_str().to_lowercase())
            .unwrap_or_default();
        // The static target can keep the original name (`Demo`); only the
        // dynamic target needs a `shared` marker (`DemoShared`).
        let linkage = if name.contains("shared") {
            AppleLinkage::Shared
        } else {
            AppleLinkage::Static
        };
        if !seen.insert(linkage) {
            bail!(
                "Package.swift needs exactly one static and one shared binaryTarget for --linkage all"
            );
        }
        let artifact = artifacts
            .iter()
            .find(|artifact| artifact.linkage == linkage)
            .with_context(|| format!("Package.swift has no {linkage} artifact to update"))?;
        let updated =
            update_binary_target(block.as_str(), &artifact.upload.url, &artifact.checksum)?;
        replacements.push((block.start(), block.end(), updated));
    }

    let mut output = text.to_string();
    for (start, end, replacement) in replacements.into_iter().rev() {
        output.replace_range(start..end, &replacement);
    }
    Ok(output)
}

pub(super) fn publish(command: &PublishCommand, verbose: bool) -> Result<()> {
    let project = std::env::current_dir()?;
    let pods = matches!(command.manager, AppleManager::Cocoapods | AppleManager::All);
    let spm = matches!(command.manager, AppleManager::Spm | AppleManager::All);
    validate_apple_registry(command)?;
    let configuration: toml::Value =
        toml::from_str(&fs::read_to_string(project.join("CCGO.toml"))?)?;
    let suffix_policy = SharedSuffixPolicy::from_config(&configuration);
    let pod_names = ApplePodNames::from_cli_and_config(command, &configuration);
    let release = apple_release(command, &configuration)?;
    let version = release.version.as_str();
    let temporary = tempfile::tempdir()?;
    let source = apple_artifact_source(command, &project, release.name, version)?;
    let artifacts = prepare_artifacts(
        command,
        &configuration,
        &source,
        temporary.path(),
        &release,
        spm,
    )?;
    let (base_spec, swift) = load_apple_manifests(&project, version, pods, spm)?;
    let target_count = swift
        .as_ref()
        .map(|text| binary_target_count(text))
        .transpose()?;
    let has_shared = artifacts
        .iter()
        .any(|artifact| artifact.linkage == AppleLinkage::Shared);
    let swift = swift
        .map(|text| {
            let updated = if target_count.unwrap_or(0) <= 1 {
                let chosen = artifacts
                    .iter()
                    .find(|artifact| artifact.linkage == AppleLinkage::Static)
                    .unwrap_or(&artifacts[0]);
                swift_manifest(&text, &chosen.upload.url, &chosen.checksum)?
            } else if artifacts.len() == 1 {
                swift_manifest_matching(&text, &artifacts[0])?
            } else {
                swift_manifest_pair(&text, &artifacts)?
            };
            if has_shared {
                rewrite_shared_spm_dependencies(&updated, &suffix_policy)
            } else {
                Ok(updated)
            }
        })
        .transpose()?;
    let output = manifest_dir(&project, version)?;
    let mut podspecs = Vec::new();
    if let Some(base_spec) = &base_spec {
        for artifact in &artifacts {
            let spec = podspec_for(base_spec, artifact, &suffix_policy, &pod_names)?;
            let path = write_podspec(&project, version, &artifact.artifact_id, &spec)?;
            podspecs.push((path, artifact.upload.url.clone()));
        }
    }
    if let Some(swift) = swift.as_deref() {
        fs::write(output.join("Package.swift"), swift)?;
    }
    for artifact in &artifacts {
        artifact.upload.upload(&artifact.archive)?;
        println!(
            "Apple binary URL: {}\nSHA-256: {}",
            artifact.upload.url, artifact.checksum
        );
    }
    println!("Generated manifests: {}", output.display());
    for (path, url) in &podspecs {
        publish_podspec(command, &project, path, url, verbose)?;
    }
    if spm {
        println!("Commit the generated Package.swift to your SPM wrapper repository, then push its version tag.");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn publish_command(args: &[&str]) -> PublishCommand {
        let cli = <crate::cli::Cli as clap::Parser>::try_parse_from(args).unwrap();
        match cli.command {
            crate::cli::Commands::Publish(command) => *command,
            _ => unreachable!("expected publish command"),
        }
    }

    #[test]
    fn pod_names_come_from_cli_then_config() {
        let config: toml::Value = toml::from_str(
            "[publish.apple]\npod_name = \"ConfigName\"\nshared_pod_name = \"ConfigShared\"\n",
        )
        .unwrap();

        let from_config = ApplePodNames::from_cli_and_config(
            &publish_command(&[
                "ccgo",
                "publish",
                "apple",
                "--artifact-provider",
                "github",
                "--github-repo",
                "company/sdk",
            ]),
            &config,
        );
        assert_eq!(from_config.static_name.as_deref(), Some("ConfigName"));
        assert_eq!(from_config.shared_name.as_deref(), Some("ConfigShared"));

        let from_cli = ApplePodNames::from_cli_and_config(
            &publish_command(&[
                "ccgo",
                "publish",
                "apple",
                "--artifact-provider",
                "github",
                "--github-repo",
                "company/sdk",
                "--pod-name",
                "CliName",
                "--shared-pod-name",
                "CliShared",
            ]),
            &config,
        );
        assert_eq!(from_cli.static_name.as_deref(), Some("CliName"));
        assert_eq!(from_cli.shared_name.as_deref(), Some("CliShared"));
    }

    #[test]
    fn shared_suffix_policy_defaults_to_declared_dependencies() {
        let config: toml::Value = toml::from_str(
            "[publish.apple]\nno_shared_suffix = [\"Pinned\"]\n\n[[dependencies]]\nname = \"Dep\"\n\n[[dependencies]]\nname = \"Pinned\"\n",
        )
        .unwrap();
        let policy = SharedSuffixPolicy::from_config(&config);
        assert!(policy.should_suffix("Dep"));
        assert!(policy.should_suffix("Dep/Sub"));
        assert!(!policy.should_suffix("Pinned"));
        assert!(!policy.should_suffix("Alamofire"));
    }

    #[test]
    fn shared_suffix_policy_accepts_an_explicit_allowlist() {
        let config: toml::Value =
            toml::from_str("[publish.apple]\nshared_suffix = [\"External\"]\n").unwrap();
        let policy = SharedSuffixPolicy::from_config(&config);
        assert!(policy.should_suffix("External"));
        assert!(!policy.should_suffix("Other"));
    }

    #[test]
    fn swift_shared_target_rewrites_external_product_dependencies() {
        let text = r#"
.target(name: "Demo", dependencies: [.product(name: "Dep", package: "dep")]),
.target(name: "DemoShared", dependencies: [.binaryTarget(name: "DemoSharedBinary"), .product(name: "Dep", package: "dep")])
"#;
        let policy = SharedSuffixPolicy {
            dependencies: ["dep".to_string()].into_iter().collect(),
            exclude: BTreeSet::new(),
        };
        let updated = rewrite_shared_spm_dependencies(text, &policy).unwrap();
        assert!(updated.contains(".product(name: \"DepShared\", package: \"dep\")"));
        assert_eq!(updated.matches(".product(name: \"Dep\",").count(), 1);
    }

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
