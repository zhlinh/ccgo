//! Run command implementation
//!
//! Builds and runs executable targets (examples or binaries) on the local host.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};
use clap::Args;

use crate::build::cmake::{BuildType, CMakeConfig};
use crate::build::toolchains::detect_default_compiler;
use crate::config::CcgoConfig;

/// Target type to run
#[derive(Debug, Clone)]
enum RunTarget {
    /// Run an example from examples/ directory
    Example(String),
    /// Run a binary target from [[bin]]
    Bin(String),
}

/// Run an executable target (example or binary)
#[derive(Args, Debug)]
pub struct RunCommand {
    /// Name of the example to run
    #[arg(long, group = "target")]
    pub example: Option<String>,

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

    /// Arguments to pass to the executable
    #[arg(last = true)]
    pub args: Vec<String>,
}

impl RunCommand {
    /// Execute the run command
    pub fn execute(self, verbose: bool) -> Result<()> {
        // Load project configuration
        let config = CcgoConfig::load()?;
        let project_root = std::env::current_dir()?;

        // Get package info
        let package = config.require_package()?;

        // Resolve the target to run
        let target = self.resolve_target(&config, &project_root)?;

        if verbose {
            match &target {
                RunTarget::Example(name) => eprintln!("Running example: {}", name),
                RunTarget::Bin(name) => eprintln!("Running binary: {}", name),
            }
        }

        // Get source file path
        let source_path = self.get_source_path(&config, &project_root, &target)?;
        if !source_path.exists() {
            bail!("Source file not found: {}", source_path.display());
        }

        // Determine target name
        let target_name = match &target {
            RunTarget::Example(name) => format!("example_{}", name),
            RunTarget::Bin(name) => name.clone(),
        };

        // Build the executable
        let build_dir = project_root.join("target").join("run").join(&target_name);
        let executable = self.build_target(
            &project_root,
            &build_dir,
            &target_name,
            &source_path,
            &package.name,
            verbose,
        )?;

        // Run the executable (unless build-only)
        if !self.build_only {
            self.run_executable(&executable, verbose)?;
        }

        Ok(())
    }

    /// Resolve which target to run
    fn resolve_target(&self, config: &CcgoConfig, project_root: &Path) -> Result<RunTarget> {
        // Explicit target specified
        if let Some(name) = &self.example {
            return Ok(RunTarget::Example(name.clone()));
        }
        if let Some(name) = &self.bin {
            return Ok(RunTarget::Bin(name.clone()));
        }

        // Auto-discover: prefer examples, then bins
        // First, check configured examples
        if !config.examples.is_empty() {
            return Ok(RunTarget::Example(config.examples[0].name.clone()));
        }

        // Auto-discover examples from examples/ directory
        let examples_dir = project_root.join("examples");
        if examples_dir.exists() {
            if let Some(example) = self.find_first_example(&examples_dir)? {
                return Ok(RunTarget::Example(example));
            }
        }

        // Check configured binaries
        if !config.bins.is_empty() {
            return Ok(RunTarget::Bin(config.bins[0].name.clone()));
        }

        bail!(
            "No target specified and no examples or binaries found.\n\
             Use --example <name> or --bin <name> to specify a target,\n\
             or create an examples/ directory with .cpp files."
        );
    }

    /// Find first example in examples directory
    fn find_first_example(&self, examples_dir: &Path) -> Result<Option<String>> {
        let entries = std::fs::read_dir(examples_dir)
            .with_context(|| format!("Failed to read examples directory: {}", examples_dir.display()))?;

        let mut examples: Vec<String> = Vec::new();

        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                // Check for .cpp, .cc, .cxx files
                if let Some(ext) = path.extension() {
                    if ext == "cpp" || ext == "cc" || ext == "cxx" {
                        if let Some(stem) = path.file_stem() {
                            examples.push(stem.to_string_lossy().to_string());
                        }
                    }
                }
            } else if path.is_dir() {
                // Check for main.cpp in subdirectory
                let main_cpp = path.join("main.cpp");
                if main_cpp.exists() {
                    if let Some(name) = path.file_name() {
                        examples.push(name.to_string_lossy().to_string());
                    }
                }
            }
        }

        examples.sort();
        Ok(examples.into_iter().next())
    }

    /// Get the source file path for the target
    fn get_source_path(&self, config: &CcgoConfig, project_root: &Path, target: &RunTarget) -> Result<PathBuf> {
        match target {
            RunTarget::Example(name) => {
                // Check configured examples first
                if let Some(example_config) = config.examples.iter().find(|e| &e.name == name) {
                    if let Some(path) = &example_config.path {
                        return Ok(project_root.join(path));
                    }
                }

                // Default paths for examples
                let examples_dir = project_root.join("examples");

                // Try examples/{name}.cpp
                let cpp_path = examples_dir.join(format!("{}.cpp", name));
                if cpp_path.exists() {
                    return Ok(cpp_path);
                }

                // Try examples/{name}.cc
                let cc_path = examples_dir.join(format!("{}.cc", name));
                if cc_path.exists() {
                    return Ok(cc_path);
                }

                // Try examples/{name}/main.cpp
                let main_path = examples_dir.join(name).join("main.cpp");
                if main_path.exists() {
                    return Ok(main_path);
                }

                bail!(
                    "Example '{}' not found. Searched:\n  - {}\n  - {}\n  - {}",
                    name,
                    cpp_path.display(),
                    cc_path.display(),
                    main_path.display()
                );
            }
            RunTarget::Bin(name) => {
                // Must be configured in [[bin]]
                if let Some(bin_config) = config.bins.iter().find(|b| &b.name == name) {
                    return Ok(project_root.join(&bin_config.path));
                }

                bail!(
                    "Binary target '{}' not found in CCGO.toml.\n\
                     Add it with:\n\n\
                     [[bin]]\n\
                     name = \"{}\"\n\
                     path = \"src/bin/{}.cpp\"",
                    name, name, name
                );
            }
        }
    }

    /// Build the executable target
    fn build_target(
        &self,
        project_root: &Path,
        build_dir: &Path,
        target_name: &str,
        source_path: &Path,
        lib_name: &str,
        verbose: bool,
    ) -> Result<PathBuf> {
        // Create build directory
        std::fs::create_dir_all(build_dir)
            .with_context(|| format!("Failed to create build directory: {}", build_dir.display()))?;

        // Detect compiler
        let compiler = detect_default_compiler()
            .context("Failed to detect C++ compiler. Please ensure GCC or Clang is installed.")?;

        if verbose {
            eprintln!("Using compiler: {:?}", compiler.compiler_type);
            eprintln!("Build directory: {}", build_dir.display());
        }

        // Generate CMakeLists.txt
        let cmake_content = self.generate_cmake(project_root, target_name, source_path, lib_name)?;
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
            .variable("CMAKE_C_COMPILER", compiler.cc.display().to_string())
            .variable("CMAKE_CXX_COMPILER", compiler.cxx.display().to_string())
            .variable("PROJECT_SOURCE_DIR_OVERRIDE", project_root.display().to_string())
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
        project_root: &Path,
        target_name: &str,
        source_path: &Path,
        lib_name: &str,
    ) -> Result<String> {
        let source_relative = source_path
            .strip_prefix(project_root)
            .unwrap_or(source_path);

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
set(SOURCE_FILE "${{PROJECT_ROOT}}/{source_path}")

# Add executable
add_executable({target_name} ${{SOURCE_FILE}})

# Try to find and link the library if it exists
set(LIB_DIR "${{PROJECT_ROOT}}/target/run/lib")
if(EXISTS "${{LIB_DIR}}")
    target_link_directories({target_name} PRIVATE ${{LIB_DIR}})
    target_link_libraries({target_name} PRIVATE {lib_name} || true)
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
            source_path = source_relative.display(),
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
            build_dir.join("Release").join(format!("{}.exe", target_name)),
        ];

        for path in &candidates {
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
            eprintln!("\nRunning: {} {}", executable.display(), self.args.join(" "));
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

/// List available examples in the project
pub fn list_examples(project_root: &Path, config: &CcgoConfig) -> Result<Vec<String>> {
    let mut examples = Vec::new();

    // Add configured examples
    for example in &config.examples {
        examples.push(example.name.clone());
    }

    // Discover examples from examples/ directory
    let examples_dir = project_root.join("examples");
    if examples_dir.exists() {
        let entries = std::fs::read_dir(&examples_dir)?;
        for entry in entries {
            let entry = entry?;
            let path = entry.path();

            let name = if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "cpp" || ext == "cc" || ext == "cxx" {
                        path.file_stem().map(|s| s.to_string_lossy().to_string())
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else if path.is_dir() && path.join("main.cpp").exists() {
                path.file_name().map(|s| s.to_string_lossy().to_string())
            } else {
                None
            };

            if let Some(name) = name {
                if !examples.contains(&name) {
                    examples.push(name);
                }
            }
        }
    }

    examples.sort();
    examples.dedup();
    Ok(examples)
}

/// List available binary targets in the project
pub fn list_bins(config: &CcgoConfig) -> Vec<String> {
    config.bins.iter().map(|b| b.name.clone()).collect()
}
