//! Publish command implementation

use anyhow::{Context, Result, bail};
use clap::{Args, ValueEnum};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;

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
