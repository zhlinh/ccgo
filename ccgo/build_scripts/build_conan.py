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
    exec_command,
    load_ccgo_config,
    CCGO_CMAKE_DIR,
)


def run_cmd(cmd_args: list, cwd: str = None) -> int:
    """Run a command and return exit code."""
    cmd = " ".join(cmd_args)
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
    except subprocess.TimeoutExpired:
        print("Warning: Conan profile check timed out")
    except Exception as e:
        print(f"Warning: Could not check/create Conan profile: {e}")


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

    # Add config_options - pass CCGO_CMAKE_DIR as a variable
    conanfile_content += f'''    def config_options(self):
        if self.settings.os == "Windows":
            del self.options.fPIC

    def configure(self):
        if self.options.shared:
            self.options.rm_safe("fPIC")

    def layout(self):
        cmake_layout(self)

    def generate(self):
        tc = CMakeToolchain(self)
        # Set CCGO_CMAKE_DIR for ccgo build system
        tc.variables["CCGO_CMAKE_DIR"] = "{CCGO_CMAKE_DIR}"
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

    # Change to project directory and load CCGO configuration
    original_dir = os.getcwd()
    os.chdir(project_dir)
    config = load_ccgo_config()
    os.chdir(original_dir)
    if not config:
        print("ERROR: Failed to load CCGO.toml configuration")
        sys.exit(1)

    # Generate conanfile.py if it doesn't exist
    conanfile_path = os.path.join(project_dir, 'conanfile.py')
    if not os.path.exists(conanfile_path):
        print("\nGenerating conanfile.py...")
        generate_conanfile(project_dir, config)
    else:
        print(f"Using existing conanfile.py at {conanfile_path}")

    # Determine build mode (default: create)
    build_mode = 'create'

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
        name = config.get('PROJECT_NAME_LOWER', 'unknown')
        version = config.get('CONFIG_PROJECT_VERSION', '1.0.0')

        print(f"Package: {name}/{version}")
        print("\nTo use this package in another project, add to conanfile.txt or conanfile.py:")
        print(f"  {name}/{version}")

        sys.exit(0)
    else:
        print("\n=== Conan Build Failed ===")
        sys.exit(1)


if __name__ == "__main__":
    main()