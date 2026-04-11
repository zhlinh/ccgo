//! Documentation command implementation

use anyhow::{anyhow, Context, Result};
use clap::Args;
use console::style;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Check if a command is available in PATH
fn check_command_installed(command: &str) -> bool {
    which::which(command).is_ok()
}

/// Get the available pip command (prefer pip over pip3)
fn get_pip_command() -> Option<&'static str> {
    if check_command_installed("pip") {
        Some("pip")
    } else if check_command_installed("pip3") {
        Some("pip3")
    } else {
        None
    }
}

/// Check if a Python package is installed
fn check_python_package_installed(package: &str) -> bool {
    Command::new("python3")
        .args(["-c", &format!("import {}", package)])
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Install Python dependencies from requirements.txt
fn install_python_requirements(requirements_file: &Path) -> Result<()> {
    let pip_cmd = get_pip_command().ok_or_else(|| {
        anyhow!("Neither pip nor pip3 found in PATH. Please install Python pip first.")
    })?;

    println!(
        "{}",
        style(format!(
            "Installing dependencies from {}...",
            requirements_file.display()
        ))
        .cyan()
    );

    // Try without --break-system-packages first
    let status = Command::new(pip_cmd)
        .args(["install", "-r"])
        .arg(requirements_file)
        .status()
        .with_context(|| format!("Failed to execute {}", pip_cmd))?;

    if status.success() {
        println!(
            "{}",
            style("Dependencies installed successfully!\n").green()
        );
        return Ok(());
    }

    // If failed, try with --break-system-packages (needed for PEP 668 compliant systems like Ubuntu 24.04+)
    println!(
        "{}",
        style("Retrying with --break-system-packages for PEP 668 compliance...").yellow()
    );

    let status = Command::new(pip_cmd)
        .args(["install", "--break-system-packages", "-r"])
        .arg(requirements_file)
        .status()
        .with_context(|| format!("Failed to execute {}", pip_cmd))?;

    if !status.success() {
        return Err(anyhow!(
            "Failed to install dependencies.\n\
             You may need to create a virtual environment:\n\
             python3 -m venv .venv && source .venv/bin/activate && pip install -r {}",
            requirements_file.display()
        ));
    }

    println!(
        "{}",
        style("Dependencies installed successfully!\n").green()
    );
    Ok(())
}

/// Documentation engine detected in the project
enum DocEngine {
    MkDocs(PathBuf),  // path to project dir containing mkdocs.yml
    Doxygen(PathBuf), // path to Doxyfile or Doxyfile.in
}

/// Find the documentation engine configured in the project
fn find_doc_engine(start_dir: &Path) -> Result<DocEngine> {
    // First check for mkdocs.yml
    let mkdocs_yml = start_dir.join("mkdocs.yml");
    if mkdocs_yml.is_file() {
        return Ok(DocEngine::MkDocs(start_dir.to_path_buf()));
    }

    // Check immediate subdirectories for mkdocs.yml
    if let Ok(entries) = std::fs::read_dir(start_dir) {
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let mkdocs_yml = entry.path().join("mkdocs.yml");
            if mkdocs_yml.is_file() {
                return Ok(DocEngine::MkDocs(entry.path()));
            }
        }
    }

    // Check for Doxyfile / Doxyfile.in
    for name in &["Doxyfile", "Doxyfile.in"] {
        let doxyfile = start_dir.join(name);
        if doxyfile.is_file() {
            return Ok(DocEngine::Doxygen(doxyfile));
        }
        let doxyfile = start_dir.join("docs").join(name);
        if doxyfile.is_file() {
            return Ok(DocEngine::Doxygen(doxyfile));
        }
    }

    Err(anyhow!(
        "No documentation configuration found in project directory.\n\
         Supported: mkdocs.yml (MkDocs) or Doxyfile/Doxyfile.in (Doxygen).\n\
         Expected location: <project>/mkdocs.yml, <project>/docs/Doxyfile, etc."
    ))
}

/// Build documentation using Doxygen
fn build_doxygen(doxyfile: &Path, project_dir: &Path, open: bool) -> Result<()> {
    if !check_command_installed("doxygen") {
        return Err(anyhow!(
            "Doxygen is not installed or not in PATH.\n\
             Install it with:\n\
               macOS: brew install doxygen\n\
               Ubuntu: sudo apt-get install doxygen\n\
               Windows: choco install doxygen"
        ));
    }

    println!("Using Doxygen: {}", doxyfile.display());

    let doxyfile_path = if doxyfile.extension().is_some_and(|e| e == "in") {
        // Doxyfile.in needs variable substitution — generate a temporary Doxyfile
        let content = std::fs::read_to_string(doxyfile)
            .with_context(|| format!("Failed to read {}", doxyfile.display()))?;

        // Get project name from CCGO.toml if available
        let project_name = project_dir
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        let output_dir = project_dir.join("target").join("docs").join("doxygen");
        std::fs::create_dir_all(&output_dir)?;

        let content = content
            .replace("@CMAKE_CURRENT_SOURCE_DIR@", &project_dir.to_string_lossy())
            .replace("@PROJECT_SOURCE_DIR@", &project_dir.to_string_lossy())
            .replace("@CMAKE_SOURCE_DIR@", &project_dir.to_string_lossy())
            .replace("@PROJECT_NAME@", &project_name)
            .replace("@DOXYGEN_OUTPUT_DIR@", &output_dir.to_string_lossy());

        let generated = project_dir.join("target").join("docs").join("Doxyfile");
        std::fs::write(&generated, content)?;
        generated
    } else {
        doxyfile.to_path_buf()
    };

    println!("Running: doxygen {}\n", doxyfile_path.display());

    let status = Command::new("doxygen")
        .arg(&doxyfile_path)
        .current_dir(project_dir)
        .status()
        .context("Failed to execute doxygen")?;

    if !status.success() {
        return Err(anyhow!("Doxygen documentation build failed"));
    }

    println!(
        "\n{}",
        style("Documentation built successfully!").green().bold()
    );

    // Try to find and open the generated index.html
    let possible_outputs = [
        project_dir
            .join("target")
            .join("docs")
            .join("doxygen")
            .join("html")
            .join("index.html"),
        project_dir.join("docs").join("_html").join("index.html"),
    ];

    for index_path in &possible_outputs {
        if index_path.exists() {
            println!(
                "Documentation location: {}",
                index_path.parent().unwrap().display()
            );
            if open {
                println!("\nOpening documentation in browser...");
                let url = format!("file://{}", index_path.display());
                if let Err(e) = open::that(&url) {
                    println!("Warning: Failed to open browser: {}", e);
                    println!("You can manually open: {}", url);
                }
            }
            return Ok(());
        }
    }

    println!("Documentation output directory: check Doxyfile OUTPUT_DIRECTORY setting");
    Ok(())
}

/// Print the installation status of a dependency
fn print_dep_status(label: &str, installed: bool) {
    let status = if installed {
        style("✓ installed").green()
    } else {
        style("✗ missing").red()
    };
    println!("  {label} {status}");
}

/// Prompt user to install missing deps, or print skip instructions
fn prompt_install_deps(requirements_file: &Path) -> Result<()> {
    print!(
        "{}",
        style("Would you like to install missing dependencies now? [Y/n] ")
            .cyan()
            .bold()
    );
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim().to_lowercase();

    if input.is_empty() || input == "y" || input == "yes" {
        install_python_requirements(requirements_file)?;
    } else {
        let pip_cmd = get_pip_command().unwrap_or("pip");
        println!(
            "{}",
            style("Skipping dependency installation. Documentation build may fail.\n").yellow()
        );
        println!("You can install dependencies later with:");
        println!("  {} install -r {}", pip_cmd, requirements_file.display());
        println!("Or run:");
        println!("  ccgo doc --install-deps\n");
    }

    Ok(())
}

/// Check and optionally install MkDocs dependencies
fn check_and_install_deps(project_dir: &Path, auto_install: bool) -> Result<()> {
    // Check if docs/requirements.txt exists
    let requirements_file = project_dir.join("docs").join("requirements.txt");
    if !requirements_file.is_file() {
        return Ok(());
    }

    // Check if key packages are installed
    let mkdocs_installed = check_python_package_installed("mkdocs");
    let material_installed = check_python_package_installed("material");
    let mkdoxy_installed = check_python_package_installed("mkdoxy");

    if mkdocs_installed && material_installed && mkdoxy_installed {
        return Ok(());
    }

    // Some dependencies are missing
    println!("{}", style("MkDocs dependencies check:").yellow().bold());
    print_dep_status("mkdocs:         ", mkdocs_installed);
    print_dep_status("mkdocs-material:", material_installed);
    print_dep_status("mkdoxy:         ", mkdoxy_installed);
    println!();

    if auto_install {
        install_python_requirements(&requirements_file)?;
        return Ok(());
    }

    prompt_install_deps(&requirements_file)?;

    Ok(())
}

/// Generate MkDocs documentation
#[derive(Args, Debug)]
pub struct DocCommand {
    /// Open documentation in browser after building
    #[arg(long)]
    pub open: bool,

    /// Start MkDocs development server with live reload
    #[arg(long)]
    pub serve: bool,

    /// Port for development server (default: 8000, used with --serve)
    #[arg(long, default_value = "8000")]
    pub port: u16,

    /// Clean build artifacts before generating documentation
    #[arg(long)]
    pub clean: bool,

    /// Automatically install missing dependencies without prompting
    #[arg(long)]
    pub install_deps: bool,
}

impl DocCommand {
    /// Execute the doc command
    pub fn execute(self, _verbose: bool) -> Result<()> {
        println!("{}", style("Building project documentation...\n").bold());

        // Get current working directory
        let start_dir =
            std::env::current_dir().context("Failed to get current working directory")?;

        // Find documentation engine
        let doc_engine = find_doc_engine(&start_dir)?;

        // If Doxygen project, handle it separately
        let project_dir = match &doc_engine {
            DocEngine::Doxygen(doxyfile) => {
                // For Doxygen, --serve is not supported
                if self.serve {
                    return Err(anyhow!("--serve is not supported for Doxygen projects. Use --open to view after build."));
                }
                let project_dir = doxyfile
                    .parent()
                    .and_then(|p| {
                        // If Doxyfile is in docs/, use parent as project dir
                        if p.file_name().is_some_and(|n| n == "docs") {
                            p.parent()
                        } else {
                            Some(p)
                        }
                    })
                    .unwrap_or(&start_dir);
                return build_doxygen(doxyfile, project_dir, self.open);
            }
            DocEngine::MkDocs(dir) => dir.clone(),
        };

        println!("Project directory: {}", project_dir.display());

        // Check and install Python dependencies
        check_and_install_deps(&project_dir, self.install_deps)?;

        // Check dependencies
        if !check_command_installed("mkdocs") {
            let pip_cmd = get_pip_command().unwrap_or("pip");
            return Err(anyhow!(
                "MkDocs is not installed or not in PATH.\n\
                 Install it with: {} install ccgo[docs]\n\
                 Or install from project requirements: {} install -r docs/requirements.txt",
                pip_cmd,
                pip_cmd
            ));
        }

        if !check_command_installed("doxygen") {
            println!(
                "{}",
                style(
                    "WARNING: Doxygen is not installed or not in PATH.\n\
                     MkDoxy requires Doxygen to generate API documentation.\n\
                     Install it with:\n\
                       macOS: brew install doxygen\n\
                       Ubuntu: sudo apt-get install doxygen\n\
                       Windows: choco install doxygen\n\
                     API documentation may not be generated.\n"
                )
                .yellow()
            );
        }

        // Handle --serve mode (development server with live reload)
        if self.serve {
            return Self::run_serve(&project_dir, self.port);
        }

        Self::run_build(&project_dir, self.clean, self.open)
    }

    /// Start MkDocs development server with live reload
    fn run_serve(project_dir: &Path, port: u16) -> Result<()> {
        println!("Starting MkDocs development server on port {port}...");
        println!("Documentation URL: http://127.0.0.1:{port}/");
        println!("Press Ctrl+C to stop the server\n");

        let status = Command::new("mkdocs")
            .args(["serve", "-a", &format!("127.0.0.1:{port}")])
            .current_dir(project_dir)
            .status()
            .context("Failed to execute mkdocs serve")?;

        if !status.success() {
            println!("\nServer stopped");
        }
        Ok(())
    }

    /// Build MkDocs documentation
    fn run_build(project_dir: &Path, clean: bool, open: bool) -> Result<()> {
        println!("Mode: Build documentation\n");

        // Output to target/docs/site/ directory
        let site_dir = project_dir.join("target").join("docs").join("site");
        let mut cmd = Command::new("mkdocs");
        cmd.args(["build", "--site-dir"])
            .arg(&site_dir)
            .current_dir(project_dir);

        if clean {
            cmd.arg("--clean");
            println!("Clean build enabled");
        }

        println!("Running: mkdocs build --site-dir {}", site_dir.display());
        if clean {
            println!("         --clean");
        }
        println!();

        let status = cmd.status().context("Failed to execute mkdocs build")?;

        if !status.success() {
            return Err(anyhow!("Documentation build failed"));
        }

        println!(
            "\n{}",
            style("Documentation built successfully!").green().bold()
        );

        // Get output path
        let index_path = site_dir.join("index.html");

        if index_path.exists() {
            println!("Documentation location: {}", site_dir.display());

            // Handle --open option
            if open {
                println!("\nOpening documentation in browser...");
                let url = format!("file://{}", index_path.display());
                if let Err(e) = open::that(&url) {
                    println!("Warning: Failed to open browser: {}", e);
                    println!("You can manually open: {}", url);
                }
            }
        } else {
            println!("\nWarning: Documentation output not found at expected location");
            println!("   Expected: {}", index_path.display());
        }

        Ok(())
    }
}
