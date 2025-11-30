#
# Copyright 2024 zhlinh and ccgo Project Authors. All rights reserved.
# Use of this source code is governed by a MIT-style
# license that can be found at
#
# https://opensource.org/license/MIT
#
# The above copyright notice and this permission
# notice shall be included in all copies or
# substantial portions of the Software.

import os
import sys
import argparse
import subprocess
import shutil
import platform
from pathlib import Path

# setup path
# >>>>>>>>>>>>>>
SCRIPT_PATH = os.path.split(os.path.realpath(__file__))[0]
PROJECT_ROOT_PATH = os.path.dirname(SCRIPT_PATH)
sys.path.append(SCRIPT_PATH)
sys.path.append(PROJECT_ROOT_PATH)
PACKAGE_NAME = os.path.basename(SCRIPT_PATH)
# <<<<<<<<<<<<<
# import this project modules
try:
    from ccgo.utils.context.namespace import CliNameSpace
    from ccgo.utils.context.context import CliContext
    from ccgo.utils.context.command import CliCommand
    from ccgo.utils.cmd.cmd_util import exec_command
except ImportError:
    from utils.context.namespace import CliNameSpace
    from utils.context.context import CliContext
    from utils.context.command import CliCommand
    from utils.cmd.cmd_util import exec_command


class Check(CliCommand):
    def description(self) -> str:
        return """
        This is a subcommand to check platform dependencies and configurations.

        Examples:
            ccgo check android      # Check Android development environment
            ccgo check ios          # Check iOS development environment
            ccgo check all          # Check all platforms
        """

    def get_target_list(self) -> list:
        return ["all", "android", "ios", "macos", "windows", "linux", "ohos"]

    def cli(self) -> CliNameSpace:
        parser = argparse.ArgumentParser(
            prog="ccgo check",
            formatter_class=argparse.RawDescriptionHelpFormatter,
            description=self.description(),
        )
        parser.add_argument(
            "target",
            metavar=f"{self.get_target_list()}",
            type=str,
            choices=self.get_target_list(),
            nargs="?",
            default="all",
            help="Platform to check (default: all)",
        )
        parser.add_argument(
            "--verbose",
            action="store_true",
            help="Show detailed information",
        )
        module_name = os.path.splitext(os.path.basename(__file__))[0]
        input_argv = [x for x in sys.argv[1:] if x != module_name]
        args, unknown = parser.parse_known_args(input_argv)
        return args

    def exec(self, context: CliContext, args: CliNameSpace):
        print(f"🔍 Checking {args.target} platform configuration...\n")

        checker = PlatformChecker(verbose=args.verbose)

        if args.target == "all":
            checker.check_all()
        elif args.target == "android":
            checker.check_android()
        elif args.target == "ios":
            checker.check_ios()
        elif args.target == "macos":
            checker.check_macos()
        elif args.target == "windows":
            checker.check_windows()
        elif args.target == "linux":
            checker.check_linux()
        elif args.target == "ohos":
            checker.check_ohos()

        checker.print_summary()


class PlatformChecker:
    def __init__(self, verbose=False):
        self.verbose = verbose
        self.results = {}
        self.warnings = []
        self.errors = []
        self.current_os = platform.system()

    def run_command(self, cmd, shell=True):
        """Run a command and return output"""
        try:
            result = subprocess.run(
                cmd,
                shell=shell,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True,
                timeout=10,
            )
            return result.returncode == 0, result.stdout.strip(), result.stderr.strip()
        except subprocess.TimeoutExpired:
            return False, "", "Command timed out"
        except Exception as e:
            return False, "", str(e)

    def check_command_exists(self, command, friendly_name=None):
        """Check if a command exists in PATH"""
        name = friendly_name or command
        exists = shutil.which(command) is not None

        if exists:
            # Try to get version
            success, version, _ = self.run_command(f"{command} --version")
            if not success:
                success, version, _ = self.run_command(f"{command} -version")

            version_str = version.split("\n")[0] if version else ""
            self.print_ok(f"{name}: Found {version_str if version_str else ''}")
            return True, version_str
        else:
            self.print_error(f"{name}: Not found")
            return False, None

    def check_env_var(self, var_name, should_exist_as_dir=False):
        """Check if environment variable is set"""
        value = os.environ.get(var_name)

        if value:
            if should_exist_as_dir:
                if os.path.isdir(value):
                    self.print_ok(f"{var_name}: {value}")
                    return True, value
                else:
                    self.print_error(
                        f"{var_name}: Set to '{value}' but directory doesn't exist"
                    )
                    return False, value
            else:
                self.print_ok(f"{var_name}: {value}")
                return True, value
        else:
            self.print_error(f"{var_name}: Not set")
            return False, None

    def print_ok(self, msg):
        """Print success message"""
        print(f"  ✅ {msg}")

    def print_error(self, msg):
        """Print error message"""
        print(f"  ❌ {msg}")
        self.errors.append(msg)

    def print_warning(self, msg):
        """Print warning message"""
        print(f"  ⚠️  {msg}")
        self.warnings.append(msg)

    def print_info(self, msg):
        """Print info message"""
        print(f"  ℹ️  {msg}")

    def print_section(self, title):
        """Print section header"""
        print(f"\n{'='*60}")
        print(f"  {title}")
        print(f"{'='*60}")

    def check_cmake(self):
        """Check CMake installation"""
        self.print_section("CMake")
        exists, version = self.check_command_exists("cmake", "CMake")

        if exists and version:
            # Extract version number
            import re

            match = re.search(r"(\d+)\.(\d+)\.(\d+)", version)
            if match:
                major, minor, patch = map(int, match.groups())
                if major < 3 or (major == 3 and minor < 20):
                    self.print_warning(
                        f"CMake version {major}.{minor}.{patch} is old. Recommended: 3.20+"
                    )

        return exists

    def check_gradle(self):
        """Check Gradle installation (global or wrapper)"""
        # First check for global gradle
        gradle_exists = shutil.which("gradle") is not None

        if gradle_exists:
            # Try to get version
            success, version, _ = self.run_command("gradle --version")
            if success:
                # Extract version from output
                for line in version.split("\n"):
                    if "Gradle" in line:
                        self.print_ok(f"Gradle: {line.strip()}")
                        return True
            else:
                self.print_ok("Gradle: Found in PATH")
                return True

        # Check if we're in a project directory with gradlew
        gradlew_files = []
        if os.path.isfile("./gradlew"):
            gradlew_files.append("./gradlew")
        if os.path.isfile("./gradlew.bat"):
            gradlew_files.append("./gradlew.bat")

        if gradlew_files:
            self.print_ok(f"Gradle Wrapper: Found ({', '.join(gradlew_files)})")
            # Try to get version from wrapper
            if self.current_os != "Windows" and "./gradlew" in gradlew_files:
                success, version, _ = self.run_command("./gradlew --version")
            elif self.current_os == "Windows" and "./gradlew.bat" in gradlew_files:
                success, version, _ = self.run_command("gradlew.bat --version")
            else:
                success = False

            if success and version:
                for line in version.split("\n"):
                    if "Gradle" in line:
                        self.print_info(f"  {line.strip()}")
            return True

        # Neither found
        self.print_warning("Gradle: Not found globally")
        self.print_info(
            "Gradle is typically used via Gradle Wrapper (./gradlew) in Android projects"
        )
        self.print_info(
            "To install globally: https://gradle.org/install/"
        )
        # Return True because Gradle Wrapper is the preferred method
        # and will be available in generated projects
        return True

    def check_python(self):
        """Check Python installation"""
        self.print_section("Python")

        # Check python3
        exists, version = self.check_command_exists("python3", "Python 3")

        if not exists:
            # Try python
            exists, version = self.check_command_exists("python", "Python")

        if exists:
            # Check if it's Python 3.x
            success, version_output, _ = self.run_command("python3 --version")
            if not success:
                success, version_output, _ = self.run_command("python --version")

            if "Python 3" not in version_output:
                self.print_warning("Python 2 detected. Python 3.7+ is required")

        return exists

    def check_android(self):
        """Check Android development environment"""
        self.print_section("Android Platform")

        if self.verbose:
            self.print_info(f"Current OS: {self.current_os}")

        # Check Java (similar to Flutter's _checkJavaVersion)
        java_exists, java_version = self.check_command_exists("java", "Java")
        javac_exists, _ = self.check_command_exists("javac", "Java Compiler")

        # Check JAVA_HOME
        java_home_exists, java_home = self.check_env_var(
            "JAVA_HOME", should_exist_as_dir=True
        )

        # Check Android SDK
        android_home_exists, android_home = self.check_env_var(
            "ANDROID_HOME", should_exist_as_dir=True
        )

        # Detailed Android SDK validation (inspired by Flutter's validateSdkWellFormed)
        sdk_well_formed = True
        if android_home and android_home_exists:
            sdk_well_formed = self.validate_android_sdk(android_home)

        # Check Android NDK
        ndk_home_exists, ndk_home = self.check_env_var(
            "ANDROID_NDK_HOME", should_exist_as_dir=True
        )
        if not ndk_home_exists and android_home:
            # Check if NDK is in default location
            default_ndk = os.path.join(android_home, "ndk")
            if os.path.isdir(default_ndk):
                ndk_versions = [d for d in os.listdir(default_ndk)
                               if os.path.isdir(os.path.join(default_ndk, d))]
                if ndk_versions:
                    self.print_warning(
                        f"ANDROID_NDK_HOME not set, but NDK found at {default_ndk}"
                    )
                    self.print_info(
                        f"Available NDK versions: {', '.join(sorted(ndk_versions))}"
                    )

        # Check CMake
        cmake_exists = self.check_cmake()

        # Check cmdline-tools (important for Flutter-like workflows)
        cmdline_tools_exists = False
        if android_home and android_home_exists:
            cmdline_tools_exists = self.check_android_cmdline_tools(android_home)

        # Store results
        self.results["android"] = {
            "java": java_exists,
            "java_home": java_home_exists,
            "android_sdk": android_home_exists and sdk_well_formed,
            "android_ndk": ndk_home_exists,
            "cmake": cmake_exists,
            "cmdline_tools": cmdline_tools_exists,
        }

        # Recommendations
        if not java_home_exists:
            self.print_info("Set JAVA_HOME to your JDK installation path")
        if not android_home_exists:
            self.print_info(
                "Set ANDROID_HOME to your Android SDK path"
            )
        if not ndk_home_exists:
            self.print_info("Set ANDROID_NDK_HOME to your Android NDK path")

    def validate_android_sdk(self, sdk_path):
        """Validate Android SDK structure (inspired by Flutter's validateSdkWellFormed)"""
        # Check for adb in platform-tools
        adb_name = "adb.exe" if self.current_os == "Windows" else "adb"
        adb_path = os.path.join(sdk_path, "platform-tools", adb_name)

        if not os.path.isfile(adb_path):
            self.print_error(f"Android SDK platform-tools not found: {adb_path}")
            self.print_info("Run: sdkmanager 'platform-tools'")
            return False
        else:
            if self.verbose:
                self.print_info(f"Found adb at {adb_path}")

        # Check for platforms directory
        platforms_dir = os.path.join(sdk_path, "platforms")
        if not os.path.isdir(platforms_dir):
            self.print_error(f"Android platforms directory not found: {platforms_dir}")
            self.print_info("Run: sdkmanager 'platforms;android-<version>'")
            return False

        # List available platforms
        platforms = [d for d in os.listdir(platforms_dir)
                    if os.path.isdir(os.path.join(platforms_dir, d)) and d.startswith("android-")]

        if not platforms:
            self.print_error("No Android platforms found")
            self.print_info("Run: sdkmanager 'platforms;android-34'")
            return False

        # Get latest platform version
        platform_versions = []
        for p in platforms:
            try:
                ver = int(p.replace("android-", ""))
                platform_versions.append((ver, p))
            except ValueError:
                continue

        if platform_versions:
            latest_ver, latest_platform = max(platform_versions)
            if self.verbose:
                self.print_info(f"Latest Android platform: {latest_platform} (API {latest_ver})")

            # Warn if platform is too old (< 28, like Flutter)
            if latest_ver < 28:
                self.print_warning(
                    f"Android platform API {latest_ver} is old. Recommended: API 28+"
                )

        # Check for build-tools
        build_tools_dir = os.path.join(sdk_path, "build-tools")
        if not os.path.isdir(build_tools_dir):
            self.print_error(f"Android build-tools not found: {build_tools_dir}")
            self.print_info("Run: sdkmanager 'build-tools;<version>'")
            return False

        build_tools = [d for d in os.listdir(build_tools_dir)
                      if os.path.isdir(os.path.join(build_tools_dir, d))]

        if not build_tools:
            self.print_error("No Android build-tools versions found")
            self.print_info("Run: sdkmanager 'build-tools;34.0.0'")
            return False

        if self.verbose:
            self.print_info(f"Build-tools versions: {', '.join(sorted(build_tools))}")

        return True

    def check_android_cmdline_tools(self, sdk_path):
        """Check for Android SDK command-line tools"""
        # Check for sdkmanager
        sdkmanager_paths = [
            os.path.join(sdk_path, "cmdline-tools", "latest", "bin", "sdkmanager"),
            os.path.join(sdk_path, "cmdline-tools", "latest", "bin", "sdkmanager.bat"),
            os.path.join(sdk_path, "tools", "bin", "sdkmanager"),
            os.path.join(sdk_path, "tools", "bin", "sdkmanager.bat"),
        ]

        sdkmanager_found = None
        for path in sdkmanager_paths:
            if os.path.isfile(path):
                sdkmanager_found = path
                break

        if sdkmanager_found:
            self.print_ok(f"Android SDK Command-line Tools: Found")
            if self.verbose:
                self.print_info(f"  sdkmanager at {sdkmanager_found}")
            return True
        else:
            self.print_warning("Android SDK Command-line Tools: Not found")
            self.print_info(
                "Install from Android Studio SDK Manager or download from:"
            )
            self.print_info(
                "  https://developer.android.com/studio#command-tools"
            )
            return False

    def check_ios(self):
        """Check iOS development environment"""
        self.print_section("iOS Platform")

        if self.current_os != "Darwin":
            self.print_warning("iOS development requires macOS")
            return

        # Check Xcode
        xcode_exists, xcode_path, _ = self.run_command("xcode-select -p")
        if xcode_exists:
            self.print_ok(f"Xcode: Installed at {xcode_path}")

            # Get Xcode version
            success, version, _ = self.run_command("xcodebuild -version")
            if success:
                self.print_info(f"Version: {version.split(chr(10))[0]}")
        else:
            self.print_error("Xcode: Not installed")
            self.print_info("Install from App Store or run: xcode-select --install")

        # Check xcodebuild
        xcodebuild_exists, _ = self.check_command_exists("xcodebuild", "xcodebuild")

        # Check cocoapods
        pod_exists, pod_version = self.check_command_exists("pod", "CocoaPods")
        if not pod_exists:
            self.print_info("Install CocoaPods: sudo gem install cocoapods")

        # Check CMake
        cmake_exists = self.check_cmake()

        # Check iOS SDK
        if xcodebuild_exists:
            success, sdks, _ = self.run_command("xcodebuild -showsdks")
            if success and "iOS" in sdks:
                self.print_ok("iOS SDK: Available")
                if self.verbose:
                    for line in sdks.split("\n"):
                        if "iOS" in line:
                            self.print_info(f"  {line.strip()}")

        self.results["ios"] = {
            "xcode": xcode_exists,
            "xcodebuild": xcodebuild_exists,
            "cocoapods": pod_exists,
            "cmake": cmake_exists,
        }

    def check_macos(self):
        """Check macOS development environment"""
        self.print_section("macOS Platform")

        if self.current_os != "Darwin":
            self.print_warning("macOS builds require macOS")
            return

        # Check Xcode (same as iOS)
        xcode_exists, xcode_path, _ = self.run_command("xcode-select -p")
        if xcode_exists:
            self.print_ok(f"Xcode: Installed at {xcode_path}")
        else:
            self.print_error("Xcode: Not installed")

        # Check Clang
        clang_exists, clang_version = self.check_command_exists("clang", "Clang")

        # Check CMake
        cmake_exists = self.check_cmake()

        self.results["macos"] = {
            "xcode": xcode_exists,
            "clang": clang_exists,
            "cmake": cmake_exists,
        }

    def check_windows(self):
        """Check Windows development environment"""
        self.print_section("Windows Platform")

        if self.current_os != "Windows":
            self.print_warning(
                "Windows builds require Windows OS (or cross-compilation setup)"
            )

        # Check MSVC
        if self.current_os == "Windows":
            # Check for Visual Studio
            vs_paths = [
                "C:\\Program Files\\Microsoft Visual Studio",
                "C:\\Program Files (x86)\\Microsoft Visual Studio",
            ]

            vs_found = False
            for vs_path in vs_paths:
                if os.path.isdir(vs_path):
                    vs_found = True
                    self.print_ok(f"Visual Studio: Found at {vs_path}")

                    # List installed versions
                    if self.verbose:
                        for year in ["2022", "2019", "2017"]:
                            year_path = os.path.join(vs_path, year)
                            if os.path.isdir(year_path):
                                self.print_info(f"  Visual Studio {year} installed")
                    break

            if not vs_found:
                self.print_error("Visual Studio: Not found")
                self.print_info(
                    "Install Visual Studio 2019 or later with C++ development tools"
                )

            # Check cl.exe (MSVC compiler)
            cl_exists, _ = self.check_command_exists("cl", "MSVC Compiler (cl.exe)")
            if not cl_exists:
                self.print_warning(
                    "cl.exe not in PATH. You may need to run from Visual Studio Developer Command Prompt"
                )
        else:
            self.print_info(
                "Running on non-Windows OS. Cross-compilation tools needed for Windows builds"
            )

        # Check CMake
        cmake_exists = self.check_cmake()

        self.results["windows"] = {
            "cmake": cmake_exists,
        }

    def check_linux(self):
        """Check Linux development environment"""
        self.print_section("Linux Platform")

        if self.current_os != "Linux":
            self.print_warning(
                "Linux builds require Linux OS (or cross-compilation setup)"
            )

        # Check GCC
        gcc_exists, gcc_version = self.check_command_exists("gcc", "GCC")

        # Check G++
        gxx_exists, gxx_version = self.check_command_exists("g++", "G++")

        # Check Clang (optional)
        clang_exists, _ = self.check_command_exists("clang", "Clang (optional)")

        # Check make
        make_exists, _ = self.check_command_exists("make", "Make")

        # Check CMake
        cmake_exists = self.check_cmake()

        # Check required libraries (if on Linux)
        if self.current_os == "Linux":
            self.print_info("Checking common development libraries...")

            # This is platform-specific, could check for libstdc++, etc.
            # For now, just note it
            self.print_info(
                "Ensure development libraries are installed (build-essential, etc.)"
            )

        self.results["linux"] = {
            "gcc": gcc_exists,
            "gxx": gxx_exists,
            "make": make_exists,
            "cmake": cmake_exists,
        }

    def check_ohos(self):
        """Check OpenHarmony development environment"""
        self.print_section("OpenHarmony (OHOS) Platform")

        # Check OHOS SDK
        ohos_sdk_exists, ohos_sdk = self.check_env_var(
            "OHOS_SDK_HOME", should_exist_as_dir=True
        )
        if not ohos_sdk_exists:
            ohos_sdk_exists, ohos_sdk = self.check_env_var(
                "HOS_SDK_HOME", should_exist_as_dir=True
            )

        if ohos_sdk:
            # Check for native SDK
            native_sdk = os.path.join(ohos_sdk, "native")
            if os.path.isdir(native_sdk):
                self.print_ok(f"OHOS Native SDK: Found at {native_sdk}")
            else:
                self.print_warning(f"OHOS Native SDK not found in {ohos_sdk}")

        # Check Node.js (required for hvigorw)
        node_exists, node_version = self.check_command_exists("node", "Node.js")
        npm_exists, _ = self.check_command_exists("npm", "npm")

        # Check hvigorw
        hvigorw_exists, _ = self.check_command_exists("hvigorw", "hvigorw")
        if not hvigorw_exists:
            self.print_info(
                "hvigorw is usually installed per-project. Check project's node_modules"
            )

        # Check ohpm
        ohpm_exists, _ = self.check_command_exists(
            "ohpm", "ohpm (OpenHarmony Package Manager)"
        )
        if not ohpm_exists:
            self.print_info("Install ohpm from OpenHarmony SDK")

        # Check CMake
        cmake_exists = self.check_cmake()

        self.results["ohos"] = {
            "ohos_sdk": ohos_sdk_exists,
            "nodejs": node_exists,
            "npm": npm_exists,
            "hvigorw": hvigorw_exists,
            "ohpm": ohpm_exists,
            "cmake": cmake_exists,
        }

        if not ohos_sdk_exists:
            self.print_info(
                "Set OHOS_SDK_HOME or HOS_SDK_HOME to your OpenHarmony SDK path"
            )

    def check_all(self):
        """Check all platforms"""
        self.print_info(f"Checking all platform configurations on {self.current_os}")

        # Always check common tools
        self.check_python()

        # Check platform-specific based on current OS
        if self.current_os == "Darwin":
            self.check_macos()
            self.check_ios()
            self.check_android()
            self.check_ohos()
        elif self.current_os == "Linux":
            self.check_linux()
            self.check_android()
            self.check_ohos()
        elif self.current_os == "Windows":
            self.check_windows()
            self.check_android()
            self.check_ohos()
        else:
            self.print_warning(f"Unknown OS: {self.current_os}")
            # Still check common platforms
            self.check_android()

    def print_summary(self):
        """Print summary of check results"""
        self.print_section("Summary")

        total_checks = len(self.results)
        if total_checks == 0:
            self.print_info("No checks performed")
            return

        # Count platforms with all dependencies met
        platforms_ok = 0
        platforms_partial = 0
        platforms_failed = 0

        for platform, checks in self.results.items():
            all_ok = all(checks.values())
            any_ok = any(checks.values())

            if all_ok:
                platforms_ok += 1
                status = "✅ READY"
            elif any_ok:
                platforms_partial += 1
                status = "⚠️  PARTIAL"
            else:
                platforms_failed += 1
                status = "❌ NOT READY"

            print(f"  {platform.upper()}: {status}")

            if self.verbose:
                for check, result in checks.items():
                    symbol = "✅" if result else "❌"
                    print(f"    {symbol} {check}")

        print(f"\n{'='*60}")
        print(f"  Total Platforms Checked: {total_checks}")
        print(f"  ✅ Ready: {platforms_ok}")
        print(f"  ⚠️  Partial: {platforms_partial}")
        print(f"  ❌ Not Ready: {platforms_failed}")

        if self.errors:
            print(f"\n  Total Errors: {len(self.errors)}")
        if self.warnings:
            print(f"  Total Warnings: {len(self.warnings)}")

        print(f"{'='*60}\n")

        if platforms_ok == total_checks:
            print("🎉 All checked platforms are ready for development!")
        elif platforms_partial > 0 or platforms_failed > 0:
            print("💡 Some platforms need additional setup. See details above.")
            sys.exit(1)
