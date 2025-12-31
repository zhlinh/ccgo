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

use super::BuildInfo;

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
        // Format: {PROJECT}_{PLATFORM}_SDK-{version}-{publish_suffix}.zip
        // Example: CCGONOW_ANDROID_SDK-1.0.2-beta.18-dirty.zip
        let archive_name = format!(
            "{}_{}_SDK-{}-{}.zip",
            self.name.to_uppercase(),
            self.platform.to_uppercase(),
            self.version,
            self.publish_suffix
        );
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
        // Format: {PROJECT}_{PLATFORM}_SDK-{version}-{publish_suffix}-SYMBOLS.zip
        // Example: CCGONOW_ANDROID_SDK-1.0.2-beta.18-dirty-SYMBOLS.zip
        let archive_name = format!(
            "{}_{}_SDK-{}-{}-SYMBOLS.zip",
            self.name.to_uppercase(),
            self.platform.to_uppercase(),
            self.version,
            self.publish_suffix
        );
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

/// Copy a directory recursively
fn copy_dir_all(src: &Path, dst: &Path) -> Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let path = entry.path();
        let dest_path = dst.join(entry.file_name());

        if path.is_dir() {
            copy_dir_all(&path, &dest_path)?;
        } else {
            std::fs::copy(&path, &dest_path)?;
        }
    }
    Ok(())
}

/// Print the tree structure of a ZIP archive
///
/// Example output:
/// ```text
///     ZIP contents:
///     ├── lib/
///     │   └── android/
///     │       ├── shared/
///     │       │   └── arm64-v8a/
///     │       │       └── libccgonow.so (75.36 KB)
///     │       └── static/
///     │           └── arm64-v8a/
///     │               └── libccgonow.a (116.83 KB)
///     └── meta/
///         └── Android/
///             ├── build_info.json (0.29 KB)
///             └── archive_info.json (0.61 KB)
/// ```
pub fn print_zip_tree(archive_path: &Path, indent: &str) -> Result<()> {
    use zip::ZipArchive;

    let file = File::open(archive_path)
        .with_context(|| format!("Failed to open archive: {}", archive_path.display()))?;

    let mut zip = ZipArchive::new(file)
        .with_context(|| format!("Failed to read ZIP archive: {}", archive_path.display()))?;

    // Build directory tree structure
    let mut tree: BTreeMap<String, TreeNode> = BTreeMap::new();

    for i in 0..zip.len() {
        let file = zip.by_index(i)?;
        let path = file.name();

        // Skip directories (entries ending with /)
        if path.ends_with('/') {
            continue;
        }

        let parts: Vec<&str> = path.split('/').collect();
        let mut current = &mut tree;

        for (idx, part) in parts.iter().enumerate() {
            let is_last = idx == parts.len() - 1;

            if is_last {
                // This is a file
                current.insert(
                    part.to_string(),
                    TreeNode::File {
                        size: file.size(),
                        path: path.to_string(),
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

    // Print tree structure
    eprintln!("{}ZIP contents:", indent);
    print_tree_level(&tree, indent, "", true);

    Ok(())
}

/// Tree node type for directory structure
enum TreeNode {
    File { size: u64, path: String },
    Dir(BTreeMap<String, TreeNode>),
}

/// Recursively print a level of the tree structure
fn print_tree_level(
    tree: &BTreeMap<String, TreeNode>,
    base_indent: &str,
    prefix: &str,
    _is_root: bool,
) {
    let items: Vec<_> = tree.iter().collect();
    let len = items.len();

    for (i, (name, node)) in items.iter().enumerate() {
        let is_last = i == len - 1;
        let connector = if is_last { "└── " } else { "├── " };

        match node {
            TreeNode::File { size, .. } => {
                // Format size
                let size_str = if size >= &(1024 * 1024) {
                    format!("{:.2} MB", *size as f64 / (1024.0 * 1024.0))
                } else if size >= &1024 {
                    format!("{:.2} KB", *size as f64 / 1024.0)
                } else {
                    format!("{} B", size)
                };

                eprintln!("{}{}{}{} ({})", base_indent, prefix, connector, name, size_str);
            }
            TreeNode::Dir(children) => {
                eprintln!("{}{}{}{}/", base_indent, prefix, connector, name);

                // Recurse into subdirectory
                let new_prefix = if is_last {
                    format!("{}    ", prefix)
                } else {
                    format!("{}│   ", prefix)
                };

                print_tree_level(children, base_indent, &new_prefix, false);
            }
        }
    }
}
