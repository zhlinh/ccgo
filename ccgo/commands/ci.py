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
from utils.context.namespace import CliNameSpace
from utils.context.context import CliContext
from utils.context.command import CliCommand


class Ci(CliCommand):
    def description(self) -> str:
        return """Build all platforms for CI/CD pipeline.

This command builds multiple platforms in a single run, suitable for
continuous integration and release workflows.

EXAMPLES:
    # Build all platforms in debug mode
    ccgo ci

    # Build all platforms in release mode
    ccgo ci --release

    # Build specific platforms
    ccgo ci --platforms android,ios,macos

    # Skip certain platforms
    ccgo ci --skip-platforms windows,linux

    # Specify Android/OHOS architectures
    ccgo ci --arch armeabi-v7a,arm64-v8a

    # Use environment variables (backward compatibility)
    export CI_IS_RELEASE=1
    export CI_BUILD_ANDROID=1
    export CI_BUILD_IOS=1
    ccgo ci --use-env

ENVIRONMENT VARIABLES:
    CI_IS_RELEASE          Set to 1 for release builds (default: beta)
    CI_BUILD_ANDROID       Set to 1 to build Android
    CI_BUILD_IOS           Set to 1 to build iOS
    CI_BUILD_MACOS         Set to 1 to build macOS
    CI_BUILD_WINDOWS       Set to 1 to build Windows
    CI_BUILD_LINUX         Set to 1 to build Linux
    CI_BUILD_OHOS          Set to 1 to build OpenHarmony
    CI_BUILD_KMP           Set to 1 to build Kotlin Multiplatform

SUPPORTED PLATFORMS:
    android, ios, macos, windows, linux, ohos, kmp
        """

    def get_all_platforms(self) -> list:
        """Get list of all supported platforms"""
        return ["android", "ios", "macos", "windows", "linux", "ohos", "kmp"]

    def cli(self) -> CliNameSpace:
        parser = argparse.ArgumentParser(
            prog="ccgo ci",
            formatter_class=argparse.RawDescriptionHelpFormatter,
            description=self.description(),
        )

        # Build mode
        parser.add_argument(
            "--release",
            action="store_true",
            help="Build as release (default: beta/debug)",
        )

        # Platform selection
        parser.add_argument(
            "--platforms",
            type=str,
            help="Comma-separated list of platforms to build (e.g., android,ios,macos). Default: all platforms",
        )

        parser.add_argument(
            "--skip-platforms",
            type=str,
            help="Comma-separated list of platforms to skip",
        )

        # Architecture options
        parser.add_argument(
            "--arch",
            type=str,
            default="armeabi-v7a,arm64-v8a,x86_64",
            help="Architectures for Android/OHOS (default: armeabi-v7a,arm64-v8a,x86_64)",
        )

        # Output options
        parser.add_argument(
            "--archive-dir",
            type=str,
            help="Directory for build archives (default: ./build/archives)",
        )

        parser.add_argument(
            "--no-archive",
            action="store_true",
            help="Skip creating build archives",
        )

        # CI environment variable support
        parser.add_argument(
            "--use-env",
            action="store_true",
            help="Use CI_BUILD_* environment variables to determine which platforms to build",
        )

        module_name = os.path.splitext(os.path.basename(__file__))[0]
        input_argv = [x for x in sys.argv[1:] if x != module_name]
        args, unknown = parser.parse_known_args(input_argv)
        return args

    def get_platforms_from_env(self) -> list:
        """Get platforms to build from environment variables"""
        platforms = []
        env_map = {
            "CI_BUILD_ANDROID": "android",
            "CI_BUILD_IOS": "ios",
            "CI_BUILD_MACOS": "macos",
            "CI_BUILD_WINDOWS": "windows",
            "CI_BUILD_LINUX": "linux",
            "CI_BUILD_OHOS": "ohos",
            "CI_BUILD_KMP": "kmp",
        }

        for env_var, platform in env_map.items():
            if os.environ.get(env_var) == "1":
                platforms.append(platform)

        return platforms

    def get_platforms_to_build(self, args: CliNameSpace) -> list:
        """Determine which platforms to build based on arguments and environment"""
        all_platforms = self.get_all_platforms()

        # Check if using environment variables
        if args.use_env or (not args.platforms and os.environ.get("CI_BUILD_ANDROID")):
            platforms = self.get_platforms_from_env()
            if not platforms:
                print(
                    "WARNING: --use-env specified but no CI_BUILD_* environment variables set"
                )
                print("Building all platforms by default")
                platforms = all_platforms
        elif args.platforms:
            # Parse comma-separated platform list
            platforms = [p.strip() for p in args.platforms.split(",")]
            # Validate platforms
            invalid = [p for p in platforms if p not in all_platforms]
            if invalid:
                print(f"ERROR: Invalid platforms: {', '.join(invalid)}")
                print(f"Valid platforms: {', '.join(all_platforms)}")
                sys.exit(1)
        else:
            # Build all platforms by default
            platforms = all_platforms

        # Remove skipped platforms
        if args.skip_platforms:
            skip = [p.strip() for p in args.skip_platforms.split(",")]
            platforms = [p for p in platforms if p not in skip]

        return platforms

    def exec(self, context: CliContext, args: CliNameSpace):
        print("=" * 80)
        print("CCGO CI Build")
        print("=" * 80)

        # Determine build mode
        is_release = args.release or os.environ.get("CI_IS_RELEASE") == "1"
        build_mode = "RELEASE" if is_release else "BETA/DEBUG"

        print(f"\nBuild Mode: {build_mode}")

        # Get platforms to build
        platforms = self.get_platforms_to_build(args)

        if not platforms:
            print("\nERROR: No platforms selected for building")
            sys.exit(1)

        print(f"Platforms: {', '.join(platforms)}")
        print(f"Architecture: {args.arch}")

        if args.archive_dir:
            print(f"Archive Directory: {args.archive_dir}")

        print("\n" + "=" * 80 + "\n")

        # Get current working directory (project directory)
        try:
            project_dir = os.getcwd()
        except (OSError, FileNotFoundError) as e:
            project_dir = os.environ.get("PWD")
            if not project_dir or not os.path.exists(project_dir):
                print(f"ERROR: Current working directory no longer exists: {e}")
                sys.exit(1)

        # Build each platform
        build_results = {}
        failed_platforms = []

        for platform in platforms:
            print(f"\n{'='*80}")
            print(f"Building {platform.upper()}")
            print(f"{'='*80}\n")

            # Construct build command
            cmd_parts = ["ccgo", "build", platform]

            # Add architecture for Android/OHOS
            if platform in ["android", "ohos"]:
                cmd_parts.extend(["--arch", args.arch])

            cmd = " ".join(cmd_parts)
            print(f"Command: {cmd}\n")

            # Execute build
            exit_code = os.system(cmd)

            if exit_code != 0:
                print(f"\n❌ {platform} build FAILED with exit code {exit_code}")
                failed_platforms.append(platform)
                build_results[platform] = "FAILED"
            else:
                print(f"\n✅ {platform} build SUCCEEDED")
                build_results[platform] = "SUCCESS"

        # Print summary
        print("\n" + "=" * 80)
        print("BUILD SUMMARY")
        print("=" * 80 + "\n")

        print(f"Build Mode: {build_mode}")
        print(f"\nResults:")
        for platform, result in build_results.items():
            status_icon = "✅" if result == "SUCCESS" else "❌"
            print(f"  {status_icon} {platform:12s}: {result}")

        # Archive builds if requested
        if not args.no_archive:
            archive_dir = args.archive_dir or os.path.join(
                project_dir, "build", "archives"
            )
            print(f"\n📦 Build artifacts can be archived to: {archive_dir}")
            print("   (Archive creation not yet implemented)")

        # Exit with error if any builds failed
        if failed_platforms:
            print(f"\n❌ CI Build FAILED")
            print(f"   Failed platforms: {', '.join(failed_platforms)}")
            sys.exit(1)
        else:
            print(f"\n✅ CI Build SUCCEEDED")
            print(f"   All {len(platforms)} platforms built successfully")
            sys.exit(0)
