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

use crate::builder::archive::ArchiveBuilder;
use crate::builder::{BuildContext, BuildResult, PlatformBuilder};

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

    /// Check if native libraries already exist for a platform.
    ///
    /// Checks three layouts in order:
    /// 1. Legacy pyccgo: `cmake_build/{Platform}/static/`
    /// 2. New ccgo: `ccgo_build/{mode[-profile]}/{platform}/static/`
    /// 3. Legacy Rust ccgo: `cmake_build/{mode}/{platform}/static/`
    fn native_libs_exist(&self, ctx: &BuildContext, platform: &str) -> bool {
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

        // 1. Legacy pyccgo path: cmake_build/{Platform}/static/
        let legacy_pyccgo = ctx.project_root.join("cmake_build").join(pyccgo_platform).join("static");
        if Self::has_native_libs(&legacy_pyccgo) {
            return true;
        }

        let mode = if ctx.options.release { "release" } else { "debug" };

        // 2. New ccgo path: ccgo_build/{mode[-profile]}/{platform}/static/
        if ctx.ccgo_build_root.is_dir() {
            if let Ok(entries) = std::fs::read_dir(&ctx.ccgo_build_root) {
                for entry in entries.flatten() {
                    if !entry.file_type().map(|ft| ft.is_dir()).unwrap_or(false) {
                        continue;
                    }
                    let subdir_name = entry.file_name().to_string_lossy().to_string();
                    if subdir_name == mode || subdir_name.starts_with(&format!("{mode}-")) {
                        let candidate = entry.path().join(platform).join("static");
                        if Self::has_native_libs(&candidate) {
                            return true;
                        }
                    }
                }
            }
        }

        // 3. Legacy Rust ccgo path: cmake_build/{mode}/{platform}/static/
        let legacy_rust = ctx.project_root.join("cmake_build").join(mode).join(platform).join("static");
        Self::has_native_libs(&legacy_rust)
    }

    /// Return true if `dir` contains at least one `.a` file or an `xcframework` bundle.
    fn has_native_libs(dir: &std::path::Path) -> bool {
        if !dir.exists() {
            return false;
        }
        let out_dir = dir.join("out");
        if out_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(&out_dir) {
                for entry in entries.flatten() {
                    if entry.path().extension().is_some_and(|e| e == "a") {
                        return true;
                    }
                }
            }
        }
        dir.join("xcframework").exists()
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
/// Every KMP target group, in build order.
    const ALL_TARGETS: &'static [&'static str] =
        &["android", "desktop", "ios", "macos", "linux", "windows"];

    /// Target groups this host can actually produce.
    fn host_targets() -> Vec<&'static str> {
        let mut t = vec!["android", "desktop"];
        if cfg!(target_os = "macos") {
            t.extend(["ios", "macos"]);
        }
        if cfg!(target_os = "linux") {
            t.push("linux");
        }
        if cfg!(target_os = "windows") {
            t.push("windows");
        }
        t
    }

    /// Target groups to build: `[kmp].targets` narrowed to what the host can
    /// produce, or everything the host can produce when unset.
    ///
    /// Narrowing only — asking for `ios` on Linux drops it rather than failing,
    /// same as the host gating this replaced. An unknown name is an error
    /// though: it is a typo, and silently building nothing is worse.
    fn selected_targets(ctx: &BuildContext) -> Result<Vec<&'static str>> {
        let host = Self::host_targets();
        let requested = ctx
            .config
            .kmp
            .as_ref()
            .map(|k| k.targets.as_slice())
            .unwrap_or(&[]);

        if requested.is_empty() {
            return Ok(host);
        }

        let mut out: Vec<&'static str> = Vec::new();
        for name in requested {
            let known = Self::ALL_TARGETS
                .iter()
                .find(|t| t.eq_ignore_ascii_case(name))
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "Unknown [kmp].targets entry: '{}'. Known: {}",
                        name,
                        Self::ALL_TARGETS.join(", ")
                    )
                })?;
            if host.contains(known) && !out.contains(known) {
                out.push(known);
            }
        }
        Ok(out)
    }

    /// Gradle klib tasks for a target group. Android and desktop have none:
    /// they are covered by `assemble*` and `desktopJar`.
    fn gradle_tasks_for(target: &str) -> &'static [&'static str] {
        match target {
            "desktop" => &["desktopJar"],
            "ios" => &[
                "iosArm64MainKlibrary",
                "iosX64MainKlibrary",
                "iosSimulatorArm64MainKlibrary",
            ],
            "macos" => &["macosArm64MainKlibrary", "macosX64MainKlibrary"],
            "linux" => &["linuxX64MainKlibrary", "linuxArm64MainKlibrary"],
            _ => &[],
        }
    }

/// Gradle tasks for the selected targets.
    fn gradle_task_list(ctx: &BuildContext) -> Result<Vec<&'static str>> {
        let selected = Self::selected_targets(ctx)?;
        let mut tasks = vec!["clean"];

        // `assemble*` is the androidTarget's task; skip it when Android is out.
        if selected.contains(&"android") {
            tasks.push(if ctx.options.release {
                "assembleRelease"
            } else {
                "assemble"
            });
        }

        for target in &selected {
            tasks.extend(Self::gradle_tasks_for(target));
        }
        Ok(tasks)
    }

    fn build_native_libraries(&self, ctx: &BuildContext) -> Result<()> {
        eprintln!("\n{}", "=".repeat(80));
        eprintln!("Building Native Libraries for KMP");
        eprintln!("{}\n", "=".repeat(80));

        let platforms = Self::selected_targets(ctx)?;

        // Get the current executable path to call ccgo
        let ccgo_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("ccgo"));

        for platform in platforms {
            // Check if native libraries already exist
            if self.native_libs_exist(ctx, platform) {
                eprintln!(
                    "✅ {} native libraries already exist, skipping build.\n",
                    platform
                );
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
            eprintln!(
                "Running: {} {} (in {})",
                gradlew,
                args.join(" "),
                kmp_dir.display()
            );
        }

        let mut cmd = Command::new(gradlew);
        cmd.current_dir(&kmp_dir);
        cmd.args(args);

        // Tell the build script which ccgo_build/<mode> tree holds the native
        // libraries it should wire into cinterop and jniLibs. Gradle cannot work
        // this out on its own -- ccgo passes neither -P properties nor
        // ORG_GRADLE_PROJECT_* env vars -- so without it a project has to hardcode
        // one mode and silently reads the wrong tree (or none at all, producing
        // klibs with no native code) whenever the other mode is built.
        cmd.arg(format!(
            "-PccgoBuildType={}",
            if ctx.options.release {
                "release"
            } else {
                "debug"
            }
        ));

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
    fn collect_klib_artifacts(
        &self,
        classes_dir: &PathBuf,
        outputs: &mut Vec<PathBuf>,
    ) -> Result<()> {
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
                    if path.extension().is_some_and(|e| e == "klib") {
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
                    if path.extension().is_some_and(|e| e == "klib") {
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
                            let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
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
            let file_name = output.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let ext = output.extension().and_then(|e| e.to_str()).unwrap_or("");

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
        let has_gradle_config =
            kmp_dir.join("build.gradle.kts").exists() || kmp_dir.join("build.gradle").exists();
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

        let tasks = Self::gradle_task_list(ctx)?;

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

#[cfg(test)]
mod target_tests {
    use super::*;
    use crate::builder::{BuildContext, BuildOptions};

    fn ctx(toml: &str) -> BuildContext {
        let toml = format!("[package]\nname = \"demo\"\nversion = \"1.0.0\"\n\n{toml}");
        let config = toml::from_str(&toml).expect("toml should parse");
        BuildContext::new(
            std::path::PathBuf::from("/tmp/test"),
            config,
            BuildOptions::default(),
        )
    }

    #[test]
    fn unset_targets_keeps_every_host_target() {
        let selected = KmpBuilder::selected_targets(&ctx("")).unwrap();
        assert_eq!(selected, KmpBuilder::host_targets());
        assert!(selected.contains(&"android"));
    }

    #[test]
    fn android_only_drops_desktop_and_the_klib_tasks() {
        let selected =
            KmpBuilder::selected_targets(&ctx("[kmp]\ntargets = [\"android\"]\n")).unwrap();
        assert_eq!(selected, vec!["android"]);
        // android contributes assemble*, never a klib task
        assert!(KmpBuilder::gradle_tasks_for("android").is_empty());
        for dropped in ["desktop", "ios", "macos", "linux"] {
            assert!(!selected.contains(&dropped), "{dropped} should be gone");
        }
    }

    #[test]
    fn a_target_the_host_cannot_build_is_dropped_not_fatal() {
        // ios on Linux, linux on macOS: narrowing, same as the old host gating.
        let selected =
            KmpBuilder::selected_targets(&ctx("[kmp]\ntargets = [\"android\", \"ios\"]\n"))
                .unwrap();
        assert!(selected.contains(&"android"));
        assert_eq!(
            selected.contains(&"ios"),
            KmpBuilder::host_targets().contains(&"ios")
        );
    }

    #[test]
    fn a_typo_is_an_error_not_an_empty_build() {
        let err = KmpBuilder::selected_targets(&ctx("[kmp]\ntargets = [\"andriod\"]\n"))
            .expect_err("typo must not silently build nothing");
        assert!(err.to_string().contains("andriod"), "{err}");
    }

    #[test]
    fn ohos_is_not_a_kmp_target() {
        assert!(KmpBuilder::selected_targets(&ctx("[kmp]\ntargets = [\"ohos\"]\n")).is_err());
    }
}
