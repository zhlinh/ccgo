//! Package command implementation

use anyhow::{anyhow, Context, Result};
use chrono::Local;
use clap::Args;
use console::style;
use serde::Serialize;
use std::collections::HashSet;
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;
use zip::{write::FileOptions, ZipArchive, ZipWriter};

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
    let config: crate::config::CcgoConfig = toml::from_str(&content)
        .context("Failed to parse CCGO.toml")?;
    let pkg = config.package.context("CCGO.toml must have a [package] section")?;
    Ok(pkg.name)
}

/// Get version from args, git, or CCGO.toml
/// Resolve the version string for packaging.
///
/// Cargo-aligned semantics (Plan A):
///   * --version argument wins if provided.
///   * Otherwise read `[package].version` from CCGO.toml.
///   * Git state (tag/commits/dirty) is NOT consulted — the author decides
///     the version by editing CCGO.toml, just like Cargo.toml.
fn get_version(config_file: &Path, version_arg: Option<&str>, _release: bool) -> String {
    if let Some(version) = version_arg {
        return version.to_string();
    }

    if let Ok(content) = fs::read_to_string(config_file) {
        if let Ok(config) = toml::from_str::<crate::config::CcgoConfig>(&content) {
            if let Some(pkg) = config.package {
                return pkg.version;
            }
        }
    }

    // Fallback when CCGO.toml is missing/invalid — keep previous date format
    // so packaging still produces a named artifact instead of crashing.
    Local::now().format("%Y%m%d").to_string()
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

/// Generate a minimal CCGO.toml string to embed at the ZIP root.
/// Contains [package] metadata and only zip-sourced [[dependencies]].
fn generate_embedded_ccgo_toml(config_file: &Path) -> Result<String> {
    let content = fs::read_to_string(config_file)
        .context("Failed to read CCGO.toml")?;

    // Use the full config parser to access dependencies
    let config: crate::config::CcgoConfig = toml::from_str(&content)
        .context("Failed to parse CCGO.toml")?;

    let pkg = config.package.as_ref()
        .context("CCGO.toml must have a [package] section")?;

    let mut out = format!(
        "[package]\nname = \"{}\"\nversion = \"{}\"\n",
        pkg.name,
        pkg.version
    );

    if let Some(ref desc) = pkg.description {
        out.push_str(&format!("description = \"{}\"\n", desc));
    }

    // Only embed zip-sourced transitive deps (skip git/path deps)
    for dep in &config.dependencies {
        if let Some(ref zip) = dep.zip {
            out.push_str(&format!(
                "\n[[dependencies]]\nname = \"{}\"\nversion = \"{}\"\nzip = \"{}\"\n",
                dep.name, dep.version, zip
            ));
        }
    }

    Ok(out)
}

/// Merge multiple ZIP files into a single unified SDK ZIP
fn merge_zips(
    zip_files: &[PathBuf],
    output_zip_path: &Path,
    project_name: &str,
    version: &str,
    config_file: &Path,
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

    // Embed CCGO.toml at ZIP root for use as prebuilt dep
    match generate_embedded_ccgo_toml(config_file) {
        Ok(toml_content) => {
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            zip.start_file("CCGO.toml", options)?;
            zip.write_all(toml_content.as_bytes())?;
            println!("   ✓ Embedded CCGO.toml into ZIP root");
        }
        Err(e) => {
            eprintln!("   ⚠️  Could not embed CCGO.toml: {}", e);
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

/// Publish packaged artifacts to a git distribution branch using a worktree.
///
/// Steps:
///  1. Check that we are in a git repository.
///  2. Create (or reset) a temporary git worktree for `branch`.
///  3. Copy every file from `package_dir` into the worktree root.
///  4. Commit the changes with a version-stamped message.
///  5. Optionally push the branch to `origin`.
///  6. Remove the worktree.
fn publish_to_dist_branch(
    project_dir: &Path,
    package_dir: &Path,
    branch: &str,
    project_name: &str,
    version: &str,
    push: bool,
) -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("Publishing to dist branch: {}", branch);
    println!("{}", "=".repeat(80));

    // Verify we are inside a git repo
    let git_check = Command::new("git")
        .current_dir(project_dir)
        .args(["rev-parse", "--git-dir"])
        .output()
        .context("Failed to run git")?;
    if !git_check.status.success() {
        return Err(anyhow!("Not inside a git repository"));
    }

    // Locate the .git directory so we can place the worktree alongside it
    let git_dir_out = String::from_utf8_lossy(&git_check.stdout).trim().to_string();
    // git_dir_out is relative or absolute; make it absolute
    let git_dir = if Path::new(&git_dir_out).is_absolute() {
        PathBuf::from(&git_dir_out)
    } else {
        project_dir.join(&git_dir_out)
    };
    let repo_root = git_dir.parent().unwrap_or(project_dir);

    let worktree_path = repo_root.join(".ccgo-dist-worktree");

    // Remove stale worktree if it already exists
    if worktree_path.exists() {
        println!("   🧹 Removing stale worktree...");
        let _ = Command::new("git")
            .current_dir(project_dir)
            .args(["worktree", "remove", "--force", worktree_path.to_str().unwrap()])
            .status();
        if worktree_path.exists() {
            fs::remove_dir_all(&worktree_path)?;
        }
    }

    // Check if the branch exists (locally)
    let branch_exists = Command::new("git")
        .current_dir(project_dir)
        .args(["rev-parse", "--verify", branch])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);

    if branch_exists {
        // Checkout existing branch into worktree
        println!("   📂 Checking out existing branch '{}'...", branch);
        let status = Command::new("git")
            .current_dir(project_dir)
            .args(["worktree", "add", worktree_path.to_str().unwrap(), branch])
            .status()
            .context("Failed to create git worktree")?;
        if !status.success() {
            return Err(anyhow!("git worktree add failed"));
        }
    } else {
        // Create orphan branch in worktree (no parent commits)
        println!("   📂 Creating new orphan branch '{}'...", branch);
        let status = Command::new("git")
            .current_dir(project_dir)
            .args(["worktree", "add", "--orphan", "-b", branch, worktree_path.to_str().unwrap()])
            .status()
            .context("Failed to create git worktree with orphan branch")?;
        if !status.success() {
            return Err(anyhow!("git worktree add --orphan failed"));
        }
    }

    // Clear the worktree (keep .git file that links back to main repo)
    println!("   🧹 Clearing worktree...");
    if let Ok(entries) = fs::read_dir(&worktree_path) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            if name == ".git" {
                continue;
            }
            let p = entry.path();
            if p.is_dir() {
                fs::remove_dir_all(&p)?;
            } else {
                fs::remove_file(&p)?;
            }
        }
    }

    // Copy all packaged files into the worktree root
    println!("   📦 Copying artifacts...");
    let mut copied = 0usize;
    for entry in fs::read_dir(package_dir)
        .context("Failed to read package directory")?
        .flatten()
    {
        let src = entry.path();
        let dst = worktree_path.join(entry.file_name());
        fs::copy(&src, &dst)
            .with_context(|| format!("Failed to copy {} to worktree", src.display()))?;
        println!(
            "   ✓ {}",
            entry.file_name().to_string_lossy()
        );
        copied += 1;
    }

    if copied == 0 {
        // Clean up and bail
        let _ = Command::new("git")
            .current_dir(project_dir)
            .args(["worktree", "remove", "--force", worktree_path.to_str().unwrap()])
            .status();
        return Err(anyhow!("No artifacts found in package directory"));
    }

    // Stage all changes
    Command::new("git")
        .current_dir(&worktree_path)
        .args(["add", "-A"])
        .status()
        .context("Failed to stage dist files")?;

    // Check if there is anything to commit
    let porcelain = Command::new("git")
        .current_dir(&worktree_path)
        .args(["status", "--porcelain"])
        .output()
        .context("Failed to check git status")?;

    if porcelain.stdout.is_empty() {
        println!("   ℹ️  No changes to commit (dist branch is up-to-date)");
    } else {
        let commit_msg = format!("dist: {} v{}", project_name, version);
        println!("   💾 Committing: {}", commit_msg);
        let status = Command::new("git")
            .current_dir(&worktree_path)
            .args(["commit", "-m", &commit_msg])
            .status()
            .context("Failed to commit dist artifacts")?;
        if !status.success() {
            return Err(anyhow!("git commit failed in dist branch"));
        }
    }

    // Remove worktree (branch persists in the repo)
    let _ = Command::new("git")
        .current_dir(project_dir)
        .args(["worktree", "remove", "--force", worktree_path.to_str().unwrap()])
        .status();

    println!(
        "\n   ✅ Artifacts committed to branch '{}'",
        branch
    );

    // Push if requested
    if push {
        println!("   📤 Pushing branch '{}' to origin...", branch);
        let status = Command::new("git")
            .current_dir(project_dir)
            .args(["push", "origin", branch])
            .status()
            .context("Failed to push dist branch")?;
        if !status.success() {
            return Err(anyhow!("git push failed for branch '{}'", branch));
        }
        println!("   ✅ Pushed '{}' to origin", branch);
    } else {
        println!(
            "   💡 Use --dist-push to push '{}' to remote",
            branch
        );
    }

    Ok(())
}

/// Package SDK for distribution
#[derive(Args, Debug)]
#[command(disable_version_flag = true)]
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

    /// Publish packaged artifacts to a git distribution branch (e.g. "dist")
    #[arg(long)]
    pub dist_branch: Option<String>,

    /// Shorthand for --dist-branch dist
    #[arg(long)]
    pub dist: bool,

    /// Push the dist branch to remote after committing
    #[arg(long)]
    pub dist_push: bool,
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
            // Merged cross-platform artifact produced by `ccgo package`.
            // Namespaced with `CCGO_PACKAGE` to distinguish it from the
            // per-platform `<NAME>_<PLATFORM>_SDK-…` archives that the
            // build step emits (those keep their legacy naming).
            let sdk_zip_name = format!(
                "{}_CCGO_PACKAGE-{}.zip",
                project_name.to_uppercase(),
                version_clean
            );
            let sdk_zip_path = output_path.join(&sdk_zip_name);

            merge_zips(&all_zip_files, &sdk_zip_path, &project_name, &version, &config_file)?;

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

        // Publish to dist branch if requested
        let branch = if let Some(ref b) = self.dist_branch {
            Some(b.as_str())
        } else if self.dist {
            Some("dist")
        } else {
            None
        };

        if let Some(branch) = branch {
            let version_clean = version.strip_prefix('v').unwrap_or(&version);
            publish_to_dist_branch(
                &project_dir,
                &output_path,
                branch,
                &project_name,
                version_clean,
                self.dist_push,
            )?;
        }

        Ok(())
    }
}

/// Install packaged SDK into the global CCGO package cache.
///
/// Layout: ~/.ccgo/packages/<name>/<version>/
/// Contents: CCGO.toml (from source project) + include/ + lib/<platform>/...
///
/// The source is the merged unified SDK produced by `ccgo package`
/// (i.e. <output_path>/<NAME>_CCGO_PACKAGE-<version>-<suffix>.zip). If the
/// zip is not found, falls back to copying the unpacked contents of
/// <output_path>/.
pub(crate) fn install_to_local_cache(
    project_dir: &Path,
    output_path: &Path,
    project_name: &str,
    version: &str,
) -> Result<()> {
    println!("\n{}", "=".repeat(80));
    println!("Installing to Global CCGO Cache");
    println!("{}", "=".repeat(80));

    let dest = prepare_cache_dest(project_name, version)?;
    let unified_zip = find_unified_sdk_zip(output_path, project_name);

    if let Some(zip_path) = unified_zip {
        println!("📦 Source: {}", zip_path.display());
        extract_unified_zip_to(&zip_path, &dest, project_name)?;
    } else {
        println!("📦 Source: {} (directory copy)", output_path.display());
        copy_dir_recursive(output_path, &dest)?;
    }

    ensure_ccgo_toml_present(project_dir, &dest)?;

    println!(
        "\n{}",
        style(format!(
            "✅ Installed: {} {} -> {}",
            project_name,
            version,
            dest.display()
        ))
        .green()
        .bold()
    );
    println!(
        "\n💡 Other projects can now depend on this via CCGO.toml:\n    [[dependencies]]\n    name = \"{}\"\n    version = \"{}\"\n",
        project_name.to_lowercase(),
        version
    );

    Ok(())
}

/// Resolve `$CCGO_HOME/packages/<name>/<ver>/` and wipe any prior install.
fn prepare_cache_dest(project_name: &str, version: &str) -> Result<PathBuf> {
    let ccgo_home = if let Ok(custom) = std::env::var("CCGO_HOME") {
        PathBuf::from(custom)
    } else {
        let home = std::env::var("HOME")
            .map_err(|_| anyhow!("HOME env not set; cannot determine global cache path"))?;
        PathBuf::from(home).join(".ccgo")
    };
    let dest = ccgo_home
        .join("packages")
        .join(project_name.to_lowercase())
        .join(version);
    if dest.exists() {
        println!("🧹 Removing existing: {}", dest.display());
        fs::remove_dir_all(&dest)?;
    }
    fs::create_dir_all(&dest)?;
    Ok(dest)
}

/// Locate the merged unified SDK zip produced by `ccgo package` (not the
/// symbols/archive variants) so it can be re-extracted into the cache.
fn find_unified_sdk_zip(output_path: &Path, project_name: &str) -> Option<PathBuf> {
    let prefix = format!("{}_CCGO_PACKAGE-", project_name.to_uppercase());
    let entries = fs::read_dir(output_path).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        if name.starts_with(&prefix)
            && name.ends_with(".zip")
            && !name.contains("SYMBOLS")
            && !name.contains("ARCHIVE")
        {
            return Some(path);
        }
    }
    None
}

/// Extract a unified SDK zip into `dest`, stripping the leading
/// `<project-name>/` top-level directory (if present).
fn extract_unified_zip_to(zip_path: &Path, dest: &Path, project_name: &str) -> Result<()> {
    use std::io;

    let file = fs::File::open(zip_path)
        .with_context(|| format!("Failed to open {}", zip_path.display()))?;
    let mut archive = zip::ZipArchive::new(file)
        .with_context(|| format!("Failed to read {} as zip", zip_path.display()))?;
    let prefix = project_name.to_lowercase();

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i)?;
        let rel = match entry.enclosed_name() {
            Some(p) => p.to_path_buf(),
            None => continue,
        };
        let stripped = strip_top_level(&rel, &prefix);
        if stripped.as_os_str().is_empty() {
            continue;
        }
        let target = dest.join(&stripped);
        if entry.is_dir() {
            fs::create_dir_all(&target)?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut out = fs::File::create(&target)?;
            io::copy(&mut entry, &mut out)?;
        }
    }
    Ok(())
}

/// Copy the project's `CCGO.toml` into the cache if the zip didn't include one.
fn ensure_ccgo_toml_present(project_dir: &Path, dest: &Path) -> Result<()> {
    let dest_toml = dest.join("CCGO.toml");
    if dest_toml.exists() {
        return Ok(());
    }
    let src_toml = project_dir.join("CCGO.toml");
    if src_toml.exists() {
        fs::copy(&src_toml, &dest_toml)?;
    }
    Ok(())
}

/// Strip a known top-level directory component from a zip entry path.
/// If the first component matches `prefix`, returns the remainder.
/// Otherwise returns the path unchanged.
fn strip_top_level(rel: &Path, prefix: &str) -> PathBuf {
    let mut components = rel.components();
    if let Some(first) = components.next() {
        if first.as_os_str().to_string_lossy() == prefix {
            return components.as_path().to_path_buf();
        }
    }
    rel.to_path_buf()
}

/// Recursively copy a directory's contents into another directory.
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    fs::create_dir_all(dst)?;
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let from = entry.path();
        let to = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_recursive(&from, &to)?;
        } else if ty.is_file() {
            fs::copy(&from, &to)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_embedded_ccgo_toml_with_zip_deps() {
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("CCGO.toml");
        fs::write(&config_path, r#"
[package]
name = "netcomm"
version = "1.2.3"
description = "netcomm SDK"

[[dependencies]]
name = "foundrycomm"
version = "1.0.0"
zip = "https://cdn.example.com/foundrycomm_CCGO_PACKAGE-1.0.0.zip"

[[dependencies]]
name = "locallib"
version = "0.5.0"
git = "https://github.com/example/locallib.git"
"#).unwrap();

        let result = generate_embedded_ccgo_toml(&config_path).unwrap();
        assert!(result.contains("[package]"));
        assert!(result.contains("name = \"netcomm\""));
        assert!(result.contains("version = \"1.2.3\""));
        assert!(result.contains("description = \"netcomm SDK\""));
        // Only zip deps included
        assert!(result.contains("foundrycomm"));
        assert!(result.contains("zip = \"https://cdn.example.com/foundrycomm_CCGO_PACKAGE-1.0.0.zip\""));
        // git deps NOT included
        assert!(!result.contains("locallib"));
    }

    #[test]
    fn test_generate_embedded_ccgo_toml_no_deps() {
        let tmp = tempfile::tempdir().unwrap();
        let config_path = tmp.path().join("CCGO.toml");
        fs::write(&config_path, r#"
[package]
name = "standalone"
version = "2.0.0"
"#).unwrap();

        let result = generate_embedded_ccgo_toml(&config_path).unwrap();
        assert!(result.contains("name = \"standalone\""));
        assert!(!result.contains("[[dependencies]]"));
    }
}
