//! Kotlin Multiplatform (KMP) platform builder
//!
//! Builds KMP library for all supported platforms using Gradle.
//! This is a pure Rust implementation that directly runs Gradle commands.
//!
//! KMP requires native C/C++ libraries to be built first before Gradle can
//! compile the Kotlin/Native targets with cinterop.

use std::path::PathBuf;
use std::process::Command;
use std::time::Instant;

use anyhow::{bail, Context, Result};

use crate::build::archive::ArchiveBuilder;
use crate::build::{BuildContext, BuildResult, PlatformBuilder};

/// KMP platform builder
pub struct KmpBuilder {}

impl KmpBuilder {
    pub fn new() -> Self {
        Self {}
    }

    /// Get the gradlew command based on platform
    fn gradlew_cmd() -> &'static str {
        if cfg!(target_os = "windows") {
            "gradlew.bat"
        } else {
            "./gradlew"
        }
    }

    /// Check if native libraries already exist for a platform
    ///
    /// Checks both pyccgo path (cmake_build/{Platform}/) and Rust ccgo path
    /// (cmake_build/{debug|release}/{platform}/)
    fn native_libs_exist(&self, ctx: &BuildContext, platform: &str) -> bool {
        let cmake_build = ctx.project_root.join("cmake_build");

        // Platform name mapping for pyccgo paths (capitalized)
        let pyccgo_platform = match platform {
            "ios" => "iOS",
            "macos" => "macOS",
            "tvos" => "tvOS",
            "watchos" => "watchOS",
            "android" => "Android",
            "linux" => "Linux",
            "windows" => "Windows",
            _ => platform,
        };

        // Check pyccgo path: cmake_build/{Platform}/static/
        let pyccgo_path = cmake_build.join(pyccgo_platform).join("static");
        if pyccgo_path.exists() {
            // Check if there's a .a file in out/ or directly
            let out_dir = pyccgo_path.join("out");
            if out_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&out_dir) {
                    for entry in entries.flatten() {
                        if entry.path().extension().map_or(false, |e| e == "a") {
                            return true;
                        }
                    }
                }
            }
            // Also check for xcframework (iOS/macOS)
            let xcframework = pyccgo_path.join("xcframework");
            if xcframework.exists() {
                return true;
            }
        }

        // Check Rust ccgo path: cmake_build/{debug|release}/{platform}/static/
        let mode = if ctx.options.release { "release" } else { "debug" };
        let rust_path = cmake_build.join(mode).join(platform).join("static");
        if rust_path.exists() {
            let out_dir = rust_path.join("out");
            if out_dir.exists() {
                if let Ok(entries) = std::fs::read_dir(&out_dir) {
                    for entry in entries.flatten() {
                        if entry.path().extension().map_or(false, |e| e == "a") {
                            return true;
                        }
                    }
                }
            }
            let xcframework = rust_path.join("xcframework");
            if xcframework.exists() {
                return true;
            }
        }

        false
    }

    /// Build native C/C++ libraries required for KMP cinterop
    ///
    /// This builds native libraries for the current platform using ccgo:
    /// - Android (always, cross-platform)
    /// - iOS + macOS (on macOS)
    /// - Linux (on Linux)
    /// - Windows (on Windows)
    ///
    /// Skips platforms that already have native libraries built.
    fn build_native_libraries(&self, ctx: &BuildContext) -> Result<()> {
        eprintln!("\n{}", "=".repeat(80));
        eprintln!("Building Native Libraries for KMP");
        eprintln!("{}\n", "=".repeat(80));

        // Determine which platforms to build based on current OS
        let mut platforms = vec!["android"]; // Always build Android

        #[cfg(target_os = "macos")]
        {
            platforms.push("ios");
            platforms.push("macos");
        }

        #[cfg(target_os = "linux")]
        {
            platforms.push("linux");
        }

        #[cfg(target_os = "windows")]
        {
            platforms.push("windows");
        }

        // Get the current executable path to call ccgo
        let ccgo_exe = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("ccgo"));

        for platform in platforms {
            // Check if native libraries already exist
            if self.native_libs_exist(ctx, platform) {
                eprintln!("✅ {} native libraries already exist, skipping build.\n", platform);
                continue;
            }

            eprintln!("\n--- Building {} native libraries ---\n", platform);

            // Build using: ccgo build <platform> --native-only
            // --native-only already skips archive creation
            let mut cmd = Command::new(&ccgo_exe);
            cmd.current_dir(&ctx.project_root);
            cmd.args(["build", platform, "--native-only"]);

            if ctx.options.release {
                cmd.arg("--release");
            }

            if ctx.options.verbose {
                cmd.arg("--verbose");
                eprintln!("Executing: {:?}", cmd);
            }

            let status = cmd.status();

            match status {
                Ok(s) if s.success() => {
                    eprintln!("\n✅ {} native libraries built successfully.\n", platform);
                }
                Ok(s) => {
                    eprintln!(
                        "\n⚠️  WARNING: {} build failed with exit code {:?}",
                        platform,
                        s.code()
                    );
                    eprintln!("   KMP may not work correctly on {}.\n", platform);
                    // Don't exit, continue with other platforms
                }
                Err(e) => {
                    eprintln!("\n⚠️  WARNING: Failed to build {}: {}", platform, e);
                    eprintln!("   KMP may not work correctly on {}.\n", platform);
                }
            }
        }

        eprintln!("\n{}", "=".repeat(80));
        eprintln!("Native Libraries Build Complete");
        eprintln!("{}\n", "=".repeat(80));

        Ok(())
    }

    /// Run a Gradle command in the KMP directory
    fn run_gradle(&self, ctx: &BuildContext, args: &[&str]) -> Result<()> {
        let kmp_dir = ctx.project_root.join("kmp");
        let gradlew = Self::gradlew_cmd();

        if ctx.options.verbose {
            eprintln!("Running: {} {} (in {})", gradlew, args.join(" "), kmp_dir.display());
        }

        let mut cmd = Command::new(gradlew);
        cmd.current_dir(&kmp_dir);
        cmd.args(args);

        // Add common Gradle options
        cmd.arg("--no-daemon");
        // Disable configuration cache to avoid Kotlin/Native issues
        cmd.arg("--no-configuration-cache");
        if !ctx.options.verbose {
            cmd.arg("--quiet");
        }

        let status = cmd
            .status()
            .with_context(|| format!("Failed to execute {} in {}", gradlew, kmp_dir.display()))?;

        if !status.success() {
            bail!("Gradle command failed: {} {}", gradlew, args.join(" "));
        }

        Ok(())
    }

    /// Find build outputs in the KMP directory
    fn find_build_outputs(&self, ctx: &BuildContext) -> Result<Vec<PathBuf>> {
        let kmp_dir = ctx.project_root.join("kmp");
        let mut outputs = Vec::new();

        // Common KMP build output locations (matching pyccgo structure):
        // - build/libs/*.jar (JVM/Desktop)
        // - build/outputs/aar/*.aar (Android)
        // - build/classes/kotlin/{target}/main/klib/*.klib (Native main klib)
        // - build/classes/kotlin/{target}/main/cinterop/*.klib (Native cinterop klib)

        // Collect JAR files from build/libs/
        let jar_dir = kmp_dir.join("build/libs");
        if jar_dir.exists() {
            self.collect_artifacts(&jar_dir, &mut outputs)?;
        }

        // Collect AAR files from build/outputs/aar/
        let aar_dir = kmp_dir.join("build/outputs/aar");
        if aar_dir.exists() {
            self.collect_artifacts(&aar_dir, &mut outputs)?;
        }

        // Collect klib files from build/classes/kotlin/{target}/main/
        // This is the key difference from the old implementation
        let classes_dir = kmp_dir.join("build/classes/kotlin");
        if classes_dir.exists() {
            self.collect_klib_artifacts(&classes_dir, &mut outputs)?;
        }

        // Also check submodule builds
        let shared_jar_dir = kmp_dir.join("shared/build/libs");
        if shared_jar_dir.exists() {
            self.collect_artifacts(&shared_jar_dir, &mut outputs)?;
        }
        let shared_aar_dir = kmp_dir.join("shared/build/outputs/aar");
        if shared_aar_dir.exists() {
            self.collect_artifacts(&shared_aar_dir, &mut outputs)?;
        }

        Ok(outputs)
    }

    /// Collect klib artifacts from build/classes/kotlin/{target}/main/
    /// Returns tuples of (klib_path, target_name, klib_type) where klib_type is "klib" or "cinterop"
    fn collect_klib_artifacts(&self, classes_dir: &PathBuf, outputs: &mut Vec<PathBuf>) -> Result<()> {
        if !classes_dir.exists() {
            return Ok(());
        }

        // Iterate over target directories (e.g., iosArm64, macosX64, etc.)
        for target_entry in std::fs::read_dir(classes_dir)? {
            let target_entry = target_entry?;
            let target_path = target_entry.path();
            if !target_path.is_dir() {
                continue;
            }

            let main_dir = target_path.join("main");
            if !main_dir.exists() {
                continue;
            }

            // Check for main klib directory
            let klib_dir = main_dir.join("klib");
            if klib_dir.exists() {
                for entry in std::fs::read_dir(&klib_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "klib") {
                        outputs.push(path);
                    }
                }
            }

            // Check for cinterop directory
            let cinterop_dir = main_dir.join("cinterop");
            if cinterop_dir.exists() {
                for entry in std::fs::read_dir(&cinterop_dir)? {
                    let entry = entry?;
                    let path = entry.path();
                    if path.extension().map_or(false, |e| e == "klib") {
                        outputs.push(path);
                    }
                }
            }
        }

        Ok(())
    }

    /// Recursively collect artifact files
    fn collect_artifacts(&self, dir: &PathBuf, outputs: &mut Vec<PathBuf>) -> Result<()> {
        if !dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_dir() {
                // Check if it's an XCFramework or similar bundle
                if let Some(ext) = path.extension() {
                    if ext == "xcframework" || ext == "framework" {
                        outputs.push(path);
                        continue;
                    }
                }
                // Recurse into subdirectories
                self.collect_artifacts(&path, outputs)?;
            } else if path.is_file() {
                // Collect relevant artifacts
                if let Some(ext) = path.extension() {
                    let ext_str = ext.to_str().unwrap_or("");
                    match ext_str {
                        "jar" | "aar" | "klib" => {
                            // Skip sources and javadoc jars
                            let file_name = path.file_name()
                                .and_then(|n| n.to_str())
                                .unwrap_or("");
                            if !file_name.contains("-sources") && !file_name.contains("-javadoc") {
                                outputs.push(path);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(())
    }

    /// Create SDK archive from build outputs
    /// Matches pyccgo directory structure:
    /// - lib/kmp/android/{aar}
    /// - lib/kmp/desktop/{jar}
    /// - lib/kmp/native/{target}/klib/{klib}
    /// - lib/kmp/native/{target}/cinterop/{cinterop_klib}
    fn create_sdk_archive(&self, ctx: &BuildContext, outputs: &[PathBuf]) -> Result<PathBuf> {
        let archive = ArchiveBuilder::new(
            ctx.lib_name().to_string(),
            ctx.version().to_string(),
            ctx.publish_suffix().to_string(),
            ctx.options.release,
            "kmp".to_string(),
            ctx.output_dir.clone(),
        )?;

        // Organize outputs by type (matching pyccgo structure)
        for output in outputs {
            let file_name = output.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            let ext = output.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");

            // Get the path as string for analysis
            let path_str = output.to_string_lossy();

            let dest_path = match ext {
                "jar" => {
                    // Skip metadata jars (they are not main artifacts)
                    if file_name.contains("-metadata") {
                        continue;
                    }
                    format!("lib/kmp/desktop/{}", file_name)
                }
                "aar" => format!("lib/kmp/android/{}", file_name),
                "klib" => {
                    // Determine target and type from path
                    // Path format: build/classes/kotlin/{target}/main/{klib|cinterop}/{file}.klib
                    let is_cinterop = path_str.contains("/cinterop/");
                    let klib_type = if is_cinterop { "cinterop" } else { "klib" };

                    // Extract target name from path (e.g., iosArm64, macosX64)
                    let target = self.extract_target_from_path(&path_str);

                    format!("lib/kmp/native/{}/{}/{}", target, klib_type, file_name)
                }
                "xcframework" | "framework" => format!("lib/apple/{}", file_name),
                _ => continue,
            };

            if output.is_dir() {
                archive.add_directory(output, &dest_path)?;
            } else {
                archive.add_file(output, &dest_path)?;
            }
        }

        // Create the SDK archive
        let link_type = ctx.options.link_type.to_string();
        archive.create_sdk_archive(&[], &link_type)
    }

    /// Extract target name from klib path
    /// Path format: .../build/classes/kotlin/{target}/main/...
    fn extract_target_from_path(&self, path: &str) -> String {
        // Look for pattern: /kotlin/{target}/main/
        if let Some(kotlin_idx) = path.find("/kotlin/") {
            let after_kotlin = &path[kotlin_idx + 8..]; // Skip "/kotlin/"
            if let Some(main_idx) = after_kotlin.find("/main/") {
                return after_kotlin[..main_idx].to_string();
            }
        }

        // Fallback: try to determine from common patterns
        if path.contains("iosArm64") {
            "iosArm64".to_string()
        } else if path.contains("iosX64") {
            "iosX64".to_string()
        } else if path.contains("iosSimulatorArm64") {
            "iosSimulatorArm64".to_string()
        } else if path.contains("macosArm64") {
            "macosArm64".to_string()
        } else if path.contains("macosX64") {
            "macosX64".to_string()
        } else if path.contains("linuxX64") {
            "linuxX64".to_string()
        } else if path.contains("linuxArm64") {
            "linuxArm64".to_string()
        } else {
            "common".to_string()
        }
    }
}

impl PlatformBuilder for KmpBuilder {
    fn platform_name(&self) -> &str {
        "kmp"
    }

    fn default_architectures(&self) -> Vec<String> {
        // KMP builds for all architectures automatically
        vec![]
    }

    fn validate_prerequisites(&self, ctx: &BuildContext) -> Result<()> {
        // Check if KMP directory exists
        let kmp_dir = ctx.project_root.join("kmp");
        if !kmp_dir.exists() {
            bail!(
                "KMP directory not found: {}\n\
                 Please ensure your project has the KMP module configured",
                kmp_dir.display()
            );
        }

        // Check if gradlew exists in KMP directory
        let gradlew_name = if cfg!(target_os = "windows") {
            "gradlew.bat"
        } else {
            "gradlew"
        };
        let gradlew = kmp_dir.join(gradlew_name);
        if !gradlew.exists() {
            bail!(
                "gradlew not found in KMP directory: {}\n\
                 Please ensure the KMP module is properly initialized with Gradle Wrapper",
                kmp_dir.display()
            );
        }

        // Check if build.gradle.kts or build.gradle exists
        let has_gradle_config = kmp_dir.join("build.gradle.kts").exists()
            || kmp_dir.join("build.gradle").exists();
        if !has_gradle_config {
            bail!(
                "No Gradle build file found in KMP directory: {}\n\
                 Expected build.gradle.kts or build.gradle",
                kmp_dir.display()
            );
        }

        if ctx.options.verbose {
            eprintln!("KMP prerequisites validated");
        }

        Ok(())
    }

    fn build(&self, ctx: &BuildContext) -> Result<BuildResult> {
        let start = Instant::now();

        // Validate prerequisites first
        self.validate_prerequisites(ctx)?;

        if ctx.options.verbose {
            eprintln!("Building {} for KMP...", ctx.lib_name());
        }

        // Step 1: Build native C/C++ libraries first
        // KMP cinterop requires native .a/.so files to link against
        self.build_native_libraries(ctx)?;

        // Step 2: Build KMP using Gradle
        eprintln!("\n{}", "=".repeat(80));
        eprintln!("Building Kotlin Multiplatform Library");
        eprintln!("{}\n", "=".repeat(80));

        // Determine the Gradle tasks to run based on platform
        let mut tasks = vec!["clean"];

        if ctx.options.release {
            tasks.push("assembleRelease");
        } else {
            tasks.push("assemble");
        }

        // Add desktop JAR task
        tasks.push("desktopJar");

        // Add platform-specific native targets
        #[cfg(target_os = "macos")]
        {
            tasks.extend([
                "iosArm64MainKlibrary",
                "iosX64MainKlibrary",
                "iosSimulatorArm64MainKlibrary",
                "macosArm64MainKlibrary",
                "macosX64MainKlibrary",
            ]);
        }

        #[cfg(target_os = "linux")]
        {
            tasks.extend([
                "linuxX64MainKlibrary",
                "linuxArm64MainKlibrary",
            ]);
        }

        // Run Gradle build with all tasks
        self.run_gradle(ctx, &tasks)?;

        // Find build outputs
        let outputs = self.find_build_outputs(ctx)?;

        if outputs.is_empty() {
            bail!(
                "No KMP build outputs found.\n\
                 Please check if the KMP project is configured correctly."
            );
        }

        if ctx.options.verbose {
            eprintln!("Found {} build outputs:", outputs.len());
            for output in &outputs {
                eprintln!("  - {}", output.display());
            }
        }

        // Create SDK archive
        let sdk_archive = self.create_sdk_archive(ctx, &outputs)?;

        let duration = start.elapsed();

        if ctx.options.verbose {
            eprintln!(
                "KMP build completed in {:.2}s: {}",
                duration.as_secs_f64(),
                sdk_archive.display()
            );
        }

        Ok(BuildResult {
            sdk_archive,
            symbols_archive: None,
            aar_archive: None,
            duration_secs: duration.as_secs_f64(),
            architectures: vec![], // KMP builds for all architectures
        })
    }

    fn clean(&self, ctx: &BuildContext) -> Result<()> {
        // Clean KMP build directory
        let kmp_dir = ctx.project_root.join("kmp");
        if kmp_dir.exists() {
            let build_dir = kmp_dir.join("build");
            if build_dir.exists() {
                std::fs::remove_dir_all(&build_dir)
                    .with_context(|| format!("Failed to clean {}", build_dir.display()))?;
            }

            // Also clean .gradle directory
            let gradle_dir = kmp_dir.join(".gradle");
            if gradle_dir.exists() {
                std::fs::remove_dir_all(&gradle_dir)
                    .with_context(|| format!("Failed to clean {}", gradle_dir.display()))?;
            }

            // Clean shared module build if exists
            let shared_build = kmp_dir.join("shared/build");
            if shared_build.exists() {
                std::fs::remove_dir_all(&shared_build)
                    .with_context(|| format!("Failed to clean {}", shared_build.display()))?;
            }
        }

        // Clean target/kmp directory
        let target_dir = ctx.project_root.join("target").join("kmp");
        if target_dir.exists() {
            std::fs::remove_dir_all(&target_dir)
                .with_context(|| format!("Failed to clean {}", target_dir.display()))?;
        }

        // Clean debug/release kmp directories
        for subdir in &["debug", "release"] {
            let kmp_target = ctx.project_root.join("target").join(subdir).join("kmp");
            if kmp_target.exists() {
                std::fs::remove_dir_all(&kmp_target)
                    .with_context(|| format!("Failed to clean {}", kmp_target.display()))?;
            }
        }

        Ok(())
    }
}

impl Default for KmpBuilder {
    fn default() -> Self {
        Self::new()
    }
}
