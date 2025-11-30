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
import subprocess
from pathlib import Path
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
SCRIPT_PATH = os.path.split(os.path.realpath(__file__))[0]
PROJECT_ROOT_PATH = os.path.dirname(SCRIPT_PATH)
sys.path.append(SCRIPT_PATH)
sys.path.append(PROJECT_ROOT_PATH)
PACKAGE_NAME = os.path.basename(SCRIPT_PATH)

# import this project modules
try:
    from ccgo.utils.context.namespace import CliNameSpace
    from ccgo.utils.context.context import CliContext
    from ccgo.utils.context.command import CliCommand
except ImportError:
    from utils.context.namespace import CliNameSpace
    from utils.context.context import CliContext
    from utils.context.command import CliCommand


class Install(CliCommand):
    # Global CCGO home directory
    CCGO_HOME = os.path.join(os.path.expanduser("~"), ".ccgo")

    def description(self) -> str:
        return """Install project dependencies from CCGO.toml.

This command reads dependencies from CCGO.toml and installs them
into the project's .ccgo/deps/ directory using a global cache.

DEPENDENCY RESOLUTION:
- Dependencies are cached globally in ~/.ccgo/
- Project dependencies are linked/copied to .ccgo/deps/
- Manual third-party libraries go in third_party/ (not managed by ccgo)

EXAMPLES:
    # Install all dependencies
    ccgo install

    # Install specific dependency
    ccgo install libfoo

    # Force reinstall all dependencies
    ccgo install --force

    # Install for specific platform
    ccgo install --platform android

    # Clean global cache
    ccgo install --clean-cache

    # Use copy instead of symlink (for Windows or special cases)
    ccgo install --copy

CCGO.toml FORMAT:
    [dependencies]
    libfoo = { version = "1.0.0", source = "https://example.com/LIBFOO_SDK-1.0.0.zip" }
    libbar = { path = "../libbar/target/package/LIBBAR_SDK-1.0.0" }
    libbaz = { version = "2.1.0", source = "/absolute/path/to/LIBBAZ_SDK-2.1.0.tar.gz" }

    # Platform-specific dependencies
    [dependencies.android]
    libandroid = { version = "1.0.0", source = "https://example.com/libandroid.zip" }

    [dependencies.ios]
    libios = { version = "1.0.0", source = "https://example.com/libios.zip" }

DIRECTORY STRUCTURE:
    ~/.ccgo/                               Global CCGO home
    ├── cache/                             Downloaded archives
    └── registry/                          Extracted dependencies

    project/
    ├── CCGO.toml                          Dependency declaration
    ├── CCGO.toml.lock                     Version lock file
    ├── .ccgo/deps/                        Installed dependencies (not committed)
    ├── third_party/                       Manual third-party libs (committed)
    └── vendor/                            Vendored dependencies (optional, committed)

OPTIONS:
    --force                Force reinstall even if already installed
    --platform <name>      Install only platform-specific dependencies
    --clean-cache          Clean global cache before installing
    --copy                 Copy files instead of using symlinks
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
            help="Clean global cache before installing",
        )

        parser.add_argument(
            "--copy",
            action="store_true",
            help="Copy files instead of using symlinks",
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
            print(" ⚠️  ERROR: CCGO.toml not found in project directory")
            sys.exit(1)

        if not tomllib:
            print(" ⚠️  ERROR: tomllib not available. Install 'tomli' for Python < 3.11")
            sys.exit(1)

        try:
            with open(config_file, "rb") as f:
                config = tomllib.load(f)
                return config.get("dependencies", {})
        except Exception as e:
            print(f" ⚠️  ERROR: Failed to parse CCGO.toml: {e}")
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
                    if source.endswith((".zip", ".tar.gz", ".tgz")):
                        return ("local_archive", source)
                    else:
                        return ("local_dir", source)

        print(f"ERROR: No valid source found in dependency config: {dep_config}")
        sys.exit(1)

    def download_file(self, url: str, dest_path: str):
        """Download file from URL with progress indication"""
        print(f"   Downloading from {url}...")

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
        print(f"   Extracting {os.path.basename(archive_path)}...")

        try:
            os.makedirs(dest_dir, exist_ok=True)

            if archive_path.endswith(".zip"):
                with zipfile.ZipFile(archive_path, "r") as zip_ref:
                    zip_ref.extractall(dest_dir)
            elif archive_path.endswith((".tar.gz", ".tgz")):
                with tarfile.open(archive_path, "r:gz") as tar_ref:
                    tar_ref.extractall(dest_dir)
            else:
                print(f"   ✗ Unsupported archive format: {archive_path}")
                return False

            print(f"   ✓ Extracted to {dest_dir}")
            return True

        except Exception as e:
            print(f"   ✗ Extraction failed: {e}")
            return False

    def get_cache_path(self, source: str):
        """Generate global cache file path based on source URL/path"""
        cache_dir = os.path.join(self.CCGO_HOME, "cache")
        os.makedirs(cache_dir, exist_ok=True)

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

    def get_registry_path(self, dep_name: str, source: str, version: str = None):
        """Generate global registry path for extracted dependency"""
        registry_dir = os.path.join(self.CCGO_HOME, "registry")
        os.makedirs(registry_dir, exist_ok=True)

        # Create unique hash based on source and version
        hash_input = f"{dep_name}:{source}:{version or ''}"
        source_hash = hashlib.sha256(hash_input.encode()).hexdigest()[:16]

        # Registry entry name: dep_name-hash
        registry_name = f"{dep_name}-{source_hash}"
        return os.path.join(registry_dir, registry_name)

    def create_symlink_or_copy(self, source: str, target: str, use_copy: bool = False):
        """Create symlink or copy based on platform and settings"""
        if os.path.exists(target):
            if os.path.islink(target):
                os.unlink(target)
            elif os.path.isdir(target):
                shutil.rmtree(target)
            else:
                os.remove(target)

        if use_copy:
            print(f"   Copying to {target}...")
            shutil.copytree(source, target, symlinks=True)
        else:
            # Try to create symlink
            try:
                os.symlink(source, target)
                print(f"   Linked to {target}")
            except OSError as e:
                # Fallback to copy on Windows or permission errors
                print(f" ⚠️ Symlink failed ({e}), falling back to copy...")
                shutil.copytree(source, target, symlinks=True)
                print(f"   Copied to {target}")

    def install_dependency(
        self, dep_name: str, dep_config, project_dir: str, args: CliNameSpace
    ):
        """Install a single dependency

        Dependencies are:
        1. Downloaded/extracted to global cache (~/.ccgo/registry/)
        2. Linked/copied to project's .ccgo/deps/

        Returns:
            tuple: (success: bool, install_info: dict)
                install_info contains source_type, source, install_path, git_info, checksum, etc.
        """
        print(f"\n Installing {dep_name}...")

        # Resolve source
        source_type, source_location = self.resolve_dependency_source(
            dep_config, project_dir
        )
        print(f"   Source type: {source_type}")
        print(f"   Source: {source_location}")

        # Get version if available
        version = None
        if isinstance(dep_config, dict) and "version" in dep_config:
            version = dep_config["version"]

        # Prepare .ccgo/deps directory
        deps_dir = os.path.join(project_dir, ".ccgo", "deps")
        os.makedirs(deps_dir, exist_ok=True)
        install_path = os.path.join(deps_dir, dep_name)

        # Base install info
        install_info = {
            "source_type": source_type,
            "source": source_location,
            "install_path": install_path,
            "installed_at": datetime.now().isoformat(),
        }

        if version:
            install_info["version"] = version

        # Check if already installed
        if os.path.exists(install_path) and not args.force:
            print(f"   {dep_name} already installed (use --force to reinstall)")
            if source_type == "local_dir":
                install_info["git_info"] = self.get_git_info(source_location)
            return (True, install_info)

        # Remove existing installation if force
        if os.path.exists(install_path):
            print(f"   Removing existing installation...")
            if os.path.islink(install_path):
                os.unlink(install_path)
            else:
                shutil.rmtree(install_path)

        # Handle based on source type
        if source_type == "local_dir":
            # For local directories, link/copy directly (no global cache)
            print(f"   Installing from local directory...")
            try:
                use_copy = getattr(args, "copy", False)
                self.create_symlink_or_copy(source_location, install_path, use_copy)
                print(f"   ✓ Installed to {install_path}")
                install_info["git_info"] = self.get_git_info(source_location)
                return (True, install_info)
            except Exception as e:
                print(f"   ✗ Installation failed: {e}")
                return (False, install_info)

        elif source_type == "local_archive":
            # Extract to global registry, then link to project
            registry_path = self.get_registry_path(dep_name, source_location, version)

            # Compute checksum
            checksum = self.compute_file_checksum(source_location)
            if checksum:
                install_info["checksum"] = f"sha256:{checksum}"

            # Extract to registry if not exists or force
            if not os.path.exists(registry_path) or args.force:
                if os.path.exists(registry_path):
                    shutil.rmtree(registry_path)

                temp_extract_dir = os.path.join(self.CCGO_HOME, "temp", dep_name)
                if os.path.exists(temp_extract_dir):
                    shutil.rmtree(temp_extract_dir)

                if self.extract_archive(source_location, temp_extract_dir):
                    extracted_items = os.listdir(temp_extract_dir)
                    if len(extracted_items) == 1 and os.path.isdir(
                        os.path.join(temp_extract_dir, extracted_items[0])
                    ):
                        shutil.move(
                            os.path.join(temp_extract_dir, extracted_items[0]),
                            registry_path,
                        )
                    else:
                        shutil.move(temp_extract_dir, registry_path)
                else:
                    return (False, install_info)

            # Link/copy from registry to project
            use_copy = getattr(args, "copy", False)
            self.create_symlink_or_copy(registry_path, install_path, use_copy)
            install_info["registry_path"] = registry_path
            print(f"   ✓ Installed to {install_path}")
            return (True, install_info)

        elif source_type == "remote_url":
            # Download to global cache, extract to registry, link to project
            cache_path = self.get_cache_path(source_location)
            registry_path = self.get_registry_path(dep_name, source_location, version)

            # Download if not in cache
            if not os.path.exists(cache_path) or args.force:
                if not self.download_file(source_location, cache_path):
                    return (False, install_info)

            # Compute checksum
            checksum = self.compute_file_checksum(cache_path)
            if checksum:
                install_info["checksum"] = f"sha256:{checksum}"

            # Extract to registry if not exists or force
            if not os.path.exists(registry_path) or args.force:
                if os.path.exists(registry_path):
                    shutil.rmtree(registry_path)

                temp_extract_dir = os.path.join(self.CCGO_HOME, "temp", dep_name)
                if os.path.exists(temp_extract_dir):
                    shutil.rmtree(temp_extract_dir)

                if self.extract_archive(cache_path, temp_extract_dir):
                    extracted_items = os.listdir(temp_extract_dir)
                    if len(extracted_items) == 1 and os.path.isdir(
                        os.path.join(temp_extract_dir, extracted_items[0])
                    ):
                        shutil.move(
                            os.path.join(temp_extract_dir, extracted_items[0]),
                            registry_path,
                        )
                    else:
                        shutil.move(temp_extract_dir, registry_path)
                else:
                    return (False, install_info)

            # Link/copy from registry to project
            use_copy = getattr(args, "copy", False)
            self.create_symlink_or_copy(registry_path, install_path, use_copy)
            install_info["registry_path"] = registry_path
            print(f"   ✓ Installed to {install_path}")
            return (True, install_info)

        return (False, install_info)

    def get_git_info(self, path: str):
        """Get git information for a local path dependency

        Handles both standalone git repos and subdirectories within a git repo.
        """
        git_info = {}
        if not os.path.isdir(path):
            return git_info

        # Check if path is inside a git repository (even if not at root)
        try:
            result = subprocess.run(
                ["git", "rev-parse", "--is-inside-work-tree"],
                cwd=path,
                capture_output=True,
                text=True,
                timeout=10,
            )
            if result.returncode != 0 or result.stdout.strip() != "true":
                return git_info
        except Exception:
            return git_info

        try:
            # Get current commit hash
            result = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=path,
                capture_output=True,
                text=True,
                timeout=10,
            )
            if result.returncode == 0:
                git_info["revision"] = result.stdout.strip()

            # Get current branch
            result = subprocess.run(
                ["git", "rev-parse", "--abbrev-ref", "HEAD"],
                cwd=path,
                capture_output=True,
                text=True,
                timeout=10,
            )
            if result.returncode == 0:
                git_info["branch"] = result.stdout.strip()

            # Get remote URL
            result = subprocess.run(
                ["git", "config", "--get", "remote.origin.url"],
                cwd=path,
                capture_output=True,
                text=True,
                timeout=10,
            )
            if result.returncode == 0:
                git_info["remote_url"] = result.stdout.strip()

            # Check if dirty
            result = subprocess.run(
                ["git", "status", "--porcelain"],
                cwd=path,
                capture_output=True,
                text=True,
                timeout=10,
            )
            if result.returncode == 0:
                git_info["dirty"] = len(result.stdout.strip()) > 0

        except Exception:
            pass

        return git_info

    def generate_lock_file(self, project_dir: str, installed_deps: dict):
        """Generate CCGO.toml.lock file with installed dependency information"""
        lock_file_path = os.path.join(project_dir, "CCGO.toml.lock")

        lock_data = {
            "metadata": {
                "version": "1.0",
                "generated_at": datetime.now().isoformat(),
                "generator": "ccgo install",
            },
            "dependencies": {},
        }

        for dep_name, dep_info in installed_deps.items():
            lock_entry = {
                "source_type": dep_info.get("source_type", "unknown"),
                "source": dep_info.get("source", ""),
                "installed_at": dep_info.get(
                    "installed_at", datetime.now().isoformat()
                ),
                "install_path": dep_info.get("install_path", ""),
            }

            # Add version if available
            if "version" in dep_info:
                lock_entry["version"] = dep_info["version"]

            # Add git info for local path dependencies
            if "git_info" in dep_info and dep_info["git_info"]:
                lock_entry["git"] = dep_info["git_info"]

            # Add checksum for remote/archive dependencies
            if "checksum" in dep_info:
                lock_entry["checksum"] = dep_info["checksum"]

            lock_data["dependencies"][dep_name] = lock_entry

        # Write lock file in TOML format
        try:
            with open(lock_file_path, "w") as f:
                f.write("# CCGO.toml.lock - Auto-generated lock file\n")
                f.write("# Do not edit this file manually\n")
                f.write("# Regenerate with: ccgo install --force\n\n")

                # Write metadata section
                f.write("[metadata]\n")
                f.write(f'version = "{lock_data["metadata"]["version"]}"\n')
                f.write(f'generated_at = "{lock_data["metadata"]["generated_at"]}"\n')
                f.write(f'generator = "{lock_data["metadata"]["generator"]}"\n\n')

                # Write each dependency
                for dep_name, dep_info in lock_data["dependencies"].items():
                    f.write(f"[dependencies.{dep_name}]\n")
                    f.write(f'source_type = "{dep_info["source_type"]}"\n')
                    f.write(f'source = "{dep_info["source"]}"\n')
                    f.write(f'installed_at = "{dep_info["installed_at"]}"\n')
                    f.write(f'install_path = "{dep_info["install_path"]}"\n')

                    if "version" in dep_info:
                        f.write(f'version = "{dep_info["version"]}"\n')

                    if "checksum" in dep_info:
                        f.write(f'checksum = "{dep_info["checksum"]}"\n')

                    if "git" in dep_info:
                        git_info = dep_info["git"]
                        f.write(f"\n[dependencies.{dep_name}.git]\n")
                        if "revision" in git_info:
                            f.write(f'revision = "{git_info["revision"]}"\n')
                        if "branch" in git_info:
                            f.write(f'branch = "{git_info["branch"]}"\n')
                        if "remote_url" in git_info:
                            f.write(f'remote_url = "{git_info["remote_url"]}"\n')
                        if "dirty" in git_info:
                            f.write(
                                f'dirty = {"true" if git_info["dirty"] else "false"}\n'
                            )

                    f.write("\n")

            print(f"\n Generated lock file: {lock_file_path}")
            return True

        except Exception as e:
            print(f"\n⚠️  Failed to generate lock file: {e}")
            return False

    def compute_file_checksum(self, file_path: str):
        """Compute SHA256 checksum of a file"""
        sha256_hash = hashlib.sha256()
        try:
            with open(file_path, "rb") as f:
                for byte_block in iter(lambda: f.read(4096), b""):
                    sha256_hash.update(byte_block)
            return sha256_hash.hexdigest()
        except Exception:
            return None

    def update_gitignore(self, project_dir: str):
        """Ensure .ccgo/ is in .gitignore"""
        gitignore_path = os.path.join(project_dir, ".gitignore")
        ccgo_pattern = ".ccgo/"

        # Check if .gitignore exists and if .ccgo/ is already in it
        if os.path.exists(gitignore_path):
            with open(gitignore_path, "r") as f:
                content = f.read()
                if ccgo_pattern in content or ".ccgo" in content:
                    return  # Already ignored

            # Append .ccgo/ to existing .gitignore
            with open(gitignore_path, "a") as f:
                f.write(f"\n# CCGO dependencies (auto-generated)\n{ccgo_pattern}\n")
            print(f"   Added {ccgo_pattern} to .gitignore")
        else:
            # Create new .gitignore
            with open(gitignore_path, "w") as f:
                f.write(f"# CCGO dependencies\n{ccgo_pattern}\n")
            print(f"   Created .gitignore with {ccgo_pattern}")

    def exec(self, context: CliContext, args: CliNameSpace):
        print("=" * 80)
        print("CCGO Install - Install Project Dependencies")
        print("=" * 80)

        # Get current working directory
        try:
            project_dir = os.getcwd()
        except (OSError, FileNotFoundError) as e:
            project_dir = os.environ.get("PWD")
            if not project_dir or not os.path.exists(project_dir):
                print(f"ERROR: Current working directory no longer exists: {e}")
                sys.exit(1)

        print(f"\nProject directory: {project_dir}")
        print(f"Global CCGO home: {self.CCGO_HOME}")

        # Clean global cache if requested
        if args.clean_cache:
            if os.path.exists(self.CCGO_HOME):
                print(f"\n Cleaning global cache: {self.CCGO_HOME}")
                shutil.rmtree(self.CCGO_HOME)

        # Parse dependencies from CCGO.toml
        print("\n Reading dependencies from CCGO.toml...")
        dependencies = self.parse_dependencies(project_dir)

        if not dependencies:
            print("   ⚠️  No dependencies found in CCGO.toml")
            return

        # Filter dependencies
        deps_to_install = {}

        # Handle general dependencies
        for dep_name, dep_config in dependencies.items():
            # Skip platform-specific sections
            if dep_name in [
                "android",
                "ios",
                "macos",
                "tvos",
                "watchos",
                "windows",
                "linux",
                "ohos",
            ]:
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
        installed_deps = {}  # Collect info for lock file

        for dep_name, dep_config in deps_to_install.items():
            success, install_info = self.install_dependency(
                dep_name, dep_config, project_dir, args
            )
            if success:
                installed_count += 1
                installed_deps[dep_name] = install_info
            else:
                failed_count += 1

        # Generate lock file if any dependencies were installed
        if installed_deps:
            self.generate_lock_file(project_dir, installed_deps)
            # Update .gitignore to exclude .ccgo/
            self.update_gitignore(project_dir)

        # Summary
        print(f"\n{'='*80}")
        print("Installation Summary")
        print(f"{'='*80}\n")
        print(f"✓ Successfully installed: {installed_count}")
        print(f"  Dependencies installed to: .ccgo/deps/")
        if failed_count > 0:
            print(f"✗ Failed: {failed_count}")
        print()

        if failed_count > 0:
            sys.exit(1)
