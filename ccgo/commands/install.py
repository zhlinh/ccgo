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
import json
import hashlib
import urllib.request
import urllib.parse
import zipfile
import tarfile
from pathlib import Path

# Try to import tomli for Python < 3.11, tomllib for Python >= 3.11
try:
    import tomllib
except ModuleNotFoundError:
    try:
        import tomli as tomllib
    except ModuleNotFoundError:
        tomllib = None

# setup path
SCRIPT_PATH = os.path.split(os.path.realpath(__file__))[0]
PROJECT_ROOT_PATH = os.path.dirname(SCRIPT_PATH)
sys.path.append(SCRIPT_PATH)
sys.path.append(PROJECT_ROOT_PATH)
PACKAGE_NAME = os.path.basename(SCRIPT_PATH)

# import this project modules
from utils.context.namespace import CliNameSpace
from utils.context.context import CliContext
from utils.context.command import CliCommand


class Install(CliCommand):
    def description(self) -> str:
        return """Install project dependencies from CCGO.toml.

This command reads dependencies from CCGO.toml and installs them
into the project's third_party directory. Supports:
- Local file paths (absolute or relative)
- Remote URLs (HTTP/HTTPS)
- Version constraints
- Platform-specific dependencies

EXAMPLES:
    # Install all dependencies
    ccgo install

    # Install specific dependency
    ccgo install libfoo

    # Force reinstall all dependencies
    ccgo install --force

    # Install for specific platform
    ccgo install --platform android

    # Clean dependency cache
    ccgo install --clean-cache

CCGO.toml FORMAT:
    [dependencies]
    libfoo = { version = "1.0.0", source = "https://example.com/libfoo_SDK-1.0.0.zip" }
    libbar = { path = "../libbar/sdk_package/libbar_SDK-1.0.0" }
    libbaz = { version = "2.1.0", source = "/absolute/path/to/libbaz_SDK-2.1.0.tar.gz" }

    # Platform-specific dependencies
    [dependencies.android]
    libandroid = { version = "1.0.0", source = "https://example.com/libandroid.zip" }

    [dependencies.ios]
    libios = { version = "1.0.0", source = "https://example.com/libios.zip" }

OUTPUT:
    Dependencies are installed to:
    - third_party/<dep_name>/              Extracted dependency SDK
    - third_party/<dep_name>/lib/          Platform-specific libraries
    - third_party/<dep_name>/include/      Header files
    - .ccgo/cache/                         Downloaded archives cache

OPTIONS:
    --force                Force reinstall even if already installed
    --platform <name>      Install only platform-specific dependencies
    --clean-cache          Clean download cache before installing
    --cache-dir <dir>      Custom cache directory (default: .ccgo/cache)
        """

    def cli(self) -> CliNameSpace:
        parser = argparse.ArgumentParser(
            prog="ccgo install",
            formatter_class=argparse.RawDescriptionHelpFormatter,
            description=self.description(),
        )

        parser.add_argument(
            "dependency",
            nargs="?",
            help="Specific dependency to install (default: install all)",
        )

        parser.add_argument(
            "--force",
            action="store_true",
            help="Force reinstall even if already installed",
        )

        parser.add_argument(
            "--platform",
            type=str,
            help="Install only platform-specific dependencies",
        )

        parser.add_argument(
            "--clean-cache",
            action="store_true",
            help="Clean download cache before installing",
        )

        parser.add_argument(
            "--cache-dir",
            type=str,
            default=".ccgo/cache",
            help="Custom cache directory (default: .ccgo/cache)",
        )

        module_name = os.path.splitext(os.path.basename(__file__))[0]
        input_argv = [x for x in sys.argv[1:] if x != module_name]
        args, unknown = parser.parse_known_args(input_argv)
        return args

    def parse_dependencies(self, project_dir: str):
        """Parse dependencies from CCGO.toml"""
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

        if not config_file:
            print("ERROR: CCGO.toml not found in project directory")
            sys.exit(1)

        if not tomllib:
            print("ERROR: tomllib not available. Install 'tomli' for Python < 3.11")
            sys.exit(1)

        try:
            with open(config_file, 'rb') as f:
                config = tomllib.load(f)
                return config.get('dependencies', {})
        except Exception as e:
            print(f"ERROR: Failed to parse CCGO.toml: {e}")
            sys.exit(1)

    def resolve_dependency_source(self, dep_config, project_dir: str):
        """Resolve dependency source to a usable path or URL

        Returns:
            tuple: (source_type, source_location)
                source_type: 'local_dir', 'local_archive', 'remote_url'
                source_location: absolute path or URL
        """
        if isinstance(dep_config, str):
            # Simple string format: "1.0.0" or path
            dep_config = {"version": dep_config}

        # Check for explicit path
        if "path" in dep_config:
            path = dep_config["path"]
            # Convert relative path to absolute
            if not os.path.isabs(path):
                path = os.path.abspath(os.path.join(project_dir, path))

            if os.path.isdir(path):
                return ("local_dir", path)
            elif os.path.isfile(path):
                return ("local_archive", path)
            else:
                print(f"ERROR: Path does not exist: {path}")
                sys.exit(1)

        # Check for remote source
        if "source" in dep_config:
            source = dep_config["source"]
            # Check if it's a URL
            if source.startswith("http://") or source.startswith("https://"):
                return ("remote_url", source)
            # Check if it's a local path
            elif os.path.exists(source):
                if os.path.isabs(source):
                    path = source
                else:
                    path = os.path.abspath(os.path.join(project_dir, source))

                if os.path.isdir(path):
                    return ("local_dir", path)
                elif os.path.isfile(path):
                    return ("local_archive", path)
            else:
                # Treat as absolute path even if it doesn't exist yet
                if os.path.isabs(source):
                    if source.endswith(('.zip', '.tar.gz', '.tgz')):
                        return ("local_archive", source)
                    else:
                        return ("local_dir", source)

        print(f"ERROR: No valid source found in dependency config: {dep_config}")
        sys.exit(1)

    def download_file(self, url: str, dest_path: str):
        """Download file from URL with progress indication"""
        print(f"   📥 Downloading from {url}...")

        try:
            # Create destination directory if needed
            os.makedirs(os.path.dirname(dest_path), exist_ok=True)

            # Download with progress
            def reporthook(count, block_size, total_size):
                if total_size > 0:
                    percent = min(int(count * block_size * 100 / total_size), 100)
                    sys.stdout.write(f"\r   Progress: {percent}%")
                    sys.stdout.flush()

            urllib.request.urlretrieve(url, dest_path, reporthook)
            print()  # New line after progress
            print(f"   ✓ Downloaded to {dest_path}")
            return True

        except Exception as e:
            print(f"\n   ✗ Download failed: {e}")
            if os.path.exists(dest_path):
                os.remove(dest_path)
            return False

    def extract_archive(self, archive_path: str, dest_dir: str):
        """Extract zip or tar.gz archive"""
        print(f"   📦 Extracting {os.path.basename(archive_path)}...")

        try:
            os.makedirs(dest_dir, exist_ok=True)

            if archive_path.endswith('.zip'):
                with zipfile.ZipFile(archive_path, 'r') as zip_ref:
                    zip_ref.extractall(dest_dir)
            elif archive_path.endswith(('.tar.gz', '.tgz')):
                with tarfile.open(archive_path, 'r:gz') as tar_ref:
                    tar_ref.extractall(dest_dir)
            else:
                print(f"   ✗ Unsupported archive format: {archive_path}")
                return False

            print(f"   ✓ Extracted to {dest_dir}")
            return True

        except Exception as e:
            print(f"   ✗ Extraction failed: {e}")
            return False

    def get_cache_path(self, cache_dir: str, source: str):
        """Generate cache file path based on source URL/path"""
        # Create hash of source for cache filename
        source_hash = hashlib.md5(source.encode()).hexdigest()[:12]

        # Extract filename from source
        if source.startswith("http"):
            parsed = urllib.parse.urlparse(source)
            filename = os.path.basename(parsed.path)
        else:
            filename = os.path.basename(source)

        # Combine hash and filename
        cache_filename = f"{source_hash}_{filename}"
        return os.path.join(cache_dir, cache_filename)

    def install_dependency(self, dep_name: str, dep_config, project_dir: str, args: CliNameSpace):
        """Install a single dependency"""
        print(f"\n📦 Installing {dep_name}...")

        # Resolve source
        source_type, source_location = self.resolve_dependency_source(dep_config, project_dir)
        print(f"   Source type: {source_type}")
        print(f"   Source: {source_location}")

        # Prepare third_party directory
        third_party_dir = os.path.join(project_dir, "third_party", dep_name)

        # Check if already installed
        if os.path.exists(third_party_dir) and not args.force:
            print(f"   ⚠️  {dep_name} already installed (use --force to reinstall)")
            return True

        # Remove existing installation if force
        if os.path.exists(third_party_dir):
            print(f"   🗑️  Removing existing installation...")
            shutil.rmtree(third_party_dir)

        # Handle based on source type
        if source_type == "local_dir":
            # Copy directory directly
            print(f"   📂 Copying from local directory...")
            try:
                shutil.copytree(source_location, third_party_dir, symlinks=True)
                print(f"   ✓ Installed to {third_party_dir}")
                return True
            except Exception as e:
                print(f"   ✗ Installation failed: {e}")
                return False

        elif source_type == "local_archive":
            # Extract local archive
            temp_extract_dir = os.path.join(project_dir, ".ccgo", "temp", dep_name)
            if os.path.exists(temp_extract_dir):
                shutil.rmtree(temp_extract_dir)

            if self.extract_archive(source_location, temp_extract_dir):
                # Move extracted content to third_party
                extracted_items = os.listdir(temp_extract_dir)
                if len(extracted_items) == 1 and os.path.isdir(os.path.join(temp_extract_dir, extracted_items[0])):
                    # Archive contains single directory, use it as source
                    shutil.move(os.path.join(temp_extract_dir, extracted_items[0]), third_party_dir)
                else:
                    # Archive contains multiple items, use temp_extract_dir as source
                    shutil.move(temp_extract_dir, third_party_dir)

                print(f"   ✓ Installed to {third_party_dir}")
                return True
            return False

        elif source_type == "remote_url":
            # Download and extract remote archive
            cache_dir = os.path.join(project_dir, args.cache_dir)
            os.makedirs(cache_dir, exist_ok=True)

            cache_path = self.get_cache_path(cache_dir, source_location)

            # Download if not in cache
            if not os.path.exists(cache_path) or args.force:
                if not self.download_file(source_location, cache_path):
                    return False

            # Extract from cache
            temp_extract_dir = os.path.join(project_dir, ".ccgo", "temp", dep_name)
            if os.path.exists(temp_extract_dir):
                shutil.rmtree(temp_extract_dir)

            if self.extract_archive(cache_path, temp_extract_dir):
                # Move extracted content to third_party
                extracted_items = os.listdir(temp_extract_dir)
                if len(extracted_items) == 1 and os.path.isdir(os.path.join(temp_extract_dir, extracted_items[0])):
                    # Archive contains single directory, use it as source
                    shutil.move(os.path.join(temp_extract_dir, extracted_items[0]), third_party_dir)
                else:
                    # Archive contains multiple items, use temp_extract_dir as source
                    shutil.move(temp_extract_dir, third_party_dir)

                print(f"   ✓ Installed to {third_party_dir}")
                return True
            return False

        return False

    def exec(self, context: CliContext, args: CliNameSpace):
        print("="*80)
        print("CCGO Install - Install Project Dependencies")
        print("="*80)

        # Get current working directory
        try:
            project_dir = os.getcwd()
        except (OSError, FileNotFoundError) as e:
            project_dir = os.environ.get('PWD')
            if not project_dir or not os.path.exists(project_dir):
                print(f"ERROR: Current working directory no longer exists: {e}")
                sys.exit(1)

        print(f"\nProject directory: {project_dir}")

        # Clean cache if requested
        if args.clean_cache:
            cache_dir = os.path.join(project_dir, args.cache_dir)
            if os.path.exists(cache_dir):
                print(f"\n🧹 Cleaning cache directory: {cache_dir}")
                shutil.rmtree(cache_dir)

        # Parse dependencies from CCGO.toml
        print("\n📖 Reading dependencies from CCGO.toml...")
        dependencies = self.parse_dependencies(project_dir)

        if not dependencies:
            print("   ⚠️  No dependencies found in CCGO.toml")
            return

        # Filter dependencies
        deps_to_install = {}

        # Handle general dependencies
        for dep_name, dep_config in dependencies.items():
            # Skip platform-specific sections
            if dep_name in ["android", "ios", "macos", "tvos", "watchos", "windows", "linux", "ohos"]:
                continue

            # If specific dependency requested, filter
            if args.dependency and dep_name != args.dependency:
                continue

            deps_to_install[dep_name] = dep_config

        # Handle platform-specific dependencies
        if args.platform and args.platform in dependencies:
            platform_deps = dependencies[args.platform]
            if isinstance(platform_deps, dict):
                for dep_name, dep_config in platform_deps.items():
                    if args.dependency and dep_name != args.dependency:
                        continue
                    deps_to_install[f"{args.platform}_{dep_name}"] = dep_config

        if not deps_to_install:
            if args.dependency:
                print(f"   ⚠️  Dependency '{args.dependency}' not found in CCGO.toml")
            else:
                print("   ⚠️  No dependencies to install")
            return

        print(f"\nFound {len(deps_to_install)} dependency(ies) to install:")
        for dep_name in deps_to_install.keys():
            print(f"  - {dep_name}")

        # Install each dependency
        print(f"\n{'='*80}")
        print("Installing Dependencies")
        print(f"{'='*80}")

        installed_count = 0
        failed_count = 0

        for dep_name, dep_config in deps_to_install.items():
            if self.install_dependency(dep_name, dep_config, project_dir, args):
                installed_count += 1
            else:
                failed_count += 1

        # Summary
        print(f"\n{'='*80}")
        print("Installation Summary")
        print(f"{'='*80}\n")
        print(f"✓ Successfully installed: {installed_count}")
        if failed_count > 0:
            print(f"✗ Failed: {failed_count}")
        print()

        if failed_count > 0:
            sys.exit(1)
