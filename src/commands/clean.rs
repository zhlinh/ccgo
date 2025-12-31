//! Clean command implementation

use anyhow::{Context, Result};
use clap::Args;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// Clean build artifacts
#[derive(Args, Debug)]
pub struct CleanCommand {
    /// Target to clean (all, android, ios, macos, ohos, kmp, examples)
    #[arg(default_value = "all")]
    pub target: String,

    /// Show what would be deleted
    #[arg(long)]
    pub dry_run: bool,

    /// Skip confirmation prompt
    #[arg(short = 'y', long)]
    pub yes: bool,

    /// Clean only cmake_build directory
    #[arg(long)]
    pub native_only: bool,
}

struct ProjectCleaner {
    project_dir: PathBuf,
    dry_run: bool,
    skip_confirm: bool,
    cleaned_dirs: Vec<String>,
    cleaned_size: u64,
    failed_dirs: Vec<(String, String)>,
}

impl ProjectCleaner {
    fn new(project_dir: PathBuf, dry_run: bool, skip_confirm: bool) -> Self {
        Self {
            project_dir,
            dry_run,
            skip_confirm,
            cleaned_dirs: Vec::new(),
            cleaned_size: 0,
            failed_dirs: Vec::new(),
        }
    }

    fn get_dir_size(path: &Path) -> u64 {
        let mut total_size = 0u64;
        if let Ok(entries) = WalkDir::new(path).into_iter().collect::<Result<Vec<_>, _>>() {
            for entry in entries {
                if let Ok(metadata) = entry.metadata() {
                    if metadata.is_file() {
                        total_size += metadata.len();
                    }
                }
            }
        }
        total_size
    }

    fn format_size(size_bytes: u64) -> String {
        const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
        let mut size = size_bytes as f64;
        let mut unit_idx = 0;

        while size >= 1024.0 && unit_idx < UNITS.len() - 1 {
            size /= 1024.0;
            unit_idx += 1;
        }

        format!("{:.2} {}", size, UNITS[unit_idx])
    }

    fn remove_directory(&mut self, dir_path: &Path, display_name: Option<&str>) -> bool {
        if !dir_path.exists() {
            return false;
        }

        if !dir_path.is_dir() {
            return false;
        }

        let size = Self::get_dir_size(dir_path);
        let name = display_name.unwrap_or_else(|| {
            dir_path
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
        });

        if self.dry_run {
            println!("  [DRY RUN] Would remove: {} ({})", name, Self::format_size(size));
            return true;
        }

        match fs::remove_dir_all(dir_path) {
            Ok(_) => {
                self.cleaned_dirs.push(name.to_string());
                self.cleaned_size += size;
                println!("  ✅ Removed: {} ({})", name, Self::format_size(size));
                true
            }
            Err(e) => {
                self.failed_dirs.push((name.to_string(), e.to_string()));
                println!("  ❌ Failed to remove {}: {}", name, e);
                false
            }
        }
    }

    fn confirm_clean(&self, message: &str) -> bool {
        if self.skip_confirm {
            return true;
        }

        print!("{} (y/N): ", message);
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();
        let input = input.trim().to_lowercase();

        input == "y" || input == "yes"
    }

    fn clean_bin(&mut self) {
        println!("\n{}", "=".repeat(60));
        println!("  Cleaning bin/ directory");
        println!("{}", "=".repeat(60));

        let bin_dir = self.project_dir.join("bin");

        if !bin_dir.exists() {
            println!("  ℹ️  bin/ directory does not exist");
            return;
        }

        if !self.dry_run && !self.skip_confirm {
            if !self.confirm_clean("  Remove bin/ directory?") {
                println!("  ⏭️  Skipped");
                return;
            }
        }

        self.remove_directory(&bin_dir, Some("bin/"));
    }

    fn clean_cmake(&mut self) {
        println!("\n{}", "=".repeat(60));
        println!("  Cleaning cmake_build/ directory");
        println!("{}", "=".repeat(60));

        let cmake_dir = self.project_dir.join("cmake_build");

        if !cmake_dir.exists() {
            println!("  ℹ️  cmake_build/ directory does not exist");
            return;
        }

        if !self.dry_run && !self.skip_confirm {
            if !self.confirm_clean("  Remove cmake_build/ directory?") {
                println!("  ⏭️  Skipped");
                return;
            }
        }

        self.remove_directory(&cmake_dir, Some("cmake_build/"));
    }

    fn clean_android(&mut self) {
        println!("\n{}", "=".repeat(60));
        println!("  Cleaning Android build caches");
        println!("{}", "=".repeat(60));

        let android_dir = self.project_dir.join("android");

        if !android_dir.exists() {
            println!("  ℹ️  android/ directory does not exist");
            return;
        }

        // Clean android/build/
        let build_dir = android_dir.join("build");
        if build_dir.exists() {
            self.remove_directory(&build_dir, Some("android/build/"));
        }

        // Clean android/.gradle/
        let gradle_dir = android_dir.join(".gradle");
        if gradle_dir.exists() {
            self.remove_directory(&gradle_dir, Some("android/.gradle/"));
        }

        // Clean android/*/build/ for all subprojects
        if let Ok(entries) = fs::read_dir(&android_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let subproject_build = entry.path().join("build");
                        if subproject_build.exists() {
                            let name = format!("android/{}/build/", entry.file_name().to_string_lossy());
                            self.remove_directory(&subproject_build, Some(&name));
                        }
                    }
                }
            }
        }
    }

    fn clean_ios(&mut self) {
        println!("\n{}", "=".repeat(60));
        println!("  Cleaning iOS/macOS build caches");
        println!("{}", "=".repeat(60));

        // Clean Pods cache
        let pods_dir = self.project_dir.join("ios/Pods");
        if pods_dir.exists() {
            self.remove_directory(&pods_dir, Some("ios/Pods/"));
        }

        // Clean DerivedData
        let derived_data = self.project_dir.join("ios/DerivedData");
        if derived_data.exists() {
            self.remove_directory(&derived_data, Some("ios/DerivedData/"));
        }

        // Clean build directories
        let ios_build = self.project_dir.join("ios/build");
        if ios_build.exists() {
            self.remove_directory(&ios_build, Some("ios/build/"));
        }
    }

    fn clean_macos(&mut self) {
        println!("\n{}", "=".repeat(60));
        println!("  Cleaning macOS build caches");
        println!("{}", "=".repeat(60));

        // Clean macOS build if separate from iOS
        let macos_build = self.project_dir.join("macos/build");
        if macos_build.exists() {
            self.remove_directory(&macos_build, Some("macos/build/"));
        }
    }

    fn clean_ohos(&mut self) {
        println!("\n{}", "=".repeat(60));
        println!("  Cleaning OHOS build caches");
        println!("{}", "=".repeat(60));

        let ohos_dir = self.project_dir.join("ohos");

        if !ohos_dir.exists() {
            println!("  ℹ️  ohos/ directory does not exist");
            return;
        }

        // Clean ohos/build/
        let build_dir = ohos_dir.join("build");
        if build_dir.exists() {
            self.remove_directory(&build_dir, Some("ohos/build/"));
        }

        // Clean ohos/.hvigor/
        let hvigor_dir = ohos_dir.join(".hvigor");
        if hvigor_dir.exists() {
            self.remove_directory(&hvigor_dir, Some("ohos/.hvigor/"));
        }

        // Clean ohos/*/build/ for all subprojects
        if let Ok(entries) = fs::read_dir(&ohos_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let dir_name = entry.file_name();
                        let dir_name_str = dir_name.to_string_lossy();
                        if dir_name_str != ".hvigor" && dir_name_str != "build" {
                            let subproject_build = entry.path().join("build");
                            if subproject_build.exists() {
                                let name = format!("ohos/{}/build/", dir_name_str);
                                self.remove_directory(&subproject_build, Some(&name));
                            }
                        }
                    }
                }
            }
        }
    }

    fn clean_kmp(&mut self) {
        println!("\n{}", "=".repeat(60));
        println!("  Cleaning KMP build caches");
        println!("{}", "=".repeat(60));

        let kmp_dir = self.project_dir.join("kmp");

        if !kmp_dir.exists() {
            println!("  ℹ️  kmp/ directory does not exist");
            return;
        }

        // Clean kmp/build/
        let build_dir = kmp_dir.join("build");
        if build_dir.exists() {
            self.remove_directory(&build_dir, Some("kmp/build/"));
        }

        // Clean kmp/.gradle/
        let gradle_dir = kmp_dir.join(".gradle");
        if gradle_dir.exists() {
            self.remove_directory(&gradle_dir, Some("kmp/.gradle/"));
        }
    }

    fn clean_examples(&mut self) {
        println!("\n{}", "=".repeat(60));
        println!("  Cleaning examples build caches");
        println!("{}", "=".repeat(60));

        let examples_dir = self.project_dir.join("examples");

        if !examples_dir.exists() {
            println!("  ℹ️  examples/ directory does not exist");
            return;
        }

        // Find all build directories in examples
        for entry in WalkDir::new(&examples_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_dir() {
                let dir_name = entry.file_name().to_string_lossy();
                if dir_name == "build" || dir_name == ".gradle" || dir_name == ".hvigor" {
                    let rel_path = entry
                        .path()
                        .strip_prefix(&self.project_dir)
                        .unwrap()
                        .to_string_lossy()
                        .to_string();
                    self.remove_directory(entry.path(), Some(&rel_path));
                }
            }
        }
    }

    fn clean_platform(&mut self, platform: &str) {
        match platform {
            "android" => self.clean_android(),
            "ios" => self.clean_ios(),
            "macos" => self.clean_macos(),
            "ohos" => self.clean_ohos(),
            "kmp" => self.clean_kmp(),
            "examples" => self.clean_examples(),
            _ => {
                eprintln!("Unknown platform: {}", platform);
            }
        }
    }

    fn clean_all(&mut self) {
        println!("\n{}", "=".repeat(60));
        println!("  Cleaning ALL build artifacts and caches");
        println!("{}", "=".repeat(60));

        if !self.dry_run && !self.skip_confirm {
            if !self.confirm_clean("\n⚠️  This will remove ALL build artifacts. Continue?") {
                println!("  ⏭️  Aborted");
                return;
            }
        }

        // Clean in order
        self.clean_bin();
        self.clean_cmake();
        self.clean_android();
        self.clean_ios();
        self.clean_macos();
        self.clean_ohos();
        self.clean_kmp();
        self.clean_examples();

        // Also clean root .gradle if exists
        let root_gradle = self.project_dir.join(".gradle");
        if root_gradle.exists() {
            println!("\n{}", "=".repeat(60));
            println!("  Cleaning root .gradle/");
            println!("{}", "=".repeat(60));
            self.remove_directory(&root_gradle, Some(".gradle/"));
        }
    }

    fn print_summary(&self) {
        println!("\n{}", "=".repeat(60));
        println!("  Cleaning Summary");
        println!("{}", "=".repeat(60));

        if self.dry_run {
            println!("  [DRY RUN MODE - No files were actually deleted]");
        }

        if !self.cleaned_dirs.is_empty() {
            println!("  ✅ Successfully cleaned {} directories:", self.cleaned_dirs.len());
            for dir_name in &self.cleaned_dirs {
                println!("     - {}", dir_name);
            }
            println!("\n  💾 Total space freed: {}", Self::format_size(self.cleaned_size));
        } else {
            println!("  ℹ️  No directories were cleaned");
        }

        if !self.failed_dirs.is_empty() {
            println!("\n  ❌ Failed to clean {} directories:", self.failed_dirs.len());
            for (dir_name, error) in &self.failed_dirs {
                println!("     - {}: {}", dir_name, error);
            }
        }

        println!("{}\n", "=".repeat(60));

        if self.dry_run {
            println!("💡 Tip: Run without --dry-run to actually delete the files");
        }
    }
}

impl CleanCommand {
    /// Execute the clean command
    pub fn execute(self, _verbose: bool) -> Result<()> {
        println!("Cleaning build artifacts and caches...\n");

        // Get current working directory (project directory)
        let project_dir = std::env::current_dir()
            .context("Failed to get current working directory")?;

        // Find CCGO.toml to locate project directory
        let project_subdir = Self::find_project_dir(&project_dir)?;

        let mut cleaner = ProjectCleaner::new(project_subdir, self.dry_run, self.yes);

        // Determine what to clean based on arguments
        if self.native_only {
            // Only clean cmake_build directory
            cleaner.clean_cmake();
        } else if self.target == "all" {
            // Clean all platforms
            cleaner.clean_all();
        } else {
            // Clean specific platform
            cleaner.clean_platform(&self.target);
        }

        cleaner.print_summary();

        Ok(())
    }

    fn find_project_dir(current_dir: &Path) -> Result<PathBuf> {
        // Check if CCGO.toml exists in subdirectories
        if let Ok(entries) = fs::read_dir(current_dir) {
            for entry in entries.flatten() {
                if let Ok(file_type) = entry.file_type() {
                    if file_type.is_dir() {
                        let potential_config = entry.path().join("CCGO.toml");
                        if potential_config.exists() {
                            return Ok(entry.path());
                        }
                    }
                }
            }
        }

        // If not found in subdirectory, check current directory
        let config_path = current_dir.join("CCGO.toml");
        if config_path.exists() {
            return Ok(current_dir.to_path_buf());
        }

        // Warning if CCGO.toml not found
        eprintln!("⚠️  Warning: CCGO.toml not found. May not be in a CCGO project directory.");
        eprintln!("   Continuing anyway...\n");

        Ok(current_dir.to_path_buf())
    }
}
