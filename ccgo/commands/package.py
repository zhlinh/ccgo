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
import subprocess
from datetime import datetime

# Try to import tomli for Python < 3.11, tomllib for Python >= 3.11
try:
    import tomllib
except ModuleNotFoundError:
    try:
        import tomli as tomllib
    except ModuleNotFoundError:
        tomllib = None

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
except ImportError:
    from utils.context.namespace import CliNameSpace
    from utils.context.context import CliContext
    from utils.context.command import CliCommand


class Package(CliCommand):
    def description(self) -> str:
        return """Package all build artifacts into a distributable SDK.

This command collects build outputs from all platforms and creates
a unified SDK package containing:
- Include headers
- Platform libraries (Android, iOS, macOS, Windows, Linux, OHOS)
- KMP artifacts (if built)
- Documentation (optional)
- Sample code (optional)

The package is organized in a standard structure suitable for
distribution to SDK users.

EXAMPLES:
    # Package all platforms with default settings
    ccgo package

    # Package with specific version
    ccgo package --version 1.0.0

    # Package with documentation and samples
    ccgo package --include-docs --include-samples

    # Package specific platforms only
    ccgo package --platforms android,ios,macos

    # Create both zip and tar.gz archives
    ccgo package --format both

    # Package existing artifacts without rebuilding
    ccgo package --skip-build

    # Clean output directory before packaging
    ccgo package --clean --output ./release

OPTIONS:
    --version <version>        SDK version (default: auto-detect from git)
    --output <dir>             Output directory (default: ./target/package)
    --format <format>          Archive format: zip, tar.gz, both, none
    --platforms <list>         Comma-separated platforms to include
    --include-docs             Include documentation in package
    --include-samples          Include sample code in package
    --skip-build               Only package existing artifacts
    --clean                    Clean output directory first

OUTPUT STRUCTURE:
    <PROJECT>_SDK-<version>/
    ├── include/                       Header files
    ├── lib/                           Platform libraries
    │   ├── android/                   Android libraries
    │   │   ├── static/               Static libraries (.a)
    │   │   │   ├── arm64-v8a/
    │   │   │   ├── armeabi-v7a/
    │   │   │   └── x86_64/
    │   │   └── shared/               Shared libraries (.so)
    │   │       ├── arm64-v8a/
    │   │       ├── armeabi-v7a/
    │   │       └── x86_64/
    │   ├── ios/                       iOS libraries
    │   │   ├── static/               .xcframework, .framework, .a
    │   │   └── shared/               .xcframework, .framework, .dylib
    │   ├── macos/                     macOS libraries
    │   │   ├── static/               .framework, .xcframework, .a
    │   │   └── shared/               .framework, .xcframework, .dylib
    │   ├── tvos/                      tvOS libraries
    │   │   ├── static/               .xcframework, .framework, .a
    │   │   └── shared/               .xcframework, .framework, .dylib
    │   ├── watchos/                   watchOS libraries
    │   │   ├── static/               .xcframework, .framework, .a
    │   │   └── shared/               .xcframework, .framework, .dylib
    │   ├── windows/                   Windows libraries
    │   │   ├── static/x64/           .lib files
    │   │   └── shared/x64/           .dll, .lib files
    │   ├── linux/                     Linux libraries
    │   │   ├── static/               .a files
    │   │   └── shared/               .so files
    │   ├── ohos/                      OpenHarmony libraries
    │   │   ├── static/               Static libraries (.a)
    │   │   │   ├── arm64-v8a/
    │   │   │   ├── armeabi-v7a/
    │   │   │   └── x86_64/
    │   │   └── shared/               Shared libraries (.so)
    │   │       ├── arm64-v8a/
    │   │       ├── armeabi-v7a/
    │   │       └── x86_64/
    │   └── kmp/                       (if built) KMP artifacts
    │       ├── android/               .aar files
    │       ├── desktop/               .jar files
    │       └── native/                Native klib files
    │           ├── iosArm64/
    │           ├── macosX64/
    │           └── linuxX64/
    ├── docs/                          (optional) Documentation
    └── README.md                      Package information
        """

    def cli(self) -> CliNameSpace:
        parser = argparse.ArgumentParser(
            prog="ccgo package",
            formatter_class=argparse.RawDescriptionHelpFormatter,
            description=self.description(),
        )

        # Version and output
        parser.add_argument(
            "--version",
            type=str,
            help="SDK version (default: auto-detect from git or CCGO.toml)",
        )

        parser.add_argument(
            "--output",
            type=str,
            default="./target/package",
            help="Output directory for packaged SDK (default: ./target/package)",
        )

        # Archive format
        parser.add_argument(
            "--format",
            type=str,
            choices=["zip", "tar.gz", "both", "none"],
            default="zip",
            help="Archive format: zip, tar.gz, both, or none (default: zip)",
        )

        # Platform selection
        parser.add_argument(
            "--platforms",
            type=str,
            help="Comma-separated platforms to include (default: all built platforms)",
        )

        # Optional components
        parser.add_argument(
            "--include-docs",
            action="store_true",
            help="Include documentation in package",
        )

        parser.add_argument(
            "--include-samples",
            action="store_true",
            help="Include sample code in package",
        )

        # Build option
        parser.add_argument(
            "--skip-build",
            action="store_true",
            help="Skip building, only package existing artifacts",
        )

        parser.add_argument(
            "--clean",
            action="store_true",
            help="Clean output directory before packaging",
        )

        module_name = os.path.splitext(os.path.basename(__file__))[0]
        input_argv = [x for x in sys.argv[1:] if x != module_name]
        args, unknown = parser.parse_known_args(input_argv)
        return args

    def get_version(self, project_dir: str, args: CliNameSpace) -> str:
        """Get SDK version from args, git, or CCGO.toml"""
        if args.version:
            return args.version

        # Try git tag
        try:
            result = subprocess.run(
                ["git", "describe", "--tags", "--always"],
                cwd=project_dir,
                capture_output=True,
                text=True,
            )
            if result.returncode == 0:
                version = result.stdout.strip()
                if version:
                    return version
        except:
            pass

        # Try CCGO.toml
        config_file = None
        try:
            for subdir in os.listdir(project_dir):
                subdir_path = os.path.join(project_dir, subdir)
                if os.path.isdir(subdir_path):
                    potential_config = os.path.join(subdir_path, "CCGO.toml")
                    if os.path.isfile(potential_config):
                        config_file = potential_config
                        break
        except (OSError, PermissionError):
            pass

        if not config_file and os.path.isfile(os.path.join(project_dir, "CCGO.toml")):
            config_file = os.path.join(project_dir, "CCGO.toml")

        if config_file:
            if not tomllib:
                print("   ⚠️  Warning: tomllib not available. Install 'tomli' for Python < 3.11")
                print("   ⚠️  Using default version instead")
            else:
                try:
                    with open(config_file, 'rb') as f:
                        config = tomllib.load(f)
                        if 'project' in config and 'version' in config['project']:
                            return config['project']['version']
                except Exception as e:
                    print(f"   ⚠️  Error reading CCGO.toml: {e}")

        # Default version
        return datetime.now().strftime("%Y%m%d")

    def get_project_name(self, project_dir: str) -> str:
        """Get project name from CCGO.toml"""

        # Try to find CCGO.toml
        config_file = None
        try:
            for subdir in os.listdir(project_dir):
                subdir_path = os.path.join(project_dir, subdir)
                if os.path.isdir(subdir_path):
                    potential_config = os.path.join(subdir_path, "CCGO.toml")
                    if os.path.isfile(potential_config):
                        config_file = potential_config
                        break
        except (OSError, PermissionError):
            pass

        if not config_file and os.path.isfile(os.path.join(project_dir, "CCGO.toml")):
            config_file = os.path.join(project_dir, "CCGO.toml")

        if config_file:
            if not tomllib:
                print("   ⚠️  Warning: tomllib not available. Install 'tomli' for Python < 3.11")
                print("   ⚠️  Using default project name 'SDK'")
            else:
                try:
                    with open(config_file, 'rb') as f:
                        config = tomllib.load(f)
                        if 'project' in config and 'name' in config['project']:
                            return config['project']['name']
                except Exception as e:
                    print(f"   ⚠️  Error reading CCGO.toml: {e}")
        else:
            print("   ⚠️  Warning: CCGO.toml not found in project directory")
            print("   ⚠️  Using default project name 'SDK'")

        return "SDK"

    def collect_platform_artifacts(self, project_dir: str, platform: str, output_dir: str, project_name: str):
        """Collect ZIP artifacts for a specific platform from target/<platform> directory

        Build scripts output ZIP archives in target/<platform>/.
        This function copies ZIP files directly to the package output.
        """
        print(f"\n📦 Collecting {platform} artifacts...")

        # Check if target/<platform> directory exists
        target_platform_dir = os.path.join(project_dir, "target", platform)

        if not os.path.exists(target_platform_dir):
            print(f"   ⚠️  No artifacts found (target/{platform} does not exist)")
            print(f"   💡 Build {platform} first with: ccgo build {platform}")
            return False, []

        collected = False
        collected_files = []

        try:
            # Find ZIP archives in target/<platform>/ (recursively)
            archive_files = []
            for root, dirs, files in os.walk(target_platform_dir):
                for f in files:
                    if f.endswith(('.zip', '.aar', '.har')) and not f.startswith('ARCHIVE'):
                        full_path = os.path.join(root, f)
                        archive_files.append(full_path)

            if not archive_files:
                print(f"   ⚠️  No build archives found in target/{platform}")
                print(f"   💡 Expected .zip, .aar, or .har files")
                return False, []

            # Copy all archive files to output directory
            for archive_file in archive_files:
                filename = os.path.basename(archive_file)
                dest_path = os.path.join(output_dir, filename)
                shutil.copy2(archive_file, dest_path)
                size_mb = os.path.getsize(dest_path) / (1024 * 1024)
                print(f"   ✓ {filename} ({size_mb:.2f} MB)")
                collected = True
                collected_files.append(filename)

        except Exception as e:
            print(f"   ⚠️  Error collecting {platform} artifacts: {e}")
            import traceback
            traceback.print_exc()

        return collected, collected_files

    def collect_kmp_artifacts(self, project_dir: str, output_dir: str, project_name: str):
        """Collect KMP ZIP artifacts from target/kmp directory"""
        print(f"\n📦 Collecting KMP artifacts...")

        collected = False
        collected_files = []

        # Look for KMP artifacts in target/kmp directory
        target_kmp_dir = os.path.join(project_dir, "target", "kmp")

        if not os.path.exists(target_kmp_dir):
            print(f"   ⚠️  No KMP artifacts found (target/kmp does not exist)")
            print(f"   💡 Build KMP first with: ccgo build kmp")
            return False, []

        try:
            # Find ZIP archives in target/kmp/
            for f in os.listdir(target_kmp_dir):
                if f.endswith('.zip'):
                    src_path = os.path.join(target_kmp_dir, f)
                    dest_path = os.path.join(output_dir, f)
                    shutil.copy2(src_path, dest_path)
                    size_mb = os.path.getsize(dest_path) / (1024 * 1024)
                    print(f"   ✓ {f} ({size_mb:.2f} MB)")
                    collected = True
                    collected_files.append(f)

        except (OSError, PermissionError) as e:
            print(f"   ⚠️  Error collecting KMP artifacts: {e}")
            return False, []

        if not collected:
            print(f"   ⚠️  No KMP ZIP archives found in target/kmp")
            print(f"   💡 Build KMP first with: ccgo build kmp")

        return collected, collected_files

    def exec(self, context: CliContext, args: CliNameSpace):
        print("="*80)
        print("CCGO Package - Collect Build Artifacts")
        print("="*80)

        # Get current working directory
        try:
            project_dir = os.getcwd()
        except (OSError, FileNotFoundError) as e:
            project_dir = os.environ.get('PWD')
            if not project_dir or not os.path.exists(project_dir):
                print(f"ERROR: Current working directory no longer exists: {e}")
                sys.exit(1)

        # Get project info
        project_name = self.get_project_name(project_dir)
        version = self.get_version(project_dir, args)

        # Convert output path to absolute path
        if not os.path.isabs(args.output):
            output_path = os.path.join(project_dir, args.output)
        else:
            output_path = args.output

        print(f"\nProject: {project_name}")
        print(f"Version: {version}")
        print(f"Output: {output_path}")

        # Clean if requested
        if args.clean and os.path.exists(output_path):
            print(f"\n🧹 Cleaning output directory...")
            shutil.rmtree(output_path)

        # Create output directory
        os.makedirs(output_path, exist_ok=True)

        print(f"\n{'='*80}")
        print("Collecting Build Artifacts (ZIP files)")
        print(f"{'='*80}")

        # Collect platform artifacts - now includes conan
        platforms = ["android", "ios", "macos", "tvos", "watchos", "windows", "linux", "ohos", "conan", "include"]
        if args.platforms:
            platforms = [p.strip() for p in args.platforms.split(",")]

        collected_platforms = []
        failed_platforms = []
        all_collected_files = []

        for platform in platforms:
            success, files = self.collect_platform_artifacts(project_dir, platform, output_path, project_name)
            if success:
                collected_platforms.append(platform)
                all_collected_files.extend(files)
            else:
                failed_platforms.append(platform)

        # Collect KMP artifacts
        kmp_success, kmp_files = self.collect_kmp_artifacts(project_dir, output_path, project_name)
        if kmp_success:
            collected_platforms.append("kmp")
            all_collected_files.extend(kmp_files)
        else:
            failed_platforms.append("kmp")

        # Check if any artifacts were collected
        if not collected_platforms:
            print(f"\n{'='*80}")
            print("⚠️  WARNING: No platform artifacts found!")
            print(f"{'='*80}\n")
            print("It looks like no platforms have been built yet.")
            print("\nTo build platforms, use:")
            print("  ccgo build android")
            print("  ccgo build ios")
            print("  ccgo build all")
            print("\nThen run 'ccgo package' again.\n")
            sys.exit(1)

        # Print summary
        print(f"\n{'='*80}")
        print("Package Summary")
        print(f"{'='*80}\n")

        print(f"Output Directory: {output_path}")
        print(f"\nCollected {len(all_collected_files)} artifact(s):")
        print("-" * 60)
        for f in sorted(all_collected_files):
            file_path = os.path.join(output_path, f)
            if os.path.exists(file_path):
                size_mb = os.path.getsize(file_path) / (1024 * 1024)
                print(f"  {f} ({size_mb:.2f} MB)")
        print("-" * 60)

        # Platform status
        print(f"\n{'='*80}")
        print("Platform Status")
        print(f"{'='*80}\n")

        for platform in collected_platforms:
            print(f"  ✅ {platform.upper()}")
        for platform in failed_platforms:
            print(f"  ❌ {platform.upper()} (not built)")

        print(f"\nTotal: {len(collected_platforms)}/{len(collected_platforms) + len(failed_platforms)} platform(s)")
        print(f"{'='*80}")
        print("\n✅ Package collection complete!")
        print(f"   All artifacts are in: {output_path}\n")
