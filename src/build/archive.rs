//! ZIP archive creation and build_info.json generation
//!
//! This module handles creating SDK and symbol archives following the
//! unified CCGO archive structure. Archive paths match Python ccgo format:
//!
//! ```text
//! {PROJECT}_{PLATFORM}_SDK-{version}.zip
//! ├── lib/{platform}/static/     # Static libraries
//! ├── lib/{platform}/shared/     # Shared libraries
//! ├── include/{project}/         # Header files
//! ├── haars/{platform}/          # AAR/HAR packages (Android/OHOS)
//! └── meta/{platform}/           # Metadata files
//!     ├── build_info.json
//!     └── archive_info.json
//! ```

use std::collections::BTreeMap;
use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::Local;
use walkdir::WalkDir;
use zip::write::SimpleFileOptions;
use zip::ZipWriter;

use super::{
    AndroidBuildInfo, BuildDetails, BuildInfo, BuildInfoFull, BuildMetadata, EnvironmentInfo,
    GitInfo, IosBuildInfo, MacosBuildInfo, PlatformBuildInfo, ProjectInfo,
};

// Archive directory constants - aligned with Python ccgo build_utils.py
pub const ARCHIVE_DIR_LIB: &str = "lib";
pub const ARCHIVE_DIR_STATIC: &str = "static";
pub const ARCHIVE_DIR_SHARED: &str = "shared";
pub const ARCHIVE_DIR_INCLUDE: &str = "include";
pub const ARCHIVE_DIR_FRAMEWORKS: &str = "lib"; // Apple frameworks under lib/
pub const ARCHIVE_DIR_HAARS: &str = "haars";
pub const ARCHIVE_DIR_SYMBOLS: &str = "symbols";
pub const ARCHIVE_DIR_OBJ: &str = "obj";
pub const ARCHIVE_DIR_META: &str = "meta";
pub const BUILD_INFO_FILE: &str = "build_info.json";
pub const ARCHIVE_INFO_FILE: &str = "archive_info.json";

/// Files to exclude from archive (matching pyccgo ARCHIVE_INCLUDE_EXCLUDE_FILES)
const ARCHIVE_EXCLUDE_FILES: &[&str] = &["CPPLINT.cfg", ".clang-format", ".clang-tidy"];

/// Check if a file should be included in the archive
fn should_include_file_in_archive(filename: &str) -> bool {
    !ARCHIVE_EXCLUDE_FILES.contains(&filename)
}

/// Get the archive path for include directory, matching pyccgo behavior.
///
/// Intelligently handles include path to avoid duplication:
/// - If src_include_dir contains a project_name subdirectory, returns "include"
///   (source structure: include/ccgonow/ -> archive: include/ccgonow/)
/// - If src_include_dir does NOT contain a project_name subdirectory, returns "include/project_name"
///   (source structure: include/*.h -> archive: include/ccgonow/*.h)
///
/// # Arguments
/// * `project_name` - Project name (lowercase)
/// * `src_include_dir` - Source include directory path to check for existing project subdirectory
///
/// # Returns
/// Archive path - either "include" or "include/{project_name}"
pub fn get_unified_include_path(project_name: &str, src_include_dir: &Path) -> String {
    // Check if source directory already has a project-named subdirectory
    let project_subdir = src_include_dir.join(project_name);
    if project_subdir.is_dir() {
        // Source already has project subdirectory, don't add it again
        ARCHIVE_DIR_INCLUDE.to_string()
    } else {
        // Source doesn't have project subdirectory, add it to archive path
        format!("{}/{}", ARCHIVE_DIR_INCLUDE, project_name)
    }
}

/// Create a ZIP archive from a directory
pub fn create_archive(source_dir: &Path, archive_path: &Path) -> Result<()> {
    let file = File::create(archive_path)
        .with_context(|| format!("Failed to create archive: {}", archive_path.display()))?;

    let mut zip = ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    // Walk the source directory
    for entry in WalkDir::new(source_dir) {
        let entry = entry.context("Failed to read directory entry")?;
        let path = entry.path();
        let relative_path = path
            .strip_prefix(source_dir)
            .context("Failed to get relative path")?;

        // Skip the root directory
        if relative_path.as_os_str().is_empty() {
            continue;
        }

        let path_str = relative_path.to_string_lossy().replace('\\', "/");

        if path.is_dir() {
            zip.add_directory(&path_str, options)
                .with_context(|| format!("Failed to add directory to archive: {}", path_str))?;
        } else {
            zip.start_file(&path_str, options)
                .with_context(|| format!("Failed to start file in archive: {}", path_str))?;

            let mut file = File::open(path)
                .with_context(|| format!("Failed to open file: {}", path.display()))?;
            let mut buffer = Vec::new();
            file.read_to_end(&mut buffer)
                .with_context(|| format!("Failed to read file: {}", path.display()))?;
            zip.write_all(&buffer)
                .with_context(|| format!("Failed to write file to archive: {}", path_str))?;
        }
    }

    zip.finish().context("Failed to finish ZIP archive")?;
    Ok(())
}

// NOTE: Checksums are NOT calculated in Python ccgo, so we don't implement them
// for parity. See research.md "Finding: NO CHECKSUMS IMPLEMENTED"

/// Collect file metadata (size, no checksums) for archive_info.json
pub fn collect_file_metadata(dir: &Path) -> Result<Vec<FileMetadata>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(dir).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            let relative = path
                .strip_prefix(dir)
                .unwrap_or(path)
                .to_string_lossy()
                .replace('\\', "/");

            let metadata = std::fs::metadata(path)?;
            files.push(FileMetadata {
                path: relative,
                size: metadata.len(),
                compressed_size: 0, // Will be filled after ZIP creation
            });
        }
    }

    Ok(files)
}

/// File metadata for archive_info.json (NO checksums, matching Python)
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct FileMetadata {
    pub path: String,
    pub size: u64,
    pub compressed_size: u64,
}

/// Get current git commit hash (if in a git repository)
pub fn get_git_commit() -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// Get current git branch (if in a git repository)
pub fn get_git_branch() -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// Create build_info.json with Python ccgo compatible format
///
/// Field names and formats match Python ccgo:
/// - `project` instead of `name`
/// - `build_time` instead of `timestamp` (ISO 8601 with microseconds, local time)
/// - `build_host` for the host OS
pub fn create_build_info(
    name: &str,
    version: &str,
    platform: &str,
    architectures: &[String],
    link_type: &str,
    toolchain: Option<&str>,
) -> BuildInfo {
    BuildInfo {
        project: name.to_string(),
        platform: platform.to_string(),
        version: version.to_string(),
        link_type: link_type.to_string(),
        // Use local time with microseconds to match Python ccgo format
        build_time: Local::now().format("%Y-%m-%dT%H:%M:%S%.6f").to_string(),
        // Use capitalized OS name to match Python: Darwin, Linux, Windows
        build_host: get_build_host(),
        architectures: architectures.to_vec(),
        toolchain: toolchain.map(String::from),
        git_commit: get_git_commit(),
        git_branch: get_git_branch(),
    }
}

/// Get build host OS name matching Python ccgo format
fn get_build_host() -> String {
    match std::env::consts::OS {
        "macos" => "Darwin".to_string(),
        "linux" => "Linux".to_string(),
        "windows" => "Windows".to_string(),
        other => other.to_string(),
    }
}

/// Archive metadata for archive_info.json (matching Python ccgo format)
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ArchiveMetadata {
    pub version: String,
    pub generated_at: String,
    pub archive_name: String,
    pub archive_size: u64,
}

/// Summary information in archive_info.json
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ArchiveSummary {
    pub total_files: usize,
    pub total_size: u64,
    pub library_count: usize,
    pub platforms: Vec<String>,
    pub architectures: Vec<String>,
}

/// ArchiveInfo stored in archive_info.json (matching Python ccgo format)
/// NO checksums - Python doesn't calculate them!
#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct ArchiveInfo {
    pub archive_metadata: ArchiveMetadata,
    pub files: Vec<FileMetadata>,
    // libraries field would require binary analysis - TODO for future
    pub summary: ArchiveSummary,
}

/// Create archive_info.json (matching Python format, NO checksums)
pub fn create_archive_info(
    archive_name: &str,
    archive_size: u64,
    files: Vec<FileMetadata>,
    platform: &str,
    architectures: &[String],
) -> ArchiveInfo {
    // Calculate summary values before moving files
    let total_files = files.len();
    let total_size: u64 = files.iter().map(|f| f.size).sum();
    let library_count = files
        .iter()
        .filter(|f| {
            f.path.ends_with(".a")
                || f.path.ends_with(".so")
                || f.path.ends_with(".lib")
                || f.path.ends_with(".dll")
                || f.path.ends_with(".dylib")
        })
        .count();

    ArchiveInfo {
        archive_metadata: ArchiveMetadata {
            version: "1.0".to_string(),
            generated_at: chrono::Utc::now().to_rfc3339(),
            archive_name: archive_name.to_string(),
            archive_size,
        },
        files,
        summary: ArchiveSummary {
            total_files,
            total_size,
            library_count,
            platforms: vec![platform.to_string()],
            architectures: architectures.to_vec(),
        },
    }
}

/// Write archive_info.json to a file
pub fn write_archive_info(archive_info: &ArchiveInfo, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(archive_info).context("Failed to serialize archive_info")?;
    std::fs::write(path, json)
        .with_context(|| format!("Failed to write archive_info.json to {}", path.display()))?;
    Ok(())
}

/// Write build_info.json to a file
pub fn write_build_info(build_info: &BuildInfo, path: &Path) -> Result<()> {
    let json = serde_json::to_string_pretty(build_info).context("Failed to serialize build_info")?;
    std::fs::write(path, json)
        .with_context(|| format!("Failed to write build_info.json to {}", path.display()))?;
    Ok(())
}

/// Create comprehensive build info matching pyccgo format
///
/// This generates the full JSON structure that matches Python ccgo's output
pub fn create_build_info_full(
    project_name: &str,
    version: &str,
    platform: &str,
    project_root: &Path,
) -> BuildInfoFull {
    let now = Local::now();
    let timestamp = now.timestamp();

    // Get git information
    let git_info = get_git_info_full(project_root);

    // Get platform-specific build info
    let platform_info = get_platform_build_info(platform);

    // Get ccgo version from Cargo.toml or env
    let ccgo_version = env!("CARGO_PKG_VERSION").to_string();

    BuildInfoFull {
        build_metadata: BuildMetadata {
            version: "1.0".to_string(),
            generated_at: now.format("%Y-%m-%dT%H:%M:%S%.6f").to_string(),
            generator: "ccgo".to_string(),
        },
        project: ProjectInfo {
            name: project_name.to_uppercase(),
            version: version.to_string(),
        },
        git: git_info,
        build: BuildDetails {
            time: now.format("%Y-%m-%d %H:%M:%S").to_string(),
            timestamp,
            platform: platform.to_string(),
            platform_info,
        },
        environment: EnvironmentInfo {
            os: get_build_host(),
            os_version: get_os_version(),
            python_version: None, // Not applicable for Rust ccgo
            ccgo_version,
        },
    }
}

/// Get comprehensive git information
fn get_git_info_full(project_root: &Path) -> GitInfo {
    let branch = get_git_branch_from_path(project_root).unwrap_or_default();
    let revision = get_git_commit_from_path(project_root).unwrap_or_default();
    let revision_full = get_git_commit_full_from_path(project_root).unwrap_or_else(|| revision.clone());
    let tag = get_git_tag_from_path(project_root).unwrap_or_default();
    let is_dirty = check_git_dirty(project_root);
    let remote_url = get_git_remote_url(project_root).unwrap_or_default();

    GitInfo {
        branch,
        revision,
        revision_full,
        tag,
        is_dirty,
        remote_url,
    }
}

/// Get git branch from project path
fn get_git_branch_from_path(project_root: &Path) -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(project_root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// Get full git commit hash
fn get_git_commit_full_from_path(project_root: &Path) -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(project_root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// Get short git commit hash
fn get_git_commit_from_path(project_root: &Path) -> Option<String> {
    std::process::Command::new("git")
        .args(["rev-parse", "--short", "HEAD"])
        .current_dir(project_root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// Get git tag (if on a tag)
fn get_git_tag_from_path(project_root: &Path) -> Option<String> {
    std::process::Command::new("git")
        .args(["describe", "--tags", "--abbrev=0"])
        .current_dir(project_root)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
}

/// Check if git working tree is dirty
fn check_git_dirty(project_root: &Path) -> bool {
    std::process::Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(project_root)
        .output()
        .ok()
        .map(|o| !o.stdout.is_empty())
        .unwrap_or(false)
}

/// Get git remote URL (anonymized)
fn get_git_remote_url(project_root: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["config", "--get", "remote.origin.url"])
        .current_dir(project_root)
        .output()
        .ok()?;

    if output.status.success() {
        let url = String::from_utf8_lossy(&output.stdout).trim().to_string();
        // Anonymize the URL by masking the middle part of the username
        Some(anonymize_git_url(&url))
    } else {
        None
    }
}

/// Anonymize git URL by masking username
fn anonymize_git_url(url: &str) -> String {
    // Handle SSH format: git@github.com:username/repo.git
    // Result: /us***/repo.git
    if url.contains('@') && url.contains(':') {
        if let Some(colon_idx) = url.rfind(':') {
            let rest = &url[colon_idx + 1..];
            if let Some(slash_idx) = rest.find('/') {
                let username = &rest[..slash_idx];
                let suffix = &rest[slash_idx..];

                if username.len() > 4 {
                    let masked = format!("/{}***{}", &username[..2], &username[username.len() - 1..]);
                    return format!("{}{}", masked, suffix);
                } else if !username.is_empty() {
                    return format!("/{}***{}", &username[..1], suffix);
                }
            }
        }
    }

    // Handle HTTPS format: https://github.com/username/repo.git
    // Result: /us***/repo.git
    if let Some(idx) = url.find("//") {
        let after_protocol = &url[idx + 2..];
        // Find the host part (e.g., github.com)
        if let Some(host_end) = after_protocol.find('/') {
            let path = &after_protocol[host_end + 1..];
            if let Some(slash_idx) = path.find('/') {
                let username = &path[..slash_idx];
                let suffix = &path[slash_idx..];

                if username.len() > 4 {
                    let masked = format!("/{}***{}", &username[..2], &username[username.len() - 1..]);
                    return format!("{}{}", masked, suffix);
                } else if !username.is_empty() {
                    return format!("/{}***{}", &username[..1], suffix);
                }
            }
        }
    }

    url.to_string()
}

/// Get OS version string
fn get_os_version() -> String {
    std::process::Command::new("uname")
        .arg("-v")
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| std::env::consts::OS.to_string())
}

/// Get platform-specific build info
fn get_platform_build_info(platform: &str) -> Option<PlatformBuildInfo> {
    match platform.to_lowercase().as_str() {
        "ios" | "watchos" | "tvos" => {
            let (xcode_version, xcode_build) = get_xcode_version();
            Some(PlatformBuildInfo::Ios {
                ios: IosBuildInfo {
                    xcode_version,
                    xcode_build,
                },
            })
        }
        "macos" => {
            let (xcode_version, xcode_build) = get_xcode_version();
            Some(PlatformBuildInfo::Macos {
                macos: MacosBuildInfo {
                    xcode_version,
                    xcode_build,
                },
            })
        }
        "android" => {
            let (ndk_version, min_sdk) = get_android_ndk_info();
            Some(PlatformBuildInfo::Android {
                android: AndroidBuildInfo {
                    ndk_version,
                    stl: "c++_shared".to_string(),
                    min_sdk_version: min_sdk,
                },
            })
        }
        _ => None,
    }
}

/// Get Xcode version and build number
fn get_xcode_version() -> (String, String) {
    let output = std::process::Command::new("xcodebuild")
        .arg("-version")
        .output()
        .ok();

    if let Some(output) = output {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let lines: Vec<&str> = stdout.lines().collect();
            if lines.len() >= 2 {
                // First line: "Xcode 16.4"
                // Second line: "Build version 16F6"
                let version = lines[0].replace("Xcode ", "");
                let build = lines[1].replace("Build version ", "");
                return (version, build);
            }
        }
    }
    ("unknown".to_string(), "unknown".to_string())
}

/// Get Android NDK version and min SDK version
///
/// Reads NDK version from source.properties file in ANDROID_NDK_HOME.
/// Returns (ndk_version, min_sdk_version).
fn get_android_ndk_info() -> (String, String) {
    // Default min SDK version
    let min_sdk = "21".to_string();

    // Try to find NDK path from environment variables
    let ndk_path = std::env::var("ANDROID_NDK_HOME")
        .or_else(|_| std::env::var("ANDROID_NDK_ROOT"))
        .or_else(|_| std::env::var("ANDROID_NDK"))
        .ok();

    if let Some(ndk_path) = ndk_path {
        let source_props = std::path::Path::new(&ndk_path).join("source.properties");
        if let Ok(content) = std::fs::read_to_string(&source_props) {
            // Parse source.properties for Pkg.Revision
            // Format: Pkg.Revision = 25.2.9519653
            for line in content.lines() {
                if line.starts_with("Pkg.Revision") {
                    if let Some(version) = line.split('=').nth(1) {
                        let version = version.trim();
                        // Extract major version for "rXX" format
                        if let Some(major) = version.split('.').next() {
                            return (format!("r{}", major), min_sdk);
                        }
                    }
                }
            }
        }
    }

    ("unknown".to_string(), min_sdk)
}

/// Print build info JSON in pyccgo format
///
/// Prints the JSON wrapped in [[==BUILD_INFO_JSON==]] markers for easy parsing
pub fn print_build_info_json(build_info: &BuildInfoFull) {
    if let Ok(json) = serde_json::to_string_pretty(build_info) {
        println!("\n[[==BUILD_INFO_JSON==]]");
        println!("{}", json);
        println!("[[==END_BUILD_INFO_JSON==]]");
    }
}

/// Archive builder for creating SDK packages
#[derive(Debug)]
pub struct ArchiveBuilder {
    /// Library name
    name: String,
    /// Version
    version: String,
    /// Publish suffix (e.g., "beta.18-dirty" or "release")
    publish_suffix: String,
    /// Whether this is a release build
    is_release: bool,
    /// Platform name
    platform: String,
    /// Staging directory for archive contents
    staging_dir: PathBuf,
    /// Output directory for final archives
    output_dir: PathBuf,
}

impl ArchiveBuilder {
    /// Create a new archive builder
    pub fn new(
        name: impl Into<String>,
        version: impl Into<String>,
        publish_suffix: impl Into<String>,
        is_release: bool,
        platform: impl Into<String>,
        output_dir: PathBuf,
    ) -> Result<Self> {
        let name = name.into();
        // Convert platform name to lowercase for consistent archive paths
        let platform = platform.into().to_lowercase();

        // Create temp staging directory
        let staging_dir = tempfile::tempdir()?.keep();

        Ok(Self {
            name,
            version: version.into(),
            publish_suffix: publish_suffix.into(),
            is_release,
            platform,
            staging_dir,
            output_dir,
        })
    }

    /// Get the staging directory path
    pub fn staging_dir(&self) -> &Path {
        &self.staging_dir
    }

    /// Add a file to the archive
    pub fn add_file(&self, source: &Path, dest_relative: &str) -> Result<()> {
        let dest = self.staging_dir.join(dest_relative);
        if let Some(parent) = dest.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::copy(source, &dest)?;
        Ok(())
    }

    /// Add a directory to the archive
    pub fn add_directory(&self, source: &Path, dest_relative: &str) -> Result<()> {
        let dest = self.staging_dir.join(dest_relative);
        copy_dir_all(source, &dest)?;
        Ok(())
    }

    /// Add a directory to the archive, filtering files by extension
    ///
    /// Only files matching the specified extensions will be copied.
    /// Extensions should not include the leading dot (e.g., "so" not ".so").
    pub fn add_directory_filtered(
        &self,
        source: &Path,
        dest_relative: &str,
        extensions: &[&str],
    ) -> Result<()> {
        let dest = self.staging_dir.join(dest_relative);
        copy_dir_filtered(source, &dest, extensions)?;
        Ok(())
    }

    /// Add files from a directory (non-recursive), filtering by extension
    ///
    /// Only copies files directly in the source directory (no subdirectory recursion).
    /// Useful for Conan builds where merged library is at root level.
    pub fn add_files_flat(
        &self,
        source: &Path,
        dest_relative: &str,
        extensions: &[&str],
    ) -> Result<()> {
        let dest = self.staging_dir.join(dest_relative);
        std::fs::create_dir_all(&dest)?;

        for entry in std::fs::read_dir(source)? {
            let entry = entry?;
            let path = entry.path();

            // Skip directories - only copy files at root level
            if path.is_dir() {
                continue;
            }

            // Check if file matches any of the allowed extensions
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let matches = extensions.iter().any(|ext| {
                if let Some(file_ext) = path.extension().and_then(|e| e.to_str()) {
                    if file_ext == *ext {
                        return true;
                    }
                }
                // Also check if filename contains .ext (for versioned libs like .so.1)
                file_name.contains(&format!(".{}.", ext)) || file_name.contains(&format!(".{}", ext))
            });

            if matches {
                let dest_path = dest.join(entry.file_name());
                std::fs::copy(&path, &dest_path)?;
            }
        }
        Ok(())
    }

    /// Create the SDK archive with build_info.json and archive_info.json
    ///
    /// Archive structure follows Python ccgo format:
    /// ```text
    /// meta/{platform}/
    /// ├── build_info.json   # Build metadata
    /// └── archive_info.json # File checksums
    /// ```
    pub fn create_sdk_archive(&self, architectures: &[String], link_type: &str) -> Result<PathBuf> {
        self.create_sdk_archive_with_toolchain(architectures, link_type, None)
    }

    /// Create the SDK archive with build_info.json, archive_info.json, and optional toolchain
    pub fn create_sdk_archive_with_toolchain(
        &self,
        architectures: &[String],
        link_type: &str,
        toolchain: Option<&str>,
    ) -> Result<PathBuf> {
        // Create meta/{platform}/ directory
        let meta_dir = self.staging_dir.join(ARCHIVE_DIR_META).join(&self.platform);
        std::fs::create_dir_all(&meta_dir)?;

        // Collect file metadata BEFORE adding meta files (NO checksums!)
        let file_metadata = collect_file_metadata(&self.staging_dir)?;

        // Create build_info.json in meta/{platform}/
        let build_info = create_build_info(
            &self.name,
            &self.version,
            &self.platform,
            architectures,
            link_type,
            toolchain,
        );
        let build_info_path = meta_dir.join(BUILD_INFO_FILE);
        write_build_info(&build_info, &build_info_path)?;

        // Create archive first to get its size
        // Format: {PROJECT}_{PLATFORM}_SDK-{version}[-{publish_suffix}].zip
        // Example: CCGONOW_ANDROID_SDK-1.0.2-beta.18-dirty.zip
        // Example: CCGONOW_ANDROID_SDK-1.0.2.zip (when publish_suffix is empty)
        let archive_name = if self.publish_suffix.is_empty() {
            format!(
                "{}_{}_SDK-{}.zip",
                self.name.to_uppercase(),
                self.platform.to_uppercase(),
                self.version
            )
        } else {
            format!(
                "{}_{}_SDK-{}-{}.zip",
                self.name.to_uppercase(),
                self.platform.to_uppercase(),
                self.version,
                self.publish_suffix
            )
        };
        let archive_path = self.output_dir.join(&archive_name);
        std::fs::create_dir_all(&self.output_dir)?;

        // Create temp archive to get size (will recreate with archive_info.json)
        create_archive(&self.staging_dir, &archive_path)?;
        let archive_size = std::fs::metadata(&archive_path)?.len();

        // Create archive_info.json in meta/{platform}/
        let archive_info = create_archive_info(
            &archive_name,
            archive_size,
            file_metadata,
            &self.platform,
            architectures,
        );
        let archive_info_path = meta_dir.join(ARCHIVE_INFO_FILE);
        write_archive_info(&archive_info, &archive_info_path)?;

        // Recreate archive with archive_info.json included
        create_archive(&self.staging_dir, &archive_path)?;

        Ok(archive_path)
    }

    /// Get the library path with platform subdirectory
    ///
    /// Format: lib/{platform}/{link_type}/[{arch}/]{lib_name}
    pub fn lib_path(&self, link_type: &str, arch: Option<&str>, lib_name: &str) -> String {
        let mut parts = vec![ARCHIVE_DIR_LIB, &self.platform, link_type];
        if let Some(a) = arch {
            parts.push(a);
        }
        parts.push(lib_name);
        parts.join("/")
    }

    /// Get the static library archive path
    pub fn static_lib_path(&self, arch: Option<&str>, lib_name: &str) -> String {
        self.lib_path(ARCHIVE_DIR_STATIC, arch, lib_name)
    }

    /// Get the shared library archive path
    pub fn shared_lib_path(&self, arch: Option<&str>, lib_name: &str) -> String {
        self.lib_path(ARCHIVE_DIR_SHARED, arch, lib_name)
    }

    /// Create the symbols archive
    pub fn create_symbols_archive(&self, symbols_dir: &Path) -> Result<PathBuf> {
        // Format: {PROJECT}_{PLATFORM}_SDK-{version}[-{publish_suffix}]-SYMBOLS.zip
        // Example: CCGONOW_ANDROID_SDK-1.0.2-beta.18-dirty-SYMBOLS.zip
        // Example: CCGONOW_ANDROID_SDK-1.0.2-SYMBOLS.zip (when publish_suffix is empty)
        let archive_name = if self.publish_suffix.is_empty() {
            format!(
                "{}_{}_SDK-{}-SYMBOLS.zip",
                self.name.to_uppercase(),
                self.platform.to_uppercase(),
                self.version
            )
        } else {
            format!(
                "{}_{}_SDK-{}-{}-SYMBOLS.zip",
                self.name.to_uppercase(),
                self.platform.to_uppercase(),
                self.version,
                self.publish_suffix
            )
        };
        let archive_path = self.output_dir.join(&archive_name);

        std::fs::create_dir_all(&self.output_dir)?;
        create_archive(symbols_dir, &archive_path)?;

        Ok(archive_path)
    }

    /// Clean up the staging directory
    pub fn cleanup(self) -> Result<()> {
        std::fs::remove_dir_all(&self.staging_dir).ok();
        Ok(())
    }
}

/// Copy a directory recursively, excluding files in ARCHIVE_EXCLUDE_FILES
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let file_name = entry.file_name();
        let file_name_str = file_name.to_string_lossy();
        let dest_path = dst.join(&file_name);

        if path.is_dir() {
            copy_dir_all(&path, &dest_path)?;
        } else {
            // Skip excluded files (CPPLINT.cfg, .clang-format, .clang-tidy)
            if should_include_file_in_archive(&file_name_str) {
                std::fs::copy(&path, &dest_path)?;
            }
        }
    }
    Ok(())
}

/// Copy a directory recursively, filtering files by extension
///
/// Only copies files that match one of the specified extensions.
/// Also handles symlinks for versioned shared libraries (e.g., libfoo.so -> libfoo.so.1).
fn copy_dir_filtered(src: &Path, dst: &Path, extensions: &[&str]) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if path.is_dir() {
            copy_dir_filtered(&path, &dest_path, extensions)?;
        } else {
            // Check if file matches any of the allowed extensions
            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let matches = extensions.iter().any(|ext| {
                // Check exact extension match (e.g., .a, .so)
                if let Some(file_ext) = path.extension().and_then(|e| e.to_str()) {
                    if file_ext == *ext {
                        return true;
                    }
                }
                // Also check if filename contains .ext (for versioned libs like .so.1)
                file_name.contains(&format!(".{}.", ext)) || file_name.contains(&format!(".{}", ext))
            });

            if matches {
                std::fs::copy(&path, &dest_path)?;
            }
        }
    }
    Ok(())
}

/// Detect archive format by reading magic bytes
fn detect_archive_format(path: &Path) -> Result<ArchiveFormat> {
    let mut file = File::open(path)
        .with_context(|| format!("Failed to open archive: {}", path.display()))?;
    let mut header = [0u8; 4];
    file.read_exact(&mut header).ok();

    // ZIP magic: PK (0x50 0x4B)
    if &header[..2] == b"PK" {
        return Ok(ArchiveFormat::Zip);
    }
    // GZIP magic: 0x1F 0x8B
    if &header[..2] == [0x1F, 0x8B] {
        return Ok(ArchiveFormat::TarGz);
    }

    Ok(ArchiveFormat::Unknown)
}

/// Archive format types
#[derive(Debug, PartialEq)]
enum ArchiveFormat {
    Zip,
    TarGz,
    Unknown,
}

/// Print the tree structure of an archive (ZIP or tar.gz) with library info
///
/// Automatically detects archive format:
/// - ZIP format (.zip, .aar): Standard ZIP handling
/// - GZIP/tar.gz format (.har, .tar.gz, .tgz): tar.gz handling
///
/// For library files (.so, .a, .dylib, etc.), extracts and displays:
/// - Architecture (e.g., aarch64, x86_64)
/// - NDK version (Android only, e.g., NDK r27)
/// - API level (Android only, e.g., API 24)
///
/// Example output:
/// ```text
///     ZIP contents:
///     ├── lib/
///     │   └── android/
///     │       ├── shared/
///     │       │   └── arm64-v8a/
///     │       │       └── libccgonow.so (75.36 KB) [aarch64, NDK r27, API 24]
///     │       └── static/
///     │           └── arm64-v8a/
///     │               └── libccgonow.a (116.83 KB) [aarch64]
///     └── meta/
///         └── Android/
///             ├── build_info.json (0.29 KB)
///             └── archive_info.json (0.61 KB)
/// ```
pub fn print_zip_tree(archive_path: &Path, indent: &str) -> Result<()> {
    // Detect archive format
    let format = detect_archive_format(archive_path)?;

    match format {
        ArchiveFormat::Zip => print_zip_tree_impl(archive_path, indent),
        ArchiveFormat::TarGz => print_targz_tree(archive_path, indent),
        ArchiveFormat::Unknown => {
            // Try ZIP first, then tar.gz
            if print_zip_tree_impl(archive_path, indent).is_ok() {
                Ok(())
            } else {
                print_targz_tree(archive_path, indent)
            }
        }
    }
}

/// Print the tree structure of a ZIP archive
fn print_zip_tree_impl(archive_path: &Path, indent: &str) -> Result<()> {
    use super::elf::{get_library_info, is_library_file};
    use zip::ZipArchive;

    let file = File::open(archive_path)
        .with_context(|| format!("Failed to open archive: {}", archive_path.display()))?;

    let mut zip = ZipArchive::new(file)
        .with_context(|| format!("Failed to read ZIP archive: {}", archive_path.display()))?;

    // First pass: collect file info and library data
    let mut file_infos: Vec<(String, u64, String)> = Vec::new(); // (path, size, lib_info)

    for i in 0..zip.len() {
        let mut entry = zip.by_index(i)?;
        let path = entry.name().to_string();
        let size = entry.size();

        // Skip directories (entries ending with /)
        if path.ends_with('/') {
            continue;
        }

        // Extract library info if this is a library file
        let filename = path.split('/').last().unwrap_or(&path);
        let lib_info = if is_library_file(filename, &path) {
            // Read the file data for analysis
            let mut data = Vec::with_capacity(size as usize);
            if entry.read_to_end(&mut data).is_ok() {
                get_library_info(&data, filename, &path).to_display_string()
            } else {
                String::new()
            }
        } else {
            String::new()
        };

        file_infos.push((path, size, lib_info));
    }

    // Build directory tree structure with collected info
    let mut tree: BTreeMap<String, TreeNode> = BTreeMap::new();

    for (path, size, lib_info) in file_infos {
        let parts: Vec<&str> = path.split('/').collect();
        let mut current = &mut tree;

        for (idx, part) in parts.iter().enumerate() {
            let is_last = idx == parts.len() - 1;

            if is_last {
                // This is a file with library info
                current.insert(
                    part.to_string(),
                    TreeNode::File {
                        size,
                        lib_info: lib_info.clone(),
                    },
                );
            } else {
                // This is a directory - use entry API to get mutable reference
                current = match current
                    .entry(part.to_string())
                    .or_insert_with(|| TreeNode::Dir(BTreeMap::new()))
                {
                    TreeNode::Dir(children) => children,
                    _ => unreachable!("We just inserted a Dir"),
                };
            }
        }
    }

    // Print tree structure with library info
    eprintln!("{}ZIP contents:", indent);
    print_tree_level(&tree, indent, "");

    Ok(())
}

/// Tree node type for directory structure
enum TreeNode {
    /// File with size and optional library info string (e.g., " [aarch64, NDK r27, API 24]")
    File { size: u64, lib_info: String },
    Dir(BTreeMap<String, TreeNode>),
}

/// Recursively print a level of the tree structure with library info
fn print_tree_level(tree: &BTreeMap<String, TreeNode>, base_indent: &str, prefix: &str) {
    let items: Vec<_> = tree.iter().collect();
    let len = items.len();

    for (i, (name, node)) in items.iter().enumerate() {
        let is_last = i == len - 1;
        let connector = if is_last { "└── " } else { "├── " };

        match node {
            TreeNode::File { size, lib_info } => {
                // Format size - use MB for files >= 0.01 MB (10.24 KB), KB otherwise
                let size_mb = *size as f64 / (1024.0 * 1024.0);
                let size_str = if size_mb >= 0.01 {
                    format!("({:.2} MB)", size_mb)
                } else {
                    let size_kb = *size as f64 / 1024.0;
                    format!("({:.1} KB)", size_kb)
                };

                eprintln!(
                    "{}{}{}{} {}{}",
                    base_indent, prefix, connector, name, size_str, lib_info
                );
            }
            TreeNode::Dir(children) => {
                eprintln!("{}{}{}{}/", base_indent, prefix, connector, name);

                // Recurse into subdirectory
                let new_prefix = if is_last {
                    format!("{}    ", prefix)
                } else {
                    format!("{}│   ", prefix)
                };

                print_tree_level(children, base_indent, &new_prefix);
            }
        }
    }
}

/// Print the tree structure of a tar.gz archive (used by OHOS HAR files)
fn print_targz_tree(archive_path: &Path, indent: &str) -> Result<()> {
    use flate2::read::GzDecoder;
    use tar::Archive;

    let file = File::open(archive_path)
        .with_context(|| format!("Failed to open archive: {}", archive_path.display()))?;

    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);

    // Collect file info (no library info extraction for tar.gz - would need to extract files)
    let mut file_infos: Vec<(String, u64)> = Vec::new();

    for entry in archive.entries()? {
        let entry = entry?;
        let path = entry.path()?.to_string_lossy().to_string();
        let size = entry.size();

        // Skip directories
        if entry.header().entry_type().is_dir() {
            continue;
        }

        file_infos.push((path, size));
    }

    // Build directory tree structure
    let mut tree: BTreeMap<String, TreeNode> = BTreeMap::new();

    for (path, size) in file_infos {
        let parts: Vec<&str> = path.split('/').collect();
        let mut current = &mut tree;

        for (idx, part) in parts.iter().enumerate() {
            if part.is_empty() {
                continue;
            }

            let is_last = idx == parts.len() - 1;

            if is_last {
                current.insert(
                    part.to_string(),
                    TreeNode::File {
                        size,
                        lib_info: String::new(), // No lib info for tar.gz (would need extraction)
                    },
                );
            } else {
                current = match current
                    .entry(part.to_string())
                    .or_insert_with(|| TreeNode::Dir(BTreeMap::new()))
                {
                    TreeNode::Dir(children) => children,
                    _ => unreachable!("We just inserted a Dir"),
                };
            }
        }
    }

    // Print tree structure
    eprintln!("{}Archive contents:", indent);
    print_tree_level(&tree, indent, "");

    Ok(())
}
