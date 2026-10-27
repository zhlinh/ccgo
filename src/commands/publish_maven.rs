//! Shared direct Maven uploads for prebuilt platform artifacts.
//!
//! KMP SDKs and Android AARs are already packaged, so local and private
//! repositories can receive them without running the project's Gradle build.
//! Maven Central is intentionally excluded: it needs the Gradle/nmcp workflow
//! (portal API, signing, staged metadata).

use super::{PublishCommand, RegistryType};
use anyhow::{bail, Context, Result};
use reqwest::blocking::Client;
use sha2::{Digest, Sha256};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

pub(super) enum Destination {
    Local(PathBuf),
    Remote {
        base: String,
        username: String,
        password: String,
    },
}

impl Destination {
    pub(super) fn describe(&self) -> String {
        match self {
            Self::Local(path) => path.display().to_string(),
            Self::Remote { base, .. } => base.clone(),
        }
    }

    pub(super) fn client(&self) -> Result<Option<Client>> {
        Ok(match self {
            Self::Remote { .. } => Some(
                Client::builder()
                    .redirect(reqwest::redirect::Policy::none())
                    .timeout(std::time::Duration::from_secs(300))
                    .build()?,
            ),
            Self::Local(_) => None,
        })
    }
}

pub(super) fn load_config(cwd: &Path) -> Result<toml::Value> {
    let path = cwd.join("CCGO.toml");
    let text = fs::read_to_string(&path).with_context(|| {
        format!(
            "Publishing a prebuilt artifact needs {} for Maven coordinates",
            path.display()
        )
    })?;
    toml::from_str(&text).with_context(|| format!("Invalid {}", path.display()))
}

pub(super) fn project_identity(config: &toml::Value) -> Result<(&str, &str)> {
    let name = string_at(config, &["project", "name"])
        .or_else(|| string_at(config, &["package", "name"]))
        .context("CCGO.toml requires [project] or [package] name for publishing")?;
    let version = string_at(config, &["project", "version"])
        .or_else(|| string_at(config, &["package", "version"]))
        .context("CCGO.toml requires [project] or [package] version for publishing")?;
    Ok((name, version))
}

pub(super) fn destination(command: &PublishCommand, config: &toml::Value) -> Result<Destination> {
    match command.registry {
        RegistryType::Local => Ok(Destination::Local(local_repository(config)?)),
        RegistryType::Private => {
            let base = command
                .remote
                .clone()
                .or_else(|| env_first("CCGO_MAVEN_URL"))
                .or_else(|| config_first(config, "custom_urls"))
                .context(
                    "Private Maven publishing requires --remote <maven-url> \
                     (or CCGO_MAVEN_URL)",
                )?;
            validate_repository_url(&base)?;
            let username = env_first("CCGO_MAVEN_USERNAME")
                .or_else(|| config_first(config, "custom_usernames"))
                .unwrap_or_default();
            let password = env_first("CCGO_MAVEN_PASSWORD")
                .or_else(|| config_first(config, "custom_passwords"))
                .unwrap_or_default();
            Ok(Destination::Remote {
                base: base.trim_end_matches('/').to_owned(),
                username,
                password,
            })
        }
        RegistryType::Official => bail!(
            "Maven Central publishing requires Gradle/nmcp; use --registry private \
             (or local) with a prebuilt artifact, or run publish without --path"
        ),
    }
}

pub(super) fn upload_with_checksum(
    destination: &Destination,
    client: Option<&Client>,
    relative: &str,
    bytes: Vec<u8>,
    verbose: bool,
) -> Result<()> {
    let checksum = format!("{:x}", Sha256::digest(&bytes));
    upload(destination, client, relative, bytes, verbose)?;
    upload(
        destination,
        client,
        &format!("{relative}.sha256"),
        checksum.into_bytes(),
        verbose,
    )
}

fn upload(
    destination: &Destination,
    client: Option<&Client>,
    relative: &str,
    bytes: Vec<u8>,
    verbose: bool,
) -> Result<()> {
    match destination {
        Destination::Local(root) => {
            let path = root.join(relative);
            let parent = path.parent().context("Maven artifact has no parent")?;
            fs::create_dir_all(parent)?;
            let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
            temporary.write_all(&bytes)?;
            temporary.as_file().sync_all()?;
            temporary
                .persist(&path)
                .with_context(|| format!("Failed to write {}", path.display()))?;
            if verbose {
                println!("  → {}", path.display());
            }
        }
        Destination::Remote {
            base,
            username,
            password,
        } => {
            let url = format!("{base}/{relative}");
            let mut request = client
                .context("Missing HTTP client for Maven upload")?
                .put(&url)
                .body(bytes);
            if !username.is_empty() {
                request = request.basic_auth(username, Some(password));
            }
            let response = request.send().context("Maven upload failed")?;
            if !response.status().is_success() {
                bail!(
                    "Maven upload failed (HTTP {}) for {url}; check hosted repository permissions",
                    response.status()
                );
            }
            if verbose {
                println!("  → {url}");
            }
        }
    }
    Ok(())
}

pub(super) fn layout_path(
    group: &str,
    artifact: &str,
    version: &str,
    classifier: Option<&str>,
    extension: &str,
) -> String {
    let classifier = classifier
        .map(|classifier| format!("-{classifier}"))
        .unwrap_or_default();
    format!(
        "{}/{artifact}/{version}/{artifact}-{version}{classifier}.{extension}",
        group.replace('.', "/")
    )
}

pub(super) fn xml_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

pub(super) fn value_at<'a>(root: &'a toml::Value, path: &[&str]) -> Option<&'a toml::Value> {
    path.iter().try_fold(root, |value, key| value.get(*key))
}

pub(super) fn string_at<'a>(root: &'a toml::Value, path: &[&str]) -> Option<&'a str> {
    value_at(root, path).and_then(|value| value.as_str())
}

fn env_first(name: &str) -> Option<String> {
    std::env::var(name)
        .ok()?
        .split(',')
        .map(str::trim)
        .find(|value| !value.is_empty())
        .map(str::to_owned)
}

fn config_first(config: &toml::Value, key: &str) -> Option<String> {
    match value_at(config, &["publish", "maven", key])? {
        toml::Value::String(value) => {
            let value = value.trim();
            (!value.is_empty()).then(|| value.to_owned())
        }
        toml::Value::Array(values) => values
            .iter()
            .filter_map(|value| value.as_str())
            .map(str::trim)
            .find(|value| !value.is_empty())
            .map(str::to_owned),
        _ => None,
    }
}

pub(super) fn local_repository(config: &toml::Value) -> Result<PathBuf> {
    let configured = std::env::var("MAVEN_LOCAL_PATH")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .or_else(|| string_at(config, &["publish", "maven", "local_path"]).map(str::to_owned));
    let path = match configured {
        Some(path) => expand_home(&path),
        None => directories::BaseDirs::new()
            .map(|dirs| dirs.home_dir().join(".m2/repository"))
            .context("Cannot determine the local Maven repository")?,
    };
    Ok(path)
}

fn validate_repository_url(base: &str) -> Result<()> {
    let parsed = reqwest::Url::parse(base).context("Invalid Maven repository URL")?;
    if !matches!(parsed.scheme(), "https" | "http")
        || parsed.host_str().is_none()
        || !parsed.username().is_empty()
        || parsed.password().is_some()
        || parsed.query().is_some()
        || parsed.fragment().is_some()
    {
        bail!("Maven repository URL must use HTTP(S) without credentials, query or fragment");
    }
    Ok(())
}

fn expand_home(path: &str) -> PathBuf {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf()) {
            return home.join(rest);
        }
    }
    PathBuf::from(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_uses_maven_paths_and_classifiers() {
        assert_eq!(
            layout_path("com.example", "demo-kmp-android", "0.0.5", None, "aar"),
            "com/example/demo-kmp-android/0.0.5/demo-kmp-android-0.0.5.aar"
        );
        assert_eq!(
            layout_path(
                "com.example",
                "demo-kmp-iosarm64",
                "0.0.5",
                Some("demo-cinterop"),
                "klib"
            ),
            "com/example/demo-kmp-iosarm64/0.0.5/demo-kmp-iosarm64-0.0.5-demo-cinterop.klib"
        );
    }

    #[test]
    fn escapes_xml() {
        assert_eq!(xml_escape("a<b&c\"d'e"), "a&lt;b&amp;c&quot;d&apos;e");
    }
}
