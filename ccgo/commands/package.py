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
- Documentation (optional)
- Sample code (optional)
- KMP artifacts (optional)

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

    # Package with KMP artifacts
    ccgo package --include-kmp

    # Package existing artifacts without rebuilding
    ccgo package --skip-build

    # Clean output directory before packaging
    ccgo package --clean --output ./release

OPTIONS:
    --version <version>        SDK version (default: auto-detect from git)
    --output <dir>             Output directory (default: ./sdk_package)
    --format <format>          Archive format: zip, tar.gz, both, none
    --platforms <list>         Comma-separated platforms to include
    --include-docs             Include documentation in package
    --include-samples          Include sample code in package
    --include-kmp              Include KMP artifacts in package
    --skip-build               Only package existing artifacts
    --clean                    Clean output directory first

OUTPUT STRUCTURE:
    <project>_SDK-<version>/
    ├── include/               Header files
    ├── lib/                   Platform libraries
    │   ├── android/          .so, .aar files
    │   ├── ios/              .a, .framework files
    │   ├── macos/            .a, .dylib, .framework files
    │   ├── windows/          .lib, .dll files
    │   ├── linux/            .a, .so files
    │   └── ohos/             .so, .har files
    ├── kmp/                   (optional) KMP artifacts
    ├── docs/                  (optional) Documentation
    └── README.md              Package information
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
            default="./sdk_package",
            help="Output directory for packaged SDK (default: ./sdk_package)",
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

        parser.add_argument(
            "--include-kmp",
            action="store_true",
            help="Include Kotlin Multiplatform artifacts",
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
        """Collect artifacts for a specific platform"""
        print(f"\n📦 Collecting {platform} artifacts...")

        # Create platform output directory
        platform_output = os.path.join(output_dir, "lib", platform)
        os.makedirs(platform_output, exist_ok=True)

        collected = False

        try:
            # Platform-specific collection logic
            if platform.lower() == "android":
                # Android: Extract .so files from AAR package in bin directory
                bin_dir = os.path.join(project_dir, "bin")
                if os.path.exists(bin_dir):
                    # Find AAR files
                    import zipfile
                    for aar_file in Path(bin_dir).glob("*ANDROID*.aar"):
                        print(f"   📂 Extracting from {aar_file.name}...")
                        with zipfile.ZipFile(aar_file, 'r') as zip_ref:
                            # Extract .so files for each architecture
                            for arch in ["arm64-v8a", "armeabi-v7a", "x86_64"]:
                                arch_output = os.path.join(platform_output, arch)
                                os.makedirs(arch_output, exist_ok=True)

                                # Look for .so files in jni/arch/
                                for zip_info in zip_ref.namelist():
                                    if zip_info.startswith(f"jni/{arch}/") and zip_info.endswith(".so"):
                                        # Extract only the project's .so file, not dependencies like libc++_shared.so
                                        so_name = os.path.basename(zip_info)
                                        if project_name.lower() in so_name.lower():
                                            zip_ref.extract(zip_info, "/tmp/ccgo_extract")
                                            src = os.path.join("/tmp/ccgo_extract", zip_info)
                                            dest = os.path.join(arch_output, so_name)
                                            shutil.copy2(src, dest)
                                            print(f"   ✓ {arch}/{so_name}")
                                            collected = True
                        # Clean up extraction directory
                        if os.path.exists("/tmp/ccgo_extract"):
                            shutil.rmtree("/tmp/ccgo_extract")
                        break  # Only process first AAR file

            elif platform.lower() == "ios":
                # iOS: Prefer xcframework, then framework, then .a files
                platform_build_dir = os.path.join(project_dir, "cmake_build", "iOS")
                darwin_out = os.path.join(platform_build_dir, "Darwin.out")

                if os.path.exists(darwin_out):
                    # Method 1: Look for xcframework (highest priority)
                    xcframeworks = [f for f in Path(darwin_out).glob("*.xcframework")
                                   if project_name.lower() in f.name.lower()]
                    if xcframeworks:
                        xcframework = xcframeworks[0]
                        dest = os.path.join(platform_output, xcframework.name)
                        if os.path.exists(dest):
                            shutil.rmtree(dest)
                        shutil.copytree(xcframework, dest, symlinks=True)
                        print(f"   ✓ {xcframework.name}")
                        collected = True

                    # Method 2: Look for framework
                    if not collected:
                        frameworks = [f for f in Path(darwin_out).glob("*.framework")
                                     if project_name.lower() in f.name.lower()]
                        if frameworks:
                            framework = frameworks[0]
                            dest = os.path.join(platform_output, framework.name)
                            if os.path.exists(dest):
                                shutil.rmtree(dest)
                            shutil.copytree(framework, dest, symlinks=True)
                            print(f"   ✓ {framework.name}")
                            collected = True

                    # Method 3: Fallback to .a files
                    if not collected:
                        a_files = list(Path(darwin_out).glob(f"lib{project_name}*.a"))
                        if a_files:
                            output_lib = os.path.join(platform_output, f"{project_name}.a")
                            if len(a_files) == 1:
                                shutil.copy2(a_files[0], output_lib)
                                print(f"   ✓ {project_name}.a")
                                collected = True
                            else:
                                # Merge multiple .a files using libtool
                                lib_paths = [str(f) for f in a_files]
                                cmd = f"libtool -static -o {output_lib} {' '.join(lib_paths)}"
                                result = subprocess.run(cmd, shell=True, capture_output=True)
                                if result.returncode == 0:
                                    print(f"   ✓ {project_name}.a (merged {len(a_files)} libraries)")
                                    collected = True

            elif platform.lower() == "macos":
                # macOS: Prefer framework, then xcframework, then .a files
                platform_build_dir = os.path.join(project_dir, "cmake_build", "macOS")
                darwin_out = os.path.join(platform_build_dir, "Darwin.out")

                if os.path.exists(darwin_out):
                    # Method 1: Look for framework (highest priority for macOS)
                    frameworks = [f for f in Path(darwin_out).glob("*.framework")
                                 if project_name.lower() in f.name.lower()]
                    if frameworks:
                        framework = frameworks[0]
                        dest = os.path.join(platform_output, framework.name)
                        if os.path.exists(dest):
                            shutil.rmtree(dest)
                        shutil.copytree(framework, dest, symlinks=True)
                        print(f"   ✓ {framework.name}")
                        collected = True

                    # Method 2: Look for xcframework
                    if not collected:
                        xcframeworks = [f for f in Path(darwin_out).glob("*.xcframework")
                                       if project_name.lower() in f.name.lower()]
                        if xcframeworks:
                            xcframework = xcframeworks[0]
                            dest = os.path.join(platform_output, xcframework.name)
                            if os.path.exists(dest):
                                shutil.rmtree(dest)
                            shutil.copytree(xcframework, dest, symlinks=True)
                            print(f"   ✓ {xcframework.name}")
                            collected = True

                    # Method 3: Fallback to .a files
                    if not collected:
                        a_files = list(Path(darwin_out).glob(f"lib{project_name}*.a"))
                        if a_files:
                            output_lib = os.path.join(platform_output, f"{project_name}.a")
                            if len(a_files) == 1:
                                shutil.copy2(a_files[0], output_lib)
                                print(f"   ✓ {project_name}.a")
                                collected = True
                            else:
                                # Merge using libtool
                                lib_paths = [str(f) for f in a_files]
                                cmd = f"libtool -static -o {output_lib} {' '.join(lib_paths)}"
                                result = subprocess.run(cmd, shell=True, capture_output=True)
                                if result.returncode == 0:
                                    print(f"   ✓ {project_name}.a (merged {len(a_files)} libraries)")
                                    collected = True

            elif platform.lower() == "windows":
                # Windows: Extract .lib from zip files in bin directory
                bin_dir = os.path.join(project_dir, "bin")
                if os.path.exists(bin_dir):
                    import zipfile
                    for zip_file in Path(bin_dir).glob("*WINDOWS*.zip"):
                        print(f"   📂 Extracting from {zip_file.name}...")
                        with zipfile.ZipFile(zip_file, 'r') as zip_ref:
                            x64_output = os.path.join(platform_output, "x64")
                            os.makedirs(x64_output, exist_ok=True)

                            for zip_info in zip_ref.namelist():
                                if zip_info.endswith(".lib") and project_name.lower() in zip_info.lower():
                                    lib_name = os.path.basename(zip_info)
                                    zip_ref.extract(zip_info, "/tmp/ccgo_extract")
                                    src = os.path.join("/tmp/ccgo_extract", zip_info)
                                    dest = os.path.join(x64_output, lib_name)
                                    shutil.copy2(src, dest)
                                    print(f"   ✓ x64/{lib_name}")
                                    collected = True
                        if os.path.exists("/tmp/ccgo_extract"):
                            shutil.rmtree("/tmp/ccgo_extract")
                        break

            elif platform.lower() == "linux":
                # Linux: Collect .a files from Darwin.out
                platform_build_dir = os.path.join(project_dir, "cmake_build", "Linux")
                darwin_out = os.path.join(platform_build_dir, "Darwin.out")

                if os.path.exists(darwin_out):
                    a_files = list(Path(darwin_out).glob(f"lib{project_name}*.a"))
                    if a_files:
                        output_lib = os.path.join(platform_output, f"{project_name}.a")
                        if len(a_files) == 1:
                            shutil.copy2(a_files[0], output_lib)
                            print(f"   ✓ {project_name}.a")
                            collected = True
                        else:
                            # Merge using ar
                            lib_paths = [str(f) for f in a_files]
                            cmd = f"ar -M <<EOF\nCREATE {output_lib}\n"
                            for lib in lib_paths:
                                cmd += f"ADDLIB {lib}\n"
                            cmd += f"SAVE\nEND\nEOF"
                            result = subprocess.run(cmd, shell=True, capture_output=True)
                            if result.returncode == 0:
                                print(f"   ✓ {project_name}.a (merged {len(a_files)} libraries)")
                                collected = True

            elif platform.lower() == "ohos":
                # OHOS: Extract .so files from HAR package in bin directory
                bin_dir = os.path.join(project_dir, "bin")
                if os.path.exists(bin_dir):
                    import zipfile
                    for har_file in Path(bin_dir).glob("*OHOS*.har"):
                        print(f"   📂 Extracting from {har_file.name}...")
                        with zipfile.ZipFile(har_file, 'r') as zip_ref:
                            for arch in ["arm64-v8a", "armeabi-v7a", "x86_64"]:
                                arch_output = os.path.join(platform_output, arch)
                                os.makedirs(arch_output, exist_ok=True)

                                # Look for .so files in package/libs/arch/
                                for zip_info in zip_ref.namelist():
                                    if f"package/libs/{arch}/" in zip_info and zip_info.endswith(".so"):
                                        so_name = os.path.basename(zip_info)
                                        if project_name.lower() in so_name.lower():
                                            zip_ref.extract(zip_info, "/tmp/ccgo_extract")
                                            src = os.path.join("/tmp/ccgo_extract", zip_info)
                                            dest = os.path.join(arch_output, so_name)
                                            shutil.copy2(src, dest)
                                            print(f"   ✓ {arch}/{so_name}")
                                            collected = True
                        if os.path.exists("/tmp/ccgo_extract"):
                            shutil.rmtree("/tmp/ccgo_extract")
                        break

            if not collected:
                print(f"   ⚠️  No artifacts found for {platform}")

        except Exception as e:
            print(f"   ⚠️  Error collecting {platform} artifacts: {e}")
            import traceback
            traceback.print_exc()

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
        """Collect KMP artifacts"""
        print(f"\n📦 Collecting KMP artifacts...")

        # Look for kmp build outputs
        try:
            for subdir in os.listdir(project_dir):
                subdir_path = os.path.join(project_dir, subdir)
                if not os.path.isdir(subdir_path):
                    continue
                kmp_dir = os.path.join(subdir_path, "kmp")
                if os.path.exists(kmp_dir):
                    kmp_build = os.path.join(kmp_dir, "build")
                    if os.path.exists(kmp_build):
                        # Copy AAR files
                        aar_dir = os.path.join(kmp_build, "outputs", "aar")
                        if os.path.exists(aar_dir):
                            kmp_output = os.path.join(output_dir, "kmp", "android")
                            os.makedirs(kmp_output, exist_ok=True)
                            for aar_file in glob.glob(os.path.join(aar_dir, "*.aar")):
                                shutil.copy2(aar_file, kmp_output)
                                print(f"   ✓ {os.path.basename(aar_file)}")

                        # Copy JAR files
                        jar_dir = os.path.join(kmp_build, "libs")
                        if os.path.exists(jar_dir):
                            kmp_output = os.path.join(output_dir, "kmp", "desktop")
                            os.makedirs(kmp_output, exist_ok=True)
                            for jar_file in glob.glob(os.path.join(jar_dir, "*.jar")):
                                shutil.copy2(jar_file, kmp_output)
                                print(f"   ✓ {os.path.basename(jar_file)}")

                        return True
        except (OSError, PermissionError):
            pass

        print(f"   ⚠️  No KMP artifacts found")
        return False

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
        package_name = f"{project_name.lower()}_SDK-{version}"
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
        platforms = ["android", "ios", "macos", "windows", "linux", "ohos"]
        if args.platforms:
            platforms = [p.strip() for p in args.platforms.split(",")]

        collected_platforms = []
        for platform in platforms:
            if self.collect_platform_artifacts(project_dir, platform, package_dir, project_name):
                collected_platforms.append(platform)

        # Collect KMP artifacts
        if args.include_kmp:
            self.collect_kmp_artifacts(project_dir, package_dir, project_name)

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
                    f.write(f"- {platform.capitalize()}\n")
            else:
                f.write(f"*No platform artifacts found. Build platforms first.*\n")
            f.write(f"\n## Structure\n\n")
            f.write(f"- `include/` - Header files\n")
            if collected_platforms:
                f.write(f"- `lib/<platform>/` - Platform-specific libraries\n")
            if args.include_kmp:
                f.write(f"- `kmp/` - Kotlin Multiplatform artifacts\n")
            if args.include_docs:
                f.write(f"- `docs/` - Documentation\n")

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
            print(f"{'='*80}\n")
            print(f"SDK package created with {len(collected_platforms)} platform(s)")
        else:
            print("✅ Package structure created (no platform artifacts)")
            print(f"{'='*80}\n")
            print("Package directory structure has been created.")
            print("Build platforms first, then run 'ccgo package' again to include artifacts.")
        print()
