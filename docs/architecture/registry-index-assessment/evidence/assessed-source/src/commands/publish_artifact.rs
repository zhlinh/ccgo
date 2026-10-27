//! Shared Maven and GitHub storage for Apple binaries and CCGO packages.
use super::{ArtifactProvider, PublishCommand};
use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use std::{fs, path::Path, process::Command};

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

enum Backend {
    Maven {
        root: String,
        group: String,
        username: String,
        password: String,
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
    backend: Backend,
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
                let get = |key: &str| std::env::var(key).unwrap_or_default();
                let base = get("CCGO_MAVEN_URL");
                if base.contains(',') {
                    bail!("Artifact upload requires a single CCGO_MAVEN_URL");
                }
                let group = command.artifact_group.as_deref()
                    .or_else(|| config.get("publish")?.get(scope)?.get("maven")?.get("group_id")?.as_str())
                    .with_context(|| format!("Maven upload requires --artifact-group or [publish.{scope}.maven].group_id"))?;
                let root = maven_url(&base, group, &artifact, version)?;
                (
                    format!("{root}.zip"),
                    Backend::Maven {
                        root,
                        group: group.to_owned(),
                        username: get("CCGO_MAVEN_USERNAME"),
                        password: get("CCGO_MAVEN_PASSWORD"),
                    },
                )
            }
            ArtifactProvider::Github => {
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
                let mut url =
                    reqwest::Url::parse(&format!("https://github.com/{repo}/releases/download/"))?;
                url.path_segments_mut()
                    .unwrap()
                    .pop_if_empty()
                    .push(tag)
                    .push(&archive_name);
                (
                    url.to_string(),
                    Backend::Github {
                        repo: repo.to_owned(),
                        tag: tag.to_owned(),
                    },
                )
            }
        };
        Ok(Self {
            url,
            archive_name,
            artifact,
            version: version.to_owned(),
            backend,
        })
    }

    pub fn upload(&self, archive: &Path) -> Result<()> {
        println!("Uploading artifact to {}", self.url);
        match &self.backend {
            Backend::Maven {
                root,
                group,
                username,
                password,
            } => {
                let client = Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .timeout(std::time::Duration::from_secs(300))
                    .build()?;
                let artifact = &self.artifact;
                let version = &self.version;
                let pom = format!("<project><modelVersion>4.0.0</modelVersion><groupId>{group}</groupId><artifactId>{artifact}</artifactId><version>{version}</version><packaging>zip</packaging></project>\n");
                for (extension, content) in [("zip", fs::read(archive)?), ("pom", pom.into_bytes())]
                {
                    let hash = format!("{:x}", Sha256::digest(&content));
                    put(
                        &client,
                        &format!("{root}.{extension}"),
                        content,
                        username,
                        password,
                    )?;
                    put(
                        &client,
                        &format!("{root}.{extension}.sha256"),
                        hash.into_bytes(),
                        username,
                        password,
                    )?;
                }
            }
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
                if !Command::new("gh")
                    .args(["release", "upload", tag, "--repo", repo])
                    .arg(&named_archive)
                    .status()?
                    .success()
                {
                    bail!("GitHub release asset upload failed");
                }
            }
        }
        Ok(())
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
