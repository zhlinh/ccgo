//! Publish command implementation

use anyhow::{bail, Context, Result};
use clap::{Args, ValueEnum};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::config::CcgoConfig;
use crate::registry::{IndexMetadata, PackageEntry, PackageIndex, VersionEntry};

/// Publish target
#[derive(Debug, Clone, ValueEnum)]
pub enum PublishTarget {
    /// Android (Maven)
    Android,
    /// OpenHarmony (OHPM)
    Ohos,
    /// Apple platforms (CocoaPods/SPM)
    Apple,
    /// Conan package
    Conan,
    /// Kotlin Multiplatform
    Kmp,
    /// Documentation
    Doc,
    /// Package index (registry)
    Index,
}

impl std::fmt::Display for PublishTarget {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PublishTarget::Android => write!(f, "android"),
            PublishTarget::Ohos => write!(f, "ohos"),
            PublishTarget::Apple => write!(f, "apple"),
            PublishTarget::Conan => write!(f, "conan"),
            PublishTarget::Kmp => write!(f, "kmp"),
            PublishTarget::Doc => write!(f, "doc"),
            PublishTarget::Index => write!(f, "index"),
        }
    }
}

/// Registry type
#[derive(Debug, Clone, Default, ValueEnum)]
pub enum RegistryType {
    /// Local registry
    #[default]
    Local,
    /// Official registry
    Official,
    /// Private registry
    Private,
}

impl std::fmt::Display for RegistryType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RegistryType::Local => write!(f, "local"),
            RegistryType::Official => write!(f, "official"),
            RegistryType::Private => write!(f, "private"),
        }
    }
}

/// Apple package manager
#[derive(Debug, Clone, Default, ValueEnum)]
pub enum AppleManager {
    /// CocoaPods
    Cocoapods,
    /// Swift Package Manager
    Spm,
    /// Both
    #[default]
    All,
}

impl std::fmt::Display for AppleManager {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppleManager::Cocoapods => write!(f, "cocoapods"),
            AppleManager::Spm => write!(f, "spm"),
            AppleManager::All => write!(f, "all"),
        }
    }
}

/// Publish library to repository
#[derive(Args, Debug)]
pub struct PublishCommand {
    /// Publish target
    #[arg(value_enum)]
    pub target: PublishTarget,

    /// Registry type
    #[arg(long, value_enum, default_value_t = RegistryType::Local)]
    pub registry: RegistryType,

    /// Custom registry URL (for private)
    #[arg(long)]
    pub url: Option<String>,

    /// Remote name (for CocoaPods/Conan)
    #[arg(long)]
    pub remote_name: Option<String>,

    /// Skip build step
    #[arg(long)]
    pub skip_build: bool,

    /// Skip confirmation prompts
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Package manager (for apple)
    #[arg(long, value_enum, default_value_t = AppleManager::All)]
    pub manager: AppleManager,

    /// Push to remote (for SPM)
    #[arg(long)]
    pub push: bool,

    /// Apple platforms (for apple)
    #[arg(long)]
    pub platform: Option<String>,

    /// Allow warnings (for CocoaPods)
    #[arg(long, default_value_t = true)]
    pub allow_warnings: bool,

    /// Conan profile
    #[arg(long, default_value = "default")]
    pub profile: String,

    /// Link type (for Conan)
    #[arg(long, default_value = "both")]
    pub link_type: String,

    /// Documentation branch
    #[arg(long, default_value = "gh-pages")]
    pub doc_branch: String,

    /// Force push documentation
    #[arg(long)]
    pub doc_force: bool,

    /// Open documentation after publish
    #[arg(long)]
    pub doc_open: bool,

    // Index-specific options
    /// Index repository URL or local path (for index target)
    #[arg(long)]
    pub index_repo: Option<String>,

    /// Index registry name (for index target, default: ccgo-packages)
    #[arg(long)]
    pub index_name: Option<String>,

    /// Push changes to remote after updating index
    #[arg(long)]
    pub index_push: bool,

    /// Commit message for index update
    #[arg(long)]
    pub index_message: Option<String>,

    /// Publish a SINGLE explicit version into the index (append-only,
    /// CocoaPods `pod repo push`-style).
    ///
    /// When set, the index entry's existing `versions` array is preserved
    /// and the new version is appended. Re-publishing the same version
    /// is rejected. When ABSENT, the legacy auto-walk behavior runs and
    /// the entry is rebuilt from `git tag -l`.
    ///
    /// `--index-version` is the SemVer string written into
    /// `VersionEntry.version`. `--index-tag` is the actual git tag to
    /// validate via `git rev-parse --verify`. Either can be derived
    /// from the other:
    ///   * `--index-tag v1.0.0` alone → version stripped to `1.0.0`
    ///   * `--index-version 1.0.0` alone → tag defaults to `v1.0.0`
    ///
    /// Pass both explicitly when your tag convention isn't `v<version>`.
    #[arg(long)]
    pub index_version: Option<String>,

    /// Git tag for the single-version publish — see `--index-version`.
    #[arg(long)]
    pub index_tag: Option<String>,

    /// Generate SHA-256 checksums for each version recorded in the index.
    ///
    /// Behavior depends on whether `--archive-url-template` is set:
    ///
    /// * **With** `--archive-url-template`: hash the local published
    ///   `target/release/package/<NAME>_CCGO_PACKAGE-<version>.zip`. This
    ///   is what consumers will download, so it's what we must hash.
    ///   Versions whose local zip is absent (typical for historical tags)
    ///   silently get `None` — consumers skip verification for those.
    ///
    /// * **Without** `--archive-url-template`: hash the git source
    ///   tarball at the tag (legacy behavior; informational only since
    ///   no fetch path verifies against it today).
    #[arg(long)]
    pub checksum: bool,

    /// URL template for VersionEntry.archive_url in published index entries.
    ///
    /// Placeholders: `{name}`, `{version}`, `{tag}` are substituted at publish
    /// time. Without this flag, archive_url stays None and consumers must
    /// supply git/zip URLs explicitly in their CCGO.toml.
    ///
    /// Example:
    ///   --archive-url-template "https://artifacts.example.com/{name}/{name}_CCGO_PACKAGE-{version}.zip"
    #[arg(long)]
    pub archive_url_template: Option<String>,

    /// Archive format recorded in VersionEntry.archive_format. Only "zip" and
    /// "tar.gz" are supported by the resolver today.
    #[arg(long, default_value = "zip")]
    pub archive_format: String,
}

/// Replace `{name}`, `{version}`, and `{tag}` placeholders in a template
/// string. Order matters: `{name}` is replaced first so `{name}-{version}`
/// won't accidentally consume part of a versioned-name pattern.
fn substitute_archive_url(template: &str, name: &str, version: &str, tag: &str) -> String {
    template
        .replace("{name}", name)
        .replace("{version}", version)
        .replace("{tag}", tag)
}

/// Strip a single leading `v` or `V` from a git tag to derive a SemVer
/// string. Used by `--index-tag` when `--index-version` is omitted.
fn derive_version_from_tag(tag: &str) -> String {
    tag.strip_prefix('v')
        .or_else(|| tag.strip_prefix('V'))
        .unwrap_or(tag)
        .to_string()
}

/// Default tag form `v<version>`. Used by `--index-version` when
/// `--index-tag` is omitted. Most projects follow this convention; pass
/// `--index-tag` explicitly when yours doesn't.
fn default_tag_for_version(version: &str) -> String {
    format!("v{}", version)
}

/// Append a new VersionEntry to an existing list. Rejects duplicates so
/// re-publishing the same version surfaces a clear error rather than
/// silently rewriting history. Result is sorted descending by version
/// string (matches `git tag -l --sort=-v:refname`).
fn merge_version_entry(
    mut existing: Vec<VersionEntry>,
    new: VersionEntry,
) -> Result<Vec<VersionEntry>> {
    if let Some(dup) = existing.iter().find(|v| v.version == new.version) {
        anyhow::bail!(
            "version '{}' is already in the index (tag '{}'); the index is \
             append-only by design. Yank or hand-edit the JSON first if you \
             really intend to overwrite.",
            new.version,
            dup.tag
        );
    }
    existing.push(new);
    existing.sort_by(|a, b| b.version.cmp(&a.version));
    Ok(existing)
}

impl PublishCommand {
    /// Execute the publish command
    pub fn execute(self, verbose: bool) -> Result<()> {
        println!("Publishing library project...\n");

        match self.target {
            PublishTarget::Android => self.publish_android(verbose),
            PublishTarget::Ohos => self.publish_ohos(verbose),
            PublishTarget::Kmp => self.publish_kmp(verbose),
            PublishTarget::Conan => self.publish_conan(verbose),
            PublishTarget::Apple => self.publish_apple(verbose),
            PublishTarget::Doc => self.publish_doc(verbose),
            PublishTarget::Index => self.publish_index(verbose),
        }
    }

    fn publish_android(&self, verbose: bool) -> Result<()> {
        println!("=== Publishing Android to Maven ===\n");

        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        let android_dir = cwd.join("android");

        if !android_dir.exists() || !android_dir.is_dir() {
            bail!(
                "Error: android directory not found at {}",
                android_dir.display()
            );
        }

        // Map registry to Gradle task
        let gradle_task = match self.registry {
            RegistryType::Local => "ccgoPublishToMavenLocal",
            RegistryType::Official => "ccgoPublishToMavenCentral",
            RegistryType::Private => "ccgoPublishToMavenCustom",
        };

        println!("Publishing to {} Maven repository", self.registry);
        println!("Running: ./gradlew {}", gradle_task);
        println!("{}", "-".repeat(60));

        let mut cmd = Command::new("./gradlew");
        cmd.current_dir(&android_dir);
        cmd.arg(gradle_task);
        cmd.arg("--no-daemon");

        if self.skip_build {
            cmd.arg("-x").arg("buildAAR");
        }

        if verbose {
            cmd.arg("--info");
        }

        let status = cmd.status().context("Failed to execute gradlew")?;

        if !status.success() {
            bail!(
                "Publish failed with exit code: {}",
                status.code().unwrap_or(-1)
            );
        }

        println!("\n✅ Publish completed successfully!");
        Ok(())
    }

    fn publish_ohos(&self, verbose: bool) -> Result<()> {
        println!("=== Publishing OHOS to OHPM ===\n");

        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        let ohos_dir = cwd.join("ohos");

        if !ohos_dir.exists() || !ohos_dir.is_dir() {
            bail!("Error: ohos directory not found at {}", ohos_dir.display());
        }

        // Determine OHPM registry
        let registry_args = match self.registry {
            RegistryType::Local => vec!["publish"],
            RegistryType::Official => vec!["publish"],
            RegistryType::Private => {
                if let Some(url) = &self.url {
                    vec!["publish", "--registry", url]
                } else {
                    bail!("Private registry requires --url");
                }
            }
        };

        println!("Publishing to {} OHPM registry", self.registry);
        println!("Running: ohpm {:?}", registry_args);
        println!("{}", "-".repeat(60));

        let mut cmd = Command::new("ohpm");
        cmd.current_dir(&ohos_dir);
        cmd.args(&registry_args);

        if verbose {
            cmd.arg("--verbose");
        }

        let status = cmd.status().context("Failed to execute ohpm")?;

        if !status.success() {
            bail!(
                "Publish failed with exit code: {}",
                status.code().unwrap_or(-1)
            );
        }

        println!("\n✅ Publish completed successfully!");
        Ok(())
    }

    fn publish_kmp(&self, verbose: bool) -> Result<()> {
        println!("=== Publishing KMP to Maven ===\n");

        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        let kmp_dir = cwd.join("kmp");

        if !kmp_dir.exists() || !kmp_dir.is_dir() {
            bail!("Error: kmp directory not found at {}", kmp_dir.display());
        }

        // Map registry to Gradle task
        let gradle_task = match self.registry {
            RegistryType::Local => "publishToMavenLocal",
            RegistryType::Official => "publishAllPublicationsToMavenCentralRepository",
            RegistryType::Private => "publishAllPublicationsToMavenCustomRepository",
        };

        println!("Publishing to {} Maven repository", self.registry);
        println!("Running: ./gradlew {}", gradle_task);
        println!("{}", "-".repeat(60));

        let mut cmd = Command::new("./gradlew");
        cmd.current_dir(&kmp_dir);
        cmd.arg(gradle_task);
        cmd.arg("--no-daemon");

        if verbose {
            cmd.arg("--info");
        }

        let status = cmd.status().context("Failed to execute gradlew")?;

        if !status.success() {
            bail!(
                "Publish failed with exit code: {}",
                status.code().unwrap_or(-1)
            );
        }

        println!("\n✅ Publish completed successfully!");
        Ok(())
    }

    fn publish_conan(&self, verbose: bool) -> Result<()> {
        println!("=== Publishing to Conan ===\n");

        let cwd = std::env::current_dir().context("Failed to get current directory")?;

        // Find CCGO.toml
        let project_dir = Self::find_project_dir(&cwd)?;

        match self.registry {
            RegistryType::Local => {
                println!("Exporting to local Conan cache");

                let mut cmd = Command::new("conan");
                cmd.current_dir(&project_dir);
                cmd.args(["export", ".", "--user=ccgo", "--channel=stable"]);

                if verbose {
                    cmd.arg("-vv");
                }

                let status = cmd.status().context("Failed to execute conan export")?;

                if !status.success() {
                    bail!("Conan export failed");
                }
            }
            RegistryType::Official | RegistryType::Private => {
                let remote = if let Some(remote_name) = &self.remote_name {
                    remote_name.clone()
                } else {
                    // Get first available remote
                    self.get_conan_remotes()?
                        .first()
                        .ok_or_else(|| anyhow::anyhow!("No Conan remotes configured"))?
                        .clone()
                };

                println!("Uploading to Conan remote: {}", remote);

                // First export to local cache
                let mut cmd = Command::new("conan");
                cmd.current_dir(&project_dir);
                cmd.args(["export", ".", "--user=ccgo", "--channel=stable"]);

                let status = cmd.status().context("Failed to execute conan export")?;

                if !status.success() {
                    bail!("Conan export failed");
                }

                // Then upload to remote
                let mut cmd = Command::new("conan");
                cmd.current_dir(&project_dir);
                cmd.args(["upload", "*", "--remote", &remote, "--confirm"]);

                if verbose {
                    cmd.arg("-vv");
                }

                let status = cmd.status().context("Failed to execute conan upload")?;

                if !status.success() {
                    bail!("Conan upload failed");
                }
            }
        }

        println!("\n✅ Publish completed successfully!");
        Ok(())
    }

    fn publish_apple(&self, verbose: bool) -> Result<()> {
        println!("=== Publishing Apple platforms ===\n");

        let cwd = std::env::current_dir().context("Failed to get current directory")?;

        match self.manager {
            AppleManager::Cocoapods | AppleManager::All => {
                self.publish_cocoapods(&cwd, verbose)?;
            }
            _ => {}
        }

        match self.manager {
            AppleManager::Spm | AppleManager::All => {
                self.publish_spm(&cwd, verbose)?;
            }
            _ => {}
        }

        println!("\n✅ Publish completed successfully!");
        Ok(())
    }

    fn publish_cocoapods(&self, project_dir: &Path, verbose: bool) -> Result<()> {
        println!("Publishing to CocoaPods...");

        // Find .podspec file
        let podspec = self.find_file_with_extension(project_dir, ".podspec")?;

        match self.registry {
            RegistryType::Local => {
                println!("Validating podspec locally...");

                let mut cmd = Command::new("pod");
                cmd.current_dir(project_dir);
                cmd.args(["lib", "lint", podspec.to_str().unwrap()]);

                if self.allow_warnings {
                    cmd.arg("--allow-warnings");
                }

                if verbose {
                    cmd.arg("--verbose");
                }

                let status = cmd.status().context("Failed to execute pod lib lint")?;

                if !status.success() {
                    bail!("Pod validation failed");
                }
            }
            RegistryType::Official => {
                println!("Publishing to CocoaPods Trunk...");

                let mut cmd = Command::new("pod");
                cmd.current_dir(project_dir);
                cmd.args(["trunk", "push", podspec.to_str().unwrap()]);

                if self.allow_warnings {
                    cmd.arg("--allow-warnings");
                }

                if verbose {
                    cmd.arg("--verbose");
                }

                let status = cmd.status().context("Failed to execute pod trunk push")?;

                if !status.success() {
                    bail!("Pod trunk push failed");
                }
            }
            RegistryType::Private => {
                if let Some(remote_name) = &self.remote_name {
                    println!("Publishing to private specs repo: {}", remote_name);

                    let mut cmd = Command::new("pod");
                    cmd.current_dir(project_dir);
                    cmd.args(["repo", "push", remote_name, podspec.to_str().unwrap()]);

                    if self.allow_warnings {
                        cmd.arg("--allow-warnings");
                    }

                    if verbose {
                        cmd.arg("--verbose");
                    }

                    let status = cmd.status().context("Failed to execute pod repo push")?;

                    if !status.success() {
                        bail!("Pod repo push failed");
                    }
                } else {
                    bail!("Private registry requires --remote-name");
                }
            }
        }

        Ok(())
    }

    fn publish_spm(&self, project_dir: &Path, _verbose: bool) -> Result<()> {
        println!("Publishing to Swift Package Manager...");

        // Verify Package.swift exists
        let package_swift = project_dir.join("Package.swift");
        if !package_swift.exists() {
            bail!("Package.swift not found");
        }

        if self.push {
            println!("Pushing git tag...");

            // Get version from Package.swift or CCGO.toml
            let version = self.get_project_version(project_dir)?;

            // Create and push tag
            let mut cmd = Command::new("git");
            cmd.current_dir(project_dir);
            cmd.args(["tag", &version]);

            let status = cmd.status().context("Failed to create git tag")?;

            if !status.success() {
                eprintln!("Warning: Tag creation failed (may already exist)");
            }

            let mut cmd = Command::new("git");
            cmd.current_dir(project_dir);
            cmd.args(["push", "origin", &version]);

            let status = cmd.status().context("Failed to push git tag")?;

            if !status.success() {
                bail!("Failed to push git tag");
            }

            println!("Pushed tag: {}", version);
        } else {
            println!("Package.swift is ready for SPM (use --push to push git tag)");
        }

        Ok(())
    }

    fn publish_doc(&self, verbose: bool) -> Result<()> {
        println!("=== Publishing Documentation ===\n");

        let cwd = std::env::current_dir().context("Failed to get current directory")?;

        // Check if docs are built
        let docs_dir = cwd.join("site");
        if !docs_dir.exists() {
            println!("Building documentation...");

            let mut cmd = Command::new("mkdocs");
            cmd.current_dir(&cwd);
            cmd.args(["build", "--clean"]);

            if verbose {
                cmd.arg("--verbose");
            }

            let status = cmd.status().context("Failed to build documentation")?;

            if !status.success() {
                bail!("Documentation build failed");
            }
        }

        // Deploy to GitHub Pages
        println!("Deploying to GitHub Pages (branch: {})...", self.doc_branch);

        let mut cmd = Command::new("mkdocs");
        cmd.current_dir(&cwd);
        cmd.args(["gh-deploy", "--branch", &self.doc_branch]);

        if self.doc_force {
            cmd.arg("--force");
        }

        if verbose {
            cmd.arg("--verbose");
        }

        let status = cmd.status().context("Failed to deploy documentation")?;

        if !status.success() {
            bail!("Documentation deployment failed");
        }

        if self.doc_open {
            // Get repository URL and open
            if let Ok(output) = Command::new("git")
                .args(["remote", "get-url", "origin"])
                .output()
            {
                if let Ok(url) = String::from_utf8(output.stdout) {
                    let url = url.trim();
                    // Convert git URL to GitHub pages URL
                    if let Some(pages_url) = self.git_url_to_pages_url(url) {
                        println!("Opening documentation at: {}", pages_url);
                        let _ = open::that(pages_url);
                    }
                }
            }
        }

        println!("\n✅ Documentation published successfully!");
        Ok(())
    }

    fn publish_index(&self, verbose: bool) -> Result<()> {
        println!("=== Publishing to Package Index ===\n");

        let cwd = std::env::current_dir().context("Failed to get current directory")?;

        // Find and load CCGO.toml
        let project_dir = Self::find_project_dir(&cwd)?;
        let config_path = project_dir.join("CCGO.toml");
        let config =
            CcgoConfig::load_from_path(&config_path).context("Failed to load CCGO.toml")?;

        let package = config
            .package
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("No [package] section in CCGO.toml"))?;

        println!("📦 Package: {}", package.name);
        println!(
            "📝 Description: {}",
            package.description.as_deref().unwrap_or("No description")
        );

        // Get Git repository URL
        let git_url = self.get_git_remote_url(&project_dir)?;
        println!("🔗 Repository: {}", git_url);

        // Resolve the (version, tag) pair to publish — one entry per
        // invocation, append-only. Mirrors `pod repo push`.
        let (version, tag) = match (&self.index_version, &self.index_tag) {
            (Some(v), Some(t)) => (v.clone(), t.clone()),
            (Some(v), None) => (v.clone(), default_tag_for_version(v)),
            (None, Some(t)) => (derive_version_from_tag(t), t.clone()),
            (None, None) => bail!(
                "ccgo publish index requires --index-version and/or --index-tag.\n\n\
                 The index is append-only — each invocation publishes exactly \
                 one version. Examples:\n  \
                   ccgo publish index --index-version 1.0.0\n  \
                   ccgo publish index --index-tag v1.0.0\n  \
                   ccgo publish index --index-version 1.0.0 --index-tag custom-prefix-v1.0.0"
            ),
        };

        println!("\n🔖 Publishing single version:");
        println!("   version: {}", version);
        println!("   tag:     {}", tag);

        self.validate_tag_exists(&project_dir, &tag)?;
        let new_version_entry = self.build_single_version_entry(
            &project_dir,
            &package.name,
            &version,
            &tag,
            verbose,
        )?;

        // Determine index repository
        let index_repo = if let Some(repo) = &self.index_repo {
            repo.clone()
        } else {
            bail!("Please specify --index-repo <url> for the index repository");
        };

        let index_name = self
            .index_name
            .clone()
            .unwrap_or_else(|| "custom-index".to_string());

        println!("\n📂 Index repository: {}", index_repo);

        // Clone or update index repository
        let index_path = self.prepare_index_repo(&index_repo, &index_name, verbose)?;

        // Read existing entry (if any), append our new version, sort.
        let package_rel_path = PackageIndex::package_index_path(&package.name);
        let package_file = index_path.join(&package_rel_path);

        if let Some(parent) = package_file.parent() {
            fs::create_dir_all(parent).context("Failed to create package directory")?;
        }

        let existing_versions = Self::read_existing_versions(&package_file)?;
        let merged_versions = merge_version_entry(existing_versions, new_version_entry)?;

        let package_entry = PackageEntry {
            name: package.name.clone(),
            description: package.description.clone().unwrap_or_default(),
            repository: git_url.clone(),
            homepage: package.repository.clone(),
            license: package.license.clone(),
            keywords: Vec::new(),
            platforms: self.get_supported_platforms(&config),
            versions: merged_versions,
        };

        let json = serde_json::to_string_pretty(&package_entry)
            .context("Failed to serialize package entry")?;
        fs::write(&package_file, &json).context("Failed to write package file")?;

        println!("✅ Written: {}", package_rel_path.display());

        // Update index.json metadata
        self.update_index_metadata(&index_path)?;

        // Commit changes
        let commit_message = self.index_message.clone().unwrap_or_else(|| {
            format!(
                "Update {} to {}",
                package.name,
                package_entry
                    .versions
                    .first()
                    .map(|v| v.version.as_str())
                    .unwrap_or("unknown")
            )
        });

        self.commit_index_changes(&index_path, &commit_message, verbose)?;

        // Push if requested
        if self.index_push {
            println!("\n📤 Pushing to remote...");
            self.push_index_changes(&index_path, verbose)?;
            println!("✅ Pushed successfully!");
        } else {
            println!("\n💡 Changes committed locally. Use --index-push to push to remote.");
        }

        println!("\n✅ Package index updated successfully!");
        println!("\n📋 To use this package:");
        println!(
            "   1. Add registry: ccgo registry add {} {}",
            index_name, index_repo
        );
        println!("   2. Add dependency: [dependencies]");
        println!(
            "      {} = \"^{}\"",
            package.name,
            package_entry
                .versions
                .first()
                .map(|v| v.version.as_str())
                .unwrap_or("1.0.0")
        );

        Ok(())
    }

    fn get_git_remote_url(&self, project_dir: &Path) -> Result<String> {
        let output = Command::new("git")
            .current_dir(project_dir)
            .args(["remote", "get-url", "origin"])
            .output()
            .context("Failed to get git remote URL")?;

        if !output.status.success() {
            bail!("No git remote 'origin' found");
        }

        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();

        // Normalize URL (convert SSH to HTTPS for public repos)
        if url.starts_with("git@github.com:") {
            let path = url.trim_start_matches("git@github.com:");
            Ok(format!("https://github.com/{}", path))
        } else {
            Ok(url)
        }
    }

    /// Verify that `tag` exists in the project's local git repo. Catches
    /// typos in `--index-tag` before we go through the rest of the publish
    /// dance and write a phantom tag into the index.
    fn validate_tag_exists(&self, project_dir: &Path, tag: &str) -> Result<()> {
        let refspec = format!("refs/tags/{}", tag);
        let output = Command::new("git")
            .current_dir(project_dir)
            .args(["rev-parse", "--verify", "--quiet", &refspec])
            .output()
            .context("Failed to spawn `git rev-parse`")?;
        if !output.status.success() {
            bail!(
                "git tag '{}' not found in {}. Create the tag locally with \
                 `git tag {}` (and `git push --tags`) before publishing it \
                 to the index.",
                tag,
                project_dir.display(),
                tag
            );
        }
        Ok(())
    }

    /// Build a single VersionEntry for the explicit `(version, tag)` pair.
    /// Populates `archive_url` from `--archive-url-template` and computes
    /// the right checksum (archive-zip when archive_url is set, git-source
    /// tarball as a legacy fallback otherwise).
    fn build_single_version_entry(
        &self,
        project_dir: &Path,
        package_name: &str,
        version: &str,
        tag: &str,
        verbose: bool,
    ) -> Result<VersionEntry> {
        let archive_url = self
            .archive_url_template
            .as_deref()
            .map(|tpl| substitute_archive_url(tpl, package_name, version, tag));
        let archive_format = archive_url.as_ref().map(|_| self.archive_format.clone());

        let checksum = if self.checksum {
            if verbose {
                println!("   Computing checksum for {}...", tag);
            }
            if archive_url.is_some() {
                // Archive-mode: hash the zip consumers will download.
                self.compute_archive_zip_checksum(project_dir, package_name, version)
                    .ok()
            } else {
                // Legacy git-source hash. Informational only — no fetch
                // path verifies against this today.
                self.compute_tag_checksum(project_dir, tag).ok()
            }
        } else {
            None
        };

        Ok(VersionEntry {
            version: version.to_string(),
            tag: tag.to_string(),
            checksum,
            archive_url,
            archive_format,
            released_at: self.get_tag_date(project_dir, tag).ok(),
            yanked: false,
            yanked_reason: None,
        })
    }

    /// Read the existing `versions` array out of `<index>/<sharded>/<name>.json`,
    /// returning `Vec::new()` if the file doesn't exist yet (first publish
    /// of this package). Errors only on read or JSON-parse failures.
    fn read_existing_versions(package_file: &Path) -> Result<Vec<VersionEntry>> {
        if !package_file.exists() {
            return Ok(Vec::new());
        }
        let bytes = std::fs::read_to_string(package_file)
            .with_context(|| format!("failed to read {}", package_file.display()))?;
        let entry: PackageEntry = serde_json::from_str(&bytes).with_context(|| {
            format!(
                "failed to parse existing index entry at {}",
                package_file.display()
            )
        })?;
        Ok(entry.versions)
    }

    /// Compute SHA-256 of the published archive zip for a given version.
    ///
    /// Looks at `target/release/package/<NAME>_CCGO_PACKAGE-<version>.zip`
    /// — the standard output of `ccgo package --release`. This path is
    /// expected to match what was uploaded to the CDN that the index
    /// `archive_url` points at. Hashing the SAME bytes that consumers
    /// will download is what makes `verify_archive_checksum` work.
    ///
    /// Errors when the local zip is absent. Caller swallows the error
    /// via `.ok()` so historical tags (whose zip isn't in `target/`)
    /// just get a `None` checksum — consumers skip verification for
    /// those rather than failing the build.
    fn compute_archive_zip_checksum(
        &self,
        project_dir: &Path,
        package_name: &str,
        version: &str,
    ) -> Result<String> {
        let zip_name = format!(
            "{}_CCGO_PACKAGE-{}.zip",
            package_name.to_uppercase(),
            version
        );
        let zip_path = project_dir
            .join("target")
            .join("release")
            .join("package")
            .join(&zip_name);
        if !zip_path.is_file() {
            anyhow::bail!(
                "expected archive at {} for index checksum; \
                 run `ccgo package --release` first or skip this tag",
                zip_path.display()
            );
        }
        let bytes = std::fs::read(&zip_path)
            .with_context(|| format!("failed to read {}", zip_path.display()))?;
        Ok(format!("sha256:{:x}", Sha256::digest(&bytes)))
    }

    /// Compute SHA-256 checksum for a git tag using git archive
    fn compute_tag_checksum(&self, project_dir: &Path, tag: &str) -> Result<String> {
        // Use git archive to create a reproducible tarball and compute its hash
        let mut child = Command::new("git")
            .current_dir(project_dir)
            .args(["archive", "--format=tar.gz", tag])
            .stdout(Stdio::piped())
            .spawn()
            .context("Failed to run git archive")?;

        let stdout = child
            .stdout
            .take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture git archive output"))?;

        // Compute SHA-256 hash
        let mut hasher = Sha256::new();
        let mut reader = std::io::BufReader::new(stdout);
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = reader
                .read(&mut buffer)
                .context("Failed to read git archive output")?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let status = child.wait().context("Failed to wait for git archive")?;

        if !status.success() {
            bail!("git archive failed for tag {}", tag);
        }

        let hash = hasher.finalize();
        Ok(format!("sha256:{:x}", hash))
    }

    fn get_tag_date(&self, project_dir: &Path, tag: &str) -> Result<String> {
        let output = Command::new("git")
            .current_dir(project_dir)
            .args(["log", "-1", "--format=%aI", tag])
            .output()
            .context("Failed to get tag date")?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            bail!("Failed to get tag date")
        }
    }

    fn get_supported_platforms(&self, config: &CcgoConfig) -> Vec<String> {
        // Derive platforms from which platform configs are present
        let mut platforms = Vec::new();

        if let Some(ref platform_configs) = config.platforms {
            if platform_configs.android.is_some() {
                platforms.push("android".to_string());
            }
            if platform_configs.ios.is_some() {
                platforms.push("ios".to_string());
            }
            if platform_configs.macos.is_some() {
                platforms.push("macos".to_string());
            }
            if platform_configs.windows.is_some() {
                platforms.push("windows".to_string());
            }
            if platform_configs.linux.is_some() {
                platforms.push("linux".to_string());
            }
            if platform_configs.ohos.is_some() {
                platforms.push("ohos".to_string());
            }
        }

        // Default to common platforms if none specified
        if platforms.is_empty() {
            platforms = vec![
                "android".to_string(),
                "ios".to_string(),
                "macos".to_string(),
                "linux".to_string(),
                "windows".to_string(),
            ];
        }

        platforms
    }

    fn prepare_index_repo(&self, repo_url: &str, name: &str, verbose: bool) -> Result<PathBuf> {
        let ccgo_home = PackageIndex::new().ccgo_home_path();
        let index_work_dir = ccgo_home.join("registry").join("publish").join(name);

        if index_work_dir.exists() {
            // Pull latest changes
            println!("📥 Updating existing index clone...");
            let mut cmd = Command::new("git");
            cmd.current_dir(&index_work_dir);
            cmd.args(["pull", "--rebase"]);

            if !verbose {
                cmd.stdout(std::process::Stdio::null());
                cmd.stderr(std::process::Stdio::null());
            }

            let status = cmd.status().context("Failed to pull index repository")?;

            if !status.success() {
                // Try fresh clone if pull fails
                println!("⚠️  Pull failed, re-cloning...");
                fs::remove_dir_all(&index_work_dir)?;
                return self.prepare_index_repo(repo_url, name, verbose);
            }
        } else {
            // Clone the repository
            println!("📥 Cloning index repository...");
            fs::create_dir_all(index_work_dir.parent().unwrap())?;

            let mut cmd = Command::new("git");
            cmd.args([
                "clone",
                "--depth",
                "1",
                repo_url,
                index_work_dir.to_str().unwrap(),
            ]);

            if !verbose {
                cmd.stdout(std::process::Stdio::null());
                cmd.stderr(std::process::Stdio::null());
            }

            let status = cmd.status().context("Failed to clone index repository")?;

            if !status.success() {
                // Maybe it's a new repo, try to initialize
                println!("📝 Initializing new index repository...");
                fs::create_dir_all(&index_work_dir)?;

                Command::new("git")
                    .current_dir(&index_work_dir)
                    .args(["init"])
                    .status()
                    .context("Failed to init git repository")?;

                Command::new("git")
                    .current_dir(&index_work_dir)
                    .args(["remote", "add", "origin", repo_url])
                    .status()
                    .context("Failed to add git remote")?;

                // Create initial index.json
                let metadata = IndexMetadata {
                    version: 1,
                    name: name.to_string(),
                    description: format!("{} package index", name),
                    homepage: None,
                    package_count: 0,
                    updated_at: chrono::Utc::now().to_rfc3339(),
                };
                let json = serde_json::to_string_pretty(&metadata)?;
                fs::write(index_work_dir.join("index.json"), json)?;
            }
        }

        Ok(index_work_dir)
    }

    fn update_index_metadata(&self, index_path: &Path) -> Result<()> {
        let metadata_path = index_path.join("index.json");

        let mut metadata: IndexMetadata = if metadata_path.exists() {
            let content = fs::read_to_string(&metadata_path)?;
            serde_json::from_str(&content).unwrap_or_else(|_| IndexMetadata {
                version: 1,
                name: "ccgo-packages".to_string(),
                description: "Package index".to_string(),
                homepage: None,
                package_count: 0,
                updated_at: String::new(),
            })
        } else {
            IndexMetadata {
                version: 1,
                name: self
                    .index_name
                    .clone()
                    .unwrap_or_else(|| "ccgo-packages".to_string()),
                description: "Package index".to_string(),
                homepage: None,
                package_count: 0,
                updated_at: String::new(),
            }
        };

        // Count packages
        let mut count = 0;
        for entry in walkdir::WalkDir::new(index_path)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.path().extension().and_then(|s| s.to_str()) == Some("json")
                && entry.file_name() != "index.json"
            {
                count += 1;
            }
        }

        metadata.package_count = count;
        metadata.updated_at = chrono::Utc::now().to_rfc3339();

        let json = serde_json::to_string_pretty(&metadata)?;
        fs::write(metadata_path, json)?;

        println!("📊 Index metadata updated: {} package(s)", count);

        Ok(())
    }

    fn commit_index_changes(&self, index_path: &Path, message: &str, verbose: bool) -> Result<()> {
        // Add all changes
        let mut cmd = Command::new("git");
        cmd.current_dir(index_path);
        cmd.args(["add", "-A"]);

        if !verbose {
            cmd.stdout(std::process::Stdio::null());
        }

        cmd.status().context("Failed to stage changes")?;

        // Check if there are changes to commit
        let output = Command::new("git")
            .current_dir(index_path)
            .args(["status", "--porcelain"])
            .output()
            .context("Failed to check git status")?;

        if output.stdout.is_empty() {
            println!("ℹ️  No changes to commit");
            return Ok(());
        }

        // Commit
        let mut cmd = Command::new("git");
        cmd.current_dir(index_path);
        cmd.args(["commit", "-m", message]);

        if !verbose {
            cmd.stdout(std::process::Stdio::null());
        }

        let status = cmd.status().context("Failed to commit changes")?;

        if status.success() {
            println!("✅ Committed: {}", message);
        }

        Ok(())
    }

    fn push_index_changes(&self, index_path: &Path, verbose: bool) -> Result<()> {
        let mut cmd = Command::new("git");
        cmd.current_dir(index_path);
        cmd.args(["push", "origin", "HEAD"]);

        if !verbose {
            cmd.stderr(std::process::Stdio::null());
        }

        let status = cmd.status().context("Failed to push changes")?;

        if !status.success() {
            bail!("Failed to push to remote");
        }

        Ok(())
    }

    // Helper functions

    fn find_project_dir(current_dir: &Path) -> Result<PathBuf> {
        // Check subdirectories first
        if let Ok(entries) = fs::read_dir(current_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let config_path = entry.path().join("CCGO.toml");
                        if config_path.exists() {
                            return Ok(entry.path());
                        }
                    }
                }
            }
        }

        // Check current directory
        let config_path = current_dir.join("CCGO.toml");
        if config_path.exists() {
            return Ok(current_dir.to_path_buf());
        }

        bail!("CCGO.toml not found in project directory");
    }

    fn find_file_with_extension(&self, dir: &Path, extension: &str) -> Result<PathBuf> {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str())
                    == Some(extension.trim_start_matches('.'))
                {
                    return Ok(path);
                }
            }
        }
        bail!("No {} file found", extension);
    }

    fn get_project_version(&self, project_dir: &Path) -> Result<String> {
        let toml_path = project_dir.join("CCGO.toml");
        let content = fs::read_to_string(toml_path).context("Failed to read CCGO.toml")?;

        // Parse version from TOML
        for line in content.lines() {
            if line.trim().starts_with("version") {
                if let Some(version) = line.split('=').nth(1) {
                    return Ok(version.trim().trim_matches('"').to_string());
                }
            }
        }

        bail!("Version not found in CCGO.toml");
    }

    fn get_conan_remotes(&self) -> Result<Vec<String>> {
        let output = Command::new("conan")
            .args(["remote", "list"])
            .output()
            .context("Failed to execute conan remote list")?;

        if !output.status.success() {
            bail!("Failed to get Conan remotes");
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let remotes: Vec<String> = stdout
            .lines()
            .filter_map(|line| {
                if line.contains(':') {
                    let parts: Vec<&str> = line.split(':').collect();
                    if !parts.is_empty() {
                        let name = parts[0].trim();
                        if !name.is_empty() && !name.starts_with('#') && name != "conancenter" {
                            return Some(name.to_string());
                        }
                    }
                }
                None
            })
            .collect();

        Ok(remotes)
    }

    fn git_url_to_pages_url(&self, git_url: &str) -> Option<String> {
        // Convert git@github.com:user/repo.git to https://user.github.io/repo/
        if git_url.starts_with("git@github.com:") {
            let parts: Vec<&str> = git_url
                .trim_end_matches(".git")
                .trim_start_matches("git@github.com:")
                .split('/')
                .collect();
            if parts.len() == 2 {
                return Some(format!("https://{}.github.io/{}/", parts[0], parts[1]));
            }
        }
        // Convert https://github.com/user/repo.git to https://user.github.io/repo/
        else if git_url.starts_with("https://github.com/") {
            let parts: Vec<&str> = git_url
                .trim_end_matches(".git")
                .trim_start_matches("https://github.com/")
                .split('/')
                .collect();
            if parts.len() == 2 {
                return Some(format!("https://{}.github.io/{}/", parts[0], parts[1]));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitute_archive_url_replaces_all_placeholders() {
        let template = "https://cdn.example.com/{name}/{name}_CCGO_PACKAGE-{version}.zip";
        let result = substitute_archive_url(template, "stdcomm", "1.0.0", "v1.0.0");
        assert_eq!(
            result,
            "https://cdn.example.com/stdcomm/stdcomm_CCGO_PACKAGE-1.0.0.zip"
        );
    }

    #[test]
    fn substitute_archive_url_with_tag_placeholder() {
        let template =
            "https://gh.example.com/{name}/releases/download/{tag}/{name}-{version}.tar.gz";
        let result = substitute_archive_url(template, "leaf", "1.0.0", "v1.0.0");
        assert_eq!(
            result,
            "https://gh.example.com/leaf/releases/download/v1.0.0/leaf-1.0.0.tar.gz"
        );
    }

    #[test]
    fn compute_archive_zip_checksum_hashes_local_package_zip() {
        // Regression: --checksum + --archive-url-template must hash the
        // SAME bytes consumers will download (the published zip), not the
        // git source tarball. The fix is verified by writing a synthetic
        // zip at the standard target/release/package/<NAME>_CCGO_PACKAGE-<ver>.zip
        // path and asserting the function returns the sha256 of those bytes.
        let tmp = tempfile::TempDir::new().unwrap();
        let project_dir = tmp.path();
        let pkg_dir = project_dir.join("target/release/package");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        let payload = b"synthetic-zip-bytes-for-checksum-test";
        std::fs::write(
            pkg_dir.join("STDCOMM_CCGO_PACKAGE-1.0.0.zip"),
            payload,
        )
        .unwrap();

        let cmd = PublishCommand {
            target: PublishTarget::Index,
            registry: RegistryType::Local,
            url: None,
            remote_name: None,
            skip_build: false,
            yes: true,
            manager: AppleManager::All,
            push: false,
            platform: None,
            allow_warnings: true,
            profile: "default".into(),
            link_type: "both".into(),
            doc_branch: "gh-pages".into(),
            doc_force: false,
            doc_open: false,
            index_repo: None,
            index_name: None,
            index_push: false,
            index_message: None,
            checksum: true,
            archive_url_template: None,
            archive_format: "zip".into(),
            index_version: None,
            index_tag: None,
        };

        let got = cmd
            .compute_archive_zip_checksum(project_dir, "stdcomm", "1.0.0")
            .expect("local zip should be hashable");

        let expected = format!("sha256:{:x}", Sha256::digest(payload));
        assert_eq!(got, expected);
    }

    #[test]
    fn compute_archive_zip_checksum_errors_when_local_zip_missing() {
        let tmp = tempfile::TempDir::new().unwrap();

        let cmd = PublishCommand {
            target: PublishTarget::Index,
            registry: RegistryType::Local,
            url: None,
            remote_name: None,
            skip_build: false,
            yes: true,
            manager: AppleManager::All,
            push: false,
            platform: None,
            allow_warnings: true,
            profile: "default".into(),
            link_type: "both".into(),
            doc_branch: "gh-pages".into(),
            doc_force: false,
            doc_open: false,
            index_repo: None,
            index_name: None,
            index_push: false,
            index_message: None,
            checksum: true,
            archive_url_template: None,
            archive_format: "zip".into(),
            index_version: None,
            index_tag: None,
        };

        let err = cmd
            .compute_archive_zip_checksum(tmp.path(), "stdcomm", "9.9.9")
            .expect_err("missing local zip should error");
        let msg = err.to_string();
        assert!(
            msg.contains("STDCOMM_CCGO_PACKAGE-9.9.9.zip"),
            "expected expected-path in error, got: {msg}"
        );
        assert!(
            msg.contains("ccgo package --release"),
            "expected reproduction hint in error, got: {msg}"
        );
    }

    fn entry(version: &str, tag: &str) -> VersionEntry {
        VersionEntry {
            version: version.into(),
            tag: tag.into(),
            checksum: None,
            archive_url: None,
            archive_format: None,
            released_at: None,
            yanked: false,
            yanked_reason: None,
        }
    }

    #[test]
    fn derive_version_strips_v_prefix() {
        assert_eq!(derive_version_from_tag("v1.0.0"), "1.0.0");
        assert_eq!(derive_version_from_tag("V25.2.9519653"), "25.2.9519653");
    }

    #[test]
    fn derive_version_keeps_tag_without_v_prefix_unchanged() {
        assert_eq!(derive_version_from_tag("1.0.0"), "1.0.0");
    }

    #[test]
    fn derive_version_does_not_touch_tags_with_other_prefixes() {
        // We don't auto-strip random prefixes; if you want to publish a
        // tag like `release-1.0.0`, pass --index-version explicitly.
        assert_eq!(derive_version_from_tag("release-1.0.0"), "release-1.0.0");
    }

    #[test]
    fn default_tag_prepends_v() {
        assert_eq!(default_tag_for_version("1.0.0"), "v1.0.0");
        assert_eq!(
            default_tag_for_version("25.2.9519653"),
            "v25.2.9519653"
        );
    }

    #[test]
    fn merge_appends_to_empty_existing_versions() {
        let merged = merge_version_entry(Vec::new(), entry("1.0.0", "v1.0.0")).unwrap();
        assert_eq!(merged.len(), 1);
        assert_eq!(merged[0].version, "1.0.0");
    }

    #[test]
    fn merge_appends_to_existing_and_sorts_descending() {
        let existing = vec![entry("1.0.0", "v1.0.0"), entry("0.9.0", "v0.9.0")];
        let merged = merge_version_entry(existing, entry("2.0.0", "v2.0.0")).unwrap();
        assert_eq!(merged.len(), 3);
        assert_eq!(merged[0].version, "2.0.0");
        assert_eq!(merged[1].version, "1.0.0");
        assert_eq!(merged[2].version, "0.9.0");
    }

    #[test]
    fn merge_rejects_duplicate_version() {
        let existing = vec![entry("1.0.0", "v1.0.0")];
        let err = merge_version_entry(existing, entry("1.0.0", "v1.0.0")).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("'1.0.0'") && msg.contains("already in the index"),
            "expected duplicate-version error, got: {msg}"
        );
    }
}
