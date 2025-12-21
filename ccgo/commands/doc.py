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
import subprocess
import webbrowser

# region setup path
SCRIPT_PATH = os.path.split(os.path.realpath(__file__))[0]
PROJECT_ROOT_PATH = os.path.dirname(SCRIPT_PATH)
sys.path.append(SCRIPT_PATH)
sys.path.append(PROJECT_ROOT_PATH)
PACKAGE_NAME = os.path.basename(SCRIPT_PATH)
# endregion
# import this project modules
try:
    from ccgo.utils.context.namespace import CliNameSpace
    from ccgo.utils.context.context import CliContext
    from ccgo.utils.context.command import CliCommand
except ImportError:
    from utils.context.namespace import CliNameSpace
    from utils.context.context import CliContext
    from utils.context.command import CliCommand


def check_mkdocs_installed() -> tuple[bool, str]:
    """Check if mkdocs is available in PATH.

    Returns:
        Tuple of (is_installed, error_message)
    """
    if shutil.which("mkdocs"):
        return True, ""
    return False, (
        "MkDocs is not installed or not in PATH.\n"
        "Install it with: pip install ccgo[docs]\n"
        "Or install from project requirements: pip install -r docs/requirements.txt"
    )


def check_doxygen_installed() -> tuple[bool, str]:
    """Check if doxygen is available in PATH.

    Returns:
        Tuple of (is_installed, error_message)
    """
    if shutil.which("doxygen"):
        return True, ""
    return False, (
        "Doxygen is not installed or not in PATH.\n"
        "MkDoxy requires Doxygen to generate API documentation.\n"
        "Install it with:\n"
        "  macOS: brew install doxygen\n"
        "  Ubuntu: sudo apt-get install doxygen\n"
        "  Windows: choco install doxygen"
    )


def find_mkdocs_project(start_dir: str) -> tuple[str | None, str]:
    """Find the project directory containing mkdocs.yml.

    Searches for mkdocs.yml in start_dir and its immediate subdirectories.
    Also verifies CCGO.toml exists.

    Args:
        start_dir: Directory to start searching from

    Returns:
        Tuple of (project_dir, error_message). project_dir is None if not found.
    """
    # First check if mkdocs.yml exists in start_dir
    if os.path.isfile(os.path.join(start_dir, "mkdocs.yml")):
        if os.path.isfile(os.path.join(start_dir, "CCGO.toml")):
            return start_dir, ""
        # mkdocs.yml found but no CCGO.toml - still valid for docs
        return start_dir, ""

    # Check immediate subdirectories
    try:
        for subdir in os.listdir(start_dir):
            subdir_path = os.path.join(start_dir, subdir)
            if not os.path.isdir(subdir_path):
                continue
            if os.path.isfile(os.path.join(subdir_path, "mkdocs.yml")):
                return subdir_path, ""
    except OSError as e:
        return None, f"Error scanning directory: {e}"

    return None, (
        "mkdocs.yml not found in project directory.\n"
        "Please ensure you are in a CCGO project directory with MkDocs configured.\n"
        "Expected location: <project>/mkdocs.yml or <project>/<subdir>/mkdocs.yml"
    )


class Doc(CliCommand):
    def description(self) -> str:
        return """
        Build and view project documentation using MkDocs + MkDoxy.

        Uses MkDocs with the MkDoxy plugin to generate documentation from
        Markdown files and C++ source code comments (via Doxygen).
        The generated documentation uses Material theme and can be viewed
        in a web browser.

        Examples:
            ccgo doc                # Build documentation to target/docs/site/
            ccgo doc --open         # Build and open in browser
            ccgo doc --serve        # Start live-reload development server
            ccgo doc --serve --port 9000  # Use custom port
            ccgo doc --clean        # Clean build before generating
        """

    def cli(self) -> CliNameSpace:
        parser = argparse.ArgumentParser(
            prog="ccgo doc",
            formatter_class=argparse.RawDescriptionHelpFormatter,
            description=self.description(),
        )
        parser.add_argument(
            "--open",
            action="store_true",
            help="Open documentation in web browser after building",
        )
        parser.add_argument(
            "--serve",
            action="store_true",
            help="Start MkDocs development server with live reload",
        )
        parser.add_argument(
            "--port",
            type=int,
            default=8000,
            help="Port for development server (default: 8000, used with --serve)",
        )
        parser.add_argument(
            "--clean",
            action="store_true",
            help="Clean build artifacts before generating documentation",
        )
        module_name = os.path.splitext(os.path.basename(__file__))[0]
        input_argv = [x for x in sys.argv[1:] if x != module_name]
        args, unknown = parser.parse_known_args(input_argv)
        return args

    def exec(self, context: CliContext, args: CliNameSpace):
        print("Building project documentation with MkDocs...\n")

        # Get current working directory
        try:
            start_dir = os.getcwd()
        except (OSError, FileNotFoundError) as e:
            start_dir = os.environ.get("PWD")
            if not start_dir or not os.path.exists(start_dir):
                print(f"ERROR: Current working directory no longer exists: {e}")
                print("Please navigate to your project directory and try again.")
                sys.exit(1)

        # Find project directory with mkdocs.yml
        project_dir, error = find_mkdocs_project(start_dir)
        if not project_dir:
            print(f"ERROR: {error}")
            sys.exit(1)

        print(f"Project directory: {project_dir}")

        # Check for mkdocs.yml
        mkdocs_yml = os.path.join(project_dir, "mkdocs.yml")
        if not os.path.isfile(mkdocs_yml):
            print(f"ERROR: mkdocs.yml not found at {mkdocs_yml}")
            sys.exit(1)

        # Check dependencies
        mkdocs_ok, mkdocs_error = check_mkdocs_installed()
        if not mkdocs_ok:
            print(f"ERROR: {mkdocs_error}")
            sys.exit(1)

        doxygen_ok, doxygen_error = check_doxygen_installed()
        if not doxygen_ok:
            print(f"WARNING: {doxygen_error}")
            print("API documentation may not be generated.\n")

        # Handle --serve mode (development server with live reload)
        if args.serve:
            print(f"Starting MkDocs development server on port {args.port}...")
            print(f"Documentation URL: http://127.0.0.1:{args.port}/")
            print("Press Ctrl+C to stop the server\n")

            cmd = ["mkdocs", "serve", "-a", f"127.0.0.1:{args.port}"]
            try:
                # This blocks until Ctrl+C
                subprocess.run(cmd, cwd=project_dir, check=False)
            except KeyboardInterrupt:
                print("\n\nServer stopped")
            return

        # Build mode
        print("Mode: Build documentation\n")

        # Output to target/docs/site/ directory
        site_dir = os.path.join(project_dir, "target", "docs", "site")
        cmd = ["mkdocs", "build", "--site-dir", site_dir]
        if args.clean:
            cmd.append("--clean")
            print("Clean build enabled")

        print(f"Running: {' '.join(cmd)}")
        print()

        try:
            result = subprocess.run(cmd, cwd=project_dir, check=False)
            if result.returncode != 0:
                print("\nDocumentation build failed")
                sys.exit(result.returncode)
        except Exception as e:
            print(f"\nError running mkdocs: {e}")
            sys.exit(1)

        print("\nDocumentation built successfully!")

        # Get output path
        index_path = os.path.join(site_dir, "index.html")

        if os.path.exists(index_path):
            print(f"Documentation location: {site_dir}")

            # Handle --open option
            if args.open:
                print("\nOpening documentation in browser...")
                webbrowser.open_new_tab(f"file://{index_path}")
        else:
            print(f"\nWarning: Documentation output not found at expected location")
            print(f"   Expected: {index_path}")
