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
import zipfile
import tarfile
import glob
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
            import subprocess
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
        """Collect artifacts for a specific platform from target/<platform> directory

        Build scripts output compressed archives (ZIP/AAR/HAR) in target/<platform>/.
        This function extracts libraries from these archives and organizes them properly.
        """
        print(f"\n📦 Collecting {platform} artifacts...")

        # Check if target/<platform> directory exists
        target_platform_dir = os.path.join(project_dir, "target", platform)

        if not os.path.exists(target_platform_dir):
            print(f"   ⚠️  No artifacts found (target/{platform} does not exist)")
            print(f"   💡 Build {platform} first with: ccgo build {platform}")
            return False

        collected = False
        platform_output = os.path.join(output_dir, "lib", platform)

        try:
            # Remove existing output directory if it exists
            if os.path.exists(platform_output):
                shutil.rmtree(platform_output)

            # Find compressed archives in target/<platform>/
            import zipfile
            import tarfile
            archive_files = []

            for f in os.listdir(target_platform_dir):
                if f.startswith('(ARCHIVE)'):
                    continue  # Skip archive markers
                full_path = os.path.join(target_platform_dir, f)
                if f.endswith(('.zip', '.aar', '.har')) and os.path.isfile(full_path):
                    archive_files.append(full_path)

            if not archive_files:
                print(f"   ⚠️  No build archives found in target/{platform}")
                print(f"   💡 Expected .zip, .aar, or .har files")
                return False

            # Use the first (and typically only) archive file
            archive_file = archive_files[0]
            print(f"   📂 Extracting from {os.path.basename(archive_file)}...")

            # Create temporary extraction directory
            temp_extract_dir = os.path.join(project_dir, ".ccgo", "temp", f"extract_{platform}")
            if os.path.exists(temp_extract_dir):
                shutil.rmtree(temp_extract_dir)
            os.makedirs(temp_extract_dir, exist_ok=True)

            # Extract archive (support both ZIP and tar.gz formats)
            if archive_file.endswith('.har'):
                # HAR files are tar.gz format
                with tarfile.open(archive_file, 'r:gz') as tar_ref:
                    tar_ref.extractall(temp_extract_dir)
            else:
                # ZIP format (AAR, regular ZIP)
                with zipfile.ZipFile(archive_file, 'r') as zip_ref:
                    zip_ref.extractall(temp_extract_dir)

            # Platform-specific extraction logic
            if platform.lower() == "android":
                collected = self._extract_android_libraries(temp_extract_dir, platform_output, project_name)
            elif platform.lower() in ["ios", "macos", "tvos", "watchos"]:
                collected = self._extract_darwin_libraries(temp_extract_dir, platform_output, project_name)
            elif platform.lower() == "linux":
                collected = self._extract_linux_libraries(temp_extract_dir, platform_output, project_name)
            elif platform.lower() == "windows":
                collected = self._extract_windows_libraries(temp_extract_dir, platform_output, project_name)
            elif platform.lower() == "ohos":
                collected = self._extract_ohos_libraries(temp_extract_dir, platform_output, project_name)

            # Clean up temp directory
            if os.path.exists(temp_extract_dir):
                shutil.rmtree(temp_extract_dir)

        except Exception as e:
            print(f"   ⚠️  Error collecting {platform} artifacts: {e}")
            import traceback
            traceback.print_exc()

        return collected

    def _extract_android_libraries(self, extract_dir: str, output_dir: str, project_name: str):
        """Extract .so files from Android AAR jni/ directory"""
        jni_dir = os.path.join(extract_dir, "jni")
        if not os.path.exists(jni_dir):
            print(f"   ⚠️  No jni/ directory found in Android archive")
            return False

        collected = False
        # Organize by architecture: shared/<arch>/lib*.so
        for arch in os.listdir(jni_dir):
            arch_dir = os.path.join(jni_dir, arch)
            if not os.path.isdir(arch_dir):
                continue

            # Create output directory
            output_arch_dir = os.path.join(output_dir, "shared", arch)
            os.makedirs(output_arch_dir, exist_ok=True)

            # Copy .so files (filter out libc++_shared.so which is system library)
            for f in os.listdir(arch_dir):
                if f.endswith('.so') and f.startswith('lib') and 'c++_shared' not in f:
                    src = os.path.join(arch_dir, f)
                    dest = os.path.join(output_arch_dir, f)
                    shutil.copy2(src, dest)
                    print(f"   ✓ shared/{arch}/{f}")
                    collected = True

        return collected

    def _extract_darwin_libraries(self, extract_dir: str, output_dir: str, project_name: str):
        """Extract .xcframework or .framework from Darwin platforms (iOS/macOS/tvOS/watchOS)"""
        # Look for xcframework or framework in extracted directory
        collected = False

        for root, dirs, files in os.walk(extract_dir):
            for d in dirs:
                if d.endswith('.xcframework') or d.endswith('.framework'):
                    src = os.path.join(root, d)
                    # Organize into static/ directory
                    static_dir = os.path.join(output_dir, "static")
                    os.makedirs(static_dir, exist_ok=True)
                    dest = os.path.join(static_dir, d)

                    if os.path.exists(dest):
                        shutil.rmtree(dest)
                    shutil.copytree(src, dest, symlinks=True)
                    print(f"   ✓ static/{d}/")
                    collected = True
                    break  # Only take the first framework found
            if collected:
                break

        return collected

    def _extract_linux_libraries(self, extract_dir: str, output_dir: str, project_name: str):
        """Extract .so and .a files from Linux archive"""
        collected = False

        # Look for .so and .a files in extracted directory
        for root, dirs, files in os.walk(extract_dir):
            for f in files:
                if f.endswith('.so') or f.endswith('.a'):
                    src = os.path.join(root, f)

                    # Determine if it's static or shared
                    if f.endswith('.a'):
                        lib_type = "static"
                    else:
                        lib_type = "shared"

                    output_type_dir = os.path.join(output_dir, lib_type)
                    os.makedirs(output_type_dir, exist_ok=True)

                    dest = os.path.join(output_type_dir, f)
                    shutil.copy2(src, dest)
                    print(f"   ✓ {lib_type}/{f}")
                    collected = True

        return collected

    def _extract_windows_libraries(self, extract_dir: str, output_dir: str, project_name: str):
        """Extract .lib and .dll files from Windows archive"""
        collected = False

        # Look for .lib and .dll files in extracted directory
        for root, dirs, files in os.walk(extract_dir):
            for f in files:
                if f.endswith('.lib') or f.endswith('.dll'):
                    src = os.path.join(root, f)

                    # Determine if it's static or shared
                    if f.endswith('.lib'):
                        lib_type = "static"
                    elif f.endswith('.dll'):
                        lib_type = "shared"

                    # Windows uses x64 subdirectory
                    output_arch_dir = os.path.join(output_dir, lib_type, "x64")
                    os.makedirs(output_arch_dir, exist_ok=True)

                    dest = os.path.join(output_arch_dir, f)
                    shutil.copy2(src, dest)
                    print(f"   ✓ {lib_type}/x64/{f}")
                    collected = True

        return collected

    def _extract_ohos_libraries(self, extract_dir: str, output_dir: str, project_name: str):
        """Extract .so files from OHOS HAR package/libs/ directory"""
        libs_dir = os.path.join(extract_dir, "package", "libs")
        if not os.path.exists(libs_dir):
            # Try alternate location
            libs_dir = os.path.join(extract_dir, "libs")
            if not os.path.exists(libs_dir):
                print(f"   ⚠️  No libs/ directory found in OHOS archive")
                return False

        collected = False
        # Organize by architecture: shared/<arch>/lib*.so
        for arch in os.listdir(libs_dir):
            arch_dir = os.path.join(libs_dir, arch)
            if not os.path.isdir(arch_dir):
                continue

            # Create output directory
            output_arch_dir = os.path.join(output_dir, "shared", arch)
            os.makedirs(output_arch_dir, exist_ok=True)

            # Copy .so files
            for f in os.listdir(arch_dir):
                if f.endswith('.so') and f.startswith('lib'):
                    src = os.path.join(arch_dir, f)
                    dest = os.path.join(output_arch_dir, f)
                    shutil.copy2(src, dest)
                    print(f"   ✓ shared/{arch}/{f}")
                    collected = True

        return collected

    def collect_include_headers(self, project_dir: str, output_dir: str, project_name: str):
        """Collect include headers"""
        print(f"\n📦 Collecting include headers...")

        # Define ignore patterns for files that should not be included
        def ignore_patterns(directory, files):
            """Ignore non-header files and build artifacts"""
            ignored = []
            for name in files:
                # Ignore CPPLINT.cfg and other non-header files
                if name in ['CPPLINT.cfg', '.DS_Store', 'Thumbs.db']:
                    ignored.append(name)
                # Ignore files without extension or with non-header extensions
                elif '.' in name and not name.endswith(('.h', '.hpp', '.hxx', '.h++', '.hh')):
                    # Allow directories to be traversed
                    full_path = os.path.join(directory, name)
                    if not os.path.isdir(full_path):
                        ignored.append(name)
            return ignored

        # First check if include directory exists directly in project_dir
        include_dir = os.path.join(project_dir, "include")
        if os.path.exists(include_dir) and os.path.isdir(include_dir):
            try:
                output_include = os.path.join(output_dir, "include")
                if os.path.exists(output_include):
                    shutil.rmtree(output_include)
                shutil.copytree(include_dir, output_include, symlinks=True, ignore=ignore_patterns)
                print(f"   ✓ Copied include headers")
                file_count = sum(1 for _ in Path(output_include).rglob('*') if _.is_file())
                print(f"   ✓ {file_count} header files collected")
                return True
            except (OSError, PermissionError) as e:
                print(f"   ⚠️  Error collecting headers: {e}")

        # Look for include directory in project subdirectories
        try:
            for subdir in os.listdir(project_dir):
                subdir_path = os.path.join(project_dir, subdir)
                if not os.path.isdir(subdir_path):
                    continue
                include_dir = os.path.join(subdir_path, "include")
                if os.path.exists(include_dir) and os.path.isdir(include_dir):
                    # Copy the contents of include directory
                    # The structure should be: output_dir/include/<project_name>/...
                    output_include = os.path.join(output_dir, "include")

                    # Copy the entire include directory tree
                    if os.path.exists(output_include):
                        shutil.rmtree(output_include)
                    shutil.copytree(include_dir, output_include, symlinks=True, ignore=ignore_patterns)
                    print(f"   ✓ Copied include headers")

                    # Count files
                    file_count = sum(1 for _ in Path(output_include).rglob('*') if _.is_file())
                    print(f"   ✓ {file_count} header files collected")
                    return True
        except (OSError, PermissionError) as e:
            print(f"   ⚠️  Error collecting headers: {e}")

        print(f"   ⚠️  No include headers found")
        return False

    def collect_kmp_artifacts(self, project_dir: str, output_dir: str, project_name: str):
        """Collect KMP artifacts from target/kmp directory"""
        print(f"\n📦 Collecting KMP artifacts...")

        collected = False

        # Look for KMP artifacts in target/kmp directory
        target_kmp_dir = os.path.join(project_dir, "target", "kmp")

        if not os.path.exists(target_kmp_dir):
            print(f"   ⚠️  No KMP artifacts found (target/kmp does not exist)")
            print(f"   💡 Build KMP first with: ccgo build kmp")
            return False

        kmp_output_base = os.path.join(output_dir, "lib", "kmp")

        try:
            # Copy Android AAR files
            android_src = os.path.join(target_kmp_dir, "android")
            if os.path.exists(android_src):
                aar_files = list(Path(android_src).glob("*.aar"))
                if aar_files:
                    android_dest = os.path.join(kmp_output_base, "android")
                    os.makedirs(android_dest, exist_ok=True)
                    for aar_file in aar_files:
                        shutil.copy2(aar_file, os.path.join(android_dest, aar_file.name))
                        print(f"   ✓ android/{aar_file.name}")
                        collected = True

            # Copy Desktop JAR files
            desktop_src = os.path.join(target_kmp_dir, "desktop")
            if os.path.exists(desktop_src):
                jar_files = list(Path(desktop_src).glob("*.jar"))
                if jar_files:
                    desktop_dest = os.path.join(kmp_output_base, "desktop")
                    os.makedirs(desktop_dest, exist_ok=True)
                    for jar_file in jar_files:
                        shutil.copy2(jar_file, os.path.join(desktop_dest, jar_file.name))
                        print(f"   ✓ desktop/{jar_file.name}")
                        collected = True

            # Copy Native klib files (iOS, macOS, Linux)
            native_src = os.path.join(target_kmp_dir, "native")
            if os.path.exists(native_src):
                for platform_dir in Path(native_src).iterdir():
                    if platform_dir.is_dir():
                        platform_name = platform_dir.name
                        platform_dest = os.path.join(kmp_output_base, "native", platform_name)

                        # Only copy if there are actual files
                        has_files = any(platform_dir.rglob('*'))
                        if has_files:
                            if os.path.exists(platform_dest):
                                shutil.rmtree(platform_dest)
                            shutil.copytree(platform_dir, platform_dest, symlinks=True)
                            print(f"   ✓ native/{platform_name}/")
                            collected = True

        except (OSError, PermissionError) as e:
            print(f"   ⚠️  Error collecting KMP artifacts: {e}")
            return False

        if not collected:
            print(f"   ⚠️  No KMP artifacts found in target/kmp")
            print(f"   💡 Build KMP first with: ccgo build kmp")

        return collected

    def collect_documentation(self, project_dir: str, output_dir: str, project_name: str):
        """Collect documentation"""
        print(f"\n📦 Collecting documentation...")

        # Look for docs build outputs
        docs_build_dir = os.path.join(project_dir, "cmake_build", "Docs")

        if os.path.exists(docs_build_dir):
            # Find HTML output
            for root, dirs, files in os.walk(docs_build_dir):
                if "_html" in dirs or "html" in dirs:
                    html_dir = os.path.join(root, "_html" if "_html" in dirs else "html")
                    docs_output = os.path.join(output_dir, "docs")
                    if os.path.exists(docs_output):
                        shutil.rmtree(docs_output)
                    shutil.copytree(html_dir, docs_output)
                    print(f"   ✓ Copied from {html_dir}")
                    return True

        print(f"   ⚠️  No documentation found")
        return False

    def generate_package_json(self, package_dir: str, project_name: str, version: str, collected_platforms: list):
        """Generate ccgo-package.json with SDK metadata

        This file contains:
        - Package name, version, and generation time
        - List of platforms with their link types
        - Library files for each platform/link_type/arch
        - Dependencies (third-party libraries) information
        """
        print(f"\n📦 Generating ccgo-package.json...")

        package_metadata = {
            "name": project_name,
            "version": version,
            "generated": datetime.now().isoformat(),
            "platforms": {}
        }

        lib_dir = os.path.join(package_dir, "lib")
        if not os.path.exists(lib_dir):
            print("   ⚠️  No lib directory found, skipping metadata generation")
            return

        # Scan each platform directory
        for platform in collected_platforms:
            platform_dir = os.path.join(lib_dir, platform)
            if not os.path.exists(platform_dir):
                continue

            platform_metadata = {
                "link_types": {}
            }

            # Check for static and shared subdirectories
            for link_type in ["static", "shared"]:
                link_type_dir = os.path.join(platform_dir, link_type)
                if os.path.exists(link_type_dir):
                    link_type_metadata = self._scan_link_type_dir(link_type_dir, platform, link_type, project_name)
                    if link_type_metadata:
                        platform_metadata["link_types"][link_type] = link_type_metadata

            if platform_metadata["link_types"]:
                package_metadata["platforms"][platform] = platform_metadata

        # Write JSON file
        json_path = os.path.join(package_dir, "ccgo-package.json")
        with open(json_path, 'w') as f:
            import json
            json.dump(package_metadata, f, indent=2)

        print(f"   ✓ Generated ccgo-package.json")
        return json_path

    def _scan_link_type_dir(self, link_type_dir: str, platform: str, link_type: str, project_name: str):
        """Scan a link type directory and return metadata about libraries"""
        metadata = {}

        # For platforms with architectures (Android, OHOS, Windows)
        if platform.lower() in ["android", "ohos"]:
            metadata["architectures"] = {}
            for arch in ["arm64-v8a", "armeabi-v7a", "x86_64"]:
                arch_dir = os.path.join(link_type_dir, arch)
                if os.path.exists(arch_dir):
                    libs = []
                    for lib_file in Path(arch_dir).iterdir():
                        if lib_file.is_file():
                            libs.append({
                                "name": lib_file.name,
                                "size": lib_file.stat().st_size,
                                "path": f"lib/{platform}/{link_type}/{arch}/{lib_file.name}"
                            })
                    if libs:
                        metadata["architectures"][arch] = {"libraries": libs}

        elif platform.lower() == "windows":
            metadata["architectures"] = {}
            x64_dir = os.path.join(link_type_dir, "x64")
            if os.path.exists(x64_dir):
                libs = []
                for lib_file in Path(x64_dir).iterdir():
                    if lib_file.is_file():
                        libs.append({
                            "name": lib_file.name,
                            "size": lib_file.stat().st_size,
                            "path": f"lib/{platform}/{link_type}/x64/{lib_file.name}"
                        })
                if libs:
                    metadata["architectures"]["x64"] = {"libraries": libs}

        # For Apple platforms (iOS, macOS, tvOS, watchOS) and Linux
        else:
            libs = []
            for item in Path(link_type_dir).iterdir():
                if item.is_file():
                    libs.append({
                        "name": item.name,
                        "size": item.stat().st_size,
                        "path": f"lib/{platform}/{link_type}/{item.name}"
                    })
                elif item.is_dir() and (item.suffix in [".framework", ".xcframework"]):
                    # For frameworks, calculate total size
                    total_size = sum(f.stat().st_size for f in item.rglob('*') if f.is_file())
                    libs.append({
                        "name": item.name,
                        "size": total_size,
                        "path": f"lib/{platform}/{link_type}/{item.name}",
                        "type": "framework" if item.suffix == ".framework" else "xcframework"
                    })
            if libs:
                metadata["libraries"] = libs

        return metadata if metadata else None

    def create_archive(self, source_dir: str, output_path: str, format: str):
        """Create archive from source directory"""
        print(f"\n📦 Creating {format} archive...")

        if format == "zip":
            with zipfile.ZipFile(output_path + ".zip", 'w', zipfile.ZIP_DEFLATED) as zipf:
                for root, dirs, files in os.walk(source_dir):
                    for file in files:
                        file_path = os.path.join(root, file)
                        arcname = os.path.relpath(file_path, source_dir)
                        zipf.write(file_path, arcname)
            print(f"   ✓ Created {output_path}.zip")

        elif format == "tar.gz":
            with tarfile.open(output_path + ".tar.gz", "w:gz") as tar:
                tar.add(source_dir, arcname=os.path.basename(output_path))
            print(f"   ✓ Created {output_path}.tar.gz")

    def exec(self, context: CliContext, args: CliNameSpace):
        print("="*80)
        print("CCGO Package - Create SDK Distribution")
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

        # Create package name
        package_name = f"{project_name.upper()}_SDK-{version}"
        package_dir = os.path.join(output_path, package_name)

        # Clean if requested
        if args.clean and os.path.exists(output_path):
            print(f"\n🧹 Cleaning output directory...")
            shutil.rmtree(output_path)

        # Create output directory
        os.makedirs(package_dir, exist_ok=True)

        print(f"\n{'='*80}")
        print("Collecting Artifacts")
        print(f"{'='*80}")

        # Collect include headers (always)
        self.collect_include_headers(project_dir, package_dir, project_name)

        # Collect platform artifacts
        platforms = ["android", "ios", "macos", "tvos", "watchos", "windows", "linux", "ohos"]
        if args.platforms:
            platforms = [p.strip() for p in args.platforms.split(",")]

        collected_platforms = []
        failed_platforms = []
        for platform in platforms:
            if self.collect_platform_artifacts(project_dir, platform, package_dir, project_name):
                collected_platforms.append(platform)
            else:
                failed_platforms.append(platform)

        # Collect KMP artifacts (always try to collect if they exist)
        kmp_collected = self.collect_kmp_artifacts(project_dir, package_dir, project_name)
        if kmp_collected:
            collected_platforms.append("kmp")
        else:
            failed_platforms.append("kmp")

        # Collect documentation
        if args.include_docs:
            self.collect_documentation(project_dir, package_dir, project_name)

        # Check if any artifacts were collected
        if not collected_platforms:
            print(f"\n{'='*80}")
            print("⚠️  WARNING: No platform artifacts found!")
            print(f"{'='*80}\n")
            print("It looks like no platforms have been built yet.")
            print("\nTo build platforms, use:")
            print("  ccgo build android")
            print("  ccgo build ios")
            print("  ccgo build macos")
            print("  ccgo build windows")
            print("  ccgo build linux")
            print("  ccgo build ohos")
            print("\nOr build all platforms with:")
            print("  ccgo ci")
            print("\nPackaging will continue with available artifacts only.\n")

        # Create README
        readme_path = os.path.join(package_dir, "README.md")
        with open(readme_path, 'w') as f:
            f.write(f"# {project_name} SDK v{version}\n\n")
            f.write(f"Generated: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n\n")
            f.write(f"## Platforms\n\n")
            if collected_platforms:
                for platform in collected_platforms:
                    if platform == "kmp":
                        f.write(f"- KMP (Kotlin Multiplatform)\n")
                    else:
                        f.write(f"- {platform.capitalize()}\n")
            else:
                f.write(f"*No platform artifacts found. Build platforms first.*\n")
            f.write(f"\n## Structure\n\n")
            f.write(f"- `include/` - Header files\n")
            if collected_platforms:
                f.write(f"- `lib/` - Platform-specific libraries\n")
                for platform in collected_platforms:
                    if platform == "kmp":
                        f.write(f"  - `lib/kmp/` - Kotlin Multiplatform artifacts\n")
                        f.write(f"    - `lib/kmp/android/` - KMP Android libraries (.aar)\n")
                        f.write(f"    - `lib/kmp/desktop/` - KMP Desktop libraries (.jar)\n")
                        f.write(f"    - `lib/kmp/native/` - KMP Native libraries (iOS, macOS, Linux)\n")
                    else:
                        f.write(f"  - `lib/{platform}/` - {platform.capitalize()} libraries\n")
            if args.include_docs:
                f.write(f"- `docs/` - Documentation\n")

        # Generate ccgo-package.json with metadata
        if collected_platforms:
            self.generate_package_json(package_dir, project_name, version, collected_platforms)

        print(f"\n{'='*80}")
        print("Package Summary")
        print(f"{'='*80}\n")
        print(f"Package Name: {package_name}")
        print(f"Location: {package_dir}")
        if collected_platforms:
            print(f"Platforms: {', '.join(collected_platforms)}")
        else:
            print(f"Platforms: None (no build artifacts found)")

        # Create archive
        if args.format != "none":
            print(f"\n{'='*80}")
            print("Creating Archive")
            print(f"{'='*80}")

            archive_path = os.path.join(output_path, package_name)

            if args.format == "both":
                self.create_archive(package_dir, archive_path, "zip")
                self.create_archive(package_dir, archive_path, "tar.gz")
            else:
                self.create_archive(package_dir, archive_path, args.format)

        print(f"\n{'='*80}")
        if collected_platforms:
            print("✅ Packaging Complete!")
        else:
            print("⚠️  Package structure created (no platform artifacts)")
        print(f"{'='*80}\n")

        # Detailed platform summary (similar to ccgo build all)
        print(f"{'='*80}")
        print("Platform Summary")
        print(f"{'='*80}\n")

        # Separate native platforms from KMP
        native_success = [p for p in collected_platforms if p != "kmp"]
        native_failed = [p for p in failed_platforms if p != "kmp"]

        if native_success or native_failed:
            print("Native Platforms:")
            # Show successful platforms first
            for platform in native_success:
                print(f"  ✅ {platform.upper()}")
            # Show failed platforms
            for platform in native_failed:
                print(f"  ❌ {platform.upper()}")

        # Show KMP status
        kmp_success = "kmp" in collected_platforms
        kmp_failed = "kmp" in failed_platforms

        if kmp_success or kmp_failed:
            if native_success or native_failed:
                print()
            print("Kotlin Multiplatform:")
            if kmp_success:
                print(f"  ✅ KMP")
            elif kmp_failed:
                print(f"  ❌ KMP")

        # Summary
        total_platforms = len(collected_platforms) + len(failed_platforms)
        print(f"\nTotal: {len(collected_platforms)}/{total_platforms} platform(s) packaged")
        print(f"{'='*80}")

        if not collected_platforms:
            print("\nPackage directory structure has been created.")
            print("Build platforms first, then run 'ccgo package' again to include artifacts.")
        print()
