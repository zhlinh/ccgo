//! Kotlin Multiplatform (KMP) platform builder
//!
//! Builds KMP library for all supported platforms using Gradle.
//! This builder wraps the Python build_kmp.py script for now.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Context, Result};

use crate::build::{BuildContext, BuildResult, PlatformBuilder};
use crate::exec::python::run_python_script;

/// KMP platform builder
pub struct KmpBuilder {}

impl KmpBuilder {
    pub fn new() -> Self {
        Self {}
    }

    /// Find the Python build script in ccgo package
    fn find_build_script() -> Result<PathBuf> {
        // Check environment variable first
        if let Ok(ccgo_dir) = std::env::var("CCGO_DIR") {
            let script = PathBuf::from(ccgo_dir)
                .join("build_scripts")
                .join("build_kmp.py");
            if script.exists() {
                return Ok(script);
            }
        }

        // Try to find from Python package
        let output = std::process::Command::new("python3")
            .args(&[
                "-c",
                "import ccgo; import os; print(os.path.dirname(ccgo.__file__))",
            ])
            .output()?;

        if output.status.success() {
            let ccgo_path = String::from_utf8_lossy(&output.stdout).trim().to_string();
            let script = PathBuf::from(ccgo_path)
                .join("build_scripts")
                .join("build_kmp.py");
            if script.exists() {
                return Ok(script);
            }
        }

        bail!("Could not find build_kmp.py. Please ensure ccgo Python package is installed.")
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
        let gradlew = kmp_dir.join("gradlew");
        if !gradlew.exists() {
            bail!(
                "gradlew not found in KMP directory: {}\n\
                 Please ensure the KMP module is properly initialized",
                kmp_dir.display()
            );
        }

        // Check if Python ccgo package is available
        Self::find_build_script()
            .context("KMP build requires ccgo Python package to be installed")?;

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

        // Find the Python build script
        let build_script = Self::find_build_script()?;

        // Run the Python build script
        let status = run_python_script(&build_script, &[], Some(&ctx.project_root))?;

        if !status.success() {
            bail!("KMP build failed");
        }

        // Find the generated ZIP archive
        // Path: target/{debug|release}/kmp/{PROJECT}_KMP_SDK-{version}.zip
        let target_subdir = if ctx.options.release {
            "release"
        } else {
            "debug"
        };
        let kmp_target_dir = ctx.project_root.join("target").join(target_subdir).join("kmp");

        // Find the ZIP file
        let mut sdk_archive = None;
        if kmp_target_dir.exists() {
            for entry in std::fs::read_dir(&kmp_target_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "zip") {
                    let file_name = path.file_name().unwrap().to_str().unwrap();
                    if file_name.contains("_KMP_SDK-") {
                        sdk_archive = Some(path);
                        break;
                    }
                }
            }
        }

        let sdk_archive = sdk_archive.ok_or_else(|| {
            anyhow::anyhow!("KMP SDK archive not found in {}", kmp_target_dir.display())
        })?;

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
