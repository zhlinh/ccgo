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
import shutil
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


class Clean(CliCommand):
    def description(self) -> str:
        return """
        This is a subcommand to clean build artifacts and caches.

        Cleans the following directories:
        - bin/                    # Output binaries
        - cmake_build/            # CMake build directory
        - android/build/          # Android Gradle build cache
        - android/.gradle/        # Android Gradle cache
        - ohos/build/             # OHOS build cache
        - ohos/.hvigor/           # OHOS hvigor cache
        - kmp/build/              # KMP build cache
        - examples/*/build/       # Example projects build cache

        Examples:
            ccgo clean              # Clean all build artifacts (with confirmation)
            ccgo clean android      # Clean only Android platform
            ccgo clean --dry-run    # Preview what will be cleaned
            ccgo clean -y           # Clean all without confirmation
            ccgo clean --native-only  # Clean only cmake_build directory
        """

    def get_target_list(self) -> list:
        return ["all", "android", "ios", "macos", "ohos", "kmp", "examples"]

    def cli(self) -> CliNameSpace:
        parser = argparse.ArgumentParser(
            prog="ccgo clean",
            formatter_class=argparse.RawDescriptionHelpFormatter,
            description=self.description(),
        )
        parser.add_argument(
            "target",
            nargs='?',
            default="all",
            type=str,
            choices=self.get_target_list(),
            help="Platform to clean (default: all)",
        )
        parser.add_argument(
            "--native-only",
            action="store_true",
            help="Clean only cmake_build directory (CMake native builds)",
        )
        parser.add_argument(
            "--dry-run",
            action="store_true",
            help="Show what would be cleaned without actually deleting",
        )
        parser.add_argument(
            "-y", "--yes",
            action="store_true",
            help="Skip confirmation prompts",
        )
        module_name = os.path.splitext(os.path.basename(__file__))[0]
        input_argv = [x for x in sys.argv[1:] if x != module_name]
        args, unknown = parser.parse_known_args(input_argv)
        return args

    def exec(self, context: CliContext, args: CliNameSpace):
        print("Cleaning build artifacts and caches...\n")

        # Get current working directory (project directory)
        try:
            project_dir = os.getcwd()
        except (OSError, FileNotFoundError) as e:
            # If current directory was deleted, try to use PWD environment variable
            project_dir = os.environ.get('PWD')
            if not project_dir or not os.path.exists(project_dir):
                print(f"ERROR: Current working directory no longer exists: {e}")
                print("Please navigate to your project directory and try again.")
                sys.exit(1)
            # Try to change to the saved directory
            try:
                os.chdir(project_dir)
            except (OSError, FileNotFoundError):
                print(f"ERROR: Cannot access project directory: {project_dir}")
                sys.exit(1)

        # Check if CCGO.toml exists to verify we're in a CCGO project
        config_path = None
        for subdir in os.listdir(project_dir):
            subdir_path = os.path.join(project_dir, subdir)
            if not os.path.isdir(subdir_path):
                continue
            potential_config = os.path.join(subdir_path, "CCGO.toml")
            if os.path.isfile(potential_config):
                config_path = potential_config
                project_subdir = subdir_path
                break

        # If not found in subdirectory, check current directory
        if not config_path:
            if os.path.isfile(os.path.join(project_dir, "CCGO.toml")):
                config_path = os.path.join(project_dir, "CCGO.toml")
                project_subdir = project_dir
            else:
                print("⚠️  Warning: CCGO.toml not found. May not be in a CCGO project directory.")
                print("   Continuing anyway...\n")
                project_subdir = project_dir

        cleaner = ProjectCleaner(project_subdir, dry_run=args.dry_run, skip_confirm=args.yes)

        # Determine what to clean based on arguments
        if args.native_only:
            # Only clean cmake_build directory
            cleaner.clean_cmake()
        elif args.target == "all":
            # Clean all platforms
            cleaner.clean_all()
        else:
            # Clean specific platform
            cleaner.clean_platform(args.target)

        cleaner.print_summary()


class ProjectCleaner:
    def __init__(self, project_dir, dry_run=False, skip_confirm=False):
        self.project_dir = project_dir
        self.dry_run = dry_run
        self.skip_confirm = skip_confirm
        self.cleaned_dirs = []
        self.cleaned_size = 0
        self.failed_dirs = []

    def get_dir_size(self, path):
        """Get total size of directory in bytes"""
        total_size = 0
        try:
            for dirpath, dirnames, filenames in os.walk(path):
                for filename in filenames:
                    filepath = os.path.join(dirpath, filename)
                    try:
                        total_size += os.path.getsize(filepath)
                    except (OSError, FileNotFoundError):
                        pass
        except (OSError, PermissionError):
            pass
        return total_size

    def format_size(self, size_bytes):
        """Format bytes to human-readable size"""
        for unit in ['B', 'KB', 'MB', 'GB']:
            if size_bytes < 1024.0:
                return f"{size_bytes:.2f} {unit}"
            size_bytes /= 1024.0
        return f"{size_bytes:.2f} TB"

    def remove_directory(self, dir_path, dir_name=None):
        """Remove a directory and track the result"""
        if not os.path.exists(dir_path):
            return False

        if not os.path.isdir(dir_path):
            return False

        size = self.get_dir_size(dir_path)
        display_name = dir_name or os.path.basename(dir_path)

        if self.dry_run:
            print(f"  [DRY RUN] Would remove: {display_name} ({self.format_size(size)})")
            return True

        try:
            shutil.rmtree(dir_path)
            self.cleaned_dirs.append(display_name)
            self.cleaned_size += size
            print(f"  ✅ Removed: {display_name} ({self.format_size(size)})")
            return True
        except Exception as e:
            self.failed_dirs.append((display_name, str(e)))
            print(f"  ❌ Failed to remove {display_name}: {e}")
            return False

    def confirm_clean(self, message):
        """Ask user for confirmation"""
        if self.skip_confirm:
            return True

        response = input(f"{message} (y/N): ").strip().lower()
        return response in ['y', 'yes']

    def clean_bin(self):
        """Clean bin directory"""
        print("\n" + "="*60)
        print("  Cleaning bin/ directory")
        print("="*60)

        bin_dir = os.path.join(self.project_dir, "bin")

        if not os.path.exists(bin_dir):
            print("  ℹ️  bin/ directory does not exist")
            return

        if not self.dry_run and not self.skip_confirm:
            if not self.confirm_clean("  Remove bin/ directory?"):
                print("  ⏭️  Skipped")
                return

        self.remove_directory(bin_dir, "bin/")

    def clean_cmake(self):
        """Clean cmake_build directory"""
        print("\n" + "="*60)
        print("  Cleaning cmake_build/ directory")
        print("="*60)

        cmake_dir = os.path.join(self.project_dir, "cmake_build")

        if not os.path.exists(cmake_dir):
            print("  ℹ️  cmake_build/ directory does not exist")
            return

        if not self.dry_run and not self.skip_confirm:
            if not self.confirm_clean("  Remove cmake_build/ directory?"):
                print("  ⏭️  Skipped")
                return

        self.remove_directory(cmake_dir, "cmake_build/")

    def clean_android(self):
        """Clean Android build caches"""
        print("\n" + "="*60)
        print("  Cleaning Android build caches")
        print("="*60)

        android_dir = os.path.join(self.project_dir, "android")

        if not os.path.exists(android_dir):
            print("  ℹ️  android/ directory does not exist")
            return

        # Clean android/build/
        build_dir = os.path.join(android_dir, "build")
        if os.path.exists(build_dir):
            self.remove_directory(build_dir, "android/build/")

        # Clean android/.gradle/
        gradle_dir = os.path.join(android_dir, ".gradle")
        if os.path.exists(gradle_dir):
            self.remove_directory(gradle_dir, "android/.gradle/")

        # Clean android/*/build/ for all subprojects
        for item in os.listdir(android_dir):
            item_path = os.path.join(android_dir, item)
            if os.path.isdir(item_path):
                subproject_build = os.path.join(item_path, "build")
                if os.path.exists(subproject_build):
                    self.remove_directory(subproject_build, f"android/{item}/build/")

    def clean_ios(self):
        """Clean iOS/macOS build caches"""
        print("\n" + "="*60)
        print("  Cleaning iOS/macOS build caches")
        print("="*60)

        # Clean Pods cache
        pods_dir = os.path.join(self.project_dir, "ios", "Pods")
        if os.path.exists(pods_dir):
            self.remove_directory(pods_dir, "ios/Pods/")

        # Clean DerivedData
        derived_data = os.path.join(self.project_dir, "ios", "DerivedData")
        if os.path.exists(derived_data):
            self.remove_directory(derived_data, "ios/DerivedData/")

        # Clean build directories
        ios_build = os.path.join(self.project_dir, "ios", "build")
        if os.path.exists(ios_build):
            self.remove_directory(ios_build, "ios/build/")

    def clean_macos(self):
        """Clean macOS build caches (same as iOS)"""
        print("\n" + "="*60)
        print("  Cleaning macOS build caches")
        print("="*60)

        # Clean macOS build if separate from iOS
        macos_build = os.path.join(self.project_dir, "macos", "build")
        if os.path.exists(macos_build):
            self.remove_directory(macos_build, "macos/build/")

    def clean_ohos(self):
        """Clean OHOS build caches"""
        print("\n" + "="*60)
        print("  Cleaning OHOS build caches")
        print("="*60)

        ohos_dir = os.path.join(self.project_dir, "ohos")

        if not os.path.exists(ohos_dir):
            print("  ℹ️  ohos/ directory does not exist")
            return

        # Clean ohos/build/
        build_dir = os.path.join(ohos_dir, "build")
        if os.path.exists(build_dir):
            self.remove_directory(build_dir, "ohos/build/")

        # Clean ohos/.hvigor/
        hvigor_dir = os.path.join(ohos_dir, ".hvigor")
        if os.path.exists(hvigor_dir):
            self.remove_directory(hvigor_dir, "ohos/.hvigor/")

        # Clean ohos/*/build/ for all subprojects
        for item in os.listdir(ohos_dir):
            item_path = os.path.join(ohos_dir, item)
            if os.path.isdir(item_path) and item not in ['.hvigor', 'build']:
                subproject_build = os.path.join(item_path, "build")
                if os.path.exists(subproject_build):
                    self.remove_directory(subproject_build, f"ohos/{item}/build/")

    def clean_kmp(self):
        """Clean KMP build caches"""
        print("\n" + "="*60)
        print("  Cleaning KMP build caches")
        print("="*60)

        kmp_dir = os.path.join(self.project_dir, "kmp")

        if not os.path.exists(kmp_dir):
            print("  ℹ️  kmp/ directory does not exist")
            return

        # Clean kmp/build/
        build_dir = os.path.join(kmp_dir, "build")
        if os.path.exists(build_dir):
            self.remove_directory(build_dir, "kmp/build/")

        # Clean kmp/.gradle/
        gradle_dir = os.path.join(kmp_dir, ".gradle")
        if os.path.exists(gradle_dir):
            self.remove_directory(gradle_dir, "kmp/.gradle/")

    def clean_examples(self):
        """Clean examples build caches"""
        print("\n" + "="*60)
        print("  Cleaning examples build caches")
        print("="*60)

        examples_dir = os.path.join(self.project_dir, "examples")

        if not os.path.exists(examples_dir):
            print("  ℹ️  examples/ directory does not exist")
            return

        # Find all build directories in examples
        for root, dirs, files in os.walk(examples_dir):
            for dir_name in dirs:
                if dir_name in ['build', '.gradle', '.hvigor']:
                    full_path = os.path.join(root, dir_name)
                    rel_path = os.path.relpath(full_path, self.project_dir)
                    self.remove_directory(full_path, rel_path)

    def clean_platform(self, platform):
        """Clean specific platform caches"""
        if platform == "android":
            self.clean_android()
        elif platform == "ios":
            self.clean_ios()
        elif platform == "macos":
            self.clean_macos()
        elif platform == "ohos":
            self.clean_ohos()
        elif platform == "kmp":
            self.clean_kmp()
        elif platform == "examples":
            self.clean_examples()

    def clean_all(self):
        """Clean all build artifacts and caches"""
        print("\n" + "="*60)
        print("  Cleaning ALL build artifacts and caches")
        print("="*60)

        if not self.dry_run and not self.skip_confirm:
            if not self.confirm_clean("\n⚠️  This will remove ALL build artifacts. Continue?"):
                print("  ⏭️  Aborted")
                return

        # Clean in order
        self.clean_bin()
        self.clean_cmake()
        self.clean_android()
        self.clean_ios()
        self.clean_macos()
        self.clean_ohos()
        self.clean_kmp()
        self.clean_examples()

        # Also clean root .gradle if exists
        root_gradle = os.path.join(self.project_dir, ".gradle")
        if os.path.exists(root_gradle):
            print("\n" + "="*60)
            print("  Cleaning root .gradle/")
            print("="*60)
            self.remove_directory(root_gradle, ".gradle/")

    def print_summary(self):
        """Print summary of cleaning operation"""
        print("\n" + "="*60)
        print("  Cleaning Summary")
        print("="*60)

        if self.dry_run:
            print("  [DRY RUN MODE - No files were actually deleted]")

        if self.cleaned_dirs:
            print(f"  ✅ Successfully cleaned {len(self.cleaned_dirs)} directories:")
            for dir_name in self.cleaned_dirs:
                print(f"     - {dir_name}")
            print(f"\n  💾 Total space freed: {self.format_size(self.cleaned_size)}")
        else:
            print("  ℹ️  No directories were cleaned")

        if self.failed_dirs:
            print(f"\n  ❌ Failed to clean {len(self.failed_dirs)} directories:")
            for dir_name, error in self.failed_dirs:
                print(f"     - {dir_name}: {error}")

        print("="*60 + "\n")

        if self.dry_run:
            print("💡 Tip: Run without --dry-run to actually delete the files")
