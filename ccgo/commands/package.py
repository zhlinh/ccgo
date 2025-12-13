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
import zipfile
from datetime import datetime

# Try to import tomli for Python < 3.11, tomllib for Python >= 3.11
try:
    import tomllib
except ModuleNotFoundError:
    try:
        import tomli as tomllib
    except ModuleNotFoundError:
        tomllib = None

# region setup path
SCRIPT_PATH = os.path.split(os.path.realpath(__file__))[0]
PROJECT_ROOT_PATH = os.path.dirname(SCRIPT_PATH)
sys.path.append(SCRIPT_PATH)
sys.path.append(PROJECT_ROOT_PATH)
PACKAGE_NAME = os.path.basename(SCRIPT_PATH)
# endregion
# import this project modules
try:
    from ccgo.utils.context.namespace import CliNameSpace
    from ccgo.utils.context.context import CliContext
    from ccgo.utils.context.command import CliCommand
except ImportError:
    from utils.context.namespace import CliNameSpace
    from utils.context.context import CliContext
    from utils.context.command import CliCommand

# Try to import print_zip_tree from build_scripts
try:
    from ccgo.build_scripts.build_utils import print_zip_tree
except ImportError:
    try:
        from build_scripts.build_utils import print_zip_tree
    except ImportError:
        print_zip_tree = None


class Package(CliCommand):
    def description(self) -> str:
        return """Package all build artifacts into a distributable SDK.

This command collects build outputs from all platforms and merges them
into a unified SDK ZIP package. By default, all platform ZIPs are merged
into one unified SDK package.

EXAMPLES:
    # Package all platforms (merge into one unified ZIP)
    ccgo package

    # Keep individual ZIP files instead of merging
    ccgo package --no-merge

    # Package with specific version
    ccgo package --version 1.0.0

    # Package specific platforms only
    ccgo package --platforms android,ios,macos

OPTIONS:
    --version <version>        SDK version (default: auto-detect from git)
    --output <dir>             Output directory (default: ./target/package)
    --platforms <list>         Comma-separated platforms to include
    --no-merge                 Keep individual ZIP files instead of merging

OUTPUT STRUCTURE (merged, default):
    <PROJECT>_SDK-<version>.zip
    ├── meta/<platform>/               Build metadata (build_info.json, archive_info.json)
    ├── lib/<platform>/static|shared/  Platform native libraries
    ├── haars/<platform>/              AAR (Android) and HAR (OHOS) packages
    └── include/                       Header files

OUTPUT STRUCTURE (--no-merge):
    target/package/
    ├── <PROJECT>_ANDROID_SDK-<version>.zip
    ├── <PROJECT>_IOS_SDK-<version>.zip
    ├── <PROJECT>_KMP_SDK-<version>.zip
    └── ...
        """

    def cli(self) -> CliNameSpace:
        parser = argparse.ArgumentParser(
            prog="ccgo package",
            formatter_class=argparse.RawDescriptionHelpFormatter,
            description=self.description(),
        )

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

        parser.add_argument(
            "--platforms",
            type=str,
            help="Comma-separated platforms to include (default: all built platforms)",
        )

        parser.add_argument(
            "--no-merge",
            action="store_true",
            help="Keep individual ZIP files instead of merging into one unified SDK ZIP",
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

    def find_ccgo_toml(self, project_dir: str) -> str:
        """Find CCGO.toml in project directory or subdirectories.

        Returns the path to CCGO.toml if found, None otherwise.
        """
        # First check current directory
        if os.path.isfile(os.path.join(project_dir, "CCGO.toml")):
            return os.path.join(project_dir, "CCGO.toml")

        # Then check immediate subdirectories
        try:
            for subdir in os.listdir(project_dir):
                subdir_path = os.path.join(project_dir, subdir)
                if os.path.isdir(subdir_path):
                    potential_config = os.path.join(subdir_path, "CCGO.toml")
                    if os.path.isfile(potential_config):
                        return potential_config
        except (OSError, PermissionError):
            pass

        return None

    def get_project_name(self, config_file: str) -> str:
        """Get project name from CCGO.toml"""
        if not tomllib:
            print("   ⚠️  Warning: tomllib not available. Install 'tomli' for Python < 3.11")
            return "SDK"

        try:
            with open(config_file, 'rb') as f:
                config = tomllib.load(f)
                if 'project' in config and 'name' in config['project']:
                    return config['project']['name']
        except Exception as e:
            print(f"   ⚠️  Error reading CCGO.toml: {e}")

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

    def merge_zips(self, zip_files: list, output_zip_path: str, project_name: str, version: str):
        """Merge multiple ZIP files into a single unified SDK ZIP.

        Args:
            zip_files: List of paths to ZIP files to merge
            output_zip_path: Path for the output merged ZIP
            project_name: Project name for the SDK
            version: Version string

        Returns:
            True if merge was successful, False otherwise
        """
        import tempfile
        import json

        print(f"\n{'='*80}")
        print("Merging ZIP files into unified SDK")
        print(f"{'='*80}")

        # Create temporary directory for extraction
        with tempfile.TemporaryDirectory() as temp_dir:
            merged_dir = os.path.join(temp_dir, "merged")
            os.makedirs(merged_dir, exist_ok=True)

            platforms_merged = []

            for zip_path in zip_files:
                if not os.path.exists(zip_path):
                    continue

                filename = os.path.basename(zip_path)
                print(f"   📦 Processing: {filename}")

                try:
                    with zipfile.ZipFile(zip_path, 'r') as zf:
                        # Extract all contents to merged directory
                        for member in zf.namelist():
                            # Extract file
                            source = zf.read(member)

                            # Determine destination path
                            dest_path = os.path.join(merged_dir, member)

                            # Create parent directories
                            os.makedirs(os.path.dirname(dest_path), exist_ok=True)

                            # Skip directories
                            if member.endswith('/'):
                                continue

                            # Write file (overwrite if exists for meta files, skip otherwise)
                            if not os.path.exists(dest_path):
                                with open(dest_path, 'wb') as f:
                                    f.write(source)

                        # Track which platform was merged
                        # Try to detect platform from filename
                        fname_lower = filename.lower()
                        for plat in ['android', 'ios', 'macos', 'watchos', 'tvos', 'windows', 'linux', 'ohos', 'kmp', 'conan', 'include']:
                            if plat in fname_lower:
                                if plat not in platforms_merged:
                                    platforms_merged.append(plat)
                                break

                except Exception as e:
                    print(f"   ⚠️  Error processing {filename}: {e}")
                    continue

            # Create unified archive_info.json
            archive_info = {
                "project_name": project_name,
                "version": version,
                "platforms": platforms_merged,
                "merged": True,
                "created_at": datetime.now().isoformat(),
            }

            meta_dir = os.path.join(merged_dir, "meta")
            os.makedirs(meta_dir, exist_ok=True)
            archive_info_path = os.path.join(meta_dir, "archive_info.json")
            with open(archive_info_path, 'w') as f:
                json.dump(archive_info, f, indent=2)

            # Create the merged ZIP
            print(f"\n   📦 Creating merged SDK ZIP...")

            with zipfile.ZipFile(output_zip_path, 'w', zipfile.ZIP_DEFLATED) as zf:
                for root, dirs, files in os.walk(merged_dir):
                    for file in files:
                        file_path = os.path.join(root, file)
                        arcname = os.path.relpath(file_path, merged_dir)
                        zf.write(file_path, arcname)

            size_mb = os.path.getsize(output_zip_path) / (1024 * 1024)
            print(f"   ✅ Created: {os.path.basename(output_zip_path)} ({size_mb:.2f} MB)")
            print(f"   📍 Location: {output_zip_path}")

            return True

        return False

    def find_platform_zips(self, project_dir: str, platform: str):
        """Find all ZIP files for a platform in target/<platform>/ directory.

        Returns:
            Tuple of (list of zip file paths, platform name if found)
        """
        target_platform_dir = os.path.join(project_dir, "target", platform)

        if not os.path.exists(target_platform_dir):
            return [], None

        zip_files = []
        for root, dirs, files in os.walk(target_platform_dir):
            for f in files:
                if f.endswith('.zip') and not f.startswith('ARCHIVE'):
                    full_path = os.path.join(root, f)
                    zip_files.append(full_path)

        return zip_files, platform if zip_files else None

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

        # Check for CCGO.toml - required for package command
        config_file = self.find_ccgo_toml(project_dir)
        if not config_file:
            print(f"\n❌ ERROR: CCGO.toml not found!")
            print(f"\n   Current directory: {project_dir}")
            print(f"\n   The 'ccgo package' command must be run from a CCGO project directory")
            print(f"   (a directory containing CCGO.toml or with a subdirectory containing it).")
            print(f"\n   Please navigate to your project directory and try again:")
            print(f"   $ cd /path/to/your-project")
            print(f"   $ ccgo package")
            print()
            sys.exit(1)

        # Get project info
        project_name = self.get_project_name(config_file)
        version = self.get_version(project_dir, args)

        # Convert output path to absolute path
        if not os.path.isabs(args.output):
            output_path = os.path.join(project_dir, args.output)
        else:
            output_path = args.output

        merge_mode = not args.no_merge
        mode_str = "Merge into unified SDK" if merge_mode else "Keep individual ZIPs"

        print(f"\nProject: {project_name}")
        print(f"Version: {version}")
        print(f"Output: {output_path}")
        print(f"Mode: {mode_str}")

        # Always clean output directory to avoid stale artifacts
        if os.path.exists(output_path):
            print(f"\n🧹 Cleaning output directory...")
            shutil.rmtree(output_path)

        # Create output directory
        os.makedirs(output_path, exist_ok=True)

        print(f"\n{'='*80}")
        print("Scanning Build Artifacts")
        print(f"{'='*80}")

        # Define platforms to scan (exclude "include" as it's already in each platform's ZIP)
        platforms = ["android", "ios", "macos", "tvos", "watchos", "windows", "linux", "ohos", "conan", "kmp"]
        if args.platforms:
            platforms = [p.strip() for p in args.platforms.split(",")]

        collected_platforms = []
        failed_platforms = []
        all_zip_files = []

        for platform in platforms:
            zip_files, found_platform = self.find_platform_zips(project_dir, platform)
            if found_platform:
                collected_platforms.append(platform)
                all_zip_files.extend(zip_files)
                for zf in zip_files:
                    print(f"   ✓ Found: {os.path.basename(zf)}")
            else:
                failed_platforms.append(platform)

        # Check if any artifacts were found
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

        if merge_mode:
            # Merge all ZIPs into one unified SDK ZIP
            sdk_zip_name = f"{project_name.upper()}_SDK-{version}.zip"
            sdk_zip_path = os.path.join(output_path, sdk_zip_name)

            success = self.merge_zips(all_zip_files, sdk_zip_path, project_name, version)

            if not success:
                print(f"\n❌ Failed to merge ZIP files")
                sys.exit(1)

            # Print summary
            print(f"\n{'='*80}")
            print("Package Summary")
            print(f"{'='*80}\n")

            print(f"Platforms merged: {', '.join(collected_platforms)}")
            print(f"Output: {sdk_zip_path}")

            size_mb = os.path.getsize(sdk_zip_path) / (1024 * 1024)
            print(f"Size: {size_mb:.2f} MB")

        else:
            # Copy individual ZIPs to output directory
            print(f"\n{'='*80}")
            print("Copying Individual ZIP Files")
            print(f"{'='*80}")

            copied_files = []
            for zip_file in all_zip_files:
                filename = os.path.basename(zip_file)
                dest_path = os.path.join(output_path, filename)
                shutil.copy2(zip_file, dest_path)
                size_mb = os.path.getsize(dest_path) / (1024 * 1024)
                print(f"   ✓ {filename} ({size_mb:.2f} MB)")
                copied_files.append(filename)

            # Print summary
            print(f"\n{'='*80}")
            print("Package Summary")
            print(f"{'='*80}\n")

            print(f"Output Directory: {output_path}")
            print(f"\nCopied {len(copied_files)} artifact(s):")
            print("-" * 60)
            for f in sorted(copied_files):
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
            print(f"  ⚠️  {platform.upper()} (not built)")

        print(f"\nTotal: {len(collected_platforms)}/{len(collected_platforms) + len(failed_platforms)} platform(s)")
        print(f"{'='*80}")

        # Print package contents
        print(f"\n{'='*80}")
        print("Package Contents")
        print(f"{'='*80}\n")

        print(f"📁 {output_path}/")

        # List all files in output directory
        if os.path.exists(output_path):
            for item in sorted(os.listdir(output_path)):
                item_path = os.path.join(output_path, item)
                if os.path.isfile(item_path):
                    size_mb = os.path.getsize(item_path) / (1024 * 1024)
                    print(f"   📦 {item} ({size_mb:.2f} MB)")

                    # If it's a ZIP file, print its contents
                    if item.endswith('.zip') and print_zip_tree:
                        print_zip_tree(item_path, indent="      ")

        print(f"\n{'='*80}")
        print("\n✅ Package complete!")
        print(f"   Output: {output_path}\n")
