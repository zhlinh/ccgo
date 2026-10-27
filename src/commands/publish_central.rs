//! Publish Android AARs to Maven Central through the Central Portal.
//!
//! The Portal takes one ZIP containing the Maven repository layout, with
//! checksums and PGP signatures for every file. This module builds that bundle
//! and follows the deployment without invoking Gradle (the design mirrors
//! `kargo-maven`'s `central` module). local/private repositories use the
//! simpler direct PUT path in `publish_android_maven`.

use super::{
    android_maven::{self, Coordinates},
    kmp_maven,
    kmp_sdk::{Artifact, PreparedSdk},
    maven, PublishCommand,
};
use anyhow::{bail, Context, Result};
use base64::Engine as _;
use pgp::{
    armor::{self, BlockType},
    composed::{Deserializable, SignedSecretKey},
    crypto::hash::HashAlgorithm,
    packet::{SignatureConfig, SignatureType, Subpacket, SubpacketData},
    types::{PublicKeyTrait, SecretKeyTrait},
};
use reqwest::blocking::{multipart, Client};
use serde::Deserialize;
use sha1::Sha1;
use sha2::{Digest, Sha256, Sha512};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
    time::{Duration, Instant},
};
use zip::{write::SimpleFileOptions, ZipWriter};

const DEFAULT_PORTAL: &str = "https://central.sonatype.com";
const WAIT_LIMIT: Duration = Duration::from_secs(30 * 60);

struct Metadata {
    name: String,
    description: String,
    url: String,
    license_name: String,
    license_url: String,
    developer_name: String,
    developer_email: String,
}

/// Check the Central Portal prerequisites before a build starts.
pub(super) fn ensure_ready(config: &toml::Value, cwd: &Path) -> Result<()> {
    let mut missing: Vec<String> = Vec::new();
    if credentials(config).is_err() {
        missing.push(
            "Central Portal credentials\n      \
             export MAVEN_CENTRAL_USERNAME=<token-username>\n      \
             export MAVEN_CENTRAL_PASSWORD=<token-password>\n      \
             Generate a user token at https://central.sonatype.com/account"
                .to_string(),
        );
    }
    if let Err(error) = Signer::load(config) {
        missing.push(error.to_string());
    }
    if let Err(error) = Metadata::from_config(config, cwd) {
        missing.push(error.to_string());
    }
    if !missing.is_empty() {
        bail!(
            "Maven Central publishing is not configured yet:\n\n  • {}\n\n\
             Fix the items above and retry.",
            missing.join("\n\n  • ")
        );
    }
    Ok(())
}

pub(super) fn publish_android(command: &PublishCommand, aar: &Path, verbose: bool) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let config = maven::load_config(&cwd)?;
    ensure_ready(&config, &cwd)?;
    let coordinates = android_maven::coordinates(command, &config)?;
    let metadata = Metadata::from_config(&config, &cwd)?;
    let signer = Signer::load(&config)?;
    let (username, password) = credentials(&config)?;
    let base = portal_url();
    let token = base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
    let client = Client::builder()
        .timeout(Duration::from_secs(300))
        .build()?;

    println!(
        "Publishing {}:{}:{} to Maven Central via the Central Portal",
        coordinates.group, coordinates.artifact_id, coordinates.version
    );
    let bundle = build_bundle(&coordinates, &metadata, &signer, aar)?;
    let name = format!(
        "{}:{}:{}",
        coordinates.group, coordinates.artifact_id, coordinates.version
    );
    let deployment = upload(&client, &base, &token, &name, &bundle.bundle)?;
    println!("  Uploaded bundle; deployment ID: {deployment}");
    wait_for_deployment(&client, &base, &token, &deployment, verbose)?;
    println!("\n✅ Publish completed successfully!");
    Ok(())
}

pub(super) fn publish_kmp(
    _command: &PublishCommand,
    sdk: &PreparedSdk,
    verbose: bool,
) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    let config = maven::load_config(&cwd)?;
    ensure_ready(&config, &cwd)?;
    let coordinates = kmp_maven::coordinates(&config, sdk)?;
    let metadata = Metadata::from_config(&config, &cwd)?;
    let signer = Signer::load(&config)?;
    let (username, password) = credentials(&config)?;
    let base = portal_url();
    let token = base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
    let client = Client::builder()
        .timeout(Duration::from_secs(300))
        .build()?;

    println!(
        "Publishing KMP SDK {} {} to Maven Central via the Central Portal",
        sdk.project, sdk.version
    );
    let bundle = build_kmp_bundle(sdk, &coordinates, &metadata, &signer)?;
    let name = format!(
        "{}:{}:{}",
        coordinates.group, coordinates.artifact_base, sdk.version
    );
    let deployment = upload(&client, &base, &token, &name, &bundle.bundle)?;
    println!("  Uploaded bundle; deployment ID: {deployment}");
    wait_for_deployment(&client, &base, &token, &deployment, verbose)?;
    println!("\n✅ Publish completed successfully!");
    Ok(())
}

/// Publish a single ZIP artifact (Apple binary, Index package or Doc archive).
pub(super) fn publish_artifact(
    config: &toml::Value,
    group: &str,
    artifact: &str,
    version: &str,
    packaging: &str,
    archive: &Path,
    verbose: bool,
) -> Result<()> {
    let cwd = std::env::current_dir().context("Failed to get current directory")?;
    ensure_ready(config, &cwd)?;
    let metadata = Metadata::from_config(config, &cwd)?;
    let signer = Signer::load(config)?;
    let (username, password) = credentials(config)?;
    let base = portal_url();
    let token = base64::engine::general_purpose::STANDARD.encode(format!("{username}:{password}"));
    let client = Client::builder()
        .timeout(Duration::from_secs(300))
        .build()?;

    println!("Publishing {group}:{artifact}:{version} to Maven Central via the Central Portal");
    let bundle = build_archive_bundle(
        group, artifact, version, packaging, archive, &metadata, &signer,
    )?;
    let name = format!("{group}:{artifact}:{version}");
    let deployment = upload(&client, &base, &token, &name, &bundle.bundle)?;
    println!("  Uploaded bundle; deployment ID: {deployment}");
    wait_for_deployment(&client, &base, &token, &deployment, verbose)?;
    println!("\n✅ Publish completed successfully!");
    Ok(())
}

fn build_archive_bundle(
    group: &str,
    artifact: &str,
    version: &str,
    packaging: &str,
    archive: &Path,
    metadata: &Metadata,
    signer: &Signer,
) -> Result<Bundle> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("repository");
    let version_dir = root
        .join(group.replace('.', "/"))
        .join(artifact)
        .join(version);
    fs::create_dir_all(&version_dir)?;
    let base = format!("{artifact}-{version}");

    let files = [
        (
            format!("{base}.{packaging}"),
            fs::read(archive)
                .with_context(|| format!("Failed to read {}", archive.display()))?,
        ),
        (
            format!("{base}.pom"),
            central_pom(
                &metadata.name,
                &metadata.description,
                group,
                artifact,
                version,
                packaging,
                &[],
                metadata,
            )
            .into_bytes(),
        ),
        (
            format!("{base}-sources.jar"),
            simple_jar(&[
                (
                    "META-INF/MANIFEST.MF",
                    b"Manifest-Version: 1.0\n".as_slice(),
                ),
                (
                    "README.txt",
                    b"This artifact was published from a prebuilt archive; sources are not bundled.\n"
                        .as_slice(),
                ),
            ])?,
        ),
        (
            format!("{base}-javadoc.jar"),
            simple_jar(&[
                (
                    "META-INF/MANIFEST.MF",
                    b"Manifest-Version: 1.0\n".as_slice(),
                ),
                (
                    "index.html",
                    b"<!doctype html><title>Javadoc</title><p>Javadoc is not bundled in this prebuilt artifact.</p>"
                        .as_slice(),
                ),
            ])?,
        ),
    ];

    for (name, bytes) in files {
        let path = version_dir.join(name);
        fs::write(&path, &bytes)?;
        write_checksums(&path, &bytes)?;
        fs::write(append_extension(&path, ".asc"), signer.sign(&bytes)?)?;
    }

    let bundle = temporary.path().join("bundle.zip");
    write_bundle(&root, &bundle)?;
    Ok(Bundle {
        bundle,
        _temporary: temporary,
    })
}

fn build_kmp_bundle(
    sdk: &PreparedSdk,
    coordinates: &kmp_maven::Coordinates,
    metadata: &Metadata,
    signer: &Signer,
) -> Result<Bundle> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("repository");
    let dependencies: Vec<_> = coordinates
        .dependencies
        .iter()
        .map(|dependency| {
            (
                dependency.group.clone(),
                dependency.artifact.clone(),
                dependency.version.clone(),
            )
        })
        .collect();
    let mut targets: BTreeMap<&str, Vec<&Artifact>> = BTreeMap::new();
    for artifact in &sdk.artifacts {
        targets.entry(&artifact.target).or_default().push(artifact);
    }

    for (target, artifacts) in targets {
        stage_kmp_target(
            &root,
            sdk,
            coordinates,
            metadata,
            signer,
            target,
            &artifacts,
            &dependencies,
        )?;
    }

    let bundle = temporary.path().join("bundle.zip");
    write_bundle(&root, &bundle)?;
    Ok(Bundle {
        bundle,
        _temporary: temporary,
    })
}

#[allow(clippy::too_many_arguments)]
fn stage_kmp_target(
    root: &Path,
    sdk: &PreparedSdk,
    coordinates: &kmp_maven::Coordinates,
    metadata: &Metadata,
    signer: &Signer,
    target: &str,
    artifacts: &[&Artifact],
    dependencies: &[(String, String, String)],
) -> Result<()> {
    let artifact_id = format!("{}-{}", coordinates.artifact_base, target);
    super::artifact::coordinate(&artifact_id)?;
    let main = artifacts
        .iter()
        .find(|artifact| artifact.classifier.is_none())
        .with_context(|| format!("KMP SDK target {target} has no main artifact"))?;
    let packaging = kmp_maven::extension(&main.path)?;
    let version_dir = root
        .join(coordinates.group.replace('.', "/"))
        .join(&artifact_id)
        .join(&sdk.version);
    fs::create_dir_all(&version_dir)?;
    let base = format!("{artifact_id}-{}", sdk.version);

    let mut files = Vec::new();
    for artifact in artifacts {
        let extension = kmp_maven::extension(&artifact.path)?;
        let layout = maven::layout_path(
            &coordinates.group,
            &artifact_id,
            &sdk.version,
            artifact.classifier.as_deref(),
            extension,
        );
        let name = Path::new(&layout)
            .file_name()
            .context("KMP artifact has no filename")?
            .to_string_lossy()
            .into_owned();
        files.push((
            name,
            fs::read(&artifact.path)
                .with_context(|| format!("Failed to read {}", artifact.path.display()))?,
        ));
    }
    files.push((
        format!("{base}.pom"),
        central_pom(
            &format!("{} KMP - {target}", sdk.project),
            &format!("Prebuilt {} KMP SDK ({target})", sdk.project),
            &coordinates.group,
            &artifact_id,
            &sdk.version,
            packaging,
            dependencies,
            metadata,
        )
        .into_bytes(),
    ));
    files.push((
        format!("{base}-sources.jar"),
        simple_jar(&[
            (
                "META-INF/MANIFEST.MF",
                b"Manifest-Version: 1.0\n".as_slice(),
            ),
            (
                "README.txt",
                b"This artifact was published from a prebuilt KMP SDK; sources are not bundled.\n"
                    .as_slice(),
            ),
        ])?,
    ));
    files.push((
        format!("{base}-javadoc.jar"),
        simple_jar(&[
            (
                "META-INF/MANIFEST.MF",
                b"Manifest-Version: 1.0\n".as_slice(),
            ),
            (
                "index.html",
                b"<!doctype html><title>Javadoc</title><p>Javadoc is not bundled in this prebuilt artifact.</p>"
                    .as_slice(),
            ),
        ])?,
    ));

    for (name, bytes) in files {
        let path = version_dir.join(name);
        fs::write(&path, &bytes)?;
        write_checksums(&path, &bytes)?;
        fs::write(append_extension(&path, ".asc"), signer.sign(&bytes)?)?;
    }
    Ok(())
}

fn build_bundle(
    coordinates: &Coordinates,
    metadata: &Metadata,
    signer: &Signer,
    aar: &Path,
) -> Result<Bundle> {
    let temporary = tempfile::tempdir()?;
    let root = temporary.path().join("repository");
    let version_dir = root
        .join(coordinates.group.replace('.', "/"))
        .join(&coordinates.artifact_id)
        .join(&coordinates.version);
    fs::create_dir_all(&version_dir)?;

    let base = format!("{}-{}", coordinates.artifact_id, coordinates.version);
    let files = [
        (
            format!("{base}.aar"),
            fs::read(aar)
                .with_context(|| format!("Failed to read {}", aar.display()))?,
        ),
        (
            format!("{base}.pom"),
            central_pom(
                &metadata.name,
                &metadata.description,
                &coordinates.group,
                &coordinates.artifact_id,
                &coordinates.version,
                "aar",
                &coordinates.dependencies,
                metadata,
            )
            .into_bytes(),
        ),
        (
            format!("{base}-sources.jar"),
            simple_jar(&[
                (
                    "META-INF/MANIFEST.MF",
                    b"Manifest-Version: 1.0\n".as_slice(),
                ),
                (
                    "README.txt",
                    b"This artifact was published from a prebuilt AAR; sources are not bundled.\n"
                        .as_slice(),
                ),
            ])?,
        ),
        (
            format!("{base}-javadoc.jar"),
            simple_jar(&[
                (
                    "META-INF/MANIFEST.MF",
                    b"Manifest-Version: 1.0\n".as_slice(),
                ),
                (
                    "index.html",
                    b"<!doctype html><title>Javadoc</title><p>Javadoc is not bundled in this prebuilt artifact.</p>"
                        .as_slice(),
                ),
            ])?,
        ),
    ];

    for (name, bytes) in files {
        let path = version_dir.join(&name);
        fs::write(&path, &bytes)?;
        write_checksums(&path, &bytes)?;
        let signature = signer.sign(&bytes)?;
        fs::write(append_extension(&path, ".asc"), signature)?;
    }

    let bundle = temporary.path().join("bundle.zip");
    write_bundle(&root, &bundle)?;
    Ok(Bundle {
        bundle,
        _temporary: temporary,
    })
}

struct Bundle {
    bundle: PathBuf,
    _temporary: tempfile::TempDir,
}

#[allow(clippy::too_many_arguments)]
fn central_pom(
    name: &str,
    description: &str,
    group: &str,
    artifact: &str,
    version: &str,
    packaging: &str,
    dependencies: &[(String, String, String)],
    metadata: &Metadata,
) -> String {
    let mut pom = String::from(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <project xmlns=\"http://maven.apache.org/POM/4.0.0\" \
         xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" \
         xsi:schemaLocation=\"http://maven.apache.org/POM/4.0.0 \
         https://maven.apache.org/xsd/maven-4.0.0.xsd\">\n\
         \x20 <modelVersion>4.0.0</modelVersion>\n",
    );
    for (tag, value) in [
        ("groupId", group),
        ("artifactId", artifact),
        ("version", version),
        ("packaging", packaging),
        ("name", name),
        ("description", description),
        ("url", metadata.url.as_str()),
    ] {
        pom.push_str(&format!("  <{tag}>{}</{tag}>\n", maven::xml_escape(value)));
    }
    pom.push_str("  <licenses>\n    <license>\n");
    pom.push_str(&format!(
        "      <name>{}</name>\n",
        maven::xml_escape(&metadata.license_name)
    ));
    pom.push_str(&format!(
        "      <url>{}</url>\n",
        maven::xml_escape(&metadata.license_url)
    ));
    pom.push_str("    </license>\n  </licenses>\n");
    pom.push_str("  <developers>\n    <developer>\n");
    pom.push_str(&format!(
        "      <id>{}</id>\n",
        maven::xml_escape(&metadata.developer_name)
    ));
    pom.push_str(&format!(
        "      <name>{}</name>\n",
        maven::xml_escape(&metadata.developer_name)
    ));
    pom.push_str(&format!(
        "      <email>{}</email>\n",
        maven::xml_escape(&metadata.developer_email)
    ));
    pom.push_str("    </developer>\n  </developers>\n");
    pom.push_str("  <scm>\n");
    pom.push_str(&format!(
        "    <url>{}</url>\n",
        maven::xml_escape(&metadata.url)
    ));
    pom.push_str(&format!(
        "    <connection>{}</connection>\n",
        maven::xml_escape(&format!("scm:git:{}", metadata.url))
    ));
    pom.push_str(&format!(
        "    <developerConnection>{}</developerConnection>\n",
        maven::xml_escape(&format!("scm:git:{}", metadata.url))
    ));
    pom.push_str("  </scm>\n");
    if !dependencies.is_empty() {
        pom.push_str("  <dependencies>\n");
        for (group, artifact, version) in dependencies {
            pom.push_str("    <dependency>\n");
            for (tag, value) in [
                ("groupId", group.as_str()),
                ("artifactId", artifact.as_str()),
                ("version", version.as_str()),
            ] {
                pom.push_str(&format!(
                    "      <{tag}>{}</{tag}>\n",
                    maven::xml_escape(value)
                ));
            }
            pom.push_str("    </dependency>\n");
        }
        pom.push_str("  </dependencies>\n");
    }
    pom.push_str("</project>\n");
    pom
}

impl Metadata {
    fn from_config(config: &toml::Value, cwd: &Path) -> Result<Self> {
        let (project, _) = maven::project_identity(config)?;
        let description = maven::string_at(config, &["project", "description"])
            .or_else(|| maven::string_at(config, &["package", "description"]))
            .map(str::to_owned)
            .unwrap_or_else(|| format!("The {project} SDK"));
        let url = maven::string_at(config, &["project", "repository"])
            .or_else(|| maven::string_at(config, &["package", "repository"]))
            .map(str::to_owned)
            .or_else(|| git_config(cwd, "remote.origin.url"))
            .map(|url| normalize_repo_url(&url))
            .context(
                "Maven Central needs a project URL/SCM: set [project].repository or configure \
                 a git origin URL",
            )?;
        let license = maven::string_at(config, &["project", "license"])
            .or_else(|| maven::string_at(config, &["package", "license"]))
            .unwrap_or("MIT");
        let (license_name, license_url) = license_url(license);
        let (developer_name, developer_email) = developer(config, cwd, project);
        Ok(Self {
            name: project.to_owned(),
            description,
            url,
            license_name,
            license_url,
            developer_name,
            developer_email,
        })
    }
}

fn normalize_repo_url(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("git@") {
        if let Some((host, path)) = rest.split_once(':') {
            return format!("https://{host}/{}", path.trim_start_matches('/'));
        }
    }
    if let Some(rest) = url.strip_prefix("ssh://") {
        let rest = rest.strip_prefix("git@").unwrap_or(rest);
        if let Some((host, path)) = rest.split_once('/') {
            return format!("https://{host}/{path}");
        }
    }
    url.to_owned()
}

fn license_url(license: &str) -> (String, String) {
    let license = license.trim();
    let url = match license.to_ascii_lowercase().as_str() {
        "mit" => "https://opensource.org/licenses/MIT",
        "apache-2.0" | "apache 2.0" => "https://www.apache.org/licenses/LICENSE-2.0",
        "bsd-3-clause" => "https://opensource.org/licenses/BSD-3-Clause",
        "bsd-2-clause" => "https://opensource.org/licenses/BSD-2-Clause",
        "isc" => "https://opensource.org/licenses/ISC",
        _ => "https://opensource.org/licenses/",
    };
    (license.to_owned(), url.to_owned())
}

fn developer(config: &toml::Value, cwd: &Path, project: &str) -> (String, String) {
    if let Some(toml::Value::Array(authors)) = maven::value_at(config, &["project", "authors"])
        .or_else(|| maven::value_at(config, &["package", "authors"]))
    {
        if let Some(author) = authors.iter().find_map(|author| author.as_str()) {
            let (name, email) = parse_author(author);
            return (name, email);
        }
    }
    let name = git_config(cwd, "user.name").unwrap_or_else(|| project.to_owned());
    let email = git_config(cwd, "user.email")
        .filter(|email| !email.is_empty())
        .unwrap_or_else(|| "unknown@example.com".to_owned());
    (name, email)
}

fn parse_author(author: &str) -> (String, String) {
    if let (Some(open), Some(close)) = (author.find('<'), author.find('>')) {
        if open < close {
            let name = author[..open].trim();
            let email = author[open + 1..close].trim();
            return (
                if name.is_empty() {
                    "Unknown".to_owned()
                } else {
                    name.to_owned()
                },
                if email.is_empty() {
                    "unknown@example.com".to_owned()
                } else {
                    email.to_owned()
                },
            );
        }
    }
    (author.trim().to_owned(), "unknown@example.com".to_owned())
}

fn git_config(cwd: &Path, key: &str) -> Option<String> {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(["config", "--get", key])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let value = String::from_utf8(output.stdout).ok()?;
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_owned())
}

enum Signer {
    InMemory {
        key: Box<SignedSecretKey>,
        password: String,
    },
    Gpg(PathBuf),
}

impl Signer {
    fn load(config: &toml::Value) -> Result<Self> {
        let key = std::env::var("SIGNING_IN_MEMORY_KEY")
            .ok()
            .filter(|value| !value.trim().is_empty())
            .or_else(|| maven::string_at(config, &["publish", "signing_key"]).map(str::to_owned))
            .or_else(|| {
                maven::string_at(config, &["publish", "maven", "signing_key"]).map(str::to_owned)
            });
        if let Some(key) = key {
            let key = key
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .replace("\\n", "\n");
            let (key, _headers) = SignedSecretKey::from_armor_single(key.as_bytes())
                .context("Invalid SIGNING_IN_MEMORY_KEY (expected an ASCII-armored secret key)")?;
            let password = std::env::var("SIGNING_IN_MEMORY_KEY_PASSWORD")
                .ok()
                .or_else(|| {
                    maven::string_at(config, &["publish", "signing_key_password"])
                        .map(str::to_owned)
                })
                .or_else(|| {
                    maven::string_at(config, &["publish", "maven", "signing_key_password"])
                        .map(str::to_owned)
                })
                .unwrap_or_default();
            return Ok(Self::InMemory {
                key: Box::new(key),
                password,
            });
        }
        if let Ok(gpg) = which::which("gpg") {
            let has_secret_key = Command::new(&gpg)
                .args(["--batch", "--list-secret-keys"])
                .output()
                .map(|output| {
                    output.status.success()
                        && String::from_utf8_lossy(&output.stdout).contains("sec")
                })
                .unwrap_or(false);
            if has_secret_key {
                return Ok(Self::Gpg(gpg));
            }
        }
        bail!(
            "Maven Central needs PGP signatures: set SIGNING_IN_MEMORY_KEY and \
             SIGNING_IN_MEMORY_KEY_PASSWORD, or install gpg with a secret key"
        )
    }

    fn sign(&self, bytes: &[u8]) -> Result<Vec<u8>> {
        match self {
            Self::InMemory { key, password } => {
                let key = key.as_ref();
                // Prefer a signing subkey (the common GPG layout); fall back to
                // the primary key for keys that sign with the primary directly.
                let subkey = key
                    .secret_subkeys
                    .iter()
                    .find(|subkey| subkey.public_key().is_signing_key());
                let signature = if let Some(subkey) = subkey {
                    let mut config = SignatureConfig::v4(
                        SignatureType::Binary,
                        subkey.algorithm(),
                        HashAlgorithm::SHA2_256,
                    );
                    config.hashed_subpackets = vec![
                        Subpacket::regular(
                            SubpacketData::SignatureCreationTime(chrono::Utc::now()),
                        ),
                        Subpacket::regular(SubpacketData::Issuer(subkey.key_id())),
                    ];
                    config
                        .sign(subkey, || password.clone(), bytes)
                        .context("Failed to create a PGP signature")?
                } else {
                    let mut config = SignatureConfig::v4(
                        SignatureType::Binary,
                        key.algorithm(),
                        HashAlgorithm::SHA2_256,
                    );
                    config.hashed_subpackets = vec![
                        Subpacket::regular(
                            SubpacketData::SignatureCreationTime(chrono::Utc::now()),
                        ),
                        Subpacket::regular(SubpacketData::Issuer(key.key_id())),
                    ];
                    config
                        .sign(key, || password.clone(), bytes)
                        .context("Failed to create a PGP signature")?
                };
                let mut armored = Vec::new();
                armor::write(&signature, BlockType::Signature, &mut armored, None, true)?;
                Ok(armored)
            }
            Self::Gpg(gpg) => {
                let temporary = tempfile::Builder::new().suffix(".asc").tempfile()?;
                let input = tempfile::NamedTempFile::new()?;
                fs::write(input.path(), bytes)?;
                let status = Command::new(gpg)
                    .args(["--batch", "--yes", "--armor", "--detach-sign", "--output"])
                    .arg(temporary.path())
                    .arg(input.path())
                    .status()
                    .context("Failed to execute gpg")?;
                if !status.success() {
                    bail!("gpg failed to sign the Maven Central bundle");
                }
                Ok(fs::read(temporary.path())?)
            }
        }
    }
}

fn credentials(config: &toml::Value) -> Result<(String, String)> {
    let env = |names: &[&str]| {
        names
            .iter()
            .find_map(|name| std::env::var(name).ok().filter(|value| !value.is_empty()))
    };
    let username = env(&["MAVEN_CENTRAL_USERNAME", "SONATYPE_USERNAME"]).or_else(|| {
        maven::string_at(config, &["publish", "maven", "central_username"]).map(str::to_owned)
    });
    let password = env(&["MAVEN_CENTRAL_PASSWORD", "SONATYPE_PASSWORD"]).or_else(|| {
        maven::string_at(config, &["publish", "maven", "central_password"]).map(str::to_owned)
    });
    match (username, password) {
        (Some(username), Some(password)) => Ok((username, password)),
        _ => bail!(
            "Maven Central publishing needs a Central Portal user token; set \
             MAVEN_CENTRAL_USERNAME and MAVEN_CENTRAL_PASSWORD (or \
             [publish.maven].central_username/central_password in CCGO.toml)"
        ),
    }
}

fn portal_url() -> String {
    std::env::var("CCGO_CENTRAL_URL")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| DEFAULT_PORTAL.to_owned())
        .trim_end_matches('/')
        .to_owned()
}

fn upload(client: &Client, base: &str, token: &str, name: &str, bundle: &Path) -> Result<String> {
    let bytes = fs::read(bundle).with_context(|| format!("Failed to read {}", bundle.display()))?;
    let form = multipart::Form::new().part(
        "bundle",
        multipart::Part::bytes(bytes)
            .file_name("bundle.zip")
            .mime_str("application/octet-stream")?,
    );
    let mut url = reqwest::Url::parse(&format!("{base}/api/v1/publisher/upload"))
        .context("Invalid Central Portal URL")?;
    url.query_pairs_mut()
        .append_pair("name", name)
        .append_pair("publishingType", "AUTOMATIC");
    let response = client
        .post(url)
        .bearer_auth(token)
        .multipart(form)
        .send()
        .context("Failed to connect to the Central Portal")?;
    let status = response.status();
    let text = response.text().unwrap_or_default();
    if status.is_success() {
        let id = text.trim().trim_matches('"').to_owned();
        if id.is_empty() {
            bail!("The Central Portal accepted the bundle but returned no deployment id");
        }
        Ok(id)
    } else if matches!(status.as_u16(), 401 | 403) {
        bail!(
            "The Central Portal refused the credentials (HTTP {status}); use a user token \
             from {DEFAULT_PORTAL}/account"
        );
    } else {
        bail!(
            "The Central Portal rejected the bundle (HTTP {status}): {}",
            text.trim()
        );
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct DeploymentStatus {
    #[serde(default)]
    deployment_state: String,
    #[serde(default)]
    errors: Option<serde_json::Value>,
}

fn wait_for_deployment(
    client: &Client,
    base: &str,
    token: &str,
    id: &str,
    verbose: bool,
) -> Result<()> {
    let started = Instant::now();
    let mut last = String::new();
    loop {
        let mut url = reqwest::Url::parse(&format!("{base}/api/v1/publisher/status"))
            .context("Invalid Central Portal URL")?;
        url.query_pairs_mut().append_pair("id", id);
        let response = client
            .post(url)
            .bearer_auth(token)
            .send()
            .context("Failed to read the Central Portal deployment status")?;
        let status = response.status();
        let text = response.text().unwrap_or_default();
        if !status.is_success() {
            bail!(
                "Could not read deployment {id} (HTTP {status}): {}",
                text.trim()
            );
        }
        let state: DeploymentStatus =
            serde_json::from_str(&text).context("Unexpected Central Portal status response")?;
        if verbose && state.deployment_state != last {
            println!("  Deployment {id}: {}", state.deployment_state);
            last = state.deployment_state.clone();
        }
        match state.deployment_state.as_str() {
            "PUBLISHED" | "PUBLISHING" => return Ok(()),
            "FAILED" => {
                let errors = state
                    .errors
                    .map(|errors| format!(": {errors}"))
                    .unwrap_or_default();
                bail!("Maven Central rejected deployment {id}{errors}");
            }
            _ => {}
        }
        if started.elapsed() > WAIT_LIMIT {
            bail!(
                "Deployment {id} is still {} after {} minutes; follow it at \
                 {DEFAULT_PORTAL}/publishing/deployments",
                state.deployment_state,
                WAIT_LIMIT.as_secs() / 60
            );
        }
        std::thread::sleep(Duration::from_secs(5));
    }
}

fn write_checksums(path: &Path, bytes: &[u8]) -> Result<()> {
    let checksums = [
        ("md5", format!("{:x}", md5::compute(bytes))),
        ("sha1", format!("{:x}", Sha1::digest(bytes))),
        ("sha256", format!("{:x}", Sha256::digest(bytes))),
        ("sha512", format!("{:x}", Sha512::digest(bytes))),
    ];
    for (extension, value) in checksums {
        fs::write(append_extension(path, &format!(".{extension}")), value)?;
    }
    Ok(())
}

fn append_extension(path: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}{suffix}", path.display()))
}

fn simple_jar(entries: &[(&str, &[u8])]) -> Result<Vec<u8>> {
    let mut writer = ZipWriter::new(std::io::Cursor::new(Vec::new()));
    for (name, bytes) in entries {
        writer.start_file(*name, SimpleFileOptions::default())?;
        writer.write_all(bytes)?;
    }
    Ok(writer.finish()?.into_inner())
}

fn write_bundle(root: &Path, destination: &Path) -> Result<()> {
    let mut files: Vec<_> = walkdir::WalkDir::new(root)
        .into_iter()
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.into_path())
        .collect();
    files.sort();
    let mut writer = ZipWriter::new(fs::File::create(destination)?);
    let options = SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .last_modified_time(zip::DateTime::default());
    for path in files {
        let relative = path.strip_prefix(root)?;
        writer.start_file(relative.to_string_lossy().replace('\\', "/"), options)?;
        let mut file = fs::File::open(&path)?;
        std::io::copy(&mut file, &mut writer)?;
    }
    writer.finish()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pgp::composed::{KeyType, SecretKeyParamsBuilder};

    fn signer() -> Signer {
        let params = SecretKeyParamsBuilder::default()
            .key_type(KeyType::Ed25519)
            .can_certify(true)
            .can_sign(true)
            .primary_user_id("Test <test@example.com>".into())
            .build()
            .unwrap();
        let secret = params.generate(rand::thread_rng()).unwrap();
        let signed = secret.sign(rand::thread_rng(), String::new).unwrap();
        let armored = signed.to_armored_string(None.into()).unwrap();
        let (key, _) = SignedSecretKey::from_armor_single(armored.as_bytes()).unwrap();
        Signer::InMemory {
            key: Box::new(key),
            password: String::new(),
        }
    }

    fn metadata() -> Metadata {
        Metadata {
            name: "demo".into(),
            description: "Demo SDK".into(),
            url: "https://example.com/demo".into(),
            license_name: "MIT".into(),
            license_url: "https://opensource.org/licenses/MIT".into(),
            developer_name: "Test".into(),
            developer_email: "test@example.com".into(),
        }
    }

    #[test]
    fn uploads_the_bundle_and_follows_the_deployment() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let base = format!("http://{}", listener.local_addr().unwrap());
        let server = std::thread::spawn(move || {
            let mut requests = Vec::new();
            for _ in 0..2 {
                let (mut stream, _) = listener.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(10)))
                    .unwrap();
                let mut data = Vec::new();
                let mut buffer = [0; 4096];
                loop {
                    let size = stream.read(&mut buffer).unwrap();
                    assert!(size > 0);
                    data.extend_from_slice(&buffer[..size]);
                    if let Some(end) = data.windows(4).position(|w| w == b"\r\n\r\n") {
                        let headers = String::from_utf8_lossy(&data[..end]).to_lowercase();
                        let length: usize = headers
                            .lines()
                            .find_map(|line| line.strip_prefix("content-length: "))
                            .unwrap_or("0")
                            .parse()
                            .unwrap();
                        if data.len() >= end + 4 + length {
                            break;
                        }
                    }
                }
                let request = String::from_utf8_lossy(&data).into_owned();
                let response = if request.starts_with("POST /api/v1/publisher/upload") {
                    "HTTP/1.1 201 Created\r\nContent-Length: 5\r\nConnection: close\r\n\r\ndep-1"
                        .to_owned()
                } else {
                    let body = r#"{"deploymentId":"dep-1","deploymentState":"PUBLISHED"}"#;
                    format!(
                        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                        body.len()
                    )
                };
                stream.write_all(response.as_bytes()).unwrap();
                requests.push(request);
            }
            requests
        });

        let temporary = tempfile::tempdir().unwrap();
        let bundle = temporary.path().join("bundle.zip");
        fs::write(&bundle, b"zip-bytes").unwrap();
        let client = Client::builder().build().unwrap();
        let id = upload(
            &client,
            &base,
            "dXNlcjpwYXNz",
            "com.example:demo:1.2.3",
            &bundle,
        )
        .unwrap();
        assert_eq!(id, "dep-1");
        wait_for_deployment(&client, &base, "dXNlcjpwYXNz", &id, false).unwrap();

        let requests = server.join().unwrap();
        assert!(requests[0].starts_with(
            "POST /api/v1/publisher/upload?name=com.example%3Ademo%3A1.2.3&publishingType=AUTOMATIC"
        ));
        assert!(requests[0]
            .to_lowercase()
            .contains("authorization: bearer dxnlcjpwyxnz"));
        assert!(requests[0].contains("name=\"bundle\"; filename=\"bundle.zip\""));
        assert!(requests[1].starts_with("POST /api/v1/publisher/status?id=dep-1"));
    }

    #[test]
    fn signs_and_bundles_a_central_layout() {
        let temporary = tempfile::tempdir().unwrap();
        let aar = temporary.path().join("demo.aar");
        fs::write(&aar, b"aar-bytes").unwrap();
        let coordinates = Coordinates {
            project: "demo".into(),
            group: "com.example".into(),
            artifact_id: "demo".into(),
            version: "1.2.3".into(),
            dependencies: vec![(
                "org.example".to_owned(),
                "dep".to_owned(),
                "1.0.0".to_owned(),
            )],
        };
        let signer = signer();
        let signature = signer.sign(b"probe").unwrap();
        assert!(String::from_utf8_lossy(&signature).starts_with("-----BEGIN PGP SIGNATURE-----"));
        assert!(String::from_utf8_lossy(&signature).ends_with("-----END PGP SIGNATURE-----\n"));

        let bundle = build_bundle(&coordinates, &metadata(), &signer, &aar).unwrap();
        let mut archive = zip::ZipArchive::new(fs::File::open(&bundle.bundle).unwrap()).unwrap();
        let prefix = "com/example/demo/1.2.3";
        for name in [
            "demo-1.2.3.aar",
            "demo-1.2.3.pom",
            "demo-1.2.3-sources.jar",
            "demo-1.2.3-javadoc.jar",
        ] {
            for suffix in ["", ".asc", ".md5", ".sha1", ".sha256", ".sha512"] {
                let path = format!("{prefix}/{name}{suffix}");
                assert!(
                    archive.by_name(&path).is_ok(),
                    "missing bundle member {path}"
                );
            }
        }
        let mut pom = String::new();
        {
            use std::io::Read;
            let mut file = archive
                .by_name(&format!("{prefix}/demo-1.2.3.pom"))
                .unwrap();
            file.read_to_string(&mut pom).unwrap();
        }
        assert!(pom.contains("<licenses>"));
        assert!(pom.contains("<developers>"));
        assert!(pom.contains("<scm>"));
        assert!(pom.contains("<artifactId>dep</artifactId>"));
    }
}
