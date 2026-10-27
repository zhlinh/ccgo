//! Publish command implementation

use anyhow::{bail, Context, Result};
use clap::{Args, ValueEnum};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use crate::config::CcgoConfig;
use crate::registry::{PackageEntry, PackageIndex, VersionEntry};

#[path = "publish_input.rs"]
mod input;

#[path = "publish_kmp_sdk.rs"]
mod kmp_sdk;

#[path = "publish_apple_artifact.rs"]
mod apple_artifact;

#[path = "publish_artifact.rs"]
mod artifact;

#[path = "publish_doc_artifact.rs"]
mod doc_artifact;

#[path = "publish_conan.rs"]
mod conan;

#[path = "publish_index_destination.rs"]
mod index_destination;

#[path = "publish_index_recovery.rs"]
mod index_recovery;

#[derive(Debug, Clone, ValueEnum)]
pub enum ArtifactProvider {
    Maven,
    Github,
}

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
    /// CCGO package (registry; index is a compatibility alias)
    #[value(name = "package", alias = "index")]
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
            PublishTarget::Index => write!(f, "package"),
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

    /// Private destination: Maven/OHPM URL, or configured CocoaPods/Conan/SPM/Index repository name
    #[arg(long, visible_aliases = ["url", "remote-name"], value_name = "URL_OR_NAME")]
    pub remote: Option<String>,

    /// Skip build step
    #[arg(long)]
    pub skip_build: bool,

    /// Publish from a project/SDK directory, platform file, or supported ZIP.
    /// ZIP inputs are supported for Android, OHOS, KMP, Conan, Doc, and Apple with --artifact-provider.
    /// Relative paths are resolved from the current working directory.
    #[arg(long, value_name = "PATH")]
    pub path: Option<PathBuf>,

    /// Upload an Apple binary, Index package or documentation ZIP
    #[arg(long, value_enum)]
    pub artifact_provider: Option<ArtifactProvider>,

    /// Maven groupId for ZIP artifacts
    #[arg(long, requires = "artifact_provider")]
    pub artifact_group: Option<String>,

    /// Maven artifactId (default: <project>-apple, <project>-ccgo or <project>-doc)
    #[arg(long, requires = "artifact_provider")]
    pub artifact_id: Option<String>,

    /// GitHub owner/repository for release assets
    #[arg(long, requires = "artifact_provider")]
    pub github_repo: Option<String>,

    /// Publish a release build instead of debug
    ///
    /// Mirrors `ccgo build --release`. Publishing runs Gradle, and the Gradle
    /// plugin calls `ccgo build ... --native-only` back: without this the
    /// nested build defaults to debug and the published artifact carries
    /// `ccgo_build/debug` libraries.
    #[arg(long)]
    pub release: bool,

    /// C++ runtime to publish against (Android only).
    ///
    /// Mirrors `ccgo build android --stl`. The Gradle plugin derives the
    /// `-stdembed` artifact name from this, so publishing has to state it too:
    /// without it the plugin falls back to `[android].stl` in CCGO.toml and a
    /// c++_static build gets published under the plain, shared-runtime name.
    #[arg(long, value_enum)]
    pub stl: Option<crate::commands::build::StlKind>,

    /// Skip confirmation prompts
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Package manager (for apple)
    #[arg(long, value_enum, default_value_t = AppleManager::All)]
    pub manager: AppleManager,

    /// Publish SPM/Index state, or upload a Doc artifact (otherwise validate only)
    #[arg(long, visible_alias = "index-push")]
    pub push: bool,

    /// Apple platforms (for apple)
    #[arg(long)]
    pub platform: Option<String>,

    /// Allow warnings (for CocoaPods)
    #[arg(long, default_value_t = true)]
    pub allow_warnings: bool,

    #[command(flatten)]
    pub conan: conan::ConanOptions,

    /// Conan host profiles (comma-separated matrix)
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
    /// Local index directory (with --registry local)
    #[arg(long)]
    pub index_repo: Option<String>,

    /// Local index name (default: local; remote indexes use --remote)
    #[arg(long)]
    pub index_name: Option<String>,

    /// Commit message for index update
    #[arg(long)]
    pub index_message: Option<String>,

    /// Release version for SPM/Index (default: project/package version in CCGO.toml)
    #[arg(long, visible_alias = "index-version")]
    pub version: Option<String>,

    /// Release Git tag for SPM/Index (default: v<version>)
    #[arg(long, visible_alias = "index-tag")]
    pub tag: Option<String>,

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
/// string. Used by `--tag` when `--version` is omitted.
fn derive_version_from_tag(tag: &str) -> String {
    tag.strip_prefix('v')
        .or_else(|| tag.strip_prefix('V'))
        .unwrap_or(tag)
        .to_string()
}

/// Default tag form `v<version>`. Used by `--version` when
/// `--tag` is omitted. Most projects follow this convention; pass
/// `--tag` explicitly when yours doesn't.
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
             append-only by design. Publish a new version to change its content.",
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
    pub fn execute(mut self, verbose: bool) -> Result<()> {
        self.validate_version_options()?;
        if matches!(self.target, PublishTarget::Apple)
            && matches!(self.manager, AppleManager::Spm | AppleManager::All)
        {
            self.validate_spm_registry()?;
        }
        if self.artifact_provider.is_some() {
            if !matches!(
                self.target,
                PublishTarget::Apple | PublishTarget::Index | PublishTarget::Doc
            ) {
                bail!("--artifact-provider is supported only for apple, index and doc");
            }
            if matches!(self.target, PublishTarget::Doc) {
                let _temporary_input = self.prepare_publish_path()?;
                return doc_artifact::publish(&self, verbose);
            }
            if matches!(self.target, PublishTarget::Apple) {
                return apple_artifact::publish(&self, verbose);
            }
            if self.archive_url_template.is_some() {
                bail!("--artifact-provider generates the archive URL; omit --archive-url-template");
            }
            if self.archive_format != "zip" {
                bail!("Index artifact providers support ZIP archives only");
            }
        }
        let temporary_input = self.prepare_publish_path()?;
        println!("Publishing library project...\n");

        self.publish_target(verbose, temporary_input.is_some())
    }

    fn publish_target(&self, verbose: bool, temporary_input: bool) -> Result<()> {
        match self.target {
            PublishTarget::Android => self.publish_android(verbose),
            PublishTarget::Ohos => self.publish_ohos(verbose),
            PublishTarget::Kmp => self.publish_kmp(verbose),
            PublishTarget::Conan => self.publish_conan(verbose),
            PublishTarget::Apple => self.publish_apple(verbose),
            PublishTarget::Doc => self.publish_doc(verbose, temporary_input),
            PublishTarget::Index => self.publish_index(verbose),
        }
    }

    fn validate_version_options(&self) -> Result<()> {
        if (self.version.is_some() || self.tag.is_some())
            && !(matches!(self.target, PublishTarget::Doc) && self.artifact_provider.is_some())
            && !matches!(self.target, PublishTarget::Index)
            && !(matches!(self.target, PublishTarget::Apple)
                && (matches!(self.manager, AppleManager::Spm) || self.artifact_provider.is_some()))
        {
            bail!("--version/--tag are supported for index, apple --manager spm, or Apple/Doc artifact hosting");
        }
        Ok(())
    }

    fn accepts_publish_file(&self, path: &Path, extension: &str) -> bool {
        match self.target {
            PublishTarget::Android => matches!(extension, "aar" | "zip"),
            PublishTarget::Ohos => matches!(extension, "har" | "zip"),
            PublishTarget::Kmp | PublishTarget::Index => extension == "zip",
            PublishTarget::Apple => match self.manager {
                AppleManager::Cocoapods => extension == "podspec",
                AppleManager::Spm => {
                    path.file_name().and_then(|s| s.to_str()) == Some("Package.swift")
                }
                AppleManager::All => false,
            },
            PublishTarget::Conan => matches!(extension, "py" | "zip"),
            PublishTarget::Doc => matches!(extension, "yml" | "yaml" | "zip"),
        }
    }

    fn configure_android_runtime(&self, cmd: &mut Command, android_dir: &Path) {
        // Hand the resolved runtime to Gradle, same as the android builder does.
        // The plugin reads CCGO_ANDROID_STL first, then [android].stl from
        // CCGO.toml; it decides the `-stdembed` artifact name. Publishing a
        // c++_static build without this yields a package whose name says shared
        // while its .so embeds the runtime -- worse than either choice alone.
        let config = self.android_publish_config(android_dir);
        let stl = crate::builder::resolve_stl(
            self.stl.as_ref().map(|s| s.as_str()),
            config
                .as_ref()
                .and_then(|c| c.android.as_ref())
                .and_then(|a| a.stl.as_deref()),
        );
        cmd.env("CCGO_ANDROID_STL", &stl);
        println!("  C++ runtime: {}", stl);

        let build_type = if self.release { "release" } else { "debug" };
        cmd.env("CCGO_BUILD_TYPE", build_type);
        cmd.arg(format!("-PccgoBuildType={build_type}"));
        println!("  Build type:  {}", build_type);
    }

    fn open_published_documentation(&self, project_dir: &Path) {
        if self.doc_open {
            // Get repository URL and open
            if let Ok(output) = Command::new("git")
                .current_dir(project_dir)
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
    }

    fn validate_publication_worktree(
        work_path: &Path,
        destination: &index_destination::Destination,
    ) -> Result<()> {
        if work_path.join(".git/ccgo-yank.json").exists() {
            bail!("An interrupted yank is pending; retry that yank command before publishing");
        }
        if !destination.local && work_path.join(".git").exists() {
            crate::registry::index_writer::verify_origin(work_path, &destination.repository)?;
        }
        Ok(())
    }

    fn validate_publish_path(&self, path: &Path) -> Result<()> {
        if !path.exists() {
            bail!("Publish path does not exist: {}", path.display());
        }
        if matches!(self.target, PublishTarget::Index) {
            if self.artifact_provider.is_none()
                && (!self.checksum || self.archive_url_template.is_none())
            {
                bail!("--path for index requires --checksum and --archive-url-template; the archive must be hosted separately");
            }
            if self.archive_format != "zip" {
                bail!("--path for index currently supports --archive-format zip only");
            }
        }
        if path.is_dir() {
            return Ok(());
        }
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
        if matches!(self.target, PublishTarget::Apple) && extension == "zip" {
            bail!("Apple ZIP input requires --artifact-provider maven or github");
        }
        let valid = self.accepts_publish_file(path, extension);
        if !path.is_file() || !valid {
            bail!("Unsupported --path input for {}: {}. Use a project/SDK directory or a platform-specific file (apple files require the matching --manager)", self.target, path.display());
        }
        Ok(())
    }

    fn prepare_publish_path(&mut self) -> Result<Option<tempfile::TempDir>> {
        let Some(path) = self.path.as_deref() else {
            return Ok(None);
        };
        self.validate_publish_path(path)?;
        let mut path = fs::canonicalize(path)
            .with_context(|| format!("Failed to resolve publish path {}", path.display()))?;
        let mut temporary = None;
        if matches!(
            self.target,
            PublishTarget::Android
                | PublishTarget::Ohos
                | PublishTarget::Conan
                | PublishTarget::Doc
        ) && path.is_file()
            && path.extension().and_then(|s| s.to_str()) == Some("zip")
        {
            let directory = tempfile::tempdir()?;
            input::extract_zip(&path, directory.path())?;
            path = directory.path().to_path_buf();
            temporary = Some(directory);
        }
        if path.is_dir() {
            path = self.resolve_publish_directory(path, temporary.is_some())?;
        }
        self.path = Some(fs::canonicalize(&path).with_context(|| {
            format!(
                "Failed to resolve prepared publish input {}",
                path.display()
            )
        })?);
        Ok(temporary)
    }

    fn resolve_publish_directory(&self, path: PathBuf, extracted: bool) -> Result<PathBuf> {
        Ok(match self.target {
            PublishTarget::Android if !extracted && path.join("gradlew").is_file() => path,
            PublishTarget::Android => input::unique_file(&path, "aar", true)?,
            PublishTarget::Ohos if !extracted && path.join("oh-package.json5").is_file() => path,
            PublishTarget::Ohos => input::unique_file(&path, "har", true)?,
            PublishTarget::Kmp => path,
            PublishTarget::Apple => self.validate_apple_directory(path)?,
            PublishTarget::Conan => Self::resolve_conan_directory(path, extracted)?,
            PublishTarget::Doc => Self::resolve_doc_directory(path, extracted)?,
            PublishTarget::Index => input::unique_file(&path, "zip", true)?,
        })
    }

    fn resolve_doc_directory(path: PathBuf, extracted: bool) -> Result<PathBuf> {
        if extracted {
            input::archive_descriptor(&path, &["mkdocs.yml", "mkdocs.yaml"])
        } else {
            input::mkdocs_config(&path)
        }
    }

    fn resolve_conan_directory(path: PathBuf, extracted: bool) -> Result<PathBuf> {
        if extracted {
            return input::archive_descriptor(&path, &["conanfile.py"]);
        }
        let recipe = path.join("conanfile.py");
        if !recipe.is_file() {
            bail!("conanfile.py not found in {}", path.display());
        }
        Ok(recipe)
    }

    fn validate_apple_directory(&self, path: PathBuf) -> Result<PathBuf> {
        if matches!(self.manager, AppleManager::Cocoapods | AppleManager::All) {
            input::unique_file(&path, "podspec", false)?;
        }
        if matches!(self.manager, AppleManager::Spm | AppleManager::All)
            && !path.join("Package.swift").is_file()
        {
            bail!("Package.swift not found in {}", path.display());
        }
        Ok(path)
    }

    fn publish_android(&self, verbose: bool) -> Result<()> {
        println!("=== Publishing Android to Maven ===\n");

        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        let artifact = self.path.as_ref().filter(|path| path.is_file());
        let android_dir = self
            .path
            .as_ref()
            .filter(|path| path.is_dir())
            .cloned()
            .unwrap_or_else(|| cwd.join("android"));

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
        if matches!(self.registry, RegistryType::Private) {
            if let Some(remote) = &self.remote {
                cmd.env("CCGO_MAVEN_URL", remote);
            }
        }
        cmd.arg(gradle_task);
        cmd.arg("--no-daemon");

        // Gradle silently accepts unknown -P properties. Fail before executing
        // tasks if an older plugin would ignore the requested artifact path.
        let publish_guard =
            self.configure_android_publish_input(&mut cmd, &android_dir, artifact)?;

        if let Some(path) = artifact {
            cmd.arg(format!("-PccgoPublishPath={}", path.display()));
        }
        if self.skip_build || artifact.is_some() {
            cmd.arg("-PccgoSkipPublishBuild=true");
        }

        self.configure_android_runtime(&mut cmd, &android_dir);

        if self.skip_build {
            cmd.arg("-x").arg("buildAAR");
        }

        if verbose {
            cmd.arg("--info");
        }

        let status = cmd.status().context("Failed to execute gradlew")?;
        drop(publish_guard);

        if !status.success() {
            bail!(
                "Publish failed with exit code: {}",
                status.code().unwrap_or(-1)
            );
        }

        println!("\n✅ Publish completed successfully!");
        Ok(())
    }

    fn configure_android_publish_input(
        &self,
        cmd: &mut Command,
        android_dir: &Path,
        artifact: Option<&PathBuf>,
    ) -> Result<Option<tempfile::NamedTempFile>> {
        Ok(if artifact.is_some() || self.skip_build {
            let guard = tempfile::Builder::new().suffix(".gradle").tempfile()?;
            fs::write(
                guard.path(),
                r#"
gradle.taskGraph.whenReady {
    if (gradle.rootProject.allprojects.any { it.projectDir.canonicalPath == System.getenv('CCGO_PUBLISH_PROJECT_DIR') } &&
        !gradle.rootProject.allprojects.any {
        it.extensions.extraProperties.has('ccgoPublishPathSupported') &&
        it.extensions.extraProperties.get('ccgoPublishPathSupported') == true
    }) {
        throw new GradleException('This CCGO Gradle plugin does not support explicit publish paths/build skipping. Update the CCGO Gradle plugin together with the CLI.')
    }
}
"#,
            )?;
            cmd.arg("--init-script").arg(guard.path());
            cmd.env("CCGO_PUBLISH_PROJECT_DIR", fs::canonicalize(android_dir)?);
            Some(guard)
        } else {
            None
        })
    }

    fn android_publish_config(&self, android_dir: &Path) -> Option<CcgoConfig> {
        if self.path.as_ref().is_some_and(|path| path.is_dir()) {
            let local = android_dir.join("CCGO.toml");
            let config_path = if local.is_file() {
                local
            } else {
                android_dir
                    .parent()
                    .unwrap_or(android_dir)
                    .join("CCGO.toml")
            };
            CcgoConfig::load_from_path(&config_path).ok()
        } else {
            CcgoConfig::load().ok()
        }
    }

    fn publish_ohos(&self, verbose: bool) -> Result<()> {
        println!("=== Publishing OHOS to OHPM ===\n");

        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        let ohos_dir = self
            .path
            .as_ref()
            .filter(|path| path.is_dir())
            .cloned()
            .unwrap_or_else(|| cwd.join("ohos"));

        if self.path.is_none() && (!ohos_dir.exists() || !ohos_dir.is_dir()) {
            bail!("Error: ohos directory not found at {}", ohos_dir.display());
        }

        let artifact = match self.path.as_ref().filter(|path| path.is_file()) {
            Some(path) => path.clone(),
            None => input::unique_file(&ohos_dir, "har", true).context(
                "OHPM publish/prepublish requires a built HAR; use --path to select one",
            )?,
        };
        let mut cmd = Command::new("ohpm");
        cmd.current_dir(if self.path.as_ref().is_some_and(|path| path.is_file()) {
            &cwd
        } else {
            &ohos_dir
        });
        match self.registry {
            RegistryType::Local => {
                println!("Validating HAR locally with ohpm prepublish...");
                cmd.arg("prepublish");
            }
            RegistryType::Official => {
                cmd.args([
                    "publish",
                    "--publish_registry",
                    "https://ohpm.openharmony.cn/ohpm/",
                ]);
            }
            RegistryType::Private => {
                let url = self
                    .remote
                    .as_deref()
                    .context("Private OHPM registry requires --remote")?;
                cmd.args(["publish", "--publish_registry", url]);
            }
        }
        cmd.arg(&artifact);
        if verbose {
            println!("Running: {:?}", cmd);
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

    fn kmp_publish_task(&self, prebuilt: bool) -> &'static str {
        match self.registry {
            RegistryType::Local if prebuilt => "ccgoPublishToMavenLocal",
            RegistryType::Official if prebuilt => "ccgoPublishToMavenCentral",
            RegistryType::Private if prebuilt => "ccgoPublishToMavenCustom",
            RegistryType::Local => "publishToMavenLocal",
            RegistryType::Official => "publishAllPublicationsToMavenCentralRepository",
            RegistryType::Private => "publishAllPublicationsToMavenCustomRepository",
        }
    }

    fn publish_kmp(&self, verbose: bool) -> Result<()> {
        println!("=== Publishing KMP to Maven ===\n");

        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        let sdk = self
            .path
            .as_deref()
            .filter(|path| !path.join("gradlew").is_file())
            .map(kmp_sdk::PreparedSdk::prepare)
            .transpose()?;
        let kmp_dir = if sdk.is_some() {
            cwd.join("kmp")
        } else {
            self.path.clone().unwrap_or_else(|| cwd.join("kmp"))
        };

        if !kmp_dir.exists() || !kmp_dir.is_dir() {
            bail!("Error: kmp directory not found at {}", kmp_dir.display());
        }

        // Map registry to Gradle task
        let gradle_task = self.kmp_publish_task(sdk.is_some());

        println!("Publishing to {} Maven repository", self.registry);
        println!("Running: ./gradlew {}", gradle_task);
        println!("{}", "-".repeat(60));

        let build_type = if self.release { "release" } else { "debug" };
        if sdk.is_some() {
            println!("Publishing prebuilt SDK artifacts (build skipped)");
        } else {
            println!("Build type: {}", build_type);
        }

        let mut cmd = Command::new("./gradlew");
        cmd.current_dir(&kmp_dir);
        if matches!(self.registry, RegistryType::Private) {
            if let Some(remote) = &self.remote {
                cmd.env("CCGO_MAVEN_URL", remote);
            }
        }
        if let Some(sdk) = &sdk {
            cmd.env("CCGO_PUBLISH_PROJECT_DIR", fs::canonicalize(&kmp_dir)?);
            cmd.arg(format!("-PccgoKmpSdkManifest={}", sdk.manifest.display()));
            cmd.arg("--no-configuration-cache");
            cmd.arg("--init-script").arg(&sdk.guard);
        }
        // Two channels, because two different consumers need it: the env var
        // reaches the plugin's nested `ccgo build`, the property reaches the
        // project's own build.gradle.kts (which picks the cinterop libraryPath).
        cmd.env("CCGO_BUILD_TYPE", build_type);
        cmd.arg(format!("-PccgoBuildType={build_type}"));
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
        conan::publish(self, verbose)
    }

    fn publish_apple(&self, verbose: bool) -> Result<()> {
        println!("=== Publishing Apple platforms ===\n");

        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        let project_dir = match &self.path {
            Some(path) if path.is_dir() => path.as_path(),
            Some(path) => path
                .parent()
                .context("Apple input has no parent directory")?,
            None => cwd.as_path(),
        };

        match self.manager {
            AppleManager::Cocoapods | AppleManager::All => {
                self.publish_cocoapods(
                    project_dir,
                    verbose,
                    matches!(self.manager, AppleManager::All) && !self.push,
                )?;
            }
            _ => {}
        }

        match self.manager {
            AppleManager::Spm | AppleManager::All => {
                self.publish_spm(project_dir, verbose)?;
            }
            _ => {}
        }

        println!("\n✅ Publish completed successfully!");
        Ok(())
    }

    fn configure_pod_options(&self, cmd: &mut Command, verbose: bool) {
        if self.allow_warnings {
            cmd.arg("--allow-warnings");
        }
        if verbose {
            cmd.arg("--verbose");
        }
    }

    fn publish_cocoapods(
        &self,
        project_dir: &Path,
        verbose: bool,
        validation_only: bool,
    ) -> Result<()> {
        println!("Publishing to CocoaPods...");

        // Find .podspec file
        let podspec = match &self.path {
            Some(path) if path.is_file() => path.clone(),
            Some(path) => input::unique_file(path, "podspec", false)?,
            None => self.find_file_with_extension(project_dir, ".podspec")?,
        };
        let project_dir = if self.path.is_some() {
            podspec
                .parent()
                .context("Podspec has no parent directory")?
        } else {
            project_dir
        };

        let registry = if validation_only {
            RegistryType::Local
        } else {
            self.registry.clone()
        };
        self.run_pod_publish(project_dir, &podspec, registry, verbose)
    }

    fn run_pod_publish(
        &self,
        project_dir: &Path,
        podspec: &Path,
        registry: RegistryType,
        verbose: bool,
    ) -> Result<()> {
        match registry {
            RegistryType::Local => {
                println!("Validating podspec locally...");

                let mut cmd = Command::new("pod");
                cmd.current_dir(project_dir);
                cmd.args(["lib", "lint", podspec.to_str().unwrap()]);

                self.configure_pod_options(&mut cmd, verbose);

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

                self.configure_pod_options(&mut cmd, verbose);

                let status = cmd.status().context("Failed to execute pod trunk push")?;

                if !status.success() {
                    bail!("Pod trunk push failed");
                }
            }
            RegistryType::Private => {
                if let Some(remote_name) = &self.remote {
                    println!("Publishing to private specs repo: {}", remote_name);

                    let mut cmd = Command::new("pod");
                    cmd.current_dir(project_dir);
                    cmd.args(["repo", "push", remote_name, podspec.to_str().unwrap()]);

                    self.configure_pod_options(&mut cmd, verbose);

                    let status = cmd.status().context("Failed to execute pod repo push")?;

                    if !status.success() {
                        bail!("Pod repo push failed");
                    }
                } else {
                    bail!("Private registry requires --remote");
                }
            }
        }

        Ok(())
    }

    fn validate_spm_registry(&self) -> Result<()> {
        match self.registry {
            RegistryType::Official => bail!("SPM has no official central publishing registry. Use --registry local to validate, or --registry private --remote <git-remote> --push to publish a version tag"),
            RegistryType::Private => {
                let remote = self.remote.as_deref().context("Private SPM publishing requires --remote <git-remote>")?;
                if remote.is_empty() || remote.starts_with('-') { bail!("Invalid SPM Git remote name"); }
            }
            RegistryType::Local => {}
        }
        Ok(())
    }

    fn publish_spm(&self, project_dir: &Path, verbose: bool) -> Result<()> {
        self.validate_spm_registry()?;
        if !project_dir.join("Package.swift").is_file() {
            bail!("Package.swift not found");
        }
        if matches!(self.registry, RegistryType::Private) {
            let remote = self.remote.as_deref().unwrap();
            if !Command::new("git")
                .current_dir(project_dir)
                .args(["remote", "get-url", remote])
                .output()?
                .status
                .success()
            {
                bail!("SPM Git remote '{remote}' not found; configure it with git remote add");
            }
        }
        println!("Validating Swift package manifest...");
        let mut swift = Command::new("swift");
        swift
            .current_dir(project_dir)
            .args(["package", "dump-package"]);
        if !verbose {
            swift.stdout(Stdio::null());
        }
        if !swift
            .status()
            .context("Failed to execute swift package dump-package")?
            .success()
        {
            bail!("Swift package manifest validation failed");
        }
        let configured_version = if self.version.is_some() || self.tag.is_some() {
            String::new()
        } else {
            self.get_project_version(project_dir)?
        };
        let (version, tag) = self.resolve_version_and_tag(&configured_version)?;
        if !Command::new("git")
            .args(["check-ref-format", &format!("refs/tags/{tag}")])
            .status()?
            .success()
        {
            bail!("Invalid release Git tag: {tag}");
        }
        if !self.push {
            println!("SPM validated: version {version}, tag {tag}. No tag created or pushed; use --push to publish.");
            return Ok(());
        }
        self.publish_spm_tag(project_dir, &tag)
    }

    fn publish_spm_tag(&self, project_dir: &Path, tag: &str) -> Result<()> {
        let git_output = |args: &[&str]| -> Result<std::process::Output> {
            Command::new("git")
                .current_dir(project_dir)
                .args(args)
                .output()
                .context("Failed to execute git")
        };
        let status = git_output(&["status", "--porcelain"])?;
        if !status.status.success() || !status.stdout.is_empty() {
            bail!("Commit all SPM wrapper changes before creating/pushing its version tag");
        }
        let head = git_output(&["rev-parse", "HEAD"])?;
        if !head.status.success() {
            bail!("SPM repository has no committed HEAD");
        }
        let tag_ref = format!("refs/tags/{tag}");
        let existing = git_output(&["rev-parse", "--verify", &format!("{tag_ref}^{{commit}}")])?;
        if existing.status.success() {
            if existing.stdout != head.stdout {
                bail!("SPM tag {tag} already points to a different commit");
            }
        } else if !git_output(&["tag", tag])?.status.success() {
            bail!("Failed to create SPM tag {tag}");
        }
        if matches!(self.registry, RegistryType::Local) {
            println!("Created/validated local SPM tag {tag}; no remote was contacted.");
            return Ok(());
        }
        let remote = self.remote.as_deref().unwrap();
        let refspec = format!("{tag_ref}:{tag_ref}");
        if !Command::new("git")
            .current_dir(project_dir)
            .args(["push", remote, &refspec])
            .status()?
            .success()
        {
            bail!("Failed to push SPM tag {tag} to {remote}");
        }
        println!("Pushed SPM tag {tag} to {remote}");
        Ok(())
    }

    fn publish_doc(&self, verbose: bool, archive_input: bool) -> Result<()> {
        println!("=== Publishing Documentation ===\n");

        let cwd = std::env::current_dir().context("Failed to get current directory")?;
        // MkDocs resolves content paths relative to its config. gh-deploy also
        // needs a git checkout, so ZIP inputs deploy from the caller's repo.
        let project_dir = match &self.path {
            _ if archive_input => cwd.as_path(),
            Some(path) => path
                .parent()
                .context("MkDocs config has no parent directory")?,
            None => cwd.as_path(),
        };

        // Check if docs are built
        let docs_dir = project_dir.join("site");
        if self.path.is_none() && !docs_dir.exists() {
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
        cmd.current_dir(project_dir);
        cmd.args(["gh-deploy", "--branch", &self.doc_branch]);
        if let Some(path) = &self.path {
            cmd.arg("--config-file").arg(path);
        }

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

        self.open_published_documentation(project_dir);

        println!("\n✅ Documentation published successfully!");
        Ok(())
    }

    /// Resolve one version, falling back to the already loaded project config.
    fn resolve_version_and_tag(&self, project_version: &str) -> Result<(String, String)> {
        let (version, tag) = match (&self.version, &self.tag) {
            (Some(v), Some(t)) => (v.clone(), t.clone()),
            (Some(v), None) => (v.clone(), default_tag_for_version(v)),
            (None, Some(t)) => (derive_version_from_tag(t), t.clone()),
            (None, None) => (
                project_version.to_owned(),
                default_tag_for_version(project_version),
            ),
        };
        semver::Version::parse(&version)
            .context("Invalid release version; use --version with --tag for custom tag prefixes")?;
        if tag.is_empty() || tag.starts_with('-') {
            bail!("Invalid release Git tag: {tag}");
        }
        Ok((version, tag))
    }

    /// Merge this invocation's version into the package's index entry and write
    /// it back. The index is append-only, so the existing versions are read
    /// first and the new one folded in rather than replacing the file.
    fn write_package_entry(
        &self,
        index_path: &std::path::Path,
        config: &crate::config::CcgoConfig,
        package: &crate::config::PackageConfig,
        git_url: &str,
        new_version_entry: crate::registry::VersionEntry,
    ) -> Result<PackageEntry> {
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
            repository: git_url.to_string(),
            homepage: package.repository.clone(),
            license: package.license.clone(),
            keywords: Vec::new(),
            platforms: self.get_supported_platforms(config),
            versions: merged_versions,
        };

        let json = serde_json::to_string_pretty(&package_entry)
            .context("Failed to serialize package entry")?;
        fs::write(&package_file, &json).context("Failed to write package file")?;

        println!("✅ Written: {}", package_rel_path.display());
        Ok(package_entry)
    }

    fn publish_index(&self, verbose: bool) -> Result<()> {
        println!("=== Publishing to Package Index ===\n");

        let cwd = std::env::current_dir().context("Failed to get current directory")?;

        // Find and load CCGO.toml
        let project_dir = Self::find_project_dir(&cwd)?;
        let config_path = project_dir.join("CCGO.toml");
        let config =
            CcgoConfig::load_from_path(&config_path).context("Failed to load CCGO.toml")?;

        let destination = index_destination::resolve(self, &config)?;
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
        let (version, tag) = self.resolve_version_and_tag(&package.version)?;

        println!("\n🔖 Publishing single version:");
        println!("   version: {}", version);
        println!("   tag:     {}", tag);

        self.validate_tag_exists(&project_dir, &tag)?;
        let mut new_version_entry =
            self.build_single_version_entry(&project_dir, &package.name, &version, &tag, verbose)?;
        let artifact = self.prepare_index_artifact(
            &project_dir,
            &config_path,
            &package.name,
            &version,
            &tag,
            &mut new_version_entry,
        )?;

        self.publish_prepared_index(
            &project_dir,
            &config,
            &git_url,
            destination,
            (new_version_entry, artifact),
            verbose,
        )
    }

    fn publish_prepared_index(
        &self,
        project_dir: &Path,
        config: &CcgoConfig,
        git_url: &str,
        destination: index_destination::Destination,
        prepared: (
            VersionEntry,
            Option<(artifact::PreparedUpload, artifact::ArchiveSnapshot)>,
        ),
        verbose: bool,
    ) -> Result<()> {
        let package = config
            .package
            .as_ref()
            .context("Missing package configuration")?;
        let (mut new_version_entry, artifact) = prepared;
        let index_repo = &destination.repository;
        let index_name = &destination.name;
        println!("\nIndex repository: {}", index_repo);
        if !self.push {
            Self::validate_index_preview(&destination, &package.name, new_version_entry)?;
            return Ok(());
        }
        let work_path = if destination.local {
            PathBuf::from(index_repo)
        } else {
            crate::registry::index_writer::publish_work_dir(index_name)
        };
        let _write_lock = crate::registry::index_writer::lock_index(&work_path)?;
        Self::validate_publication_worktree(&work_path, &destination)?;
        let pending = index_recovery::Publication::resume(
            &work_path,
            &package.name,
            git_url,
            &new_version_entry,
            verbose,
        )?;
        if let Some(pending) = &pending {
            new_version_entry = pending.version.clone();
        }
        let index_path = Self::prepare_publication_index(&destination, project_dir, verbose)?;
        crate::registry::index_writer::require_clean(&index_path)?;

        if let Some(receipt) = &pending {
            if self.complete_pending_publication(
                &index_path,
                (index_name, index_repo),
                &package.name,
                receipt,
                verbose,
            )? {
                return Ok(());
            }
        }
        let mut pending = Self::prepare_publication_receipt(
            &index_path,
            &package.name,
            git_url,
            &new_version_entry,
            pending,
        )?;
        Self::upload_index_artifact(&index_path, &mut pending, artifact.as_ref())?;
        let package_entry =
            self.write_package_entry(&index_path, config, package, git_url, new_version_entry)?;

        self.commit_index_publication(&index_path, index_name, &package_entry, verbose)?;

        self.push_and_report(
            &index_path,
            index_name,
            index_repo,
            &package.name,
            &package_entry,
            verbose,
        )?;
        pending.finish(&index_path)
    }

    fn prepare_publication_index(
        destination: &index_destination::Destination,
        project_dir: &Path,
        verbose: bool,
    ) -> Result<PathBuf> {
        if destination.local {
            index_destination::prepare_local(&destination.repository, project_dir)
        } else {
            crate::registry::index_writer::prepare_index_repo(
                &destination.repository,
                &destination.name,
                verbose,
            )
        }
    }

    fn prepare_publication_receipt(
        index_path: &Path,
        package_name: &str,
        git_url: &str,
        new_version_entry: &VersionEntry,
        pending: Option<index_recovery::Publication>,
    ) -> Result<index_recovery::Publication> {
        // Check the current destination before creating an immutable remote asset.
        merge_version_entry(
            Self::read_existing_versions(
                &index_path.join(PackageIndex::package_index_path(package_name)),
            )?,
            new_version_entry.clone(),
        )?;
        Ok(match pending {
            Some(pending) => pending,
            None => index_recovery::Publication::start(
                index_path,
                package_name,
                git_url,
                new_version_entry.clone(),
            )?,
        })
    }

    fn commit_index_publication(
        &self,
        index_path: &Path,
        index_name: &str,
        package_entry: &PackageEntry,
        verbose: bool,
    ) -> Result<()> {
        // Update index.json metadata
        crate::registry::index_writer::update_index_metadata(index_path, index_name)?;

        // Commit changes
        let commit_message = self.index_message.clone().unwrap_or_else(|| {
            format!(
                "Update {} to {}",
                package_entry.name,
                package_entry
                    .versions
                    .first()
                    .map(|v| v.version.as_str())
                    .unwrap_or("unknown")
            )
        });

        crate::registry::index_writer::commit_changes(index_path, &commit_message, verbose)?;

        Ok(())
    }

    fn validate_index_preview(
        destination: &index_destination::Destination,
        package: &str,
        version: VersionEntry,
    ) -> Result<()> {
        if destination.local {
            let file =
                Path::new(&destination.repository).join(PackageIndex::package_index_path(package));
            merge_version_entry(Self::read_existing_versions(&file)?, version.clone())?;
        }
        println!("Index validated: version {}, tag {}. No index files, commits, or remotes changed; use --push to publish. Remote duplicate versions are checked at publication.", version.version, version.tag);
        Ok(())
    }

    fn upload_index_artifact(
        index_path: &Path,
        pending: &mut index_recovery::Publication,
        artifact: Option<&(artifact::PreparedUpload, artifact::ArchiveSnapshot)>,
    ) -> Result<()> {
        if let Some((upload, snapshot)) = artifact {
            if !pending.uploaded {
                upload.upload_registry(&snapshot.path)?;
            }
            println!(
                "Artifact uploaded. If a later index step fails, the artifact remains at {}",
                upload.url
            );
        }
        pending.uploaded = true;
        pending.save(index_path)?;
        Ok(())
    }

    fn complete_pending_publication(
        &self,
        index_path: &Path,
        destination: (&str, &str),
        package_name: &str,
        pending: &index_recovery::Publication,
        verbose: bool,
    ) -> Result<bool> {
        let (index_name, index_repo) = destination;
        let file = index_path.join(PackageIndex::package_index_path(package_name));
        let versions = Self::read_existing_versions(&file)?;
        if versions
            .iter()
            .any(|version| version.version == pending.version.version)
        {
            pending.verify_entry(index_path)?;
            if !pending.uploaded {
                bail!("Pending publication has no completed artifact upload");
            }
            let entry = serde_json::from_slice(&fs::read(file)?)?;
            self.push_and_report(
                index_path,
                index_name,
                index_repo,
                package_name,
                &entry,
                verbose,
            )?;
            pending.finish(index_path)?;
            return Ok(true);
        }
        Ok(false)
    }

    fn prepare_index_artifact(
        &self,
        project_dir: &Path,
        config_path: &Path,
        package_name: &str,
        version: &str,
        tag: &str,
        new_version_entry: &mut VersionEntry,
    ) -> Result<Option<(artifact::PreparedUpload, artifact::ArchiveSnapshot)>> {
        Ok(if self.artifact_provider.is_some() {
            let archive = self.path.clone().unwrap_or_else(|| {
                project_dir.join("target/release/package").join(format!(
                    "{}_CCGO_PACKAGE-{version}.zip",
                    package_name.to_uppercase()
                ))
            });
            let snapshot = artifact::ArchiveSnapshot::new(&archive)?;
            let raw_config: toml::Value = toml::from_str(&fs::read_to_string(config_path)?)?;
            let upload = artifact::PreparedUpload::prepare(
                self,
                &raw_config,
                package_name,
                version,
                tag,
                "index",
            )?;
            new_version_entry.archive_url = Some(upload.url.clone());
            new_version_entry.archive_format = Some("zip".to_owned());
            new_version_entry.checksum = Some(snapshot.checksum.clone());
            println!(
                "Artifact URL: {}\nChecksum: {}",
                upload.url, snapshot.checksum
            );
            Some((upload, snapshot))
        } else {
            None
        })
    }

    /// Push the index commit when asked, then print how to consume the package.
    fn push_and_report(
        &self,
        index_path: &std::path::Path,
        index_name: &str,
        index_repo: &str,
        package_name: &str,
        package_entry: &PackageEntry,
        verbose: bool,
    ) -> Result<()> {
        if self.push && !matches!(self.registry, RegistryType::Local) {
            println!("\n📤 Pushing to remote...");
            crate::registry::index_writer::push_changes_retry(index_path, index_name, verbose)?;
            println!("✅ Pushed successfully!");
        } else {
            if matches!(self.registry, RegistryType::Local) {
                println!("\nIndex published locally; no remote was contacted.");
            } else {
                println!("\nChanges committed locally. Use --push to push to remote.");
            }
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
            package_name,
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
    /// typos in `--tag` before we go through the rest of the publish
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

        let checksum = if self.checksum && self.artifact_provider.is_none() {
            if verbose {
                println!("   Computing checksum for {}...", tag);
            }
            if archive_url.is_some() {
                // Archive-mode: hash the zip consumers will download.
                let result = self.compute_archive_zip_checksum(project_dir, package_name, version);
                if self.path.is_some() {
                    Some(result?)
                } else {
                    result.ok()
                }
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
        let zip_path = self.path.clone().unwrap_or_else(|| {
            project_dir
                .join("target")
                .join("release")
                .join("package")
                .join(&zip_name)
        });
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

        let config: toml::Value = toml::from_str(&content)?;
        config
            .get("project")
            .or_else(|| config.get("package"))
            .and_then(|package| package.get("version"))
            .and_then(|version| version.as_str())
            .map(str::to_owned)
            .context("Project version not found in CCGO.toml")
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
    fn remote_and_legacy_aliases_share_one_destination() {
        use clap::Parser;
        #[derive(Parser)]
        struct Cli {
            #[command(flatten)]
            publish: PublishCommand,
        }
        for flag in ["--remote", "--url", "--remote-name"] {
            let parsed =
                Cli::try_parse_from(["ccgo", "ohos", flag, "https://repo.example"]).unwrap();
            assert_eq!(
                parsed.publish.remote.as_deref(),
                Some("https://repo.example")
            );
        }
        for (first, second) in [
            ("--remote", "--url"),
            ("--remote", "--remote-name"),
            ("--url", "--remote-name"),
        ] {
            assert!(Cli::try_parse_from(["ccgo", "ohos", first, "one", second, "two"]).is_err());
        }
    }

    fn parse_publish(args: &[&str]) -> PublishCommand {
        use clap::Parser;
        let cli = crate::cli::Cli::try_parse_from(
            ["ccgo", "publish"].into_iter().chain(args.iter().copied()),
        )
        .unwrap();
        match cli.command {
            crate::cli::Commands::Publish(command) => *command,
            _ => unreachable!(),
        }
    }

    #[test]
    fn package_publish_accepts_legacy_index_alias() {
        for target in ["package", "index"] {
            let command = parse_publish(&[target]);
            assert!(matches!(command.target, PublishTarget::Index));
            assert_eq!(command.target.to_string(), "package");
        }
        let targets: Vec<_> = PublishTarget::value_variants()
            .iter()
            .map(|target| target.to_possible_value().unwrap().get_name().to_owned())
            .collect();
        assert!(targets.iter().any(|target| target == "package"));
        assert!(!targets.iter().any(|target| target == "index"));
    }

    #[test]
    fn publish_path_validates_platform_and_file_type() {
        let tmp = tempfile::tempdir().unwrap();
        let aar = tmp.path().join("custom.aar");
        fs::write(&aar, b"aar").unwrap();
        assert!(parse_publish(&["android"])
            .validate_publish_path(&aar)
            .is_ok());
        assert!(parse_publish(&["ohos"])
            .validate_publish_path(&aar)
            .is_err());
        assert!(parse_publish(&["android"])
            .validate_publish_path(tmp.path())
            .is_ok());
        assert!(parse_publish(&["android"])
            .validate_publish_path(&tmp.path().join("missing.aar"))
            .is_err());
        for target in ["conan", "doc", "kmp", "apple"] {
            assert!(parse_publish(&[target])
                .validate_publish_path(&aar)
                .is_err());
        }
        let podspec = tmp.path().join("Library.podspec");
        fs::write(&podspec, b"spec").unwrap();
        assert!(parse_publish(&["apple", "--manager", "cocoapods"])
            .validate_publish_path(&podspec)
            .is_ok());
        fs::write(tmp.path().join("gradlew"), b"wrapper").unwrap();
        assert!(parse_publish(&["kmp"])
            .validate_publish_path(tmp.path())
            .is_ok());
    }

    #[test]
    fn index_path_hashes_explicit_archive_and_propagates_read_errors() {
        let tmp = tempfile::tempdir().unwrap();
        let zip = tmp.path().join("custom.zip");
        let payload = b"custom archive";
        fs::write(&zip, payload).unwrap();
        assert!(parse_publish(&["index"])
            .validate_publish_path(&zip)
            .is_err());
        let mut command = parse_publish(&[
            "index",
            "--checksum",
            "--archive-url-template",
            "https://cdn.example/{version}.zip",
        ]);
        command.validate_publish_path(&zip).unwrap();
        command.path = Some(zip.clone());
        let entry = command
            .build_single_version_entry(tmp.path(), "lib", "1.0.0", "v1.0.0", false)
            .unwrap();
        assert_eq!(
            entry.checksum,
            Some(format!("sha256:{:x}", Sha256::digest(payload)))
        );
        fs::remove_file(zip).unwrap();
        assert!(command
            .build_single_version_entry(tmp.path(), "lib", "1.0.0", "v1.0.0", false)
            .is_err());
    }

    #[test]
    fn index_directory_resolves_one_archive_and_rejects_ambiguity() {
        let tmp = tempfile::tempdir().unwrap();
        let archive = tmp.path().join("SDK.zip");
        fs::write(&archive, b"sdk").unwrap();
        let mut command = parse_publish(&[
            "index",
            "--checksum",
            "--archive-url-template",
            "https://cdn.example/sdk.zip",
        ]);
        command.path = Some(tmp.path().to_path_buf());
        command.prepare_publish_path().unwrap();
        assert_eq!(command.path, Some(fs::canonicalize(&archive).unwrap()));
        fs::write(tmp.path().join("another.zip"), b"sdk").unwrap();
        command.path = Some(tmp.path().to_path_buf());
        assert!(command
            .prepare_publish_path()
            .err()
            .unwrap()
            .to_string()
            .contains("Multiple .zip"));
    }

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
        std::fs::write(pkg_dir.join("STDCOMM_CCGO_PACKAGE-1.0.0.zip"), payload).unwrap();

        let cmd = PublishCommand {
            target: PublishTarget::Index,
            registry: RegistryType::Local,
            remote: None,
            skip_build: false,
            path: None,
            artifact_provider: None,
            artifact_group: None,
            artifact_id: None,
            github_repo: None,
            release: false,
            stl: None,
            yes: true,
            manager: AppleManager::All,
            push: false,
            platform: None,
            allow_warnings: true,
            conan: Default::default(),
            profile: "default".into(),
            link_type: "both".into(),
            doc_branch: "gh-pages".into(),
            doc_force: false,
            doc_open: false,
            index_repo: None,
            index_name: None,
            index_message: None,
            checksum: true,
            archive_url_template: None,
            archive_format: "zip".into(),
            version: None,
            tag: None,
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
            remote: None,
            skip_build: false,
            path: None,
            artifact_provider: None,
            artifact_group: None,
            artifact_id: None,
            github_repo: None,
            release: false,
            stl: None,
            yes: true,
            manager: AppleManager::All,
            push: false,
            platform: None,
            allow_warnings: true,
            conan: Default::default(),
            profile: "default".into(),
            link_type: "both".into(),
            doc_branch: "gh-pages".into(),
            doc_force: false,
            doc_open: false,
            index_repo: None,
            index_name: None,
            index_message: None,
            checksum: true,
            archive_url_template: None,
            archive_format: "zip".into(),
            version: None,
            tag: None,
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
        // tag like `release-1.0.0`, pass --version explicitly.
        assert_eq!(derive_version_from_tag("release-1.0.0"), "release-1.0.0");
    }

    #[test]
    fn default_tag_prepends_v() {
        assert_eq!(default_tag_for_version("1.0.0"), "v1.0.0");
        assert_eq!(default_tag_for_version("25.2.9519653"), "v25.2.9519653");
    }

    #[test]
    fn index_version_defaults_to_config_and_preserves_explicit_overrides() {
        for (args, expected_version, expected_tag) in [
            (vec!["index"], "1.2.3", "v1.2.3"),
            (vec!["index", "--version", "2.0.0"], "2.0.0", "v2.0.0"),
            (vec!["index", "--tag", "v3.0.0"], "3.0.0", "v3.0.0"),
            (
                vec!["index", "--version", "4.0.0", "--tag", "release-4"],
                "4.0.0",
                "release-4",
            ),
        ] {
            let command = parse_publish(&args);
            assert_eq!(
                command.resolve_version_and_tag("1.2.3").unwrap(),
                (expected_version.to_owned(), expected_tag.to_owned())
            );
        }
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
