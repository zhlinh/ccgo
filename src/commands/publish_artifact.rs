//! Shared Maven and GitHub storage for Apple binaries and CCGO packages.
use super::{ArtifactProvider, MavenRegistry, PublishCommand};
use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(super) fn coordinate(value: &str) -> Result<&str> {
    if value.is_empty()
        || value == "."
        || value == ".."
        || !value
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"._-".contains(&c))
    {
        bail!("Invalid Maven coordinate: {value}");
    }
    Ok(value)
}

fn write_local(root: &Path, relative: &Path, bytes: &[u8]) -> Result<()> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&path, bytes)?;
    Ok(())
}

fn append_suffix(path: &Path, suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}{suffix}", path.display()))
}

fn maven_url(base: &str, group: &str, artifact: &str, version: &str) -> Result<String> {
    for part in group.split('.') {
        coordinate(part)?;
    }
    coordinate(artifact)?;
    coordinate(version)?;
    let parsed = reqwest::Url::parse(base).context("Invalid CCGO_MAVEN_URL")?;
    if !matches!(parsed.scheme(), "https" | "http")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        bail!("CCGO_MAVEN_URL must be an HTTP(S) repository URL without credentials, query or fragment");
    }
    Ok(format!(
        "{}/{}/{artifact}/{version}/{artifact}-{version}",
        base.trim_end_matches('/'),
        group.replace('.', "/")
    ))
}

fn put(client: &Client, url: &str, bytes: Vec<u8>, username: &str, password: &str) -> Result<()> {
    let mut request = client.put(url).body(bytes);
    if !username.is_empty() {
        request = request.basic_auth(username, Some(password));
    }
    let response = request.send().context("Maven upload failed")?;
    if !response.status().is_success() {
        bail!(
            "Maven upload failed (HTTP {}); check hosted repository permissions",
            response.status()
        );
    }
    Ok(())
}

fn get_existing(
    client: &Client,
    url: &str,
    username: &str,
    password: &str,
) -> Result<Option<Vec<u8>>> {
    let mut request = client.get(url);
    if !username.is_empty() {
        request = request.basic_auth(username, Some(password));
    }
    let response = request
        .send()
        .context("Failed to inspect existing Maven artifact")?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    if !response.status().is_success() {
        bail!("Cannot inspect Maven artifact (HTTP {})", response.status());
    }
    Ok(Some(response.bytes()?.to_vec()))
}

fn put_immutable(
    client: &Client,
    url: &str,
    bytes: Vec<u8>,
    username: &str,
    password: &str,
) -> Result<()> {
    if let Some(existing) = get_existing(client, url, username, password)? {
        if existing != bytes {
            bail!("Existing Maven artifact differs; refusing to overwrite {url}");
        }
        return Ok(());
    }
    let mut request = client
        .put(url)
        .header(reqwest::header::IF_NONE_MATCH, "*")
        .body(bytes.clone());
    if !username.is_empty() {
        request = request.basic_auth(username, Some(password));
    }
    let response = request.send().context("Maven upload failed")?;
    if !response.status().is_success() && !matches!(response.status().as_u16(), 409 | 412) {
        bail!("Maven upload failed (HTTP {})", response.status());
    }
    if get_existing(client, url, username, password)?.as_deref() != Some(bytes.as_slice()) {
        bail!("Maven artifact verification failed after upload: {url}");
    }
    Ok(())
}

enum Backend {
    Maven {
        root: String,
        group: String,
        username: String,
        password: String,
    },
    Central {
        group: String,
        artifact: String,
        version: String,
    },
    Local {
        root: PathBuf,
        relative: String,
        group: String,
    },
    Github {
        repo: String,
        tag: String,
    },
}

pub(super) struct PreparedUpload {
    pub url: String,
    archive_name: String,
    artifact: String,
    version: String,
    config: toml::Value,
    backend: Backend,
}

fn maven_group(command: &PublishCommand, config: &toml::Value, scope: &str) -> Result<String> {
    let group = command
        .artifact_group
        .as_deref()
        .or_else(|| {
            config
                .get("publish")?
                .get(scope)?
                .get("maven")?
                .get("group_id")?
                .as_str()
        })
        .with_context(|| {
            format!("Maven upload requires --artifact-group or [publish.{scope}.maven].group_id")
        })?;
    coordinate(group)?;
    Ok(group.to_owned())
}

fn maven_destination(
    command: &PublishCommand,
    config: &toml::Value,
    group: &str,
    artifact: &str,
    version: &str,
) -> Result<(String, Backend)> {
    match command.maven_registry {
        MavenRegistry::Official => {
            let root = maven_url("https://repo1.maven.org/maven2", group, artifact, version)?;
            Ok((
                format!("{root}.zip"),
                Backend::Central {
                    group: group.to_owned(),
                    artifact: artifact.to_owned(),
                    version: version.to_owned(),
                },
            ))
        }
        MavenRegistry::Local => {
            let root = super::maven::local_repository(config)?;
            let relative = super::maven::layout_path(group, artifact, version, None, "zip");
            let path = root.join(&relative);
            let url = reqwest::Url::from_file_path(&path)
                .map(|url| url.to_string())
                .unwrap_or_else(|_| path.display().to_string());
            Ok((
                url,
                Backend::Local {
                    root,
                    relative,
                    group: group.to_owned(),
                },
            ))
        }
        MavenRegistry::Private => {
            let get = |key: &str| std::env::var(key).unwrap_or_default();
            let base = get("CCGO_MAVEN_URL");
            if base.trim().is_empty() {
                bail!(
                    "Maven artifact upload requires CCGO_MAVEN_URL \
                     (for example https://repo.example.com/releases); set \
                     CCGO_MAVEN_USERNAME/CCGO_MAVEN_PASSWORD if the repository needs auth"
                );
            }
            if base.contains(',') {
                bail!("Artifact upload requires a single CCGO_MAVEN_URL");
            }
            let root = maven_url(&base, group, artifact, version)?;
            Ok((
                format!("{root}.zip"),
                Backend::Maven {
                    root,
                    group: group.to_owned(),
                    username: get("CCGO_MAVEN_USERNAME"),
                    password: get("CCGO_MAVEN_PASSWORD"),
                },
            ))
        }
    }
}

fn github_destination(
    command: &PublishCommand,
    tag: &str,
    archive_name: &str,
) -> Result<(String, Backend)> {
    let repo = command
        .github_repo
        .as_deref()
        .context("GitHub upload requires --github-repo owner/repo")?;
    let parts: Vec<_> = repo.split('/').collect();
    if parts.len() != 2 {
        bail!("--github-repo must be owner/repo");
    }
    for part in parts {
        coordinate(part)?;
    }
    let mut url = reqwest::Url::parse(&format!("https://github.com/{repo}/releases/download/"))?;
    url.path_segments_mut()
        .unwrap()
        .pop_if_empty()
        .push(tag)
        .push(archive_name);
    Ok((
        url.to_string(),
        Backend::Github {
            repo: repo.to_owned(),
            tag: tag.to_owned(),
        },
    ))
}

impl PreparedUpload {
    pub fn prepare(
        command: &PublishCommand,
        config: &toml::Value,
        name: &str,
        version: &str,
        tag: &str,
        scope: &str,
    ) -> Result<Self> {
        let suffix = if scope == "index" { "ccgo" } else { scope };
        let artifact = command
            .artifact_id
            .clone()
            .unwrap_or_else(|| format!("{name}-{suffix}"));
        coordinate(&artifact)?;
        coordinate(version)?;
        let archive_name = format!("{artifact}-{version}.zip");
        let (url, backend) = match command
            .artifact_provider
            .as_ref()
            .context("Missing artifact provider")?
        {
            ArtifactProvider::Maven => {
                let group = maven_group(command, config, scope)?;
                maven_destination(command, config, &group, &artifact, version)?
            }
            ArtifactProvider::Github => github_destination(command, tag, &archive_name)?,
        };
        Ok(Self {
            url,
            archive_name,
            artifact,
            version: version.to_owned(),
            config: config.clone(),
            backend,
        })
    }

    pub fn upload(&self, archive: &Path) -> Result<()> {
        self.upload_with_policy(archive, false)
    }

    pub fn upload_registry(&self, archive: &Path) -> Result<()> {
        self.upload_with_policy(archive, true)
    }

    fn upload_with_policy(&self, archive: &Path, immutable: bool) -> Result<()> {
        println!("Uploading artifact to {}", self.url);
        match &self.backend {
            Backend::Maven {
                root,
                group,
                username,
                password,
            } => self.upload_maven(archive, immutable, root, group, username, password)?,
            Backend::Central {
                group,
                artifact,
                version,
            } => {
                super::central::publish_artifact(
                    &self.config,
                    group,
                    artifact,
                    version,
                    "zip",
                    archive,
                    false,
                )?;
            }
            Backend::Local {
                root,
                relative,
                group,
            } => self.upload_local(archive, root, relative, group)?,
            Backend::Github { repo, tag } => {
                let status = Command::new("gh")
                    .args(["release", "view", tag, "--repo", repo])
                    .output()
                    .context("Failed to execute gh release view")?;
                if !status.status.success() {
                    bail!("GitHub release {tag} must already exist; create it before uploading");
                }
                // The asset name is independent of the caller's input filename.
                let temporary = tempfile::tempdir()?;
                let named_archive = temporary.path().join(&self.archive_name);
                fs::copy(archive, &named_archive)?;
                if immutable && self.github_existing_matches(repo, tag, archive)? {
                    return Ok(());
                }
                if !Command::new("gh")
                    .args(["release", "upload", tag, "--repo", repo])
                    .arg(&named_archive)
                    .status()?
                    .success()
                {
                    if immutable && self.github_existing_matches(repo, tag, archive)? {
                        return Ok(());
                    }
                    bail!("GitHub release asset upload failed");
                }
            }
        }
        Ok(())
    }

    fn upload_maven(
        &self,
        archive: &Path,
        immutable: bool,
        root: &str,
        group: &str,
        username: &str,
        password: &str,
    ) -> Result<()> {
        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(300))
            .build()?;
        let artifact = &self.artifact;
        let version = &self.version;
        let pom = format!("<project><modelVersion>4.0.0</modelVersion><groupId>{group}</groupId><artifactId>{artifact}</artifactId><version>{version}</version><packaging>zip</packaging></project>\n");
        for (extension, content) in [("zip", fs::read(archive)?), ("pom", pom.into_bytes())] {
            let hash = format!("{:x}", Sha256::digest(&content));
            let upload = if immutable { put_immutable } else { put };
            upload(
                &client,
                &format!("{root}.{extension}"),
                content,
                username,
                password,
            )?;
            upload(
                &client,
                &format!("{root}.{extension}.sha256"),
                hash.into_bytes(),
                username,
                password,
            )?;
        }
        Ok(())
    }

    fn upload_local(&self, archive: &Path, root: &Path, relative: &str, group: &str) -> Result<()> {
        let artifact = &self.artifact;
        let version = &self.version;
        let pom = format!("<project><modelVersion>4.0.0</modelVersion><groupId>{group}</groupId><artifactId>{artifact}</artifactId><version>{version}</version><packaging>zip</packaging></project>\n");
        let zip = fs::read(archive)?;
        write_local(root, Path::new(relative), &zip)?;
        let pom_relative = Path::new(relative).with_extension("pom");
        write_local(root, &pom_relative, pom.as_bytes())?;
        let zip_hash = format!("{:x}", Sha256::digest(&zip));
        write_local(
            root,
            &append_suffix(Path::new(relative), ".sha256"),
            zip_hash.as_bytes(),
        )?;
        let pom_hash = format!("{:x}", Sha256::digest(pom.as_bytes()));
        write_local(
            root,
            &append_suffix(&pom_relative, ".sha256"),
            pom_hash.as_bytes(),
        )?;
        Ok(())
    }

    fn github_existing_matches(&self, repo: &str, tag: &str, archive: &Path) -> Result<bool> {
        let temporary = tempfile::tempdir()?;
        let output = Command::new("gh")
            .args([
                "release",
                "download",
                tag,
                "--repo",
                repo,
                "--pattern",
                &self.archive_name,
                "--dir",
            ])
            .arg(temporary.path())
            .output()
            .context("Failed to inspect GitHub release asset")?;
        if !output.status.success() {
            return Ok(false); // Upload without --clobber remains a create-only operation.
        }
        if fs::read(temporary.path().join(&self.archive_name))? != fs::read(archive)? {
            bail!(
                "Existing GitHub asset differs; refusing to overwrite {}",
                self.url
            );
        }
        Ok(true)
    }
}

/// Freeze the bytes used for validation, checksums, and upload without touching input files.
pub(super) struct ArchiveSnapshot {
    pub path: std::path::PathBuf,
    pub checksum: String,
    _directory: tempfile::TempDir,
}

impl ArchiveSnapshot {
    pub fn new(source: &Path) -> Result<Self> {
        use std::io::Read;
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("archive.zip");
        fs::copy(source, &path).with_context(|| {
            format!(
                "Cannot read artifact ZIP {}; run ccgo package --release or specify --path",
                source.display()
            )
        })?;
        let mut zip =
            zip::ZipArchive::new(fs::File::open(&path)?).context("Invalid artifact ZIP")?;
        if zip.is_empty() {
            bail!("Artifact ZIP must not be empty");
        }
        for index in 0..zip.len() {
            zip.by_index(index)?
                .enclosed_name()
                .context("Unsafe path in artifact ZIP")?;
        }
        let mut file = fs::File::open(&path)?;
        let mut hasher = Sha256::new();
        let mut buffer = [0u8; 65536];
        loop {
            let read = file.read(&mut buffer)?;
            if read == 0 {
                break;
            }
            hasher.update(&buffer[..read]);
        }
        Ok(Self {
            path,
            checksum: format!("sha256:{:x}", hasher.finalize()),
            _directory: directory,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn urls_and_coordinates() {
        assert_eq!(
            maven_url(
                "https://repo.example/releases/",
                "com.example",
                "sdk-apple",
                "1.2.3"
            )
            .unwrap(),
            "https://repo.example/releases/com/example/sdk-apple/1.2.3/sdk-apple-1.2.3"
        );
        for group in ["../bad", "com..example", "a&b"] {
            assert!(maven_url("https://repo.example", group, "sdk", "1").is_err());
        }
        assert!(maven_url(
            "https://user:password@repo.example",
            "com.example",
            "sdk",
            "1"
        )
        .is_err());
    }

    #[test]
    fn maven_local_provider_uses_the_local_repository() {
        use clap::Parser;
        #[derive(Parser)]
        struct Cli {
            #[command(flatten)]
            command: PublishCommand,
        }
        let command = Cli::parse_from([
            "ccgo",
            "doc",
            "--artifact-provider",
            "maven",
            "--maven-registry",
            "local",
            "--artifact-group",
            "com.example",
        ])
        .command;
        let repository = tempfile::tempdir().unwrap();
        let config: toml::Value = toml::from_str(&format!(
            "[project]\nname='demo'\nversion='1.0.0'\n\
             [publish.maven]\nlocal_path='{}'\n",
            repository.path().display()
        ))
        .unwrap();
        let upload =
            PreparedUpload::prepare(&command, &config, "demo", "1.0.0", "v1.0.0", "doc").unwrap();
        assert!(upload.url.starts_with("file://"));
        assert!(upload
            .url
            .ends_with("com/example/demo-doc/1.0.0/demo-doc-1.0.0.zip"));

        let archive = tempfile::NamedTempFile::new().unwrap();
        std::fs::write(archive.path(), b"zip-bytes").unwrap();
        upload.upload(archive.path()).unwrap();
        let directory = repository.path().join("com/example/demo-doc/1.0.0");
        assert_eq!(
            std::fs::read(directory.join("demo-doc-1.0.0.zip")).unwrap(),
            b"zip-bytes"
        );
        assert!(directory.join("demo-doc-1.0.0.pom").is_file());
        assert!(directory.join("demo-doc-1.0.0.zip.sha256").is_file());
    }

    #[test]
    fn central_provider_uses_the_public_maven_url() {
        use clap::Parser;
        #[derive(Parser)]
        struct Cli {
            #[command(flatten)]
            command: PublishCommand,
        }
        let command = Cli::parse_from([
            "ccgo",
            "doc",
            "--artifact-provider",
            "maven",
            "--maven-registry",
            "official",
            "--artifact-group",
            "com.example",
        ])
        .command;
        let config: toml::Value =
            toml::from_str("[project]\nname='demo'\nversion='1.0.0'\n").unwrap();
        let upload =
            PreparedUpload::prepare(&command, &config, "demo", "1.0.0", "v1.0.0", "doc").unwrap();
        assert_eq!(
            upload.url,
            "https://repo1.maven.org/maven2/com/example/demo-doc/1.0.0/demo-doc-1.0.0.zip"
        );
    }
}
