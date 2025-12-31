//! CMake configuration and execution
//!
//! This module handles invoking CMake for configure, build, and install steps.

use std::path::PathBuf;
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};


/// CMake build type
#[derive(Debug, Clone, Copy, Default)]
pub enum BuildType {
    Debug,
    #[default]
    Release,
    RelWithDebInfo,
    MinSizeRel,
}

impl std::fmt::Display for BuildType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BuildType::Debug => write!(f, "Debug"),
            BuildType::Release => write!(f, "Release"),
            BuildType::RelWithDebInfo => write!(f, "RelWithDebInfo"),
            BuildType::MinSizeRel => write!(f, "MinSizeRel"),
        }
    }
}

/// CMake configuration builder
#[derive(Debug, Default)]
pub struct CMakeConfig {
    /// Source directory (where CMakeLists.txt is located)
    source_dir: PathBuf,
    /// Build directory
    build_dir: PathBuf,
    /// Install prefix
    install_prefix: Option<PathBuf>,
    /// Build type
    build_type: BuildType,
    /// CMake variables (-D options)
    variables: Vec<(String, String)>,
    /// CMake cache variables (-D with type)
    cache_variables: Vec<(String, String, String)>,
    /// Generator (e.g., "Ninja", "Unix Makefiles")
    generator: Option<String>,
    /// Toolchain file
    toolchain_file: Option<PathBuf>,
    /// Number of parallel jobs
    jobs: Option<usize>,
    /// Verbose output
    verbose: bool,
}

impl CMakeConfig {
    /// Create a new CMake configuration
    pub fn new(source_dir: PathBuf, build_dir: PathBuf) -> Self {
        Self {
            source_dir,
            build_dir,
            build_type: BuildType::Release,
            ..Default::default()
        }
    }

    /// Set the build type
    pub fn build_type(mut self, build_type: BuildType) -> Self {
        self.build_type = build_type;
        self
    }

    /// Set the install prefix
    pub fn install_prefix(mut self, prefix: PathBuf) -> Self {
        self.install_prefix = Some(prefix);
        self
    }

    /// Set a CMake variable
    pub fn variable(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.push((name.into(), value.into()));
        self
    }

    /// Set multiple CMake variables
    pub fn variables(mut self, vars: Vec<(String, String)>) -> Self {
        self.variables.extend(vars);
        self
    }

    /// Set a cache variable with type
    pub fn cache_variable(
        mut self,
        name: impl Into<String>,
        value: impl Into<String>,
        var_type: impl Into<String>,
    ) -> Self {
        self.cache_variables
            .push((name.into(), value.into(), var_type.into()));
        self
    }

    /// Set the generator
    pub fn generator(mut self, generator: impl Into<String>) -> Self {
        self.generator = Some(generator.into());
        self
    }

    /// Set the toolchain file
    pub fn toolchain_file(mut self, path: PathBuf) -> Self {
        self.toolchain_file = Some(path);
        self
    }

    /// Set number of parallel jobs
    pub fn jobs(mut self, jobs: usize) -> Self {
        self.jobs = Some(jobs);
        self
    }

    /// Enable verbose output
    pub fn verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    /// Find CMake executable
    fn find_cmake() -> Result<PathBuf> {
        which::which("cmake").context("CMake not found. Please install CMake and add it to PATH.")
    }

    /// Run CMake configure step
    pub fn configure(&self) -> Result<()> {
        let cmake = Self::find_cmake()?;

        // Create build directory if it doesn't exist
        std::fs::create_dir_all(&self.build_dir)
            .context("Failed to create CMake build directory")?;

        let mut cmd = Command::new(&cmake);
        cmd.current_dir(&self.build_dir);

        // Source directory
        cmd.arg("-S").arg(&self.source_dir);
        cmd.arg("-B").arg(&self.build_dir);

        // Build type
        cmd.arg(format!("-DCMAKE_BUILD_TYPE={}", self.build_type));

        // Install prefix
        if let Some(prefix) = &self.install_prefix {
            cmd.arg(format!("-DCMAKE_INSTALL_PREFIX={}", prefix.display()));
        }

        // Generator
        if let Some(generator) = &self.generator {
            cmd.arg("-G").arg(generator);
        }

        // Toolchain file
        if let Some(toolchain) = &self.toolchain_file {
            cmd.arg(format!(
                "-DCMAKE_TOOLCHAIN_FILE={}",
                toolchain.display()
            ));
        }

        // Variables
        for (name, value) in &self.variables {
            cmd.arg(format!("-D{}={}", name, value));
        }

        // Cache variables with type
        for (name, value, var_type) in &self.cache_variables {
            cmd.arg(format!("-D{}:{}={}", name, var_type, value));
        }

        if self.verbose {
            eprintln!("Running: {:?}", cmd);
        }

        let status = cmd
            .stdin(Stdio::null())
            .status()
            .context("Failed to run CMake configure")?;

        if !status.success() {
            bail!(
                "CMake configure failed with exit code: {:?}",
                status.code()
            );
        }

        Ok(())
    }

    /// Run CMake build step
    pub fn build(&self) -> Result<()> {
        let cmake = Self::find_cmake()?;

        let mut cmd = Command::new(&cmake);
        cmd.arg("--build").arg(&self.build_dir);

        // Parallel jobs
        if let Some(jobs) = self.jobs {
            cmd.arg("-j").arg(jobs.to_string());
        } else {
            cmd.arg("-j");
        }

        // Verbose
        if self.verbose {
            cmd.arg("--verbose");
        }

        if self.verbose {
            eprintln!("Running: {:?}", cmd);
        }

        let status = cmd
            .stdin(Stdio::null())
            .status()
            .context("Failed to run CMake build")?;

        if !status.success() {
            bail!("CMake build failed with exit code: {:?}", status.code());
        }

        Ok(())
    }

    /// Run CMake install step
    pub fn install(&self) -> Result<()> {
        let cmake = Self::find_cmake()?;

        let mut cmd = Command::new(&cmake);
        cmd.arg("--install").arg(&self.build_dir);

        if self.verbose {
            eprintln!("Running: {:?}", cmd);
        }

        let status = cmd
            .stdin(Stdio::null())
            .status()
            .context("Failed to run CMake install")?;

        if !status.success() {
            bail!("CMake install failed with exit code: {:?}", status.code());
        }

        Ok(())
    }

    /// Run configure, build, and install in sequence
    pub fn configure_build_install(&self) -> Result<()> {
        self.configure()?;
        self.build()?;
        self.install()?;
        Ok(())
    }
}

/// Check if CMake is available
pub fn is_cmake_available() -> bool {
    which::which("cmake").is_ok()
}

/// Get CMake version
pub fn cmake_version() -> Option<String> {
    let output = Command::new("cmake").arg("--version").output().ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Parse "cmake version X.Y.Z"
    stdout
        .lines()
        .next()
        .and_then(|line| line.strip_prefix("cmake version "))
        .map(|v| v.to_string())
}
