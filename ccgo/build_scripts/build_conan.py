#!/usr/bin/env python3
"""
Build script for Conan C/C++ package manager.

This script creates a Conan package for the C/C++ library, allowing it to be
consumed by other projects using Conan for dependency management.
"""

import os
import sys
import json
import subprocess
import shutil
from pathlib import Path
from typing import Dict, Any, Optional

# Import build utilities
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from build_utils import (
    run_cmd,
    load_ccgo_config,
    get_project_version,
    find_project_root,
    ensure_directory_exists,
)


def check_conan_installation() -> bool:
    """Check if Conan is installed and available."""
    try:
        result = subprocess.run(
            ["conan", "--version"],
            capture_output=True,
            text=True,
            check=False
        )
        if result.returncode == 0:
            print(f"Found Conan: {result.stdout.strip()}")
            return True
        return False
    except FileNotFoundError:
        return False


def generate_conanfile(project_dir: str, config: Dict[str, Any]) -> str:
    """
    Generate conanfile.py for the project.

    Args:
        project_dir: Root directory of the project
        config: CCGO configuration dictionary

    Returns:
        Path to the generated conanfile.py
    """
    project_config = config.get('project', {})
    conan_config = config.get('build', {}).get('conan', {})

    # Get project information
    name = conan_config.get('name', project_config.get('name', 'unknown'))
    version = conan_config.get('version', project_config.get('version', '1.0.0'))
    description = conan_config.get('description', project_config.get('description', ''))
    author = conan_config.get('author', project_config.get('author', ''))
    license = conan_config.get('license', project_config.get('license', 'MIT'))
    url = conan_config.get('url', project_config.get('url', ''))

    # Get Conan-specific settings
    settings = conan_config.get('settings', ['os', 'compiler', 'build_type', 'arch'])
    options = conan_config.get('options', {})
    default_options = conan_config.get('default_options', {})
    requires = conan_config.get('requires', [])
    build_requires = conan_config.get('build_requires', [])

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

    # Add exports and sources
    conanfile_content += '''    # Sources are located in the same place as this recipe, copy them to the recipe
    exports_sources = "CMakeLists.txt", "src/*", "include/*", "cmake/*"

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

    # Add config_options
    conanfile_content += '''    def config_options(self):
        if self.settings.os == "Windows":
            del self.options.fPIC

    def configure(self):
        if self.options.shared:
            self.options.rm_safe("fPIC")

    def layout(self):
        cmake_layout(self)

    def generate(self):
        tc = CMakeToolchain(self)
        tc.generate()

        deps = CMakeDeps(self)
        deps.generate()

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


def create_conan_package(project_dir: str, config: Dict[str, Any]) -> bool:
    """
    Create a Conan package for the project.

    Args:
        project_dir: Root directory of the project
        config: CCGO configuration dictionary

    Returns:
        True if successful, False otherwise
    """
    conan_config = config.get('build', {}).get('conan', {})

    # Get build settings
    profile = conan_config.get('profile', 'default')
    build_folder = conan_config.get('build_folder', 'cmake_build/conan')

    # Ensure build directory exists
    build_dir = os.path.join(project_dir, build_folder)
    ensure_directory_exists(build_dir)

    # Create the package
    print("\n=== Creating Conan Package ===")

    # Build command
    cmd_args = [
        "conan", "create", ".",
        "--build", "missing"
    ]

    # Add profile if specified
    if profile != 'default':
        cmd_args.extend(["--profile", profile])

    # Execute conan create
    print(f"Executing: {' '.join(cmd_args)}")
    err_code = run_cmd(cmd_args, cwd=project_dir)

    if err_code != 0:
        print(f"ERROR: Conan package creation failed with exit code {err_code}")
        return False

    print("✓ Successfully created Conan package")
    return True


def export_conan_package(project_dir: str, config: Dict[str, Any]) -> bool:
    """
    Export the Conan package to local cache without building.

    Args:
        project_dir: Root directory of the project
        config: CCGO configuration dictionary

    Returns:
        True if successful, False otherwise
    """
    print("\n=== Exporting Conan Package ===")

    # Export command
    cmd_args = ["conan", "export", "."]

    # Execute conan export
    print(f"Executing: {' '.join(cmd_args)}")
    err_code = run_cmd(cmd_args, cwd=project_dir)

    if err_code != 0:
        print(f"ERROR: Conan package export failed with exit code {err_code}")
        return False

    print("✓ Successfully exported Conan package to local cache")
    return True


def build_conan_package_locally(project_dir: str, config: Dict[str, Any]) -> bool:
    """
    Build the Conan package locally for testing.

    Args:
        project_dir: Root directory of the project
        config: CCGO configuration dictionary

    Returns:
        True if successful, False otherwise
    """
    conan_config = config.get('build', {}).get('conan', {})

    # Get build settings
    profile = conan_config.get('profile', 'default')
    build_folder = conan_config.get('build_folder', 'cmake_build/conan')

    # Ensure build directory exists
    build_dir = os.path.join(project_dir, build_folder)
    ensure_directory_exists(build_dir)

    print("\n=== Building Conan Package Locally ===")

    # Install dependencies
    print("Installing dependencies...")
    install_cmd = [
        "conan", "install", ".",
        "--output-folder", build_folder,
        "--build", "missing"
    ]

    if profile != 'default':
        install_cmd.extend(["--profile", profile])

    print(f"Executing: {' '.join(install_cmd)}")
    err_code = run_cmd(install_cmd, cwd=project_dir)

    if err_code != 0:
        print(f"ERROR: Dependency installation failed with exit code {err_code}")
        return False

    # Build the package
    print("\nBuilding package...")
    build_cmd = [
        "conan", "build", ".",
        "--output-folder", build_folder
    ]

    print(f"Executing: {' '.join(build_cmd)}")
    err_code = run_cmd(build_cmd, cwd=project_dir)

    if err_code != 0:
        print(f"ERROR: Package build failed with exit code {err_code}")
        return False

    print("✓ Successfully built Conan package locally")
    return True


def main():
    """Main entry point for Conan build script."""
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

    # Load CCGO configuration
    config = load_ccgo_config(project_dir)
    if not config:
        print("ERROR: Failed to load CCGO.toml configuration")
        sys.exit(1)

    # Check if Conan configuration exists
    conan_config = config.get('build', {}).get('conan', {})
    if not conan_config:
        print("WARNING: No Conan configuration found in CCGO.toml")
        print("Using default settings...")

    # Generate conanfile.py if it doesn't exist
    conanfile_path = os.path.join(project_dir, 'conanfile.py')
    if not os.path.exists(conanfile_path):
        print("\nGenerating conanfile.py...")
        generate_conanfile(project_dir, config)
    else:
        print(f"Using existing conanfile.py at {conanfile_path}")

    # Determine build mode
    build_mode = conan_config.get('mode', 'create')

    if build_mode == 'create':
        # Create full package (default)
        success = create_conan_package(project_dir, config)
    elif build_mode == 'export':
        # Export only (no build)
        success = export_conan_package(project_dir, config)
    elif build_mode == 'build':
        # Build locally for testing
        success = build_conan_package_locally(project_dir, config)
    else:
        print(f"ERROR: Unknown build mode '{build_mode}'")
        print("Valid modes: create, export, build")
        sys.exit(1)

    if success:
        print("\n=== Conan Build Complete ===")

        # Print package information
        project_config = config.get('project', {})
        name = conan_config.get('name', project_config.get('name', 'unknown'))
        version = conan_config.get('version', project_config.get('version', '1.0.0'))

        print(f"Package: {name}/{version}")
        print("\nTo use this package in another project, add to conanfile.txt or conanfile.py:")
        print(f"  {name}/{version}")

        sys.exit(0)
    else:
        print("\n=== Conan Build Failed ===")
        sys.exit(1)


if __name__ == "__main__":
    main()