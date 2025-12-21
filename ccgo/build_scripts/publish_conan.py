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
    get_conanfile_path,
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


def create_conan_package(project_dir: str, config: Dict[str, Any], profile: str = "default", conan_config=None, link_type: str = "both", no_remote: bool = False) -> bool:
    """
    Create a Conan package in the local cache.

    This runs `conan create <conanfile_path>` which:
    1. Exports the recipe to local cache
    2. Builds the package
    3. Stores binary in local cache

    Args:
        project_dir: Root directory of the project
        config: CCGO configuration dictionary
        profile: Conan profile to use
        conan_config: Optional ConanConfig instance for user/channel
        link_type: Library type to build: 'static', 'shared', or 'both' (default: 'both')
        no_remote: If True, don't check remote repositories (default: False)

    Returns:
        True if successful, False otherwise
    """
    print("\n=== Creating Conan Package (Local Cache) ===")
    print("This will build and install the package to your local Conan cache")
    print(f"Cache location: ~/.conan2/ (or run 'conan config home' to check)")
    print(f"Link type: {link_type}")
    if no_remote:
        print("Remote check: disabled (local-only mode)")

    # Get conanfile.py path
    conanfile_path = get_conanfile_path(project_dir, config)

    # Determine which variants to build
    build_static = link_type in ('static', 'both')
    build_shared = link_type in ('shared', 'both')

    success = True

    # Build static library
    if build_static:
        print("\n--- Building Static Library ---")
        success = success and _create_conan_variant(
            project_dir, conanfile_path, profile, conan_config, shared=False, no_remote=no_remote
        )

    # Build shared library
    if build_shared:
        print("\n--- Building Shared Library ---")
        success = success and _create_conan_variant(
            project_dir, conanfile_path, profile, conan_config, shared=True, no_remote=no_remote
        )

    if success:
        print("\nSuccessfully created Conan package(s) in local cache")
    return success


def _create_conan_variant(project_dir: str, conanfile_path: str, profile: str, conan_config, shared: bool, no_remote: bool = False) -> bool:
    """Create a single Conan package variant (static or shared)."""
    variant_name = "shared" if shared else "static"
    shared_option = "True" if shared else "False"

    # Build command
    cmd_args = [
        "conan", "create", conanfile_path,
        "--build", "missing",
        "-o", f"*:shared={shared_option}",  # Conan 2.x format
    ]

    # Add --no-remote flag to prevent checking remote repositories (local-only mode)
    if no_remote:
        cmd_args.append("--no-remote")

    # Add user and channel if available (required for Conan 2.x)
    if conan_config:
        if conan_config.user:
            cmd_args.extend(["--user", conan_config.user])
            cmd_args.extend(["--channel", conan_config.channel])
        else:
            print(f"Note: No user/channel configured, package will be {conan_config.package_name}/{conan_config.version}")
    else:
        print("Warning: ConanConfig not available, cannot add user/channel")

    # Add profile if specified
    if profile != 'default':
        cmd_args.extend(["--profile", profile])

    # Execute conan create with CCGO_CMAKE_DIR environment variable
    print(f"\nExecuting ({variant_name}): {' '.join(cmd_args)}")
    print(f"  with CCGO_CMAKE_DIR={CCGO_CMAKE_DIR}")
    err_code = run_cmd(cmd_args, cwd=project_dir, env={"CCGO_CMAKE_DIR": CCGO_CMAKE_DIR})

    if err_code != 0:
        print(f"ERROR: Conan package creation ({variant_name}) failed with exit code {err_code}")
        return False

    print(f"Successfully created {variant_name} package")
    return True


def export_conan_package(project_dir: str, config: Dict[str, Any]) -> bool:
    """
    Export the Conan recipe to local cache without building.

    Args:
        project_dir: Root directory of the project
        config: CCGO configuration dictionary

    Returns:
        True if successful, False otherwise
    """
    print("\n=== Exporting Conan Recipe ===")

    # Get conanfile.py path
    conanfile_path = get_conanfile_path(project_dir, config)

    cmd_args = ["conan", "export", conanfile_path]

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
    conan_config=None
) -> bool:
    """
    Upload the Conan package to a remote repository.

    Args:
        project_dir: Root directory of the project
        config: CCGO configuration dictionary
        remote: Name of the remote repository
        conan_config: Optional ConanConfig instance for user/channel

    Returns:
        True if successful, False otherwise
    """
    # Use ConanConfig for package reference if available
    if conan_config:
        package_ref = conan_config.get_package_reference(include_user_channel=True)
        name = conan_config.package_name
        version = conan_config.version
    else:
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

    # Show upload info
    print(f"\nUploading {package_ref} to remote '{remote}'...")

    # Upload command
    # Note: Conan 2.x uses -c instead of --confirm
    cmd_args = [
        "conan", "upload", package_ref,
        "-r", remote,
        "-c"
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

    # Upload to remote repository (requires configured remote)
    python publish_conan.py --mode remote --remote myartifactory

    # Export recipe only (no build)
    python publish_conan.py --export-only

    # List configured remotes
    python publish_conan.py --list-remotes

    # Add a remote repository first:
    conan remote add myartifactory https://mycompany.jfrog.io/artifactory/api/conan/conan-local

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
        default=None,
        help="Remote repository name for upload (required for remote mode)"
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
        "--link-type",
        choices=["static", "shared", "both"],
        default="both",
        help="Library type to build: static, shared, or both (default: both)"
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

    # Load ConanConfig for advanced configuration
    conan_config = None
    try:
        # Try importing from installed package first, then fallback to relative import
        try:
            from ccgo.utils.conan.config import load_conan_config
        except ImportError:
            # Add parent directory to path for relative import
            parent_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
            if parent_dir not in sys.path:
                sys.path.insert(0, parent_dir)
            from utils.conan.config import load_conan_config

        toml_path = os.path.join(project_dir, "CCGO.toml")
        if os.path.exists(toml_path):
            try:
                import tomllib
            except ImportError:
                import tomli as tomllib
            with open(toml_path, 'rb') as f:
                toml_data = tomllib.load(f)
            conan_config = load_conan_config(toml_data)
            print(f"\nConan Configuration:")
            print(conan_config.get_config_summary())
            # Debug: print user/channel info
            if conan_config.user:
                print(f"  Will use: {conan_config.package_name}/{conan_config.version}@{conan_config.user}/{conan_config.channel}")
            else:
                print(f"  Will use: {conan_config.package_name}/{conan_config.version} (no user/channel)")
    except Exception as e:
        import traceback
        print(f"Warning: Could not load ConanConfig: {e}")
        traceback.print_exc()

    # Get package info from ConanConfig or fallback to basic config
    if conan_config:
        name = conan_config.package_name
        version = conan_config.version
        package_ref = conan_config.get_package_reference(include_user_channel=True)
    else:
        name = config.get('PROJECT_NAME_LOWER', 'unknown')
        version = config.get('CONFIG_PROJECT_VERSION', '1.0.0')
        package_ref = f"{name}/{version}"

    # Generate conanfile.py
    print(f"\nGenerating conanfile.py with CCGO_CMAKE_DIR={CCGO_CMAKE_DIR}...")
    generate_conanfile(project_dir, config, conan_config)

    # Handle export-only mode
    if args.export_only:
        success = export_conan_package(project_dir, config)
        if success:
            print("\n" + "=" * 60)
            print("Conan Export Complete")
            print("=" * 60)
            print(f"Package: {package_ref}")
            print("\nRecipe exported to local cache. To build, run:")
            print(f"  conan install --requires={package_ref} --build=missing")
        sys.exit(0 if success else 1)

    # Handle publish modes
    if args.mode == "local":
        # Local mode: don't check remote repositories
        success = create_conan_package(project_dir, config, args.profile, conan_config, args.link_type, no_remote=True)
        if success:
            print("\n" + "=" * 60)
            print("Conan Publish Complete (Local Cache)")
            print("=" * 60)
            print(f"Package: {package_ref}")
            print("\nTo use this package in another project, add to conanfile.txt or conanfile.py:")
            print(f"  [requires]")
            print(f"  {package_ref}")
            print("\nOr install directly:")
            print(f"  conan install --requires={package_ref}")

    elif args.mode == "remote":
        # Require --remote for remote mode
        if not args.remote:
            print("ERROR: --remote is required for remote mode")
            print("\nTo add a remote repository:")
            print("  conan remote add <name> <url>")
            print("\nExample:")
            print("  conan remote add myartifactory https://mycompany.jfrog.io/artifactory/api/conan/conan-local")
            print("  ccgo publish conan --remote myartifactory")
            list_remotes()
            sys.exit(1)

        # First create package locally if not exists (remote mode: allow remote checking)
        print("Creating package locally before upload...")
        if not create_conan_package(project_dir, config, args.profile, conan_config, args.link_type, no_remote=False):
            print("ERROR: Failed to create local package. Cannot upload.")
            sys.exit(1)

        # Then upload with ConanConfig for user/channel
        success = upload_conan_package(project_dir, config, args.remote, conan_config)
        if success:
            print("\n" + "=" * 60)
            print("Conan Publish Complete (Remote)")
            print("=" * 60)
            print(f"Package: {package_ref}")
            print(f"Remote: {args.remote}")

    sys.exit(0 if success else 1)


if __name__ == "__main__":
    main()
