//! Check command implementation - Pure Rust version
//!
//! Checks platform dependencies and configurations.
//! Validates development environment setup for each platform.

use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

use anyhow::Result;
use clap::Args;

/// Check platform dependencies
#[derive(Args, Debug)]
pub struct CheckCommand {
    /// Platform to check (all, android, ios, macos, windows, linux, ohos)
    #[arg(default_value = "all")]
    pub platform: String,

    /// Show detailed information
    #[arg(long)]
    pub verbose: bool,
}

impl CheckCommand {
    /// Execute the check command
    pub fn execute(self, _verbose: bool) -> Result<()> {
        println!("🔍 Checking {} platform configuration...\n", self.platform);

        let mut checker = PlatformChecker::new(self.verbose);

        match self.platform.as_str() {
            "all" => checker.check_all(),
            "android" => checker.check_android(),
            "ios" => checker.check_ios(),
            "macos" => checker.check_macos(),
            "windows" => checker.check_windows(),
            "linux" => checker.check_linux(),
            "ohos" => checker.check_ohos(),
            _ => {
                eprintln!("Unknown platform: {}", self.platform);
                eprintln!("Valid platforms: all, android, ios, macos, windows, linux, ohos");
                std::process::exit(1);
            }
        }

        checker.print_summary();

        // Exit with error if there are errors
        if !checker.errors.is_empty() {
            std::process::exit(1);
        }

        Ok(())
    }
}

/// Platform checker
struct PlatformChecker {
    verbose: bool,
    results: HashMap<String, HashMap<String, bool>>,
    warnings: Vec<String>,
    errors: Vec<String>,
    current_os: String,
}

impl PlatformChecker {
    fn new(verbose: bool) -> Self {
        let current_os = if cfg!(target_os = "macos") {
            "Darwin".to_string()
        } else if cfg!(target_os = "linux") {
            "Linux".to_string()
        } else if cfg!(target_os = "windows") {
            "Windows".to_string()
        } else {
            "Unknown".to_string()
        };

        Self {
            verbose,
            results: HashMap::new(),
            warnings: Vec::new(),
            errors: Vec::new(),
            current_os,
        }
    }

    /// Run a command and return output
    fn run_command(&self, cmd: &str) -> (bool, String, String) {
        let shell = if cfg!(target_os = "windows") {
            "cmd"
        } else {
            "sh"
        };
        let shell_arg = if cfg!(target_os = "windows") { "/C" } else { "-c" };

        match Command::new(shell)
            .arg(shell_arg)
            .arg(cmd)
            .output()
        {
            Ok(output) => {
                let success = output.status.success();
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                (success, stdout.trim().to_string(), stderr.trim().to_string())
            }
            Err(e) => (false, String::new(), e.to_string()),
        }
    }

    /// Check if a command exists in PATH
    fn check_command_exists(&mut self, command: &str, friendly_name: Option<&str>) -> (bool, Option<String>) {
        let name = friendly_name.unwrap_or(command);

        match which::which(command) {
            Ok(_) => {
                // Try to get version
                let version_cmd = format!("{} --version", command);
                let (success, version, _) = self.run_command(&version_cmd);

                let version_str = if !success {
                    // Try alternative version flag
                    let (success2, version2, _) = self.run_command(&format!("{} -version", command));
                    if success2 {
                        version2.lines().next().unwrap_or("").to_string()
                    } else {
                        String::new()
                    }
                } else {
                    version.lines().next().unwrap_or("").to_string()
                };

                if version_str.is_empty() {
                    self.print_ok(&format!("{}: Found", name));
                } else {
                    self.print_ok(&format!("{}: Found {}", name, version_str));
                }
                (true, Some(version_str))
            }
            Err(_) => {
                self.print_error(&format!("{}: Not found", name));
                (false, None)
            }
        }
    }

    /// Check if environment variable is set
    fn check_env_var(&mut self, var_name: &str, should_exist_as_dir: bool) -> (bool, Option<String>) {
        match env::var(var_name) {
            Ok(value) => {
                if should_exist_as_dir {
                    if Path::new(&value).is_dir() {
                        self.print_ok(&format!("{}: {}", var_name, value));
                        (true, Some(value))
                    } else {
                        self.print_error(&format!("{}: Set to '{}' but directory doesn't exist", var_name, value));
                        (false, Some(value))
                    }
                } else {
                    self.print_ok(&format!("{}: {}", var_name, value));
                    (true, Some(value))
                }
            }
            Err(_) => {
                self.print_error(&format!("{}: Not set", var_name));
                (false, None)
            }
        }
    }

    fn print_ok(&self, msg: &str) {
        println!("  ✅ {}", msg);
    }

    fn print_error(&mut self, msg: &str) {
        println!("  ❌ {}", msg);
        self.errors.push(msg.to_string());
    }

    fn print_warning(&mut self, msg: &str) {
        println!("  ⚠️  {}", msg);
        self.warnings.push(msg.to_string());
    }

    fn print_info(&self, msg: &str) {
        println!("  ℹ️  {}", msg);
    }

    fn print_section(&self, title: &str) {
        println!("\n{}", "=".repeat(60));
        println!("  {}", title);
        println!("{}", "=".repeat(60));
    }

    /// Check CMake installation
    fn check_cmake(&mut self) -> bool {
        self.print_section("CMake");
        let (exists, version) = self.check_command_exists("cmake", Some("CMake"));

        if exists && version.is_some() {
            let ver_str = version.unwrap();
            // Extract version number
            if let Some(caps) = regex::Regex::new(r"(\d+)\.(\d+)\.(\d+)").ok()
                .and_then(|re| re.captures(&ver_str)) {
                if let (Some(major), Some(minor), Some(_patch)) =
                    (caps.get(1), caps.get(2), caps.get(3)) {
                    let maj: u32 = major.as_str().parse().unwrap_or(0);
                    let min: u32 = minor.as_str().parse().unwrap_or(0);

                    if maj < 3 || (maj == 3 && min < 20) {
                        self.print_warning(&format!(
                            "CMake version {}.{} is old. Recommended: 3.20+",
                            maj, min
                        ));
                    }
                }
            }
        }

        exists
    }

    /// Check Gradle installation
    #[allow(dead_code)]
    fn check_gradle(&mut self) -> bool {
        // Check for global gradle
        if which::which("gradle").is_ok() {
            let (success, version, _) = self.run_command("gradle --version");
            if success {
                for line in version.lines() {
                    if line.contains("Gradle") {
                        self.print_ok(&format!("Gradle: {}", line.trim()));
                        return true;
                    }
                }
            }
            self.print_ok("Gradle: Found in PATH");
            return true;
        }

        // Check for gradlew
        let gradlew_files: Vec<&str> = if cfg!(target_os = "windows") {
            vec!["gradlew.bat"]
        } else {
            vec!["gradlew"]
        };

        let mut found_files = Vec::new();
        for file in &gradlew_files {
            if Path::new(file).is_file() {
                found_files.push(*file);
            }
        }

        if !found_files.is_empty() {
            self.print_ok(&format!("Gradle Wrapper: Found ({})", found_files.join(", ")));
            return true;
        }

        self.print_warning("Gradle: Not found globally");
        self.print_info("Gradle is typically used via Gradle Wrapper (./gradlew) in Android projects");
        self.print_info("To install globally: https://gradle.org/install/");

        true // Return true because Gradle Wrapper is preferred
    }

    /// Check Python installation
    fn check_python(&mut self) {
        self.print_section("Python");

        // Check python3
        let (mut exists, version) = self.check_command_exists("python3", Some("Python 3"));

        if !exists {
            // Try python
            let result = self.check_command_exists("python", Some("Python"));
            exists = result.0;
        }

        if exists && version.is_some() {
            let ver_str = version.unwrap();
            if !ver_str.contains("Python 3") {
                self.print_warning("Python 2 detected. Python 3.7+ is required");
            }
        }
    }

    /// Check Android development environment
    fn check_android(&mut self) {
        self.print_section("Android Platform");

        if self.verbose {
            self.print_info(&format!("Current OS: {}", self.current_os));
        }

        // Check Java
        let (java_exists, _) = self.check_command_exists("java", Some("Java"));
        let (_javac_exists, _) = self.check_command_exists("javac", Some("Java Compiler"));

        // Check JAVA_HOME
        let (java_home_exists, _java_home) = self.check_env_var("JAVA_HOME", true);

        // Check Android SDK
        let (android_home_exists, android_home) = self.check_env_var("ANDROID_HOME", true);

        // Validate Android SDK
        let sdk_well_formed = if android_home_exists && android_home.is_some() {
            self.validate_android_sdk(android_home.as_ref().unwrap())
        } else {
            false
        };

        // Check Android NDK
        let (mut ndk_home_exists, _ndk_home) = self.check_env_var("ANDROID_NDK_HOME", true);
        if !ndk_home_exists && android_home.is_some() {
            // Check if NDK is in default location
            let default_ndk = Path::new(android_home.as_ref().unwrap()).join("ndk");
            if default_ndk.is_dir() {
                if let Ok(entries) = fs::read_dir(&default_ndk) {
                    let ndk_versions: Vec<String> = entries
                        .filter_map(|e| e.ok())
                        .filter(|e| e.path().is_dir())
                        .filter_map(|e| e.file_name().to_str().map(String::from))
                        .collect();

                    if !ndk_versions.is_empty() {
                        self.print_warning(&format!(
                            "ANDROID_NDK_HOME not set, but NDK found at {}",
                            default_ndk.display()
                        ));
                        self.print_info(&format!(
                            "Available NDK versions: {}",
                            ndk_versions.join(", ")
                        ));
                        ndk_home_exists = true;
                    }
                }
            }
        }

        // Check CMake
        let cmake_exists = self.check_cmake();

        // Check cmdline-tools
        let cmdline_tools_exists = if android_home_exists && android_home.is_some() {
            self.check_android_cmdline_tools(android_home.as_ref().unwrap())
        } else {
            false
        };

        // Store results
        let mut checks = HashMap::new();
        checks.insert("java".to_string(), java_exists);
        checks.insert("java_home".to_string(), java_home_exists);
        checks.insert("android_sdk".to_string(), android_home_exists && sdk_well_formed);
        checks.insert("android_ndk".to_string(), ndk_home_exists);
        checks.insert("cmake".to_string(), cmake_exists);
        checks.insert("cmdline_tools".to_string(), cmdline_tools_exists);
        self.results.insert("android".to_string(), checks);

        // Recommendations
        if !java_home_exists {
            self.print_info("Set JAVA_HOME to your JDK installation path");
        }
        if !android_home_exists {
            self.print_info("Set ANDROID_HOME to your Android SDK path");
        }
        if !ndk_home_exists {
            self.print_info("Set ANDROID_NDK_HOME to your Android NDK path");
        }
    }

    /// Validate Android SDK structure
    fn validate_android_sdk(&mut self, sdk_path: &str) -> bool {
        let sdk = Path::new(sdk_path);

        // Check for adb in platform-tools
        let adb_name = if cfg!(target_os = "windows") { "adb.exe" } else { "adb" };
        let adb_path = sdk.join("platform-tools").join(adb_name);

        if !adb_path.is_file() {
            self.print_error(&format!("Android SDK platform-tools not found: {}", adb_path.display()));
            self.print_info("Run: sdkmanager 'platform-tools'");
            return false;
        }

        if self.verbose {
            self.print_info(&format!("Found adb at {}", adb_path.display()));
        }

        // Check for platforms directory
        let platforms_dir = sdk.join("platforms");
        if !platforms_dir.is_dir() {
            self.print_error(&format!("Android platforms directory not found: {}", platforms_dir.display()));
            self.print_info("Run: sdkmanager 'platforms;android-<version>'");
            return false;
        }

        // List available platforms
        let platforms: Vec<String> = fs::read_dir(&platforms_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .filter_map(|e| {
                        e.file_name().to_str()
                            .filter(|n| n.starts_with("android-"))
                            .map(String::from)
                    })
                    .collect()
            })
            .unwrap_or_default();

        if platforms.is_empty() {
            self.print_error("No Android platforms found");
            self.print_info("Run: sdkmanager 'platforms;android-34'");
            return false;
        }

        // Get latest platform version
        let mut platform_versions: Vec<(u32, String)> = platforms
            .iter()
            .filter_map(|p| {
                p.strip_prefix("android-")
                    .and_then(|v| v.parse::<u32>().ok())
                    .map(|ver| (ver, p.clone()))
            })
            .collect();

        if !platform_versions.is_empty() {
            platform_versions.sort_by_key(|&(ver, _)| ver);
            let (latest_ver, latest_platform) = platform_versions.last().unwrap();

            if self.verbose {
                self.print_info(&format!("Latest Android platform: {} (API {})", latest_platform, latest_ver));
            }

            if *latest_ver < 28 {
                self.print_warning(&format!("Android platform API {} is old. Recommended: API 28+", latest_ver));
            }
        }

        // Check for build-tools
        let build_tools_dir = sdk.join("build-tools");
        if !build_tools_dir.is_dir() {
            self.print_error(&format!("Android build-tools not found: {}", build_tools_dir.display()));
            self.print_info("Run: sdkmanager 'build-tools;<version>'");
            return false;
        }

        let build_tools: Vec<String> = fs::read_dir(&build_tools_dir)
            .ok()
            .map(|entries| {
                entries
                    .filter_map(|e| e.ok())
                    .filter(|e| e.path().is_dir())
                    .filter_map(|e| e.file_name().to_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();

        if build_tools.is_empty() {
            self.print_error("No Android build-tools versions found");
            self.print_info("Run: sdkmanager 'build-tools;34.0.0'");
            return false;
        }

        if self.verbose {
            self.print_info(&format!("Build-tools versions: {}", build_tools.join(", ")));
        }

        true
    }

    /// Check for Android SDK command-line tools
    fn check_android_cmdline_tools(&mut self, sdk_path: &str) -> bool {
        let sdk = Path::new(sdk_path);
        let sdkmanager_paths = vec![
            sdk.join("cmdline-tools/latest/bin/sdkmanager"),
            sdk.join("cmdline-tools/latest/bin/sdkmanager.bat"),
            sdk.join("tools/bin/sdkmanager"),
            sdk.join("tools/bin/sdkmanager.bat"),
        ];

        for path in sdkmanager_paths {
            if path.is_file() {
                self.print_ok("Android SDK Command-line Tools: Found");
                if self.verbose {
                    self.print_info(&format!("  sdkmanager at {}", path.display()));
                }
                return true;
            }
        }

        self.print_warning("Android SDK Command-line Tools: Not found");
        self.print_info("Install from Android Studio SDK Manager or download from:");
        self.print_info("  https://developer.android.com/studio#command-tools");
        false
    }

    /// Check iOS development environment
    fn check_ios(&mut self) {
        self.print_section("iOS Platform");

        if self.current_os != "Darwin" {
            self.print_warning("iOS development requires macOS");
            return;
        }

        // Check Xcode
        let (xcode_exists, xcode_path, _) = self.run_command("xcode-select -p");
        if xcode_exists {
            self.print_ok(&format!("Xcode: Installed at {}", xcode_path));

            // Get Xcode version
            let (success, version, _) = self.run_command("xcodebuild -version");
            if success {
                let first_line = version.lines().next().unwrap_or("");
                self.print_info(&format!("Version: {}", first_line));
            }
        } else {
            self.print_error("Xcode: Not installed");
            self.print_info("Install from App Store or run: xcode-select --install");
        }

        // Check xcodebuild
        let (xcodebuild_exists, _) = self.check_command_exists("xcodebuild", Some("xcodebuild"));

        // Check cocoapods
        let (pod_exists, _) = self.check_command_exists("pod", Some("CocoaPods"));
        if !pod_exists {
            self.print_info("Install CocoaPods: sudo gem install cocoapods");
        }

        // Check CMake
        let cmake_exists = self.check_cmake();

        // Check iOS SDK
        if xcodebuild_exists {
            let (success, sdks, _) = self.run_command("xcodebuild -showsdks");
            if success && sdks.contains("iOS") {
                self.print_ok("iOS SDK: Available");
                if self.verbose {
                    for line in sdks.lines() {
                        if line.contains("iOS") {
                            self.print_info(&format!("  {}", line.trim()));
                        }
                    }
                }
            }
        }

        let mut checks = HashMap::new();
        checks.insert("xcode".to_string(), xcode_exists);
        checks.insert("xcodebuild".to_string(), xcodebuild_exists);
        checks.insert("cocoapods".to_string(), pod_exists);
        checks.insert("cmake".to_string(), cmake_exists);
        self.results.insert("ios".to_string(), checks);
    }

    /// Check macOS development environment
    fn check_macos(&mut self) {
        self.print_section("macOS Platform");

        if self.current_os != "Darwin" {
            self.print_warning("macOS builds require macOS");
            return;
        }

        // Check Xcode
        let (xcode_exists, xcode_path, _) = self.run_command("xcode-select -p");
        if xcode_exists {
            self.print_ok(&format!("Xcode: Installed at {}", xcode_path));
        } else {
            self.print_error("Xcode: Not installed");
        }

        // Check Clang
        let (clang_exists, _) = self.check_command_exists("clang", Some("Clang"));

        // Check CMake
        let cmake_exists = self.check_cmake();

        let mut checks = HashMap::new();
        checks.insert("xcode".to_string(), xcode_exists);
        checks.insert("clang".to_string(), clang_exists);
        checks.insert("cmake".to_string(), cmake_exists);
        self.results.insert("macos".to_string(), checks);
    }

    /// Check Windows development environment
    fn check_windows(&mut self) {
        self.print_section("Windows Platform");

        if self.current_os != "Windows" {
            self.print_warning("Windows builds require Windows OS (or cross-compilation setup)");
        }

        if self.current_os == "Windows" {
            // Check for Visual Studio
            let vs_paths = vec![
                r"C:\Program Files\Microsoft Visual Studio",
                r"C:\Program Files (x86)\Microsoft Visual Studio",
            ];

            let mut vs_found = false;
            for vs_path in vs_paths {
                if Path::new(vs_path).is_dir() {
                    vs_found = true;
                    self.print_ok(&format!("Visual Studio: Found at {}", vs_path));

                    if self.verbose {
                        for year in &["2022", "2019", "2017"] {
                            let year_path = Path::new(vs_path).join(year);
                            if year_path.is_dir() {
                                self.print_info(&format!("  Visual Studio {} installed", year));
                            }
                        }
                    }
                    break;
                }
            }

            if !vs_found {
                self.print_error("Visual Studio: Not found");
                self.print_info("Install Visual Studio 2019 or later with C++ development tools");
            }

            // Check cl.exe
            let (cl_exists, _) = self.check_command_exists("cl", Some("MSVC Compiler (cl.exe)"));
            if !cl_exists {
                self.print_warning("cl.exe not in PATH. You may need to run from Visual Studio Developer Command Prompt");
            }
        } else {
            self.print_info("Running on non-Windows OS. Cross-compilation tools needed for Windows builds");
        }

        // Check CMake
        let cmake_exists = self.check_cmake();

        let mut checks = HashMap::new();
        checks.insert("cmake".to_string(), cmake_exists);
        self.results.insert("windows".to_string(), checks);
    }

    /// Check Linux development environment
    fn check_linux(&mut self) {
        self.print_section("Linux Platform");

        if self.current_os != "Linux" {
            self.print_warning("Linux builds require Linux OS (or cross-compilation setup)");
        }

        // Check GCC
        let (gcc_exists, _) = self.check_command_exists("gcc", Some("GCC"));

        // Check G++
        let (gxx_exists, _) = self.check_command_exists("g++", Some("G++"));

        // Check Clang (optional)
        let (_clang_exists, _) = self.check_command_exists("clang", Some("Clang (optional)"));

        // Check make
        let (make_exists, _) = self.check_command_exists("make", Some("Make"));

        // Check CMake
        let cmake_exists = self.check_cmake();

        if self.current_os == "Linux" {
            self.print_info("Checking common development libraries...");
            self.print_info("Ensure development libraries are installed (build-essential, etc.)");
        }

        let mut checks = HashMap::new();
        checks.insert("gcc".to_string(), gcc_exists);
        checks.insert("gxx".to_string(), gxx_exists);
        checks.insert("make".to_string(), make_exists);
        checks.insert("cmake".to_string(), cmake_exists);
        self.results.insert("linux".to_string(), checks);
    }

    /// Check OpenHarmony development environment
    fn check_ohos(&mut self) {
        self.print_section("OpenHarmony (OHOS) Platform");

        // Check OHOS SDK
        let (mut ohos_sdk_exists, mut ohos_sdk) = self.check_env_var("OHOS_SDK_HOME", true);
        if !ohos_sdk_exists {
            let result = self.check_env_var("HOS_SDK_HOME", true);
            ohos_sdk_exists = result.0;
            ohos_sdk = result.1;
        }

        if ohos_sdk.is_some() {
            let native_sdk = Path::new(ohos_sdk.as_ref().unwrap()).join("native");
            if native_sdk.is_dir() {
                self.print_ok(&format!("OHOS Native SDK: Found at {}", native_sdk.display()));
            } else {
                self.print_warning(&format!("OHOS Native SDK not found in {}", ohos_sdk.unwrap()));
            }
        }

        // Check Node.js
        let (node_exists, _) = self.check_command_exists("node", Some("Node.js"));
        let (npm_exists, _) = self.check_command_exists("npm", Some("npm"));

        // Check hvigorw
        let (hvigorw_exists, _) = self.check_command_exists("hvigorw", Some("hvigorw"));
        if !hvigorw_exists {
            self.print_info("hvigorw is usually installed per-project. Check project's node_modules");
        }

        // Check ohpm
        let (ohpm_exists, _) = self.check_command_exists("ohpm", Some("ohpm (OpenHarmony Package Manager)"));
        if !ohpm_exists {
            self.print_info("Install ohpm from OpenHarmony SDK");
        }

        // Check CMake
        let cmake_exists = self.check_cmake();

        let mut checks = HashMap::new();
        checks.insert("ohos_sdk".to_string(), ohos_sdk_exists);
        checks.insert("nodejs".to_string(), node_exists);
        checks.insert("npm".to_string(), npm_exists);
        checks.insert("hvigorw".to_string(), hvigorw_exists);
        checks.insert("ohpm".to_string(), ohpm_exists);
        checks.insert("cmake".to_string(), cmake_exists);
        self.results.insert("ohos".to_string(), checks);

        if !ohos_sdk_exists {
            self.print_info("Set OHOS_SDK_HOME or HOS_SDK_HOME to your OpenHarmony SDK path");
        }
    }

    /// Check all platforms
    fn check_all(&mut self) {
        self.print_info(&format!("Checking all platform configurations on {}", self.current_os));

        // Always check common tools
        self.check_python();

        // Check platform-specific based on current OS
        match self.current_os.as_str() {
            "Darwin" => {
                self.check_macos();
                self.check_ios();
                self.check_android();
                self.check_ohos();
            }
            "Linux" => {
                self.check_linux();
                self.check_android();
                self.check_ohos();
            }
            "Windows" => {
                self.check_windows();
                self.check_android();
                self.check_ohos();
            }
            _ => {
                self.print_warning(&format!("Unknown OS: {}", self.current_os));
                self.check_android();
            }
        }
    }

    /// Print summary of check results
    fn print_summary(&self) {
        self.print_section("Summary");

        let total_checks = self.results.len();
        if total_checks == 0 {
            self.print_info("No checks performed");
            return;
        }

        // Count platforms with all dependencies met
        let mut platforms_ok = 0;
        let mut platforms_partial = 0;
        let mut platforms_failed = 0;

        for (platform, checks) in &self.results {
            let all_ok = checks.values().all(|&v| v);
            let any_ok = checks.values().any(|&v| v);

            let status = if all_ok {
                platforms_ok += 1;
                "✅ READY"
            } else if any_ok {
                platforms_partial += 1;
                "⚠️  PARTIAL"
            } else {
                platforms_failed += 1;
                "❌ NOT READY"
            };

            println!("  {}: {}", platform.to_uppercase(), status);

            if self.verbose {
                for (check, result) in checks {
                    let symbol = if *result { "✅" } else { "❌" };
                    println!("    {} {}", symbol, check);
                }
            }
        }

        println!("\n{}", "=".repeat(60));
        println!("  Total Platforms Checked: {}", total_checks);
        println!("  ✅ Ready: {}", platforms_ok);
        println!("  ⚠️  Partial: {}", platforms_partial);
        println!("  ❌ Not Ready: {}", platforms_failed);

        if !self.errors.is_empty() {
            println!("\n  Total Errors: {}", self.errors.len());
        }
        if !self.warnings.is_empty() {
            println!("  Total Warnings: {}", self.warnings.len());
        }

        println!("{}\n", "=".repeat(60));

        if platforms_ok == total_checks {
            println!("🎉 All checked platforms are ready for development!");
        } else if platforms_partial > 0 || platforms_failed > 0 {
            println!("💡 Some platforms need additional setup. See details above.");
        }
    }
}
