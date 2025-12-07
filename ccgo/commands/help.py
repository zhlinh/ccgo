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
from copier import run_copy

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


class Help(CliCommand):
    def description(self) -> str:
        return """Show detailed help information for CCGO commands.

This command displays comprehensive usage information including:
- Command syntax and options
- Platform-specific requirements
- Examples for common workflows
- Environment variables

Use 'ccgo <command> --help' for command-specific help.
        """

    def cli(self) -> CliNameSpace:
        parser = argparse.ArgumentParser(
            prog="ccgo help",
            formatter_class=argparse.RawDescriptionHelpFormatter,
            description=self.description(),
        )
        module_name = os.path.splitext(os.path.basename(__file__))[0]
        input_argv = [x for x in sys.argv[1:] if x != module_name]
        args, unknown = parser.parse_known_args(input_argv)
        return args

    def exec(self, context: CliContext, args: CliNameSpace):
        # show help
        print("\n" + "=" * 70)
        print("CCGO - Cross-Platform C++ Build Tool")
        print("=" * 70)

        print("\n1. Create a new library project (in new directory)")
        print("\n  ccgo new <project-name> [options]")
        print("\n  Options:")
        print("    --template-url <url>    Custom template repository URL")
        print(
            "    --data <key>=<value>    Template variables (can be used multiple times)"
        )
        print("    --defaults              Use default values for all prompts")
        print("\n  Examples:")
        print("    ccgo new my-project")
        print("    ccgo new my-project --defaults")
        print(
            "    ccgo new my-project --template-url=https://github.com/user/template.git"
        )
        print("    ccgo new my-project --data cpy_project_version=2.0.0")

        print("\n2. Initialize library project in current directory")
        print("\n  ccgo init [options]")
        print("\n  Options:")
        print("    --template-url <url>    Custom template repository URL")
        print(
            "    --data <key>=<value>    Template variables (can be used multiple times)"
        )
        print("    --defaults              Use default values for all prompts")
        print("    --force                 Skip confirmation prompt")
        print("\n  Examples:")
        print("    ccgo init")
        print("    ccgo init --defaults --force")

        print("\n3. Check platform dependencies")
        print("\n  ccgo check [target] [options]")
        print("\n  Targets:")
        print("    all         Check all platforms (default)")
        print("    android     Check Android development environment")
        print("    ios         Check iOS development environment")
        print("    macos       Check macOS development environment")
        print("    windows     Check Windows development environment")
        print("    linux       Check Linux development environment")
        print("    ohos        Check OpenHarmony development environment")
        print("\n  Options:")
        print("    --verbose              Show detailed information")
        print("\n  Examples:")
        print("    ccgo check")
        print("    ccgo check android")
        print("    ccgo check ios --verbose")

        print("\n4. Clean build artifacts")
        print("\n  ccgo clean [target] [options]")
        print("\n  Targets:")
        print("    all         Clean all platforms (default)")
        print("    android     Clean Android build caches")
        print("    ios         Clean iOS build caches")
        print("    macos       Clean macOS build caches")
        print("    ohos        Clean OpenHarmony build caches")
        print("    kmp         Clean Kotlin Multiplatform build caches")
        print("    examples    Clean examples build caches")
        print("\n  Options:")
        print(
            "    --native-only          Clean only cmake_build/ (native CMake builds)"
        )
        print("    --dry-run              Show what would be cleaned without deleting")
        print("    -y, --yes              Skip confirmation prompts")
        print("\n  Examples:")
        print("    ccgo clean             # Clean all (with confirmation)")
        print("    ccgo clean android     # Clean only Android")
        print("    ccgo clean --dry-run   # Preview what will be deleted")
        print("    ccgo clean -y          # Clean all without confirmation")
        print("    ccgo clean --native-only  # Clean only cmake_build/")

        print("\n5. Run tests")
        print("\n  ccgo test [options]")
        print("\n  Options:")
        print("    --build-only           Only build tests without running")
        print("    --run-only             Only run tests (assumes already built)")
        print("    --filter <pattern>     GoogleTest filter (e.g., 'MyTest*')")
        print("    --ide-project          Generate IDE project for tests")
        print("    --gtest-args <args>    Additional GoogleTest arguments")
        print("\n  Examples:")
        print("    ccgo test              # Build and run all tests")
        print("    ccgo test --build-only # Only build tests")
        print('    ccgo test --filter "MyTest*"  # Run specific tests')
        print('    ccgo test --gtest-args "--gtest_repeat=3"')

        print("\n6. Run benchmarks")
        print("\n  ccgo bench [options]")
        print("\n  Options:")
        print("    --build-only              Only build benchmarks without running")
        print(
            "    --run-only                Only run benchmarks (assumes already built)"
        )
        print(
            "    --filter <pattern>        Google Benchmark filter (e.g., 'BM_Sort*')"
        )
        print("    --ide-project             Generate IDE project for benchmarks")
        print("    --benchmark-args <args>   Additional Google Benchmark arguments")
        print(
            "    --format <format>         Output format: console, json, csv (default: console)"
        )
        print("\n  Examples:")
        print("    ccgo bench                # Build and run all benchmarks")
        print("    ccgo bench --build-only   # Only build benchmarks")
        print('    ccgo bench --filter "BM_Sort*"  # Run specific benchmarks')
        print("    ccgo bench --format json  # Output in JSON format")

        print("\n7. Build documentation")
        print("\n  ccgo doc [options]")
        print("\n  Options:")
        print("    --open                 Open documentation in browser after building")
        print("    --serve                Start local web server to view documentation")
        print("    --port <port>          Port for web server (default: 8000)")
        print("    --clean                Clean build before generating")
        print("\n  Examples:")
        print("    ccgo doc               # Build documentation")
        print("    ccgo doc --open        # Build and open in browser")
        print("    ccgo doc --serve       # Build and serve on localhost:8000")
        print("    ccgo doc --serve --port 3000  # Serve on custom port")

        print("\n8. CI/CD multi-platform build")
        print("\n  ccgo build all [options]")
        print("\n  Options:")
        print(
            "    --release                      Build as release (default: beta/debug)"
        )
        print(
            "    --archive                      Create build archives after building"
        )
        print(
            "    --platforms <list>             Comma-separated platforms (e.g., android,ios,macos)"
        )
        print("    --skip-platforms <list>        Platforms to skip")
        print("    --arch <list>                  Architectures for Android/OHOS")
        print("    --archive-dir <dir>            Directory for build archives")
        print("    --use-env                      Use CI_BUILD_* environment variables")
        print("\n  Environment Variables (for CI/CD):")
        print("    CI_IS_RELEASE=1                Build as release")
        print("    CI_BUILD_ANDROID=1             Build Android")
        print("    CI_BUILD_IOS=1                 Build iOS")
        print("    CI_BUILD_MACOS=1               Build macOS")
        print("    CI_BUILD_WINDOWS=1             Build Windows")
        print("    CI_BUILD_LINUX=1               Build Linux")
        print("    CI_BUILD_OHOS=1                Build OpenHarmony")
        print("    CI_BUILD_KMP=1                 Build Kotlin Multiplatform")
        print("\n  Examples:")
        print("    ccgo build all                 # Build all platforms (debug)")
        print("    ccgo build all --release       # Build all platforms (release)")
        print("    ccgo build all --release --archive")
        print("    ccgo build all --platforms android,ios,macos")
        print("    ccgo build all --skip-platforms windows,linux")
        print(
            "    export CI_IS_RELEASE=1 && export CI_BUILD_ANDROID=1 && ccgo build all --use-env"
        )

        print("\n9. Build a library")
        print("\n  ccgo build <target> [options]")
        print("\n  Targets:")
        print("    android     Build for Android (supports --arch)")
        print("    ios         Build for iOS")
        print("    macos       Build for macOS")
        print("    windows     Build for Windows")
        print("    linux       Build for Linux")
        print("    ohos        Build for OpenHarmony (supports --arch)")
        print("    kmp         Build Kotlin Multiplatform library")
        print("    include     Build include headers")
        print("\n  Options:")
        print("    --arch <architectures>  Comma-separated list (android/ohos only)")
        print("                           android: armeabi-v7a,arm64-v8a,x86_64")
        print("                           ohos: armeabi-v7a,arm64-v8a,x86_64")
        print("    --ide-project          Generate IDE project files")
        print("\n  Examples:")
        print("    ccgo build android --arch armeabi-v7a,arm64-v8a")
        print("    ccgo build ios")
        print("    ccgo build kmp")

        print("\n10. Package SDK for distribution")
        print("\n  ccgo package [options]")
        print("\n  Options:")
        print("    --version <version>        SDK version (default: auto-detect)")
        print(
            "    --output <dir>             Output directory (default: ./target/package)"
        )
        print(
            "    --format <format>          Archive format: zip, tar.gz, both, none (default: zip)"
        )
        print("    --platforms <list>         Platforms to include (default: all)")
        print("    --include-docs             Include documentation")
        print("    --include-samples          Include sample code")
        print("    --include-kmp              Include KMP artifacts")
        print("    --skip-build               Only package existing artifacts")
        print("    --clean                    Clean output directory first")
        print("\n  Examples:")
        print("    ccgo package                           # Package all platforms")
        print("    ccgo package --version 1.0.0           # Specify version")
        print("    ccgo package --format both --include-docs")
        print("    ccgo package --platforms android,ios,macos")

        print("\n11. Publish a library")
        print("\n  ccgo publish <target>")
        print("\n  Targets:")
        print("    android     Publish to Maven repository")
        print("    ohos        Publish to OHPM repository")
        print("    kmp         Publish KMP library to Maven (local or remote)")
        print("\n  Examples:")
        print("    ccgo publish android")
        print("    ccgo publish kmp")

        print("\n12. Get help")
        print("\n  ccgo help")
        print("\n  ccgo <command> --help")

        print("\n" + "=" * 70)
        print("For more information, visit: https://github.com/zhlinh/ccgo")
        print("=" * 70)
        print("\n")
