//! Git checkouts are published as separate generations; live trees are never reset.
use super::cache_access::{home_lock, lock_directory, CacheLock};
use anyhow::{bail, Context, Result};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::Command,
};

pub struct GitCacheSpec<'a> {
    pub name: &'a str,
    pub url: &'a str,
    pub branch: Option<&'a str>,
    pub revision: Option<&'a str>,
}

impl GitCacheSpec<'_> {
    fn key(&self) -> Result<String> {
        Ok(format!(
            "{:x}",
            md5::compute(serde_json::to_vec(&(
                self.name,
                source_identity(self.url)?,
                self.branch.filter(|_| self.revision.is_none()),
                self.revision
            ))?)
        ))
    }

    fn legacy_key(&self) -> String {
        let input = match self.revision {
            Some(rev) => format!("{}:{}:{rev}", self.name, self.url),
            None => format!("{}:{}", self.name, self.url),
        };
        format!("{:x}", md5::compute(input))
    }
}

pub struct GitCheckout {
    pub path: PathBuf,
    _writer: CacheLock,
    _home: CacheLock,
}

/// Prefer this project's locked tree over another project's refreshed pointer.
pub fn installed_path(
    project: &Path,
    spec: &GitCacheSpec<'_>,
    version: &str,
    lockfile: Option<&crate::lockfile::Lockfile>,
) -> Result<Option<PathBuf>> {
    let Some(locked) = lockfile.and_then(|lock| lock.get_package(spec.name)) else {
        return Ok(None);
    };
    let (kind, url) = locked.parse_source();
    if kind != crate::lockfile::SourceType::Git
        || (!version.is_empty() && locked.version != version)
        || source_identity(&url)? != source_identity(spec.url)?
        || locked.git.as_ref().and_then(|git| git.branch.as_deref()) != spec.branch
    {
        return Ok(None);
    }
    let path = project.join(".ccgo/deps").join(spec.name);
    if !path.join(".git").is_dir() {
        return Ok(None);
    }
    verify_checkout(
        &path,
        &GitCacheSpec {
            revision: locked.git_revision(),
            ..*spec
        },
    )?;
    Ok(Some(path.canonicalize()?))
}

/// Look up a published pointer, with read compatibility for earlier cache layouts.
/// Callers reading the returned tree must hold a shared home lifecycle lock.
pub fn cached_path(home: &Path, spec: &GitCacheSpec<'_>) -> Result<Option<PathBuf>> {
    let key = spec.key()?;
    let checkouts = home.join("git/checkouts");
    let pointer = checkouts.join(".ccgo-pointers").join(format!("{key}.json"));
    match fs::read(&pointer) {
        Ok(bytes) => {
            let generation: String =
                serde_json::from_slice(&bytes).context("Invalid Git cache pointer")?;
            if !generation.starts_with(&format!("{key}-"))
                || !generation
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-')
            {
                bail!("Invalid Git cache generation: {generation}");
            }
            let path = checkouts.join(generation);
            if !path.join(".git").is_dir() {
                bail!("Git cache generation is missing; use --force to retrieve it again");
            }
            Ok(Some(path.canonicalize()?))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => legacy_path(home, spec),
        Err(error) => Err(error.into()),
    }
}

fn legacy_path(home: &Path, spec: &GitCacheSpec<'_>) -> Result<Option<PathBuf>> {
    // Old unpinned entries did not distinguish branches. Do not guess their identity.
    if spec.revision.is_none() && spec.branch.is_some() {
        return Ok(None);
    }
    let key = spec.legacy_key();
    let candidates = [
        home.join("git/checkouts").join(&key),
        home.join("registry")
            .join(format!("{}-{}", spec.name, &key[..16])),
    ];
    candidates
        .into_iter()
        .find(|path| path.join(".git").is_dir())
        .map(|path| path.canonicalize().map_err(Into::into))
        .transpose()
}

pub fn prepare(home: &Path, spec: &GitCacheSpec<'_>, force: bool) -> Result<GitCheckout> {
    let home_guard = home_lock(home, false)?;
    let writer = CacheLock::acquire(
        &lock_directory(home)?.join(format!("git-{}.lock", spec.key()?)),
        true,
    )?;
    let path = if force {
        publish_generation(home, spec)?
    } else if let Some(path) = cached_path(home, spec)? {
        verify_checkout(&path, spec)?;
        path
    } else {
        publish_generation(home, spec)?
    };
    Ok(GitCheckout {
        path,
        _writer: writer,
        _home: home_guard,
    })
}

fn publish_generation(home: &Path, spec: &GitCacheSpec<'_>) -> Result<PathBuf> {
    let checkouts = home.join("git/checkouts");
    fs::create_dir_all(&checkouts)?;
    let checkouts = checkouts.canonicalize()?;
    let staging = tempfile::tempdir_in(&checkouts)?;
    let tree = staging.path().join("checkout");
    let mut clone = Command::new("git");
    clone.arg("clone");
    if spec.revision.is_none() {
        if let Some(branch) = spec.branch {
            clone.args(["--branch", branch]);
        }
    }
    clone.arg("--").arg(source_identity(spec.url)?).arg(&tree);
    run(&mut clone, "Git clone failed")?;
    if let Some(revision) = spec.revision {
        run(
            Command::new("git")
                .current_dir(&tree)
                .args(["checkout", "--detach", revision]),
            "Git checkout failed",
        )?;
    }
    verify_checkout(&tree, spec)?;
    let token = staging
        .path()
        .file_name()
        .context("Missing Git staging name")?
        .to_string_lossy();
    let generation = format!("{}-{}", spec.key()?, token.trim_start_matches('.'));
    let path = checkouts.join(&generation);
    fs::rename(&tree, &path)?;
    // Never expose a partial clone, or replace a pointer before validation finishes.
    write_pointer(&checkouts, &spec.key()?, &generation)?;
    Ok(path)
}

fn write_pointer(checkouts: &Path, key: &str, generation: &str) -> Result<()> {
    let pointers = checkouts.join(".ccgo-pointers");
    fs::create_dir_all(&pointers)?;
    let mut temporary = tempfile::NamedTempFile::new_in(&pointers)?;
    temporary.write_all(&serde_json::to_vec(generation)?)?;
    temporary.as_file().sync_all()?;
    temporary.persist(pointers.join(format!("{key}.json")))?;
    Ok(())
}

fn verify_checkout(path: &Path, spec: &GitCacheSpec<'_>) -> Result<()> {
    let origin = run(
        Command::new("git")
            .current_dir(path)
            .args(["remote", "get-url", "origin"]),
        "Failed to inspect Git cache origin",
    )?;
    if source_identity(origin.trim())? != source_identity(spec.url)? {
        bail!(
            "Git cache has a different origin; use --force to retrieve the configured repository"
        );
    }
    if let Some(revision) = spec.revision {
        let head = run(
            Command::new("git")
                .current_dir(path)
                .args(["rev-parse", "HEAD"]),
            "Failed to inspect Git checkout",
        )?;
        if head.trim() != revision {
            bail!("Git cache commit differs from the locked revision; use --force to retrieve it again");
        }
        let status = run(
            Command::new("git")
                .current_dir(path)
                .args(["status", "--porcelain"]),
            "Failed to inspect Git checkout",
        )?;
        if !status.is_empty() {
            bail!("Locked Git checkout is dirty; use --force to retrieve the locked commit again");
        }
    }
    Ok(())
}

fn run(command: &mut Command, message: &str) -> Result<String> {
    let output = command.output().with_context(|| message.to_owned())?;
    if !output.status.success() {
        bail!("{message}: {}", String::from_utf8_lossy(&output.stderr));
    }
    Ok(String::from_utf8(output.stdout)?)
}

fn source_identity(url: &str) -> Result<String> {
    let path = Path::new(url);
    if url.contains(':') && !path.is_absolute() {
        return Ok(url.to_owned());
    }
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut normalized = PathBuf::new();
    for component in absolute.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                normalized.pop();
            }
            other => normalized.push(other.as_os_str()),
        }
    }
    Ok(normalized.to_string_lossy().into_owned())
}
