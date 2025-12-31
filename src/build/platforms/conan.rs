//! Conan package manager platform builder
//!
//! Builds C/C++ library using Conan package manager.
//! This builder wraps the Python build_conan.py script for now.

use std::path::PathBuf;
use std::time::Instant;

use anyhow::{bail, Context, Result};

use crate::build::{BuildContext, BuildResult, PlatformBuilder};
use crate::commands::build::LinkType;
use crate::exec::python::run_python_script;

/// Conan platform builder
pub struct ConanBuilder {}

impl ConanBuilder {
    pub fn new() -> Self {
        Self {}
    }

    /// Find the Python build script in ccgo package
    fn find_build_script() -> Result<PathBuf> {
        // Check environment variable first
        if let Ok(ccgo_dir) = std::env::var("CCGO_DIR") {
            let script = PathBuf::from(ccgo_dir)
                .join("build_scripts")
                .join("build_conan.py");
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
                .join("build_conan.py");
            if script.exists() {
                return Ok(script);
            }
        }

        bail!("Could not find build_conan.py. Please ensure ccgo Python package is installed.")
    }

    /// Check if Conan is installed
    fn check_conan_installed() -> Result<String> {
        let output = std::process::Command::new("conan")
            .arg("--version")
            .output()
            .context("Failed to run 'conan --version'. Is Conan installed?")?;

        if !output.status.success() {
            bail!("Conan is not installed or not in PATH.\nPlease install: pip install conan");
        }

        let version = String::from_utf8_lossy(&output.stdout);
        Ok(version.trim().to_string())
    }
}

impl PlatformBuilder for ConanBuilder {
    fn platform_name(&self) -> &str {
        "conan"
    }

    fn default_architectures(&self) -> Vec<String> {
        // Conan builds for the host architecture by default
        vec![]
    }

    fn validate_prerequisites(&self, ctx: &BuildContext) -> Result<()> {
        // Check if Conan is installed
        let version = Self::check_conan_installed()
            .context("Conan build requires Conan to be installed")?;

        if ctx.options.verbose {
            eprintln!("Found {}", version);
        }

        // Check if Python ccgo package is available
        Self::find_build_script()
            .context("Conan build requires ccgo Python package to be installed")?;

        if ctx.options.verbose {
            eprintln!("Conan prerequisites validated");
        }

        Ok(())
    }

    fn build(&self, ctx: &BuildContext) -> Result<BuildResult> {
        let start = Instant::now();

        // Validate prerequisites first
        self.validate_prerequisites(ctx)?;

        if ctx.options.verbose {
            eprintln!("Building {} for Conan...", ctx.lib_name());
        }

        // Find the Python build script
        let build_script = Self::find_build_script()?;

        // Prepare arguments based on link type
        let link_type_arg = match ctx.options.link_type {
            LinkType::Static => "--link-type=static",
            LinkType::Shared => "--link-type=shared",
            LinkType::Both => "--link-type=both",
        };

        // Run the Python build script
        let status = run_python_script(
            &build_script,
            &[link_type_arg],
            Some(&ctx.project_root),
        )?;

        if !status.success() {
            bail!("Conan build failed");
        }

        // Python script outputs to target/conan/, we need to move it to target/{debug|release}/conan/
        let python_output_dir = ctx.project_root.join("target").join("conan");

        // Determine final output directory based on release mode
        let target_subdir = if ctx.options.release {
            "release"
        } else {
            "debug"
        };
        let final_output_dir = ctx
            .project_root
            .join("target")
            .join(target_subdir)
            .join("conan");

        // Find the ZIP file in Python output directory
        let mut temp_archive = None;
        if python_output_dir.exists() {
            for entry in std::fs::read_dir(&python_output_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "zip") {
                    let file_name = path.file_name().unwrap().to_str().unwrap();
                    if file_name.contains("_CONAN_SDK-") {
                        temp_archive = Some(path);
                        break;
                    }
                }
            }
        }

        let temp_archive = temp_archive.ok_or_else(|| {
            anyhow::anyhow!(
                "Conan SDK archive not found in {}",
                python_output_dir.display()
            )
        })?;

        // Create final output directory
        std::fs::create_dir_all(&final_output_dir)?;

        // Move the archive to the final location
        let archive_name = temp_archive.file_name().unwrap();
        let sdk_archive = final_output_dir.join(archive_name);
        std::fs::rename(&temp_archive, &sdk_archive)
            .with_context(|| {
                format!(
                    "Failed to move archive from {} to {}",
                    temp_archive.display(),
                    sdk_archive.display()
                )
            })?;

        // Move metadata files (archive_info.json, build_info.json) if they exist
        for metadata_file in &["archive_info.json", "build_info.json"] {
            let src = python_output_dir.join(metadata_file);
            if src.exists() {
                let dst = final_output_dir.join(metadata_file);
                let _ = std::fs::rename(&src, &dst);
            }
        }

        // Clean up the temporary Python output directory
        if python_output_dir.exists() {
            let _ = std::fs::remove_dir_all(&python_output_dir);
        }

        let duration = start.elapsed();

        if ctx.options.verbose {
            eprintln!(
                "Conan build completed in {:.2}s: {}",
                duration.as_secs_f64(),
                sdk_archive.display()
            );
        }

        Ok(BuildResult {
            sdk_archive,
            symbols_archive: None,
            aar_archive: None,
            duration_secs: duration.as_secs_f64(),
            architectures: vec![], // Conan builds for host architecture
        })
    }

    fn clean(&self, ctx: &BuildContext) -> Result<()> {
        // Clean cmake_build/conan directory
        let build_dir = ctx.project_root.join("cmake_build/conan");
        if build_dir.exists() {
            std::fs::remove_dir_all(&build_dir)
                .with_context(|| format!("Failed to clean {}", build_dir.display()))?;
        }

        // Clean target/conan directory (Python output)
        let target_dir = ctx.project_root.join("target/conan");
        if target_dir.exists() {
            std::fs::remove_dir_all(&target_dir)
                .with_context(|| format!("Failed to clean {}", target_dir.display()))?;
        }

        // Clean debug/release conan directories
        for subdir in &["debug", "release"] {
            let conan_target = ctx.project_root.join("target").join(subdir).join("conan");
            if conan_target.exists() {
                std::fs::remove_dir_all(&conan_target)
                    .with_context(|| format!("Failed to clean {}", conan_target.display()))?;
            }
        }

        // Clean conan/ directory build artifacts if it exists
        let conan_dir = ctx.project_root.join("conan");
        if conan_dir.exists() {
            // Clean only build directories, not conanfile.py
            let build_dir = conan_dir.join("build");
            if build_dir.exists() {
                std::fs::remove_dir_all(&build_dir)
                    .with_context(|| format!("Failed to clean {}", build_dir.display()))?;
            }
        }

        Ok(())
    }
}

impl Default for ConanBuilder {
    fn default() -> Self {
        Self::new()
    }
}
