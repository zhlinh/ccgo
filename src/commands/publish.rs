//! Publish command implementation

use anyhow::{Context, Result, bail};
use clap::{Args, ValueEnum};
use sha2::{Sha256, Digest};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::fs;

use crate::config::CcgoConfig;
use crate::registry::{PackageIndex, PackageEntry, VersionEntry, IndexMetadata};

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

    /// Generate SHA-256 checksums for each version (uses git archive)
    #[arg(long)]
    pub checksum: bool,
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

        let cwd = std::env::current_dir()
            .context("Failed to get current directory")?;
        let android_dir = cwd.join("android");

        if !android_dir.exists() || !android_dir.is_dir() {
            bail!("Error: android directory not found at {}", android_dir.display());
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

        let status = cmd.status()
            .context("Failed to execute gradlew")?;

        if !status.success() {
            bail!("Publish failed with exit code: {}", status.code().unwrap_or(-1));
        }

        println!("\n✅ Publish completed successfully!");
        Ok(())
    }

    fn publish_ohos(&self, verbose: bool) -> Result<()> {
        println!("=== Publishing OHOS to OHPM ===\n");

        let cwd = std::env::current_dir()
            .context("Failed to get current directory")?;
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

        let status = cmd.status()
            .context("Failed to execute ohpm")?;

        if !status.success() {
            bail!("Publish failed with exit code: {}", status.code().unwrap_or(-1));
        }

        println!("\n✅ Publish completed successfully!");
        Ok(())
    }

    fn publish_kmp(&self, verbose: bool) -> Result<()> {
        println!("=== Publishing KMP to Maven ===\n");

        let cwd = std::env::current_dir()
            .context("Failed to get current directory")?;
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

        let status = cmd.status()
            .context("Failed to execute gradlew")?;

        if !status.success() {
            bail!("Publish failed with exit code: {}", status.code().unwrap_or(-1));
        }

        println!("\n✅ Publish completed successfully!");
        Ok(())
    }

    fn publish_conan(&self, verbose: bool) -> Result<()> {
        println!("=== Publishing to Conan ===\n");

        let cwd = std::env::current_dir()
            .context("Failed to get current directory")?;

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

                let status = cmd.status()
                    .context("Failed to execute conan export")?;

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

                let status = cmd.status()
                    .context("Failed to execute conan export")?;

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

                let status = cmd.status()
                    .context("Failed to execute conan upload")?;

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

        let cwd = std::env::current_dir()
            .context("Failed to get current directory")?;

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

                let status = cmd.status()
                    .context("Failed to execute pod lib lint")?;

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

                let status = cmd.status()
                    .context("Failed to execute pod trunk push")?;

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

                    let status = cmd.status()
                        .context("Failed to execute pod repo push")?;

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

            let status = cmd.status()
                .context("Failed to create git tag")?;

            if !status.success() {
                eprintln!("Warning: Tag creation failed (may already exist)");
            }

            let mut cmd = Command::new("git");
            cmd.current_dir(project_dir);
            cmd.args(["push", "origin", &version]);

            let status = cmd.status()
                .context("Failed to push git tag")?;

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

        let cwd = std::env::current_dir()
            .context("Failed to get current directory")?;

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

            let status = cmd.status()
                .context("Failed to build documentation")?;

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

        let status = cmd.status()
            .context("Failed to deploy documentation")?;

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

        let cwd = std::env::current_dir()
            .context("Failed to get current directory")?;

        // Find and load CCGO.toml
        let project_dir = Self::find_project_dir(&cwd)?;
        let config_path = project_dir.join("CCGO.toml");
        let config = CcgoConfig::load_from_path(&config_path)
            .context("Failed to load CCGO.toml")?;

        let package = config.package.as_ref()
            .ok_or_else(|| anyhow::anyhow!("No [package] section in CCGO.toml"))?;

        println!("📦 Package: {}", package.name);
        println!("📝 Description: {}", package.description.as_deref().unwrap_or("No description"));

        // Get Git repository URL
        let git_url = self.get_git_remote_url(&project_dir)?;
        println!("🔗 Repository: {}", git_url);

        // Discover versions from Git tags
        println!("\n🔍 Discovering versions from Git tags...");
        if self.checksum {
            println!("   (computing SHA-256 checksums)");
        }
        let versions = self.discover_git_versions(&project_dir, self.checksum, verbose)?;

        if versions.is_empty() {
            bail!("No version tags found. Create tags with: git tag v1.0.0");
        }

        println!("   Found {} version(s):", versions.len());
        for v in versions.iter().take(5) {
            println!("   - {}", v.version);
        }
        if versions.len() > 5 {
            println!("   ... and {} more", versions.len() - 5);
        }

        // Create package entry
        let package_entry = PackageEntry {
            name: package.name.clone(),
            description: package.description.clone().unwrap_or_default(),
            repository: git_url.clone(),
            homepage: package.repository.clone(), // Use repository as homepage
            license: package.license.clone(),
            keywords: Vec::new(), // PackageConfig doesn't have keywords
            platforms: self.get_supported_platforms(&config),
            versions,
        };

        // Determine index repository
        let index_repo = if let Some(repo) = &self.index_repo {
            repo.clone()
        } else {
            // Use default or ask user
            bail!("Please specify --index-repo <url> for the index repository");
        };

        let index_name = self.index_name.clone()
            .unwrap_or_else(|| "custom-index".to_string());

        println!("\n📂 Index repository: {}", index_repo);

        // Clone or update index repository
        let index_path = self.prepare_index_repo(&index_repo, &index_name, verbose)?;

        // Write package JSON
        let package_rel_path = PackageIndex::package_index_path(&package.name);
        let package_file = index_path.join(&package_rel_path);

        // Create parent directories
        if let Some(parent) = package_file.parent() {
            fs::create_dir_all(parent)
                .context("Failed to create package directory")?;
        }

        // Write package entry
        let json = serde_json::to_string_pretty(&package_entry)
            .context("Failed to serialize package entry")?;
        fs::write(&package_file, &json)
            .context("Failed to write package file")?;

        println!("✅ Written: {}", package_rel_path.display());

        // Update index.json metadata
        self.update_index_metadata(&index_path)?;

        // Commit changes
        let commit_message = self.index_message.clone()
            .unwrap_or_else(|| format!("Update {} to {}",
                package.name,
                package_entry.versions.first().map(|v| v.version.as_str()).unwrap_or("unknown")));

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
        println!("   1. Add registry: ccgo registry add {} {}", index_name, index_repo);
        println!("   2. Add dependency: [dependencies]");
        println!("      {} = \"^{}\"", package.name,
            package_entry.versions.first().map(|v| v.version.as_str()).unwrap_or("1.0.0"));

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

    fn discover_git_versions(&self, project_dir: &Path, compute_checksum: bool, verbose: bool) -> Result<Vec<VersionEntry>> {
        let output = Command::new("git")
            .current_dir(project_dir)
            .args(["tag", "-l", "--sort=-v:refname"])
            .output()
            .context("Failed to list git tags")?;

        if !output.status.success() {
            bail!("Failed to list git tags");
        }

        let tags_str = String::from_utf8_lossy(&output.stdout);
        let mut versions = Vec::new();

        for line in tags_str.lines() {
            let tag = line.trim();
            if tag.is_empty() {
                continue;
            }

            // Parse version from tag (strip 'v' prefix if present)
            let version = if tag.starts_with('v') || tag.starts_with('V') {
                &tag[1..]
            } else {
                tag
            };

            // Validate it looks like a semver
            if crate::registry::SemVer::parse(version).is_some() {
                // Compute checksum if requested
                let checksum = if compute_checksum {
                    if verbose {
                        println!("   Computing checksum for {}...", tag);
                    }
                    self.compute_tag_checksum(project_dir, tag).ok()
                } else {
                    None
                };

                versions.push(VersionEntry {
                    version: version.to_string(),
                    tag: tag.to_string(),
                    checksum,
                    released_at: self.get_tag_date(project_dir, tag).ok(),
                    yanked: false,
                    yanked_reason: None,
                });
            }
        }

        Ok(versions)
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

        let stdout = child.stdout.take()
            .ok_or_else(|| anyhow::anyhow!("Failed to capture git archive output"))?;

        // Compute SHA-256 hash
        let mut hasher = Sha256::new();
        let mut reader = std::io::BufReader::new(stdout);
        let mut buffer = [0u8; 8192];

        loop {
            let bytes_read = reader.read(&mut buffer)
                .context("Failed to read git archive output")?;
            if bytes_read == 0 {
                break;
            }
            hasher.update(&buffer[..bytes_read]);
        }

        let status = child.wait()
            .context("Failed to wait for git archive")?;

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

            let status = cmd.status()
                .context("Failed to pull index repository")?;

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
            cmd.args(["clone", "--depth", "1", repo_url, index_work_dir.to_str().unwrap()]);

            if !verbose {
                cmd.stdout(std::process::Stdio::null());
                cmd.stderr(std::process::Stdio::null());
            }

            let status = cmd.status()
                .context("Failed to clone index repository")?;

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
                name: self.index_name.clone().unwrap_or_else(|| "ccgo-packages".to_string()),
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
                if path.extension().and_then(|s| s.to_str()) == Some(extension.trim_start_matches('.')) {
                    return Ok(path);
                }
            }
        }
        bail!("No {} file found", extension);
    }

    fn get_project_version(&self, project_dir: &Path) -> Result<String> {
        let toml_path = project_dir.join("CCGO.toml");
        let content = fs::read_to_string(toml_path)
            .context("Failed to read CCGO.toml")?;

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
            let parts: Vec<&str> = git_url.trim_end_matches(".git")
                .trim_start_matches("git@github.com:")
                .split('/')
                .collect();
            if parts.len() == 2 {
                return Some(format!("https://{}.github.io/{}/", parts[0], parts[1]));
            }
        }
        // Convert https://github.com/user/repo.git to https://user.github.io/repo/
        else if git_url.starts_with("https://github.com/") {
            let parts: Vec<&str> = git_url.trim_end_matches(".git")
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
