#!/usr/bin/env python3
"""
Publish script for Conan C/C++ package manager.

This script publishes the C/C++ library to Conan local cache or remote repository.

Modes:
- local: Create package in local Conan cache (~/.conan2/)
- remote: Upload package to remote Conan repository

For local builds without publishing, use `ccgo build conan`.
"""

import os
import sys
import subprocess
from typing import Dict, Any, Optional

# Import build utilities
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from build_utils import (
    exec_command,
    load_ccgo_config,
    CCGO_CMAKE_DIR,
)

# Import conanfile generation from build_conan
from build_conan import (
    find_project_root,
    check_conan_installation,
    generate_conanfile,
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


def create_conan_package(project_dir: str, config: Dict[str, Any], profile: str = "default") -> bool:
    """
    Create a Conan package in the local cache.

    This runs `conan create .` which:
    1. Exports the recipe to local cache
    2. Builds the package
    3. Stores binary in local cache

    Args:
        project_dir: Root directory of the project
        config: CCGO configuration dictionary
        profile: Conan profile to use

    Returns:
        True if successful, False otherwise
    """
    print("\n=== Creating Conan Package (Local Cache) ===")
    print("This will build and install the package to your local Conan cache")
    print(f"Cache location: ~/.conan2/ (or run 'conan config home' to check)")

    # Build command
    cmd_args = [
        "conan", "create", ".",
        "--build", "missing"
    ]

    # Add profile if specified
    if profile != 'default':
        cmd_args.extend(["--profile", profile])

    # Execute conan create with CCGO_CMAKE_DIR environment variable
    print(f"\nExecuting: {' '.join(cmd_args)}")
    print(f"  with CCGO_CMAKE_DIR={CCGO_CMAKE_DIR}")
    err_code = run_cmd(cmd_args, cwd=project_dir, env={"CCGO_CMAKE_DIR": CCGO_CMAKE_DIR})

    if err_code != 0:
        print(f"ERROR: Conan package creation failed with exit code {err_code}")
        return False

    print("\nSuccessfully created Conan package in local cache")
    return True


def export_conan_package(project_dir: str) -> bool:
    """
    Export the Conan recipe to local cache without building.

    Args:
        project_dir: Root directory of the project

    Returns:
        True if successful, False otherwise
    """
    print("\n=== Exporting Conan Recipe ===")

    cmd_args = ["conan", "export", "."]

    print(f"Executing: {' '.join(cmd_args)}")
    err_code = run_cmd(cmd_args, cwd=project_dir)

    if err_code != 0:
        print(f"ERROR: Conan export failed with exit code {err_code}")
        return False

    print("Successfully exported Conan recipe to local cache")
    return True


def upload_conan_package(
    project_dir: str,
    config: Dict[str, Any],
    remote: str,
    confirm: bool = False
) -> bool:
    """
    Upload the Conan package to a remote repository.

    Args:
        project_dir: Root directory of the project
        config: CCGO configuration dictionary
        remote: Name of the remote repository
        confirm: Whether to skip confirmation prompt

    Returns:
        True if successful, False otherwise
    """
    name = config.get('PROJECT_NAME_LOWER', 'unknown')
    version = config.get('CONFIG_PROJECT_VERSION', '1.0.0')
    package_ref = f"{name}/{version}"

    print(f"\n=== Uploading Conan Package to Remote ===")
    print(f"Package: {package_ref}")
    print(f"Remote: {remote}")

    # Check if remote exists
    print("\nChecking remote configuration...")
    result = subprocess.run(
        ["conan", "remote", "list"],
        capture_output=True,
        text=True,
        check=False,
        timeout=10
    )

    if result.returncode != 0:
        print("ERROR: Failed to list Conan remotes")
        return False

    if remote not in result.stdout:
        print(f"ERROR: Remote '{remote}' not found")
        print("\nAvailable remotes:")
        print(result.stdout)
        print("\nTo add a remote, use:")
        print(f"  conan remote add {remote} <URL>")
        return False

    # Confirm upload
    if not confirm:
        print(f"\nThis will upload {package_ref} to remote '{remote}'")
        response = input("Continue? [y/N]: ").strip().lower()
        if response != 'y':
            print("Upload cancelled")
            return False

    # Upload command
    cmd_args = [
        "conan", "upload", package_ref,
        "-r", remote,
        "--confirm"
    ]

    print(f"\nExecuting: {' '.join(cmd_args)}")
    err_code = run_cmd(cmd_args, cwd=project_dir)

    if err_code != 0:
        print(f"ERROR: Conan upload failed with exit code {err_code}")
        return False

    print(f"\nSuccessfully uploaded {package_ref} to {remote}")
    return True


def list_remotes() -> None:
    """List configured Conan remotes."""
    print("\n=== Configured Conan Remotes ===")
    result = subprocess.run(
        ["conan", "remote", "list"],
        capture_output=True,
        text=True,
        check=False,
        timeout=10
    )
    if result.returncode == 0:
        print(result.stdout)
    else:
        print("No remotes configured or failed to list remotes")


def main():
    """Main entry point for Conan publish script."""
    import argparse

    parser = argparse.ArgumentParser(
        description="Publish C/C++ library to Conan cache or remote repository",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Publish Modes:
    local   Create package in local Conan cache (default)
    remote  Upload package to a remote Conan repository

Examples:
    # Publish to local Conan cache
    python publish_conan.py
    python publish_conan.py --mode local

    # Upload to remote repository
    python publish_conan.py --mode remote --remote conancenter

    # Export recipe only (no build)
    python publish_conan.py --export-only

    # List configured remotes
    python publish_conan.py --list-remotes

Note:
    For local builds without publishing, use: ccgo build conan
        """
    )
    parser.add_argument(
        "--mode",
        choices=["local", "remote"],
        default="local",
        help="Publish mode: local (cache) or remote (upload)"
    )
    parser.add_argument(
        "--remote",
        type=str,
        default="conancenter",
        help="Remote repository name for upload (default: conancenter)"
    )
    parser.add_argument(
        "--profile",
        type=str,
        default="default",
        help="Conan profile to use (default: default)"
    )
    parser.add_argument(
        "--export-only",
        action="store_true",
        help="Only export recipe without building"
    )
    parser.add_argument(
        "--list-remotes",
        action="store_true",
        help="List configured Conan remotes and exit"
    )
    parser.add_argument(
        "-y", "--yes",
        action="store_true",
        help="Skip confirmation prompts"
    )

    args = parser.parse_args()

    # Check Conan installation first
    if not check_conan_installation():
        print("ERROR: Conan is not installed or not in PATH")
        print("Please install Conan: pip install conan")
        sys.exit(1)

    # Handle --list-remotes
    if args.list_remotes:
        list_remotes()
        sys.exit(0)

    # Find project root
    project_dir = find_project_root()
    if not project_dir:
        print("ERROR: Could not find project root (no CCGO.toml found)")
        sys.exit(1)

    print(f"Project directory: {project_dir}")

    # Load CCGO configuration
    original_dir = os.getcwd()
    os.chdir(project_dir)
    config = load_ccgo_config()
    os.chdir(original_dir)
    if not config:
        print("ERROR: Failed to load CCGO.toml configuration")
        sys.exit(1)

    name = config.get('PROJECT_NAME_LOWER', 'unknown')
    version = config.get('CONFIG_PROJECT_VERSION', '1.0.0')

    # Generate conanfile.py
    print(f"\nGenerating conanfile.py with CCGO_CMAKE_DIR={CCGO_CMAKE_DIR}...")
    generate_conanfile(project_dir, config)

    # Handle export-only mode
    if args.export_only:
        success = export_conan_package(project_dir)
        if success:
            print("\n" + "=" * 60)
            print("Conan Export Complete")
            print("=" * 60)
            print(f"Package: {name}/{version}")
            print("\nRecipe exported to local cache. To build, run:")
            print(f"  conan install --requires={name}/{version} --build=missing")
        sys.exit(0 if success else 1)

    # Handle publish modes
    if args.mode == "local":
        success = create_conan_package(project_dir, config, args.profile)
        if success:
            print("\n" + "=" * 60)
            print("Conan Publish Complete (Local Cache)")
            print("=" * 60)
            print(f"Package: {name}/{version}")
            print("\nTo use this package in another project, add to conanfile.txt or conanfile.py:")
            print(f"  [requires]")
            print(f"  {name}/{version}")
            print("\nOr install directly:")
            print(f"  conan install --requires={name}/{version}")

    elif args.mode == "remote":
        # First create package locally if not exists
        print("Creating package locally before upload...")
        if not create_conan_package(project_dir, config, args.profile):
            print("ERROR: Failed to create local package. Cannot upload.")
            sys.exit(1)

        # Then upload
        success = upload_conan_package(project_dir, config, args.remote, args.yes)
        if success:
            print("\n" + "=" * 60)
            print("Conan Publish Complete (Remote)")
            print("=" * 60)
            print(f"Package: {name}/{version}")
            print(f"Remote: {args.remote}")

    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
