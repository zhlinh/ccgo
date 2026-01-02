//! Kotlin Multiplatform (KMP) platform builder
//!
//! Builds KMP library for all supported platforms using Gradle.
//! This is a pure Rust implementation that directly runs Gradle commands.

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

        // Common KMP build output locations:
        // - build/libs/*.jar (JVM)
        // - build/outputs/aar/*.aar (Android)
        // - build/bin/**/*.klib (Native)
        // - build/XCFrameworks/**/*.xcframework (Apple)

        let search_dirs = vec![
            kmp_dir.join("build/libs"),
            kmp_dir.join("build/outputs/aar"),
            kmp_dir.join("build/bin"),
            kmp_dir.join("build/XCFrameworks"),
            // Also check submodule builds
            kmp_dir.join("shared/build/libs"),
            kmp_dir.join("shared/build/outputs/aar"),
        ];

        for search_dir in search_dirs {
            if !search_dir.exists() {
                continue;
            }

            self.collect_artifacts(&search_dir, &mut outputs)?;
        }

        Ok(outputs)
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
    fn create_sdk_archive(&self, ctx: &BuildContext, outputs: &[PathBuf]) -> Result<PathBuf> {
        let archive = ArchiveBuilder::new(
            ctx.lib_name().to_string(),
            ctx.version().to_string(),
            "".to_string(),  // publish_suffix
            ctx.options.release,
            "kmp".to_string(),
            ctx.output_dir.clone(),
        )?;

        // Organize outputs by type
        for output in outputs {
            let file_name = output.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("");
            let ext = output.extension()
                .and_then(|e| e.to_str())
                .unwrap_or("");

            let dest_dir = match ext {
                "jar" => "lib/jvm",
                "aar" => "lib/android",
                "klib" => {
                    // Determine platform from path
                    let path_str = output.to_string_lossy();
                    if path_str.contains("ios") {
                        "lib/native/ios"
                    } else if path_str.contains("macos") || path_str.contains("macosX64") || path_str.contains("macosArm64") {
                        "lib/native/macos"
                    } else if path_str.contains("linux") {
                        "lib/native/linux"
                    } else if path_str.contains("mingw") || path_str.contains("windows") {
                        "lib/native/windows"
                    } else {
                        "lib/native/common"
                    }
                }
                "xcframework" | "framework" => "lib/apple",
                _ => continue,
            };

            let dest_path = format!("{}/{}", dest_dir, file_name);

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

        // Determine the Gradle task to run
        let gradle_task = if ctx.options.release {
            "assemble"
        } else {
            "build"
        };

        // Run Gradle build
        self.run_gradle(ctx, &[gradle_task])?;

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
