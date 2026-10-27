//! Resolve index destinations independently from package source repositories.
use super::{PublishCommand, RegistryType};
use crate::{
    config::CcgoConfig,
    registry::{PackageIndex, DEFAULT_REGISTRY, DEFAULT_REGISTRY_URL},
};
use anyhow::{bail, Context, Result};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

pub(super) struct Destination {
    pub name: String,
    pub repository: String,
    pub local: bool,
}

fn validate_name(name: &str) -> Result<()> {
    if name.is_empty()
        || matches!(name, "." | "..")
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
    {
        bail!("Invalid index registry name: {name}");
    }
    Ok(())
}

pub(super) fn resolve(command: &PublishCommand, config: &CcgoConfig) -> Result<Destination> {
    match command.registry {
        RegistryType::Local => {
            if command.remote.is_some() {
                bail!("Index --remote requires --registry private");
            }
            let name = command.index_name.as_deref().unwrap_or("local");
            validate_name(name)?;
            let directory = if let Some(path) = &command.index_repo {
                if path.contains("://") || path.starts_with("git@") {
                    bail!("Local --index-repo must be a filesystem directory, not a remote URL");
                }
                PathBuf::from(path)
            } else {
                PackageIndex::new()
                    .ccgo_home_path()
                    .join("registry/publish")
                    .join(name)
            };
            let absolute = if directory.is_absolute() {
                directory
            } else {
                std::env::current_dir()?.join(directory)
            };
            Ok(Destination {
                name: name.to_owned(),
                repository: absolute.to_string_lossy().into_owned(),
                local: true,
            })
        }
        RegistryType::Official => {
            if command.index_repo.is_some()
                || command.remote.is_some()
                || command.index_name.is_some()
            {
                bail!("Official index is fixed to {DEFAULT_REGISTRY_URL}; use --registry private --remote <name> for another repository");
            }
            Ok(Destination {
                name: DEFAULT_REGISTRY.to_owned(),
                repository: DEFAULT_REGISTRY_URL.to_owned(),
                local: false,
            })
        }
        RegistryType::Private => {
            if command.index_repo.is_some() || command.index_name.is_some() {
                bail!("Private index uses --remote; configure its URL with ccgo registry add or [registries] in CCGO.toml");
            }
            let name = command
                .remote
                .as_deref()
                .context("Private index requires --remote <registry-name>")?;
            validate_name(name)?;
            if name == DEFAULT_REGISTRY {
                bail!("Use --registry official for the default CCGO index");
            }
            let index = PackageIndex::new();
            let repository = config.registries.get(name).cloned()
                .or_else(|| index.get_registry(name).map(|r| r.url.clone()))
                .with_context(|| format!("Index registry '{name}' is not configured; use ccgo registry add {name} <url> or [registries] in CCGO.toml"))?;
            Ok(Destination {
                name: name.to_owned(),
                repository,
                local: false,
            })
        }
    }
}

pub(super) fn prepare_local(directory: &str, source_project: &Path) -> Result<PathBuf> {
    let path = Path::new(directory);
    fs::create_dir_all(path)?;
    let path = fs::canonicalize(path)?;
    if source_project.starts_with(&path) {
        bail!("Local index must be a separate directory, not the source repository or its parent");
    }
    if !path.join(".git").exists()
        && !Command::new("git")
            .current_dir(&path)
            .arg("init")
            .status()?
            .success()
    {
        bail!("Failed to initialize local index repository");
    }
    let root = Command::new("git")
        .current_dir(&path)
        .args(["rev-parse", "--show-toplevel"])
        .output()?;
    if !root.status.success()
        || fs::canonicalize(String::from_utf8_lossy(&root.stdout).trim())? != path
    {
        bail!("Local index must be the root of its own Git working tree");
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[derive(Parser)]
    struct Cli {
        #[command(flatten)]
        command: PublishCommand,
    }

    #[test]
    fn official_destination_is_fixed() {
        let command = Cli::parse_from(["ccgo", "index", "--registry", "official"]).command;
        let config: CcgoConfig =
            toml::from_str("[package]\nname='demo'\nversion='1.0.0'\n").unwrap();
        let destination = resolve(&command, &config).unwrap();
        assert_eq!(destination.name, DEFAULT_REGISTRY);
        assert_eq!(destination.repository, DEFAULT_REGISTRY_URL);
        assert!(!destination.local);
    }

    #[test]
    fn project_registry_resolves_without_global_registration() {
        let command = Cli::parse_from([
            "ccgo",
            "index",
            "--registry",
            "private",
            "--remote",
            "company",
        ])
        .command;
        let config: CcgoConfig = toml::from_str("[package]\nname='demo'\nversion='1.0.0'\n[registries]\ncompany='https://git.example/index.git'\n").unwrap();
        let destination = resolve(&command, &config).unwrap();
        assert_eq!(destination.name, "company");
        assert_eq!(destination.repository, "https://git.example/index.git");
        assert!(!destination.local);
    }
}
