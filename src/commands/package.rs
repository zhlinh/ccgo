//! Package command implementation

use anyhow::{anyhow, Context, Result};
use chrono::Local;
use clap::Args;
use console::style;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;
use zip::{write::FileOptions, ZipArchive, ZipWriter};

/// CCGO.toml project configuration
#[derive(Debug, Deserialize)]
struct CcgoConfig {
    project: ProjectInfo,
}

#[derive(Debug, Deserialize)]
struct ProjectInfo {
    name: String,
    version: Option<String>,
}

/// Archive metadata
#[derive(Debug, Serialize)]
struct ArchiveInfo {
    project_name: String,
    version: String,
    platforms: Vec<String>,
    merged: bool,
    created_at: String,
}

/// Find CCGO.toml in project directory or subdirectories
fn find_ccgo_toml(start_dir: &Path) -> Result<PathBuf> {
    // First check current directory
    let config_file = start_dir.join("CCGO.toml");
    if config_file.is_file() {
        return Ok(config_file);
    }

    // Then check immediate subdirectories
    if let Ok(entries) = fs::read_dir(start_dir) {
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let config_file = entry.path().join("CCGO.toml");
            if config_file.is_file() {
                return Ok(config_file);
            }
        }
    }

    Err(anyhow!(
        "CCGO.toml not found!\n\n\
         Current directory: {}\n\n\
         The 'ccgo package' command must be run from a CCGO project directory\n\
         (a directory containing CCGO.toml or with a subdirectory containing it).\n\n\
         Please navigate to your project directory and try again:\n\
         $ cd /path/to/your-project\n\
         $ ccgo package",
        start_dir.display()
    ))
}

/// Get project name from CCGO.toml
fn get_project_name(config_file: &Path) -> Result<String> {
    let content = fs::read_to_string(config_file)
        .context("Failed to read CCGO.toml")?;
    let config: CcgoConfig = toml::from_str(&content)
        .context("Failed to parse CCGO.toml")?;
    Ok(config.project.name)
}

/// Get version from args, git, or CCGO.toml
fn get_version(config_file: &Path, version_arg: Option<&str>, release: bool) -> String {
    // Use provided version argument
    if let Some(version) = version_arg {
        return version.to_string();
    }

    // Try git tag with full version info (--long --dirty for beta and dirty status)
    let project_dir = config_file.parent().unwrap();
    if let Ok(output) = Command::new("git")
        .args(["describe", "--tags", "--always", "--long", "--dirty"])
        .current_dir(project_dir)
        .output()
    {
        if output.status.success() {
            if let Ok(git_version) = String::from_utf8(output.stdout) {
                let git_version = git_version.trim();
                if !git_version.is_empty() {
                    // Parse git describe output: v1.0.2-18-g<hash>-dirty
                    // Format: <tag>-<commits>-g<hash>[-dirty]
                    return format_version_from_git(git_version, release);
                }
            }
        }
    }

    // Try CCGO.toml
    if let Ok(content) = fs::read_to_string(config_file) {
        if let Ok(config) = toml::from_str::<CcgoConfig>(&content) {
            if let Some(version) = config.project.version {
                return version;
            }
        }
    }

    // Default to date
    Local::now().format("%Y%m%d").to_string()
}

/// Format version from git describe output
/// Input: v1.0.2-18-g<hash>-dirty or v1.0.2-0-g<hash>
/// Output (debug): 1.0.2-beta.18-dirty or 1.0.2 (if on exact tag)
/// Output (release): 1.0.2-release
fn format_version_from_git(git_version: &str, release: bool) -> String {
    let mut parts: Vec<&str> = git_version.split('-').collect();
    let is_dirty = parts.last() == Some(&"dirty");
    if is_dirty {
        parts.pop(); // Remove "dirty"
    }

    // Extract base version (strip 'v' prefix)
    let base_version = parts[0].strip_prefix('v').unwrap_or(parts[0]);

    // For release builds, always use -release suffix
    if release {
        return format!("{}-release", base_version);
    }

    // For debug builds, extract commit count
    if parts.len() >= 3 {
        // parts: ["v1.0.2", "18", "g<hash>"]
        let commits = parts[1];

        // If commits is 0, we're exactly on a tag
        if commits == "0" {
            if is_dirty {
                format!("{}-dirty", base_version)
            } else {
                base_version.to_string()
            }
        } else {
            // Not on exact tag, use beta.N format
            if is_dirty {
                format!("{}-beta.{}-dirty", base_version, commits)
            } else {
                format!("{}-beta.{}", base_version, commits)
            }
        }
    } else {
        // Fallback: just return base version
        if is_dirty {
            format!("{}-dirty", base_version)
        } else {
            base_version.to_string()
        }
    }
}

/// Find all ZIP files for a platform in target/debug/<platform>/ or target/release/<platform>/
fn find_platform_zips(project_dir: &Path, platform: &str, release: bool) -> Vec<PathBuf> {
    let target_dir = project_dir.join("target");
    if !target_dir.exists() {
        return Vec::new();
    }

    let mut zip_files = Vec::new();
    let platform_upper = platform.to_uppercase();

    // Determine build type based on release flag (matching ccgo build behavior)
    let build_type = if release { "release" } else { "debug" };

    // Search in target/<build_type>/<platform>
    let platform_dir = target_dir.join(build_type).join(platform);
    if platform_dir.exists() {
        for entry in WalkDir::new(&platform_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                if let Some(filename) = path.file_name() {
                    let filename_str = filename.to_string_lossy();
                    // Match ZIP files for this platform (e.g., CCGONOW_ANDROID_SDK-*.zip)
                    if filename_str.ends_with(".zip")
                        && !filename_str.starts_with("ARCHIVE")
                        && filename_str.to_uppercase().contains(&platform_upper)
                    {
                        zip_files.push(path.to_path_buf());
                    }
                }
            }
        }
    }

    // Also check legacy target/<platform> (for backward compatibility)
    let legacy_platform_dir = target_dir.join(platform);
    if legacy_platform_dir.exists() {
        for entry in WalkDir::new(&legacy_platform_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                if let Some(filename) = path.file_name() {
                    let filename_str = filename.to_string_lossy();
                    if filename_str.ends_with(".zip")
                        && !filename_str.starts_with("ARCHIVE")
                        && filename_str.to_uppercase().contains(&platform_upper)
                    {
                        zip_files.push(path.to_path_buf());
                    }
                }
            }
        }
    }

    zip_files
}

/// Merge multiple ZIP files into a single unified SDK ZIP
fn merge_zips(
    zip_files: &[PathBuf],
    output_zip_path: &Path,
    project_name: &str,
    version: &str,
) -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("Merging ZIP files into unified SDK");
    println!("{}", "=".repeat(80));

    // Create temporary directory for extraction
    let temp_dir = tempfile::tempdir()?;
    let merged_dir = temp_dir.path().join("merged");
    fs::create_dir_all(&merged_dir)?;

    let mut platforms_merged = HashSet::new();

    for zip_path in zip_files {
        let filename = zip_path.file_name().unwrap().to_string_lossy();
        println!("   📦 Processing: {}", filename);

        let file = fs::File::open(zip_path)?;
        let mut archive = ZipArchive::new(file)?;

        for i in 0..archive.len() {
            let mut file = archive.by_index(i)?;
            let outpath = merged_dir.join(file.name());

            if file.name().ends_with('/') {
                fs::create_dir_all(&outpath)?;
            } else {
                if let Some(p) = outpath.parent() {
                    fs::create_dir_all(p)?;
                }

                // Skip if file already exists (don't overwrite)
                if !outpath.exists() {
                    let mut outfile = fs::File::create(&outpath)?;
                    std::io::copy(&mut file, &mut outfile)?;
                }
            }
        }

        // Detect platform from filename
        let fname_lower = filename.to_lowercase();
        for plat in &[
            "android", "ios", "macos", "watchos", "tvos", "windows", "linux", "ohos", "kmp",
            "conan", "include",
        ] {
            if fname_lower.contains(plat) {
                platforms_merged.insert(plat.to_string());
                break;
            }
        }
    }

    // Create unified archive_info.json
    let platforms: Vec<String> = platforms_merged.into_iter().collect();
    let archive_info = ArchiveInfo {
        project_name: project_name.to_string(),
        version: version.to_string(),
        platforms,
        merged: true,
        created_at: Local::now().to_rfc3339(),
    };

    let meta_dir = merged_dir.join("meta");
    fs::create_dir_all(&meta_dir)?;
    let archive_info_path = meta_dir.join("archive_info.json");
    let archive_info_json = serde_json::to_string_pretty(&archive_info)?;
    fs::write(archive_info_path, archive_info_json)?;

    // Create the merged ZIP
    println!("\n   📦 Creating merged SDK ZIP...");

    let file = fs::File::create(output_zip_path)?;
    let mut zip = ZipWriter::new(file);
    let options: FileOptions<()> = FileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated)
        .unix_permissions(0o755);

    for entry in WalkDir::new(&merged_dir) {
        let entry = entry?;
        let path = entry.path();
        let name = path.strip_prefix(&merged_dir)?;

        if path.is_file() {
            zip.start_file(name.to_string_lossy().to_string(), options)?;
            let mut f = fs::File::open(path)?;
            let mut buffer = Vec::new();
            f.read_to_end(&mut buffer)?;
            zip.write_all(&buffer)?;
        }
    }

    zip.finish()?;

    let size_mb = fs::metadata(output_zip_path)?.len() as f64 / (1024.0 * 1024.0);
    println!(
        "   ✅ Created: {} ({:.2} MB)",
        output_zip_path.file_name().unwrap().to_string_lossy(),
        size_mb
    );
    println!("   📍 Location: {}", output_zip_path.display());

    Ok(())
}

/// Print contents of a ZIP file
fn print_zip_contents(zip_path: &Path, indent: &str) -> Result<()> {
    let file = fs::File::open(zip_path)?;
    let mut archive = ZipArchive::new(file)?;

    for i in 0..archive.len() {
        let file = archive.by_index(i)?;
        if !file.name().ends_with('/') {
            println!("{}├── {}", indent, file.name());
        }
    }

    Ok(())
}

/// Package SDK for distribution
#[derive(Args, Debug)]
pub struct PackageCommand {
    /// SDK version (default: auto-detect from git or CCGO.toml)
    #[arg(long)]
    pub version: Option<String>,

    /// Output directory for packaged SDK (default: ./target/debug/package or ./target/release/package)
    #[arg(long)]
    pub output: Option<String>,

    /// Comma-separated platforms to include (default: all built platforms)
    #[arg(long)]
    pub platforms: Option<String>,

    /// Keep individual ZIP files instead of merging into one unified SDK ZIP
    #[arg(long)]
    pub no_merge: bool,

    /// Package release builds (default: debug builds)
    #[arg(long)]
    pub release: bool,
}

impl PackageCommand {
    /// Execute the package command
    pub fn execute(self, _verbose: bool) -> Result<()> {
        println!("{}", "=".repeat(80));
        println!("CCGO Package - Collect Build Artifacts");
        println!("{}", "=".repeat(80));

        // Get current working directory
        let project_dir = std::env::current_dir()
            .context("Failed to get current working directory")?;

        // Find CCGO.toml
        let config_file = find_ccgo_toml(&project_dir)?;

        // Get project info
        let project_name = get_project_name(&config_file)?;
        let version = get_version(&config_file, self.version.as_deref(), self.release);

        // Determine build mode and default output path
        let build_mode = if self.release { "release" } else { "debug" };
        let default_output = format!("./target/{}/package", build_mode);

        // Convert output path to absolute path
        let output_str = self.output.as_deref().unwrap_or(&default_output);
        let output_path = if Path::new(output_str).is_absolute() {
            PathBuf::from(output_str)
        } else {
            project_dir.join(output_str)
        };

        let merge_mode = !self.no_merge;
        let mode_str = if merge_mode {
            "Merge into unified SDK"
        } else {
            "Keep individual ZIPs"
        };

        println!("\nProject: {}", project_name);
        println!("Version: {}", version);
        println!("Build Mode: {}", build_mode);
        println!("Output: {}", output_path.display());
        println!("Mode: {}", mode_str);

        // Clean output directory
        if output_path.exists() {
            println!("\n🧹 Cleaning output directory...");
            fs::remove_dir_all(&output_path)?;
        }

        // Create output directory
        fs::create_dir_all(&output_path)?;

        println!("\n{}", "=".repeat(80));
        println!("Scanning Build Artifacts");
        println!("{}", "=".repeat(80));

        // Define platforms to scan
        let platforms: Vec<String> = if let Some(platforms_str) = &self.platforms {
            platforms_str.split(',').map(|s| s.trim().to_string()).collect()
        } else {
            vec![
                "android", "ios", "macos", "tvos", "watchos", "windows", "linux", "ohos", "conan",
                "kmp",
            ]
            .into_iter()
            .map(String::from)
            .collect()
        };

        let mut collected_platforms = Vec::new();
        let mut failed_platforms = Vec::new();
        let mut all_zip_files = Vec::new();

        for platform in &platforms {
            let zip_files = find_platform_zips(&project_dir, platform, self.release);
            if !zip_files.is_empty() {
                collected_platforms.push(platform.clone());
                for zf in &zip_files {
                    println!("   ✓ Found: {}", zf.file_name().unwrap().to_string_lossy());
                }
                all_zip_files.extend(zip_files);
            } else {
                failed_platforms.push(platform.clone());
            }
        }

        // Check if any artifacts were found
        if collected_platforms.is_empty() {
            println!("\n{}", "=".repeat(80));
            println!("⚠️  WARNING: No platform artifacts found!");
            println!("{}", "=".repeat(80));
            println!("\nIt looks like no platforms have been built yet in {} mode.", build_mode);
            println!("\nTo build platforms, use:");
            if self.release {
                println!("  ccgo build android --release");
                println!("  ccgo build ios --release");
                println!("  ccgo build all --release");
            } else {
                println!("  ccgo build android");
                println!("  ccgo build ios");
                println!("  ccgo build all");
            }
            println!("\nThen run 'ccgo package{}' again.\n", if self.release { " --release" } else { "" });
            return Err(anyhow!("No platform artifacts found in {} mode", build_mode));
        }

        if merge_mode {
            // Merge all ZIPs into one unified SDK ZIP
            // Strip leading 'v' from version if present (e.g., v1.0.2 -> 1.0.2)
            let version_clean = version.strip_prefix('v').unwrap_or(&version);
            let sdk_zip_name = format!("{}_SDK-{}.zip", project_name.to_uppercase(), version_clean);
            let sdk_zip_path = output_path.join(&sdk_zip_name);

            merge_zips(&all_zip_files, &sdk_zip_path, &project_name, &version)?;

            // Print summary
            println!("\n{}", "=".repeat(80));
            println!("Package Summary");
            println!("{}", "=".repeat(80));
            println!("\nPlatforms merged: {}", collected_platforms.join(", "));
            println!("Output: {}", sdk_zip_path.display());

            let size_mb = fs::metadata(&sdk_zip_path)?.len() as f64 / (1024.0 * 1024.0);
            println!("Size: {:.2} MB", size_mb);
        } else {
            // Copy individual ZIPs to output directory
            println!("\n{}", "=".repeat(80));
            println!("Copying Individual ZIP Files");
            println!("{}", "=".repeat(80));

            let mut copied_files = Vec::new();
            for zip_file in &all_zip_files {
                let filename = zip_file.file_name().unwrap();
                let dest_path = output_path.join(filename);
                fs::copy(zip_file, &dest_path)?;
                let size_mb = fs::metadata(&dest_path)?.len() as f64 / (1024.0 * 1024.0);
                println!("   ✓ {} ({:.2} MB)", filename.to_string_lossy(), size_mb);
                copied_files.push(filename.to_string_lossy().to_string());
            }

            // Print summary
            println!("\n{}", "=".repeat(80));
            println!("Package Summary");
            println!("{}", "=".repeat(80));
            println!("\nOutput Directory: {}", output_path.display());
            println!("\nCopied {} artifact(s):", copied_files.len());
            println!("{}", "-".repeat(60));
            for f in &copied_files {
                let file_path = output_path.join(f);
                let size_mb = fs::metadata(&file_path)?.len() as f64 / (1024.0 * 1024.0);
                println!("  {} ({:.2} MB)", f, size_mb);
            }
            println!("{}", "-".repeat(60));
        }

        // Platform status
        println!("\n{}", "=".repeat(80));
        println!("Platform Status");
        println!("{}", "=".repeat(80));
        println!();

        for platform in &collected_platforms {
            println!("  {} {}", style("✅").green(), platform.to_uppercase());
        }
        for platform in &failed_platforms {
            println!(
                "  {} {} (not built)",
                style("⚠️").yellow(),
                platform.to_uppercase()
            );
        }

        let total_platforms = collected_platforms.len() + failed_platforms.len();
        println!(
            "\nTotal: {}/{} platform(s)",
            collected_platforms.len(),
            total_platforms
        );
        println!("{}", "=".repeat(80));

        // Print package contents
        println!("\n{}", "=".repeat(80));
        println!("Package Contents");
        println!("{}", "=".repeat(80));
        println!("\n📁 {}/", output_path.display());

        if let Ok(entries) = fs::read_dir(&output_path) {
            let mut items: Vec<_> = entries.filter_map(|e| e.ok()).collect();
            items.sort_by_key(|e| e.file_name());

            for entry in items {
                let path = entry.path();
                if path.is_file() {
                    let filename = path.file_name().unwrap().to_string_lossy();
                    let size_mb = fs::metadata(&path)?.len() as f64 / (1024.0 * 1024.0);
                    println!("   📦 {} ({:.2} MB)", filename, size_mb);

                    // If it's a ZIP file, print its contents
                    if filename.ends_with(".zip") {
                        if let Err(e) = print_zip_contents(&path, "      ") {
                            println!("      Error reading ZIP contents: {}", e);
                        }
                    }
                }
            }
        }

        println!("\n{}", "=".repeat(80));
        println!("\n{}", style("✅ Package complete!").green().bold());
        println!("   Output: {}\n", output_path.display());

        Ok(())
    }
}
