//! Dispatch application projects to their own build and run tools.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

use super::run::{CppBuild, RunCommand};
use crate::builder::cmake::{BuildType, CMakeConfig};
use crate::config::{AppConfig, AppFramework, CcgoConfig};

pub(super) fn execute(
    options: &RunCommand,
    config: &CcgoConfig,
    root: &Path,
    name: &str,
    verbose: bool,
) -> Result<()> {
    validate_options(options, name)?;
    let matches: Vec<_> = config.apps.iter().filter(|app| app.name == name).collect();
    if matches.len() > 1 {
        bail!("Duplicate [[app]] name: {name}");
    }
    let app = matches.first().copied().cloned().unwrap_or(AppConfig {
        name: name.to_string(),
        path: None,
        framework: None,
        target: None,
        executable: None,
        platform: None,
        device: None,
        build_args: Vec::new(),
        run_args: Vec::new(),
    });
    let default_path = default_path(root, name);
    let path = app
        .path
        .as_ref()
        .map(|path| root.join(path))
        .unwrap_or(default_path);
    if !path.exists() {
        bail!(
            "App path not found: {}. Set [[app]].path or create app/{name}/",
            path.display()
        );
    }
    let framework = match app.framework {
        Some(framework) => framework,
        None if is_cpp_source(&path) => AppFramework::Cpp,
        None => detect_framework(&path)?,
    };
    if framework == AppFramework::Cpp {
        return run_cpp(options, config, &app, root, &path, verbose);
    }
    if !path.is_dir() {
        bail!(
            "Application framework requires a directory: {}",
            path.display()
        );
    }
    match framework {
        AppFramework::Cpp => unreachable!(),
        AppFramework::Cmake => run_cmake(options, &app, root, &path, verbose),
        AppFramework::Flutter => run_flutter(options, &app, &path, verbose),
        AppFramework::Tauri => run_tauri(options, &app, &path, verbose),
    }
}

fn validate_options(options: &RunCommand, name: &str) -> Result<()> {
    if name.is_empty()
        || name == "."
        || name == ".."
        || !name
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
    {
        bail!("App name must contain only letters, digits, '-', '_' or '.'");
    }
    if !options.features.is_empty() || options.all_features || options.no_default_features {
        bail!("--app does not use ccgo feature flags; configure app build_args/run_args instead");
    }
    Ok(())
}

fn is_cpp_source(path: &Path) -> bool {
    path.is_file()
        && path.extension().is_some_and(|ext| {
            ["cpp", "cc", "cxx"]
                .iter()
                .any(|candidate| ext == *candidate)
        })
}

fn cpp_main(directory: &Path) -> Option<PathBuf> {
    ["cpp", "cc", "cxx"]
        .iter()
        .map(|ext| directory.join(format!("main.{ext}")))
        .find(|path| path.is_file())
}

fn default_path(root: &Path, name: &str) -> PathBuf {
    let directory = root.join("app").join(name);
    if directory.exists() {
        return directory;
    }
    ["cpp", "cc", "cxx"]
        .iter()
        .map(|ext| root.join("app").join(format!("{name}.{ext}")))
        .find(|path| path.is_file())
        .unwrap_or(directory)
}

pub(super) fn discover_first(root: &Path) -> Result<Option<String>> {
    let directory = root.join("app");
    if !directory.is_dir() {
        return Ok(None);
    }
    let mut names = Vec::new();
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        let name = if is_cpp_source(&path) {
            path.file_stem()
        } else if path.is_dir() && (cpp_main(&path).is_some() || has_manifest(&path)) {
            path.file_name()
        } else {
            None
        };
        if let Some(name) = name.and_then(|name| name.to_str()) {
            names.push(name.to_string());
        }
    }
    names.sort();
    Ok(names.into_iter().next())
}

fn has_manifest(directory: &Path) -> bool {
    directory.join("CMakeLists.txt").is_file()
        || directory.join("pubspec.yaml").is_file()
        || ["tauri.conf.json", "tauri.conf.json5", "Tauri.toml"]
            .iter()
            .any(|name| directory.join("src-tauri").join(name).is_file())
}

fn run_cpp(
    options: &RunCommand,
    config: &CcgoConfig,
    app: &AppConfig,
    root: &Path,
    path: &Path,
    verbose: bool,
) -> Result<()> {
    let source = if is_cpp_source(path) {
        path.to_path_buf()
    } else {
        cpp_main(path).context("C++ app requires a .cpp/.cc/.cxx source file or a directory containing main.cpp/main.cc/main.cxx")?
    };
    let mode = if options.release { "release" } else { "debug" };
    let build = root.join("target/run/app").join(&app.name).join(mode);
    let executable = options.build_target(
        CppBuild {
            project_root: root,
            build_dir: &build,
            target_name: &app.name,
            source_path: &source,
            lib_name: &config.require_package()?.name,
            configure_args: &app.build_args,
        },
        verbose,
    )?;
    if options.build_only {
        return Ok(());
    }
    let mut command = Command::new(executable);
    command.args(&app.run_args).args(&options.args);
    checked(&mut command, source.parent().unwrap_or(root), verbose)
}

fn detect_framework(directory: &Path) -> Result<AppFramework> {
    let mut found = Vec::new();
    if directory.join("CMakeLists.txt").is_file() {
        found.push(AppFramework::Cmake);
    }
    if directory.join("pubspec.yaml").is_file() {
        found.push(AppFramework::Flutter);
    }
    if ["tauri.conf.json", "tauri.conf.json5", "Tauri.toml"]
        .iter()
        .any(|file| directory.join("src-tauri").join(file).is_file())
    {
        found.push(AppFramework::Tauri);
    }
    match found.as_slice() {
        [framework] => Ok(*framework),
        [] if cpp_main(directory).is_some() => Ok(AppFramework::Cpp),
        [] => bail!("No supported application in {}. Expected main.cpp/.cc/.cxx, CMakeLists.txt, pubspec.yaml or src-tauri/tauri.conf.json", directory.display()),
        _ => bail!("Multiple app frameworks found in {}; set [[app]].framework explicitly", directory.display()),
    }
}

fn run_cmake(
    options: &RunCommand,
    app: &AppConfig,
    root: &Path,
    directory: &Path,
    verbose: bool,
) -> Result<()> {
    if !directory.join("CMakeLists.txt").is_file() {
        bail!(
            "CMake app requires {}",
            directory.join("CMakeLists.txt").display()
        );
    }
    let mode = if options.release { "Release" } else { "Debug" };
    let build = root
        .join("target/run/app")
        .join(&app.name)
        .join(mode.to_ascii_lowercase());
    let mut cmake = CMakeConfig::new(directory.to_path_buf(), build.clone())
        .build_type(if options.release {
            BuildType::Release
        } else {
            BuildType::Debug
        })
        .user_arguments(app.build_args.clone())
        .verbose(verbose);
    if let Some(jobs) = options.jobs {
        cmake = cmake.jobs(jobs);
    }
    cmake.configure()?;
    let mut command = Command::new(which::which("cmake").context("CMake not found")?);
    command.arg("--build").arg(&build).args(["--config", mode]);
    if let Some(target) = &app.target {
        command.args(["--target", target]);
    }
    command.arg("--parallel");
    if let Some(jobs) = options.jobs {
        command.arg(jobs.to_string());
    }
    if verbose {
        command.arg("--verbose");
    }
    checked(&mut command, directory, verbose)?;
    if options.build_only {
        return Ok(());
    }
    let executable = find_executable(&build, mode, app)?;
    let mut command = Command::new(executable);
    command.args(&app.run_args).args(&options.args);
    checked(&mut command, directory, verbose)
}

fn find_executable(build: &Path, mode: &str, app: &AppConfig) -> Result<PathBuf> {
    let default = app.target.as_deref().unwrap_or(&app.name);
    let relative = app.executable.clone().unwrap_or_else(|| {
        PathBuf::from(if cfg!(windows) {
            format!("{default}.exe")
        } else {
            default.to_string()
        })
    });
    for base in [build.to_path_buf(), build.join(mode)] {
        let mut path = base.join(&relative);
        if cfg!(windows) && path.extension().is_none() && !path.is_file() {
            path.set_extension("exe");
        }
        if path.extension().is_some_and(|ext| ext == "app") {
            if let Some(stem) = path.file_stem() {
                let binary = path.join("Contents/MacOS").join(stem);
                if binary.is_file() {
                    return Ok(binary);
                }
            }
        } else if path.is_file() {
            return Ok(path);
        }
    }
    bail!("App executable not found under {}. Set [[app]].executable relative to the build directory (for example bin/demo or Demo.app)", build.display())
}

fn run_flutter(
    options: &RunCommand,
    app: &AppConfig,
    directory: &Path,
    verbose: bool,
) -> Result<()> {
    if !directory.join("pubspec.yaml").is_file() {
        bail!(
            "Flutter app requires pubspec.yaml in {}",
            directory.display()
        );
    }
    let platform = app.platform.as_deref().unwrap_or(std::env::consts::OS);
    let mut command = platform_command("flutter");
    if options.build_only {
        command.args(["build", platform]);
    } else {
        command.arg("run");
        let default_device = ["macos", "linux", "windows"]
            .contains(&platform)
            .then_some(platform);
        if let Some(device) = app.device.as_deref().or(default_device) {
            command.args(["--device-id", device]);
        }
    }
    command.arg(if options.release {
        "--release"
    } else {
        "--debug"
    });
    if options.build_only {
        command.args(&app.build_args);
    } else {
        command.args(&app.run_args);
    }
    command.args(&options.args);
    checked(&mut command, directory, verbose)
}

fn run_tauri(options: &RunCommand, app: &AppConfig, directory: &Path, verbose: bool) -> Result<()> {
    let mut command = Command::new("cargo");
    command.args(["tauri", if options.build_only { "build" } else { "dev" }]);
    if options.build_only {
        if !options.release {
            command.arg("--debug");
        }
        command.args(&app.build_args);
    } else {
        if options.release {
            command.arg("--release");
        }
        command.args(&app.run_args);
    }
    command.args(&options.args);
    if let Some(jobs) = options.jobs {
        command.env("CARGO_BUILD_JOBS", jobs.to_string());
    }
    checked(&mut command, directory, verbose)
}

fn platform_command(program: &str) -> Command {
    // Flutter is a .bat entry point on Windows.
    if cfg!(windows) {
        let mut command = Command::new("cmd");
        command.args(["/C", program]);
        command
    } else {
        Command::new(program)
    }
}

fn checked(command: &mut Command, directory: &Path, verbose: bool) -> Result<()> {
    command
        .current_dir(directory)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    if verbose {
        eprintln!("Running app command: {command:?}");
    }
    let status = command.status().with_context(|| format!("Failed to execute app command {command:?}. Install the application's build tools first"))?;
    if !status.success() {
        bail!("App command failed ({status}): {command:?}");
    }
    Ok(())
}
