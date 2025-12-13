#!/usr/bin/env python3
"""
Build script for Conan C/C++ package manager - Local Build Mode.

This script builds the C/C++ library locally using Conan and outputs
artifacts to target/conan/ directory, following the unified output structure.

For publishing to Conan cache or remote repository, use `ccgo publish conan`.
"""

import os
import sys
import json
import subprocess
import shutil
import glob
from pathlib import Path
from typing import Dict, Any, Optional

# Import build utilities
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from build_utils import (
    exec_command,
    load_ccgo_config,
    CCGO_CMAKE_DIR,
    create_unified_archive,
    get_unified_lib_path,
    get_unified_include_path,
    get_archive_version_info,
    print_zip_tree,
    PROJECT_NAME,
    PROJECT_NAME_LOWER,
)


def run_cmd(cmd_args: list, cwd: str = None, env: dict = None) -> int:
    """Run a command and return exit code."""
    cmd = " ".join(cmd_args)

    # Prepend environment variable exports if provided
    if env:
        env_exports = " ".join([f'{k}="{v}"' for k, v in env.items()])
        cmd = f"{env_exports} {cmd}"

    if cwd:
        cmd = f"cd '{cwd}' && {cmd}"
    err_code, output = exec_command(cmd)
    if output:
        print(output)
    return err_code


def find_project_root() -> Optional[str]:
    """Find the project root directory containing CCGO.toml."""
    current = os.getcwd()
    while current != os.path.dirname(current):
        if os.path.isfile(os.path.join(current, "CCGO.toml")):
            return current
        current = os.path.dirname(current)
    # Check current directory
    if os.path.isfile(os.path.join(os.getcwd(), "CCGO.toml")):
        return os.getcwd()
    return None


def ensure_directory_exists(path: str) -> None:
    """Ensure a directory exists, creating it if necessary."""
    os.makedirs(path, exist_ok=True)


def check_conan_installation() -> bool:
    """Check if Conan is installed and available."""
    try:
        result = subprocess.run(
            ["conan", "--version"],
            capture_output=True,
            text=True,
            check=False,
            timeout=10  # 10 seconds timeout
        )
        if result.returncode == 0:
            print(f"Found Conan: {result.stdout.strip()}")
            # Check if default profile exists, create if not
            ensure_conan_profile()
            return True
        return False
    except FileNotFoundError:
        print("Conan not found. Please install Conan first:")
        print("  pip install conan")
        print("  or visit: https://conan.io/downloads")
        return False
    except subprocess.TimeoutExpired:
        print("Conan check timed out. Please verify Conan installation.")
        return False


def ensure_conan_profile() -> None:
    """Ensure Conan default profile exists."""
    try:
        # Check if default profile exists
        result = subprocess.run(
            ["conan", "profile", "show"],
            capture_output=True,
            text=True,
            check=False,
            timeout=10
        )
        if result.returncode != 0:
            # Profile doesn't exist, create it
            print("Creating Conan default profile...")
            subprocess.run(
                ["conan", "profile", "detect"],
                capture_output=True,
                text=True,
                check=False,
                timeout=30
            )
            print("Conan default profile created")

        # Update settings.yml to support newer compiler versions
        update_conan_settings()
    except subprocess.TimeoutExpired:
        print("Warning: Conan profile check timed out")
    except Exception as e:
        print(f"Warning: Could not check/create Conan profile: {e}")


def update_conan_settings() -> None:
    """
    Update Conan settings.yml to support newer compiler versions.

    Conan's default settings.yml may not include the latest compiler versions.
    This function adds missing versions for Apple Clang and Clang compilers.
    """
    import re

    try:
        # Find Conan home directory
        result = subprocess.run(
            ["conan", "config", "home"],
            capture_output=True,
            text=True,
            check=False,
            timeout=10
        )
        if result.returncode != 0:
            return

        conan_home = result.stdout.strip()
        settings_path = os.path.join(conan_home, "settings.yml")

        if not os.path.exists(settings_path):
            return

        # Read current settings
        with open(settings_path, 'r') as f:
            content = f.read()

        new_versions_to_add = ['17', '18', '19', '20']
        updated_compilers = []

        def add_versions_to_compiler(content: str, compiler_name: str, versions: list) -> str:
            """Add versions to a specific compiler section."""
            # Pattern to match compiler version list (handles both clang and apple-clang)
            # Format: compiler_name:\n    version: ["x", "y", ...]
            # Use DOTALL to match multi-line version lists
            pattern = rf'({re.escape(compiler_name)}:\s*\n\s*version:\s*\[)([\s\S]*?)(\])'

            def add_versions(match):
                prefix = match.group(1)
                existing_versions = match.group(2)
                suffix = match.group(3)

                modified = False
                for v in versions:
                    if f'"{v}"' not in existing_versions:
                        # Add to the end of the version list, before the closing bracket
                        existing_versions = existing_versions.rstrip() + f', "{v}"'
                        modified = True

                if modified:
                    updated_compilers.append(compiler_name)

                return prefix + existing_versions + suffix

            return re.sub(pattern, add_versions, content)

        # Update both clang and apple-clang versions
        new_content = add_versions_to_compiler(content, "clang", new_versions_to_add)
        new_content = add_versions_to_compiler(new_content, "apple-clang", new_versions_to_add)

        if new_content != content:
            with open(settings_path, 'w') as f:
                f.write(new_content)
            if updated_compilers:
                print(f"Updated Conan settings.yml with newer compiler versions ({', '.join(new_versions_to_add)}) for: {', '.join(updated_compilers)}")

    except Exception as e:
        print(f"Warning: Could not update Conan settings: {e}")


def generate_conanfile(project_dir: str, config: Dict[str, Any]) -> str:
    """
    Generate conanfile.py for the project.

    Args:
        project_dir: Root directory of the project
        config: CCGO configuration dictionary (from load_ccgo_config())

    Returns:
        Path to the generated conanfile.py
    """
    # Get project information from load_ccgo_config() format
    # config contains PROJECT_NAME, PROJECT_NAME_LOWER, CONFIG_PROJECT_VERSION, etc.
    name = config.get('PROJECT_NAME_LOWER', 'unknown')
    version = config.get('CONFIG_PROJECT_VERSION', '1.0.0')
    description = f"{name} library"
    author = ""
    license = "MIT"
    url = ""

    # Get Conan-specific settings (using defaults since CCGO.toml doesn't have conan section)
    settings = ['os', 'compiler', 'build_type', 'arch']
    options = {}
    default_options = {}
    requires = []
    build_requires = []

    # Generate conanfile.py content
    conanfile_content = f'''from conan import ConanFile
from conan.tools.cmake import CMake, CMakeToolchain, CMakeDeps, cmake_layout
from conan.tools.files import copy
import os


class {name.replace('-', '').replace('_', '').capitalize()}Conan(ConanFile):
    name = "{name}"
    version = "{version}"
    description = "{description}"
    author = "{author}"
    license = "{license}"
    url = "{url}"

    # Binary configuration
    settings = {settings}
    options = {{
        "shared": [True, False],
        "fPIC": [True, False],
'''

    # Add custom options
    for opt_name, opt_values in options.items():
        conanfile_content += f'        "{opt_name}": {opt_values},\n'

    conanfile_content += f'''    }}
    default_options = {{
        "shared": False,
        "fPIC": True,
'''

    # Add default option values
    for opt_name, opt_value in default_options.items():
        conanfile_content += f'        "{opt_name}": {opt_value},\n'

    conanfile_content += '    }\n\n'

    # Add exports and sources (cmake files are referenced from ccgo package, not copied)
    conanfile_content += '''    # Sources are located in the same place as this recipe, copy them to the recipe
    exports_sources = "CMakeLists.txt", "src/*", "include/*"

'''

    # Add requirements
    if requires:
        conanfile_content += '    def requirements(self):\n'
        for req in requires:
            conanfile_content += f'        self.requires("{req}")\n'
        conanfile_content += '\n'

    # Add build requirements
    if build_requires:
        conanfile_content += '    def build_requirements(self):\n'
        for req in build_requires:
            conanfile_content += f'        self.tool_requires("{req}")\n'
        conanfile_content += '\n'

    # Add config_options - dynamically find ccgo cmake directory at build time
    conanfile_content += '''    def config_options(self):
        if self.settings.os == "Windows":
            del self.options.fPIC

    def configure(self):
        if self.options.shared:
            self.options.rm_safe("fPIC")

    def layout(self):
        cmake_layout(self)

    def _get_ccgo_cmake_dir(self):
        """Get CCGO_CMAKE_DIR from ccgo package installation."""
        # First check environment variable
        cmake_dir = os.environ.get("CCGO_CMAKE_DIR")
        if cmake_dir and os.path.isdir(cmake_dir):
            return cmake_dir

        # Try to find from ccgo package installation
        try:
            import ccgo.build_scripts.build_utils as build_utils
            if hasattr(build_utils, "CCGO_CMAKE_DIR") and os.path.isdir(build_utils.CCGO_CMAKE_DIR):
                return build_utils.CCGO_CMAKE_DIR
        except ImportError:
            pass

        # Try to find ccgo package location
        try:
            import ccgo
            ccgo_path = os.path.dirname(ccgo.__file__)
            cmake_dir = os.path.join(ccgo_path, "build_scripts", "cmake")
            if os.path.isdir(cmake_dir):
                return cmake_dir
        except ImportError:
            pass

        raise RuntimeError(
            "CCGO_CMAKE_DIR not found. Please either:\\n"
            "1. Install ccgo package: pip install ccgo\\n"
            "2. Set CCGO_CMAKE_DIR environment variable"
        )

    def generate(self):
        tc = CMakeToolchain(self)
        # Dynamically get CCGO_CMAKE_DIR from ccgo package installation
        tc.variables["CCGO_CMAKE_DIR"] = self._get_ccgo_cmake_dir()
        # Pass shared option to CMake - CCGO uses CCGO_BUILD_STATIC and CCGO_BUILD_SHARED
        if self.options.shared:
            tc.variables["CCGO_BUILD_STATIC"] = "OFF"
            tc.variables["CCGO_BUILD_SHARED"] = "ON"
        else:
            tc.variables["CCGO_BUILD_STATIC"] = "ON"
            tc.variables["CCGO_BUILD_SHARED"] = "OFF"
        # Also set standard CMake variables for compatibility
        tc.variables["BUILD_SHARED_LIBS"] = "ON" if self.options.shared else "OFF"
        tc.variables["COMM_BUILD_SHARED_LIBS"] = "ON" if self.options.shared else "OFF"
        # Set submodule dependencies for shared library linking
        # Format: "module1,dep1,dep2;module2,dep1" means module1 depends on dep1 and dep2
        tc.variables["CONFIG_COMM_DEPS_MAP"] = self._detect_submodule_deps()
        tc.generate()

        deps = CMakeDeps(self)
        deps.generate()

    def _detect_submodule_deps(self):
        """
        Detect submodule dependencies by scanning source files for includes.

        Returns CMake list format: "module1;dep1,dep2;module2;dep3"
        where even indices are module names and odd indices are comma-separated deps.
        """
        import os
        import re

        src_dir = os.path.join(self.source_folder, "src")
        if not os.path.isdir(src_dir):
            return ""

        # Get list of submodules
        submodules = []
        for item in os.listdir(src_dir):
            subdir = os.path.join(src_dir, item)
            if os.path.isdir(subdir) and not item.startswith('.'):
                submodules.append(item)

        if not submodules:
            return ""

        # Scan each submodule for dependencies on other submodules
        deps_map = []
        include_pattern = re.compile(r'#include\s*[<"]' + self.name + r'/([^/"<>]+)/')

        for module in submodules:
            module_dir = os.path.join(src_dir, module)
            module_deps = set()

            # Scan all source files in this module
            for root, dirs, files in os.walk(module_dir):
                for filename in files:
                    if filename.endswith(('.c', '.cc', '.cpp', '.cxx', '.mm', '.m', '.h', '.hpp')):
                        filepath = os.path.join(root, filename)
                        try:
                            with open(filepath, 'r', encoding='utf-8', errors='ignore') as f:
                                content = f.read()
                                matches = include_pattern.findall(content)
                                for match in matches:
                                    if match != module and match in submodules:
                                        module_deps.add(match)
                        except Exception:
                            pass

            if module_deps:
                # CMake list format: "module;dep1,dep2"
                # Even index = module name, odd index = comma-separated dependencies
                deps_map.append(module)
                deps_map.append(",".join(sorted(module_deps)))

        # Format: "module1;dep1,dep2;module2;dep3"
        result = ";".join(deps_map)
        if result:
            self.output.info(f"Detected submodule dependencies: {result}")
        return result

    def build(self):
        cmake = CMake(self)
        cmake.configure()
        cmake.build()

    def package(self):
        cmake = CMake(self)
        cmake.install()

    def package_info(self):
        self.cpp_info.libs = [self.name]

        # Set the include directories
        self.cpp_info.includedirs = ["include"]

        # Set library directories
        self.cpp_info.libdirs = ["lib"]

        # Set binary directories (for shared libraries on Windows)
        if self.settings.os == "Windows" and self.options.shared:
            self.cpp_info.bindirs = ["bin"]
        else:
            self.cpp_info.bindirs = []
'''

    # Write conanfile.py
    conanfile_path = os.path.join(project_dir, 'conanfile.py')
    with open(conanfile_path, 'w') as f:
        f.write(conanfile_content)

    print(f"Generated conanfile.py at {conanfile_path}")
    return conanfile_path


def build_conan_locally(project_dir: str, config: Dict[str, Any], link_type: str = "both") -> bool:
    """
    Build the Conan package locally and output to target/conan/.

    Args:
        project_dir: Root directory of the project
        config: CCGO configuration dictionary
        link_type: Library type to build: 'static', 'shared', or 'both'

    Returns:
        True if successful, False otherwise
    """
    conan_config = config.get('build', {}).get('conan', {})

    # Get build settings
    profile = conan_config.get('profile', 'default')
    build_folder = "cmake_build/conan"

    # Clean and recreate build directory to ensure fresh build with correct options
    build_dir = os.path.join(project_dir, build_folder)
    if os.path.exists(build_dir):
        print(f"Cleaning old build directory: {build_dir}")
        shutil.rmtree(build_dir)
    ensure_directory_exists(build_dir)

    # Determine which variants to build
    build_static = link_type in ('static', 'both')
    build_shared = link_type in ('shared', 'both')

    success = True

    # Build static library
    if build_static:
        print("\n=== Building Conan Package (Static) ===")
        static_build_dir = os.path.join(build_dir, "static")
        ensure_directory_exists(static_build_dir)

        success = success and _build_variant(
            project_dir, static_build_dir, profile, shared=False
        )

    # Build shared library
    if build_shared:
        print("\n=== Building Conan Package (Shared) ===")
        shared_build_dir = os.path.join(build_dir, "shared")
        ensure_directory_exists(shared_build_dir)

        success = success and _build_variant(
            project_dir, shared_build_dir, profile, shared=True
        )

    if success:
        # Copy outputs to target/conan/
        success = _copy_to_target(project_dir, config, link_type)

    return success


def _build_variant(project_dir: str, build_dir: str, profile: str, shared: bool) -> bool:
    """Build a specific variant (static or shared)."""
    variant_name = "shared" if shared else "static"
    shared_option = "True" if shared else "False"

    # Install dependencies
    print(f"\nInstalling dependencies ({variant_name})...")
    install_cmd = [
        "conan", "install", ".",
        "--output-folder", build_dir,
        "--build", "missing",
        "-o", f"*:shared={shared_option}",  # Conan 2.x format: *:option=value
    ]

    if profile != 'default':
        install_cmd.extend(["--profile", profile])

    print(f"Executing: {' '.join(install_cmd)}")
    print(f"  with CCGO_CMAKE_DIR={CCGO_CMAKE_DIR}")
    err_code = run_cmd(install_cmd, cwd=project_dir, env={"CCGO_CMAKE_DIR": CCGO_CMAKE_DIR})

    if err_code != 0:
        print(f"ERROR: Dependency installation failed ({variant_name})")
        return False

    # Build the package
    print(f"\nBuilding package ({variant_name})...")
    build_cmd = [
        "conan", "build", ".",
        "--output-folder", build_dir,
        "-o", f"*:shared={shared_option}",  # Must pass option to build command too
    ]

    print(f"Executing: {' '.join(build_cmd)}")
    err_code = run_cmd(build_cmd, cwd=project_dir, env={"CCGO_CMAKE_DIR": CCGO_CMAKE_DIR})

    if err_code != 0:
        print(f"ERROR: Package build failed ({variant_name})")
        return False

    print(f"Successfully built {variant_name} library")
    return True


def _find_libraries_recursive(base_dir: str, extensions: list, project_name: str) -> list:
    """
    Recursively find library files in directory.

    Args:
        base_dir: Base directory to search
        extensions: List of file extensions to look for
        project_name: Project name to prioritize matching libraries

    Returns:
        List of (file_path, priority) tuples where priority indicates:
        - 2: Exact match (lib{name}.ext)
        - 1: Contains name but has suffix (lib{name}-xxx.ext)
        - 0: Does not match project name
    """
    results = []
    if not os.path.exists(base_dir):
        return results

    # Normalize project name for matching
    name_normalized = project_name.lower().replace('-', '').replace('_', '')

    for root, dirs, files in os.walk(base_dir):
        # Skip CMakeFiles and other build system directories
        dirs[:] = [d for d in dirs if d not in ['CMakeFiles', 'generators', '.cmake']]

        for filename in files:
            for ext in extensions:
                # Handle .so with version suffix (e.g., libfoo.so.1.0)
                if filename.endswith(ext) or (ext == '.so' and '.so' in filename):
                    filepath = os.path.join(root, filename)

                    # Extract library name from filename
                    # e.g., "libccgonow.dylib" -> "ccgonow"
                    # e.g., "libccgonow-api.a" -> "ccgonow-api"
                    lib_name = filename
                    if lib_name.startswith('lib'):
                        lib_name = lib_name[3:]
                    for e in ['.dylib', '.so', '.dll', '.a', '.lib']:
                        if lib_name.endswith(e):
                            lib_name = lib_name[:-len(e)]
                            break
                    # Handle version suffix like .so.1.0
                    if '.so.' in lib_name:
                        lib_name = lib_name.split('.so.')[0]

                    lib_normalized = lib_name.lower().replace('-', '').replace('_', '')

                    # Determine priority
                    if lib_normalized == name_normalized:
                        priority = 2  # Exact match
                    elif name_normalized in lib_normalized:
                        priority = 1  # Contains name (e.g., ccgonow-api)
                    else:
                        priority = 0  # No match

                    results.append((filepath, priority, filename))
                    break

    return results


def _copy_to_target(project_dir: str, config: Dict[str, Any], link_type: str) -> bool:
    """Copy build outputs to target/conan/ directory."""
    import platform as plat

    print("\n=== Copying outputs to target/conan/ ===")

    name = config.get('PROJECT_NAME_LOWER', 'unknown')
    version = config.get('CONFIG_PROJECT_VERSION', '1.0.0')

    # Target directory structure
    target_dir = os.path.join(project_dir, "target", "conan")
    lib_dir = os.path.join(target_dir, "lib")
    include_dir = os.path.join(target_dir, "include", name)

    # Clean and create target directories
    if os.path.exists(target_dir):
        shutil.rmtree(target_dir)

    build_static = link_type in ('static', 'both')
    build_shared = link_type in ('shared', 'both')

    if build_static:
        ensure_directory_exists(os.path.join(lib_dir, "static"))
    if build_shared:
        ensure_directory_exists(os.path.join(lib_dir, "shared"))
    ensure_directory_exists(include_dir)

    build_dir = os.path.join(project_dir, "cmake_build", "conan")

    # Determine library extensions based on platform
    system = plat.system()
    if system == "Windows":
        static_exts = [".lib"]
        shared_exts = [".dll"]
    elif system == "Darwin":
        static_exts = [".a"]
        shared_exts = [".dylib"]
    else:  # Linux and others
        static_exts = [".a"]
        shared_exts = [".so"]

    copied_static = 0
    copied_shared = 0

    # Copy static libraries - search recursively
    if build_static:
        static_build = os.path.join(build_dir, "static", "build", "Release")
        if not os.path.exists(static_build):
            static_build = os.path.join(build_dir, "static", "build", "Debug")
        if not os.path.exists(static_build):
            static_build = os.path.join(build_dir, "static", "build")

        if os.path.exists(static_build):
            libs = _find_libraries_recursive(static_build, static_exts, name)
            # Copy libraries matching project name (priority >= 1)
            # Priority 2 = exact match (libccgonow.a)
            # Priority 1 = contains name (libccgonow-api.a, libccgonow-base.a)
            matching_libs = [(f, p, n) for f, p, n in libs if p >= 1]

            if matching_libs:
                for lib_file, priority, filename in matching_libs:
                    dest = os.path.join(lib_dir, "static", filename)
                    shutil.copy2(lib_file, dest)
                    print(f"  Copied: {filename} -> lib/static/")
                    copied_static += 1
            else:
                print(f"  Warning: No static libraries matching '{name}' found")
                all_libs = [n for _, _, n in libs]
                if all_libs:
                    print(f"  Found libraries: {all_libs}")
                else:
                    print(f"  No .a files found in {static_build}")
        else:
            print(f"  Warning: Static build directory not found: {static_build}")

    # Copy shared libraries - search recursively
    if build_shared:
        shared_build = os.path.join(build_dir, "shared", "build", "Release")
        if not os.path.exists(shared_build):
            shared_build = os.path.join(build_dir, "shared", "build", "Debug")
        if not os.path.exists(shared_build):
            shared_build = os.path.join(build_dir, "shared", "build")

        if os.path.exists(shared_build):
            libs = _find_libraries_recursive(shared_build, shared_exts, name)
            # Copy libraries matching project name (priority >= 1)
            matching_libs = [(f, p, n) for f, p, n in libs if p >= 1]

            if matching_libs:
                for lib_file, priority, filename in matching_libs:
                    dest = os.path.join(lib_dir, "shared", filename)
                    shutil.copy2(lib_file, dest)
                    print(f"  Copied: {filename} -> lib/shared/")
                    copied_shared += 1
            else:
                print(f"  Warning: No shared libraries matching '{name}' found")
                all_libs = [n for _, _, n in libs]
                if all_libs:
                    print(f"  Found libraries: {all_libs}")
                else:
                    ext_str = '/'.join(shared_exts)
                    print(f"  No {ext_str} files found in {shared_build}")
        else:
            print(f"  Warning: Shared build directory not found: {shared_build}")

    # Copy include files from project's include directory (only header files)
    project_include = os.path.join(project_dir, "include")
    header_extensions = ('.h', '.hpp', '.hxx', '.h++', '.hh', '.inl', '.inc')

    def copy_headers_only(src_dir, dst_dir):
        """Recursively copy only header files."""
        if not os.path.exists(dst_dir):
            os.makedirs(dst_dir)
        for item in os.listdir(src_dir):
            src = os.path.join(src_dir, item)
            dst = os.path.join(dst_dir, item)
            if os.path.isdir(src):
                copy_headers_only(src, dst)
            elif item.lower().endswith(header_extensions):
                shutil.copy2(src, dst)

    if os.path.exists(project_include):
        copy_headers_only(project_include, include_dir)
        print(f"  Copied header files to include/{name}/")
    else:
        print(f"  Warning: Include directory not found: {project_include}")

    # Generate build_info.json
    build_info = {
        "name": name,
        "version": version,
        "platform": "conan",
        "link_type": link_type,
        "build_system": "conan",
        "conan_version": _get_conan_version(),
    }

    build_info_path = os.path.join(target_dir, "build_info.json")
    with open(build_info_path, 'w') as f:
        json.dump(build_info, f, indent=2)
    print(f"  Generated build_info.json")

    print(f"\nBuild artifacts available at: {target_dir}")
    return True


def print_directory_tree(directory: str, prefix: str = "", max_depth: int = 4, current_depth: int = 0) -> None:
    """
    Print directory tree structure.

    Args:
        directory: Root directory to print
        prefix: Prefix for indentation
        max_depth: Maximum depth to recurse
        current_depth: Current recursion depth
    """
    if current_depth >= max_depth:
        return

    if not os.path.exists(directory):
        return

    items = sorted(os.listdir(directory))
    dirs = [item for item in items if os.path.isdir(os.path.join(directory, item))]
    files = [item for item in items if os.path.isfile(os.path.join(directory, item))]

    # Print files first
    for i, filename in enumerate(files):
        filepath = os.path.join(directory, filename)
        size = os.path.getsize(filepath)
        size_str = _format_size(size)
        is_last = (i == len(files) - 1) and (len(dirs) == 0)
        connector = "└── " if is_last else "├── "
        print(f"{prefix}{connector}{filename} ({size_str})")

    # Print directories
    for i, dirname in enumerate(dirs):
        dirpath = os.path.join(directory, dirname)
        is_last = (i == len(dirs) - 1)
        connector = "└── " if is_last else "├── "
        print(f"{prefix}{connector}{dirname}/")
        new_prefix = prefix + ("    " if is_last else "│   ")
        print_directory_tree(dirpath, new_prefix, max_depth, current_depth + 1)


def _format_size(size: int) -> str:
    """Format file size in human-readable format."""
    if size < 1024:
        return f"{size} B"
    elif size < 1024 * 1024:
        return f"{size / 1024:.1f} KB"
    else:
        return f"{size / (1024 * 1024):.2f} MB"


def archive_conan_project(project_dir: str, config: Dict[str, Any], link_type: str = "both") -> bool:
    """
    Create a unified ZIP archive for the Conan build output.

    This function creates an archive package following the same pattern as other platforms:
    - {PROJECT_NAME}_CONAN_SDK-{version}.zip
      - lib/conan/static/lib{project}.a  (if link_type is static or both)
      - lib/conan/shared/lib{project}.so (if link_type is shared or both)
      - include/{project}/
      - build_info.json

    Args:
        project_dir: Root directory of the project
        config: CCGO configuration dictionary
        link_type: Library link type ('static', 'shared', or 'both')

    Returns:
        True if successful, False otherwise
    """
    import platform as plat

    print("\n" + "=" * 60)
    print("Creating Unified ZIP Archive")
    print("=" * 60 + "\n")

    name = config.get('PROJECT_NAME_LOWER', PROJECT_NAME_LOWER or 'unknown')
    project_upper = config.get('PROJECT_NAME', PROJECT_NAME or 'UNKNOWN').upper()

    # Get version info using unified function
    _, _, full_version = get_archive_version_info(project_dir)

    # Define paths
    target_dir = os.path.join(project_dir, "target")
    conan_dir = os.path.join(target_dir, "conan")
    lib_dir = os.path.join(conan_dir, "lib")

    # Determine library extensions based on platform
    system = plat.system()
    if system == "Windows":
        static_ext = ".lib"
        shared_ext = ".dll"
    elif system == "Darwin":
        static_ext = ".a"
        shared_ext = ".dylib"
    else:  # Linux and others
        static_ext = ".a"
        shared_ext = ".so"

    # Prepare static libraries mapping
    static_libs = {}
    if link_type in ('static', 'both'):
        static_lib_dir = os.path.join(lib_dir, "static")
        if os.path.exists(static_lib_dir):
            for filename in os.listdir(static_lib_dir):
                if filename.endswith(static_ext):
                    src_path = os.path.join(static_lib_dir, filename)
                    arc_path = get_unified_lib_path("static", lib_name=filename, platform="conan")
                    static_libs[arc_path] = src_path

    # Prepare shared libraries mapping
    shared_libs = {}
    if link_type in ('shared', 'both'):
        shared_lib_dir = os.path.join(lib_dir, "shared")
        if os.path.exists(shared_lib_dir):
            for filename in os.listdir(shared_lib_dir):
                if filename.endswith(shared_ext) or (shared_ext == '.so' and '.so' in filename):
                    src_path = os.path.join(shared_lib_dir, filename)
                    arc_path = get_unified_lib_path("shared", lib_name=filename, platform="conan")
                    shared_libs[arc_path] = src_path

    # Prepare include directories mapping
    include_dirs = {}
    headers_src = os.path.join(conan_dir, "include", name)
    if os.path.exists(headers_src):
        arc_path = get_unified_include_path(name, headers_src)
        include_dirs[arc_path] = headers_src

    # Check if we have any artifacts to archive
    if not static_libs and not shared_libs:
        print("WARNING: No libraries found to archive")
        return False

    # Create unified archive package
    main_zip_path, _ = create_unified_archive(
        output_dir=target_dir,
        project_name=project_upper,
        platform_name="CONAN",
        version=full_version,
        link_type=link_type,
        static_libs=static_libs,
        shared_libs=shared_libs,
        include_dirs=include_dirs,
        extra_info={
            "conan_version": _get_conan_version(),
            "host_platform": system.lower(),
        }
    )

    if main_zip_path and os.path.exists(main_zip_path):
        # Move ZIP to target/conan/ and clean up intermediate files
        final_zip_path = os.path.join(conan_dir, os.path.basename(main_zip_path))
        if main_zip_path != final_zip_path:
            shutil.move(main_zip_path, final_zip_path)

        # Clean up intermediate files, keep only the ZIP
        for item in os.listdir(conan_dir):
            item_path = os.path.join(conan_dir, item)
            if item_path != final_zip_path:
                if os.path.isdir(item_path):
                    shutil.rmtree(item_path)
                else:
                    os.remove(item_path)

        # Extract build_info.json from ZIP (now in meta/conan/) to target/conan/
        import zipfile
        with zipfile.ZipFile(final_zip_path, 'r') as zf:
            if 'meta/conan/build_info.json' in zf.namelist():
                # Extract to temp location and move to target directory
                zf.extract('meta/conan/build_info.json', conan_dir)
                # Move from meta/conan/ subdirectory to conan_dir root
                meta_conan_dir = os.path.join(conan_dir, 'meta', 'conan')
                meta_dir = os.path.join(conan_dir, 'meta')
                if os.path.exists(os.path.join(meta_conan_dir, 'build_info.json')):
                    shutil.move(
                        os.path.join(meta_conan_dir, 'build_info.json'),
                        os.path.join(conan_dir, 'build_info.json')
                    )
                    # Clean up empty meta directories
                    if os.path.exists(meta_conan_dir) and not os.listdir(meta_conan_dir):
                        os.rmdir(meta_conan_dir)
                    if os.path.exists(meta_dir) and not os.listdir(meta_dir):
                        os.rmdir(meta_dir)
                print(f"  Extracted build_info.json to target/conan/")

        size_mb = os.path.getsize(final_zip_path) / (1024 * 1024)
        zip_name = os.path.basename(final_zip_path)
        print(f"\n" + "=" * 60)
        print(f"Build artifacts in target/conan/:")
        print("-" * 60)
        print(f"  {zip_name} ({size_mb:.2f} MB)")

        # Print ZIP tree structure and generate archive_info.json to target directory
        if print_zip_tree:
            print_zip_tree(final_zip_path, indent="    ", generate_info_file=True)

        print("-" * 60)
        return True

    return False


def _get_conan_version() -> str:
    """Get the installed Conan version."""
    try:
        result = subprocess.run(
            ["conan", "--version"],
            capture_output=True,
            text=True,
            check=False,
            timeout=10
        )
        if result.returncode == 0:
            # Parse "Conan version X.Y.Z"
            version_str = result.stdout.strip()
            if "version" in version_str.lower():
                return version_str.split()[-1]
            return version_str
    except Exception:
        pass
    return "unknown"


def main():
    """Main entry point for Conan build script."""
    import argparse

    parser = argparse.ArgumentParser(
        description="Build C/C++ library locally using Conan",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
    python build_conan.py                    # Build both static and shared
    python build_conan.py --link-type static # Build static only
    python build_conan.py --link-type shared # Build shared only

Output:
    Build artifacts are placed in target/conan/:
    - lib/static/    Static libraries
    - lib/shared/    Shared libraries
    - include/       Header files
    - build_info.json

Note:
    To publish to Conan cache or remote, use: ccgo publish conan
        """
    )
    parser.add_argument(
        "--link-type",
        choices=["static", "shared", "both"],
        default="both",
        help="Library type to build (default: both)"
    )

    args = parser.parse_args()

    # Find project root
    project_dir = find_project_root()
    if not project_dir:
        print("ERROR: Could not find project root (no CCGO.toml found)")
        sys.exit(1)

    print(f"Project directory: {project_dir}")

    # Check Conan installation
    if not check_conan_installation():
        print("ERROR: Conan is not installed or not in PATH")
        print("Please install Conan: pip install conan")
        sys.exit(1)

    # Change to project directory and load CCGO configuration
    original_dir = os.getcwd()
    os.chdir(project_dir)
    config = load_ccgo_config()
    os.chdir(original_dir)
    if not config:
        print("ERROR: Failed to load CCGO.toml configuration")
        sys.exit(1)

    # Generate conanfile.py (always regenerate to ensure correct CCGO_CMAKE_DIR path)
    print(f"\nGenerating conanfile.py with CCGO_CMAKE_DIR={CCGO_CMAKE_DIR}...")
    generate_conanfile(project_dir, config)

    # Build locally
    success = build_conan_locally(project_dir, config, args.link_type)

    if success:
        name = config.get('PROJECT_NAME_LOWER', 'unknown')
        version = config.get('CONFIG_PROJECT_VERSION', '1.0.0')

        print("\n" + "=" * 60)
        print("Conan Local Build Complete")
        print("=" * 60)
        print(f"Package: {name}/{version}")

        # Create unified ZIP archive (like other platforms)
        # This also cleans up intermediate files, keeping only the ZIP
        archive_conan_project(project_dir, config, args.link_type)

        print("\nTo publish to Conan cache, run: ccgo publish conan")
        sys.exit(0)
    else:
        print("\n=== Conan Build Failed ===")
        sys.exit(1)


if __name__ == "__main__":
    main()
