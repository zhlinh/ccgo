//! Run command implementation
//!
//! Builds and runs C++ targets and application projects on the local host.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use clap::Args;

use crate::builder::cmake::{BuildType, CMakeConfig};
use crate::builder::toolchains::detect_default_compiler;
use crate::config::CcgoConfig;

pub(super) struct CppBuild<'a> {
    pub project_root: &'a Path,
    pub build_dir: &'a Path,
    pub target_name: &'a str,
    pub source_path: &'a Path,
    pub lib_name: &'a str,
    pub configure_args: &'a [String],
}

/// Build and run an application or binary
#[derive(Args, Debug)]
pub struct RunCommand {
    /// App to run from app/ (C++, CMake/ImGui/Qt, Flutter or Tauri)
    #[arg(long, group = "target")]
    pub app: Option<String>,

    /// Name of the binary target to run
    #[arg(long, group = "target")]
    pub bin: Option<String>,

    /// Build in release mode
    #[arg(long)]
    pub release: bool,

    /// Build only, don't run the executable
    #[arg(long)]
    pub build_only: bool,

    /// Number of parallel jobs for building
    #[arg(short, long)]
    pub jobs: Option<usize>,

    /// Features to enable (comma-separated)
    #[arg(long, short = 'F', value_delimiter = ',')]
    pub features: Vec<String>,

    /// Do not enable default features
    #[arg(long)]
    pub no_default_features: bool,

    /// Enable all available features
    #[arg(long)]
    pub all_features: bool,

    /// Arguments for the executable, or Flutter/Tauri CLI options with --app
    #[arg(last = true)]
    pub args: Vec<String>,
}

impl RunCommand {
    /// Execute the run command
    pub fn execute(self, verbose: bool) -> Result<()> {
        // Load project configuration
        let config = CcgoConfig::load()?;
        let project_root = std::env::current_dir()?;

        if let Some(name) = &self.app {
            return super::run_app::execute(&self, &config, &project_root, name, verbose);
        }
        if let Some(name) = &self.bin {
            return self.run_bin(&config, &project_root, name, verbose);
        }
        if let Some(app) = config.apps.first() {
            return super::run_app::execute(&self, &config, &project_root, &app.name, verbose);
        }
        if let Some(name) = super::run_app::discover_first(&project_root)? {
            return super::run_app::execute(&self, &config, &project_root, &name, verbose);
        }
        if let Some(bin) = config.bins.first() {
            return self.run_bin(&config, &project_root, &bin.name, verbose);
        }
        bail!("No application or binary found. Use --app <name> with app/, or configure [[app]] / [[bin]] in CCGO.toml");
    }

    fn run_bin(&self, config: &CcgoConfig, root: &Path, name: &str, verbose: bool) -> Result<()> {
        let bin = config
            .bins
            .iter()
            .find(|bin| bin.name == name)
            .with_context(|| {
                format!(
                    "Binary target '{name}' not found. Add [[bin]] with name and path to CCGO.toml"
                )
            })?;
        let source = root.join(&bin.path);
        if !source.is_file() {
            bail!("Source file not found: {}", source.display());
        }
        let executable = self.build_target(
            CppBuild {
                project_root: root,
                build_dir: &root.join("target/run").join(name),
                target_name: name,
                source_path: &source,
                lib_name: &config.require_package()?.name,
                configure_args: &[],
            },
            verbose,
        )?;
        if !self.build_only {
            self.run_executable(&executable, verbose)?;
        }
        Ok(())
    }

    /// Build the executable target
    pub(super) fn build_target(&self, target: CppBuild<'_>, verbose: bool) -> Result<PathBuf> {
        let CppBuild {
            project_root,
            build_dir,
            target_name,
            source_path,
            lib_name,
            configure_args,
        } = target;
        // Create build directory
        std::fs::create_dir_all(build_dir).with_context(|| {
            format!("Failed to create build directory: {}", build_dir.display())
        })?;

        // Detect compiler
        let compiler = detect_default_compiler()
            .context("Failed to detect C++ compiler. Please ensure GCC or Clang is installed.")?;

        if verbose {
            eprintln!("Using compiler: {:?}", compiler.compiler_type);
            eprintln!("Build directory: {}", build_dir.display());
        }

        // Generate CMakeLists.txt
        let cmake_content = self.generate_cmake(target_name, source_path, lib_name)?;
        let cmake_path = build_dir.join("CMakeLists.txt");
        std::fs::write(&cmake_path, &cmake_content)
            .with_context(|| format!("Failed to write CMakeLists.txt: {}", cmake_path.display()))?;

        // Configure CMake
        let build_type = if self.release {
            BuildType::Release
        } else {
            BuildType::Debug
        };

        let jobs = self.jobs.unwrap_or_else(|| {
            std::thread::available_parallelism()
                .map(|n| n.get())
                .unwrap_or(4)
        });

        // Build using CMakeConfig
        let cmake = CMakeConfig::new(build_dir.to_path_buf(), build_dir.to_path_buf())
            .build_type(build_type)
            .user_arguments(configure_args.to_vec())
            .variable("CMAKE_C_COMPILER", compiler.cc.display().to_string())
            .variable("CMAKE_CXX_COMPILER", compiler.cxx.display().to_string())
            .variable(
                "PROJECT_SOURCE_DIR_OVERRIDE",
                project_root.display().to_string(),
            )
            .jobs(jobs)
            .verbose(verbose);

        // CMake configure
        if verbose {
            eprintln!("Configuring CMake...");
        }
        cmake.configure()?;

        // CMake build
        if verbose {
            eprintln!("Building {}...", target_name);
        }
        cmake.build()?;

        // Find executable
        let executable = self.find_executable(build_dir, target_name)?;

        eprintln!("✓ Built: {}", executable.display());

        Ok(executable)
    }

    /// Generate CMakeLists.txt for the target
    fn generate_cmake(
        &self,
        target_name: &str,
        source_path: &Path,
        lib_name: &str,
    ) -> Result<String> {
        let source = source_path.to_string_lossy().replace('\\', "/");

        let cmake = format!(
            r#"cmake_minimum_required(VERSION 3.16)
project({target_name} CXX)

set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Project root for includes
set(PROJECT_ROOT "${{PROJECT_SOURCE_DIR}}")
if(DEFINED PROJECT_SOURCE_DIR_OVERRIDE)
    set(PROJECT_ROOT "${{PROJECT_SOURCE_DIR_OVERRIDE}}")
endif()

# Include directories
include_directories(
    ${{PROJECT_ROOT}}/include
    ${{PROJECT_ROOT}}/src
)

# Source file
set(SOURCE_FILE "{source_path}")

# Add executable
add_executable({target_name} ${{SOURCE_FILE}})

# Try to find and link the library if it exists
set(LIB_DIR "${{PROJECT_ROOT}}/target/run/lib")
if(EXISTS "${{LIB_DIR}}")
    target_link_directories({target_name} PRIVATE ${{LIB_DIR}})
    target_link_libraries({target_name} PRIVATE {lib_name})
endif()

# Platform-specific settings
if(APPLE)
    set(CMAKE_MACOSX_RPATH ON)
endif()

if(UNIX AND NOT APPLE)
    target_link_libraries({target_name} PRIVATE pthread)
endif()
"#,
            target_name = target_name,
            source_path = source,
            lib_name = lib_name,
        );

        Ok(cmake)
    }

    /// Find the built executable
    fn find_executable(&self, build_dir: &Path, target_name: &str) -> Result<PathBuf> {
        // Common locations
        let candidates = [
            build_dir.join(target_name),
            build_dir.join(format!("{}.exe", target_name)),
            build_dir.join("Debug").join(target_name),
            build_dir.join("Debug").join(format!("{}.exe", target_name)),
            build_dir.join("Release").join(target_name),
            build_dir
                .join("Release")
                .join(format!("{}.exe", target_name)),
        ];

        let debug_exe = build_dir.join("Debug").join(format!("{target_name}.exe"));
        let debug_bin = build_dir.join("Debug").join(target_name);
        for path in candidates.iter().chain([&debug_exe, &debug_bin]) {
            if path.exists() {
                return Ok(path.clone());
            }
        }

        bail!(
            "Executable not found after build. Searched:\n{}",
            candidates
                .iter()
                .map(|p| format!("  - {}", p.display()))
                .collect::<Vec<_>>()
                .join("\n")
        );
    }

    /// Run the executable with arguments
    fn run_executable(&self, executable: &Path, verbose: bool) -> Result<()> {
        if verbose {
            eprintln!(
                "\nRunning: {} {}",
                executable.display(),
                self.args.join(" ")
            );
            eprintln!("{}", "-".repeat(60));
        }

        let status = Command::new(executable)
            .args(&self.args)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
            .with_context(|| format!("Failed to execute: {}", executable.display()))?;

        if !status.success() {
            if let Some(code) = status.code() {
                bail!("Process exited with code: {}", code);
            } else {
                bail!("Process terminated by signal");
            }
        }

        Ok(())
    }
}
