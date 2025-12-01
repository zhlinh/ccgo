#!/usr/bin/env python3
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

"""
Script to publish ccgo package to PyPI.

Usage:
    python3 publish_to_pypi.py          # Publish to PyPI
    python3 publish_to_pypi.py --test   # Publish to TestPyPI
    python3 publish_to_pypi.py --check  # Only build and check, don't upload
    python3 publish_to_pypi.py --clean  # Only clean build artifacts
    python3 publish_to_pypi.py -y       # Publish without confirmation prompts

Environment Variables:
    PYPI_API_TOKEN      - API token for PyPI (used with twine --password)
    TEST_PYPI_API_TOKEN - API token for TestPyPI (used with twine --password)
"""

import os
import sys
import shutil
import subprocess
import argparse
import re


class Colors:
    """ANSI color codes for terminal output."""
    HEADER = '\033[95m'
    OKBLUE = '\033[94m'
    OKCYAN = '\033[96m'
    OKGREEN = '\033[92m'
    WARNING = '\033[93m'
    FAIL = '\033[91m'
    ENDC = '\033[0m'
    BOLD = '\033[1m'
    UNDERLINE = '\033[4m'


def print_step(message):
    """Print a step message."""
    print(f"\n{Colors.OKBLUE}{'=' * 70}{Colors.ENDC}")
    print(f"{Colors.OKBLUE}{Colors.BOLD}>>> {message}{Colors.ENDC}")
    print(f"{Colors.OKBLUE}{'=' * 70}{Colors.ENDC}\n")


def print_success(message):
    """Print a success message."""
    print(f"{Colors.OKGREEN}✓ {message}{Colors.ENDC}")


def print_warning(message):
    """Print a warning message."""
    print(f"{Colors.WARNING}⚠ {message}{Colors.ENDC}")


def print_error(message):
    """Print an error message."""
    print(f"{Colors.FAIL}✗ {message}{Colors.ENDC}")


def run_command(cmd, check=True, capture_output=False):
    """Run a shell command."""
    print(f"{Colors.OKCYAN}$ {cmd}{Colors.ENDC}")
    try:
        if capture_output:
            result = subprocess.run(
                cmd,
                shell=True,
                check=check,
                capture_output=True,
                text=True
            )
            return result.stdout.strip()
        else:
            subprocess.run(cmd, shell=True, check=check)
            return None
    except subprocess.CalledProcessError as e:
        print_error(f"Command failed: {cmd}")
        if capture_output:
            print(e.stderr)
        sys.exit(1)


def get_current_version():
    """Get current version from pyproject.toml or setup.py."""
    # Try pyproject.toml first (modern approach)
    pyproject_file = "pyproject.toml"
    if os.path.exists(pyproject_file):
        with open(pyproject_file, "r") as f:
            content = f.read()
            # Match: version = "2.1.0"
            match = re.search(r'version\s*=\s*["\']([^"\']+)["\']', content)
            if match:
                print(f"Version found in {pyproject_file}")
                return match.group(1)

    # Fallback to setup.py (legacy)
    setup_file = "setup.py"
    if os.path.exists(setup_file):
        with open(setup_file, "r") as f:
            content = f.read()
            match = re.search(r'version\s*=\s*["\']([^"\']+)["\']', content)
            if match:
                print(f"Version found in {setup_file}")
                return match.group(1)

    print_error("Could not find version in pyproject.toml or setup.py")
    sys.exit(1)


def clean_build_artifacts():
    """Clean build artifacts and temporary files."""
    print_step("Cleaning build artifacts")

    dirs_to_remove = [
        "build",
        "dist",
        "*.egg-info",
        "ccgo.egg-info",
        ".eggs",
        "__pycache__",
    ]

    for pattern in dirs_to_remove:
        if '*' in pattern:
            # Use shell expansion for patterns
            run_command(f"rm -rf {pattern}", check=False)
        else:
            if os.path.exists(pattern):
                print(f"Removing {pattern}/")
                shutil.rmtree(pattern)

    # Also clean __pycache__ directories recursively
    run_command("find . -type d -name __pycache__ -exec rm -rf {} + 2>/dev/null", check=False)
    run_command("find . -type f -name '*.pyc' -delete 2>/dev/null", check=False)

    print_success("Build artifacts cleaned")


def check_dependencies():
    """Check if required tools are installed."""
    print_step("Checking dependencies")

    required_packages = ["build", "twine"]
    missing = []

    for package in required_packages:
        try:
            __import__(package)
            print_success(f"{package} is installed")
        except ImportError:
            missing.append(package)
            print_warning(f"{package} is not installed")

    if missing:
        print(f"\n{Colors.WARNING}Installing missing packages...{Colors.ENDC}")
        run_command(f"pip3 install {' '.join(missing)}")
        print_success("Dependencies installed")
    else:
        print_success("All dependencies are installed")


def build_package():
    """Build the package distributions."""
    print_step("Building package distributions")

    # Build using python -m build (modern approach)
    run_command("python3 -m build")

    # List the built files
    if os.path.exists("dist"):
        print("\nBuilt distributions:")
        for file in os.listdir("dist"):
            print(f"  - dist/{file}")
        print_success("Package built successfully")
    else:
        print_error("dist/ directory not found after build")
        sys.exit(1)


def check_package():
    """Check the package using twine."""
    print_step("Checking package with twine")

    run_command("python3 -m twine check dist/*")
    print_success("Package check passed")


def get_git_status(skip_confirm=False):
    """Check git status."""
    print_step("Checking git status")

    # Check if there are uncommitted changes
    status = run_command("git status --porcelain", capture_output=True)
    if status:
        print_warning("There are uncommitted changes:")
        run_command("git status --short")
        if not skip_confirm:
            response = input(f"\n{Colors.WARNING}Continue anyway? (y/N): {Colors.ENDC}")
            if response.lower() != 'y':
                print("Aborted.")
                sys.exit(0)
        else:
            print_warning("Continuing anyway (--yes flag set)")
    else:
        print_success("Working directory is clean")

    # Get current branch
    branch = run_command("git branch --show-current", capture_output=True)
    print(f"Current branch: {branch}")

    # Check if tag exists for current version
    version = get_current_version()
    tags = run_command("git tag", capture_output=True)
    if f"v{version}" in tags.split('\n'):
        print_warning(f"Git tag v{version} already exists")
    else:
        print(f"Git tag v{version} does not exist yet")


def upload_to_pypi(test=False, skip_confirm=False):
    """Upload package to PyPI or TestPyPI."""
    if test:
        print_step("Uploading to TestPyPI")
        repository_url = "https://test.pypi.org/legacy/"
        repository_flag = "--repository testpypi"
        token_env_var = "TEST_PYPI_API_TOKEN"
    else:
        print_step("Uploading to PyPI")
        repository_url = "https://upload.pypi.org/legacy/"
        repository_flag = ""
        token_env_var = "PYPI_API_TOKEN"

    print(f"Target: {repository_url}")
    print()

    # Confirm upload
    version = get_current_version()
    print(f"{Colors.WARNING}About to upload version {version} to {'TestPyPI' if test else 'PyPI'}{Colors.ENDC}")
    if not skip_confirm:
        response = input(f"{Colors.WARNING}Continue? (y/N): {Colors.ENDC}")
        if response.lower() != 'y':
            print("Upload cancelled.")
            sys.exit(0)
    else:
        print_warning("Skipping confirmation (--yes flag set)")

    # Check for API token in environment variable
    api_token = os.environ.get(token_env_var)

    # Build twine command
    cmd = f"python3 -m twine upload {repository_flag} dist/*"

    if api_token:
        print_success(f"Using API token from {token_env_var} environment variable")
        # Set twine environment variables for authentication (more secure than command line args)
        env = os.environ.copy()
        env["TWINE_USERNAME"] = "__token__"
        env["TWINE_PASSWORD"] = api_token
        print(f"{Colors.OKCYAN}$ {cmd}{Colors.ENDC}")
        try:
            subprocess.run(cmd, shell=True, check=True, env=env)
        except subprocess.CalledProcessError:
            print_error(f"Command failed: {cmd}")
            sys.exit(1)
    else:
        print_warning(f"{token_env_var} environment variable not set, twine will prompt for credentials")
        run_command(cmd)

    print_success(f"Package uploaded to {'TestPyPI' if test else 'PyPI'}")

    if test:
        print(f"\n{Colors.OKGREEN}Test your package:{Colors.ENDC}")
        print(f"  pip3 install --index-url https://test.pypi.org/simple/ ccgo=={version}")
    else:
        print(f"\n{Colors.OKGREEN}Install your package:{Colors.ENDC}")
        print(f"  pip3 install ccgo=={version}")
        print(f"\n{Colors.OKGREEN}Create git tag:{Colors.ENDC}")
        print(f"  git tag -a v{version} -m 'Release version {version}'")
        print(f"  git push origin v{version}")


def install_package(dev_mode=False):
    """Install package locally."""
    print_step("Installing package" + (" in development mode" if dev_mode else ""))

    flag = "-e" if dev_mode else ""
    run_command(f"pip3 install {flag} .")

    print_success("Package installed successfully")
    if dev_mode:
        print("\n✓ Package installed in editable mode")
        print("  You can now use 'ccgo' command")
        print("  Changes to code will take effect immediately")
    else:
        print("\n✓ Package installed")
        print("  You can now use 'ccgo' command")


def uninstall_package():
    """Uninstall package."""
    print_step("Uninstalling ccgo package")

    run_command("pip3 uninstall -y ccgo")

    print_success("Package uninstalled successfully")


def main():
    parser = argparse.ArgumentParser(
        description="Publish ccgo package to PyPI",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  # Development
  python3 publish_to_pypi.py --dev         # Install in development mode
  python3 publish_to_pypi.py --install     # Install package locally
  python3 publish_to_pypi.py --uninstall   # Uninstall package

  # Publishing
  python3 publish_to_pypi.py --check       # Build and check only
  python3 publish_to_pypi.py --test        # Upload to TestPyPI
  python3 publish_to_pypi.py               # Upload to PyPI
  python3 publish_to_pypi.py --clean       # Clean build artifacts

  # Non-interactive mode (CI/CD)
  PYPI_API_TOKEN=pypi-xxx python3 publish_to_pypi.py -y
  TEST_PYPI_API_TOKEN=pypi-xxx python3 publish_to_pypi.py --test -y

Environment Variables:
  PYPI_API_TOKEN      - API token for PyPI
  TEST_PYPI_API_TOKEN - API token for TestPyPI
        """
    )
    parser.add_argument(
        "--dev",
        action="store_true",
        help="Install package in development mode (editable)"
    )
    parser.add_argument(
        "--install",
        action="store_true",
        help="Install package locally"
    )
    parser.add_argument(
        "--uninstall",
        action="store_true",
        help="Uninstall package"
    )
    parser.add_argument(
        "--test",
        action="store_true",
        help="Upload to TestPyPI instead of PyPI"
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help="Only build and check, don't upload"
    )
    parser.add_argument(
        "--clean",
        action="store_true",
        help="Only clean build artifacts and exit"
    )
    parser.add_argument(
        "--skip-git-check",
        action="store_true",
        help="Skip git status check"
    )
    parser.add_argument(
        "-y", "--yes",
        action="store_true",
        help="Skip all confirmation prompts (non-interactive mode)"
    )

    args = parser.parse_args()

    # Change to script directory
    script_dir = os.path.dirname(os.path.abspath(__file__))
    os.chdir(script_dir)

    print(f"\n{Colors.HEADER}{Colors.BOLD}")
    print("╔════════════════════════════════════════════════════════════════════╗")
    print("║                    CCGO PyPI Publishing Script                     ║")
    print("╚════════════════════════════════════════════════════════════════════╝")
    print(f"{Colors.ENDC}\n")

    version = get_current_version()
    print(f"Current version: {Colors.BOLD}{version}{Colors.ENDC}\n")

    # Development mode
    if args.dev:
        install_package(dev_mode=True)
        return

    # Install mode
    if args.install:
        install_package(dev_mode=False)
        return

    # Uninstall mode
    if args.uninstall:
        uninstall_package()
        return

    # Clean only mode
    if args.clean:
        clean_build_artifacts()
        print_success("Done!")
        return

    # Check git status
    if not args.skip_git_check:
        get_git_status(skip_confirm=args.yes)

    # Clean previous builds
    clean_build_artifacts()

    # Check and install dependencies
    check_dependencies()

    # Build package
    build_package()

    # Check package
    check_package()

    # Upload or just check
    if args.check:
        print_success("Build and check completed successfully!")
        print(f"\n{Colors.OKGREEN}To upload to PyPI, run:{Colors.ENDC}")
        print(f"  python3 publish_to_pypi.py")
        print(f"\n{Colors.OKGREEN}To upload to TestPyPI, run:{Colors.ENDC}")
        print(f"  python3 publish_to_pypi.py --test")
    else:
        upload_to_pypi(test=args.test, skip_confirm=args.yes)
        print(f"\n{Colors.OKGREEN}{Colors.BOLD}✓ Publishing completed successfully!{Colors.ENDC}\n")


if __name__ == "__main__":
    try:
        main()
    except KeyboardInterrupt:
        print(f"\n\n{Colors.WARNING}Interrupted by user{Colors.ENDC}")
        sys.exit(1)
    except Exception as e:
        print_error(f"Unexpected error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)
