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
    from ccgo.utils.cmd.cmd_util import exec_command
except ImportError:
    from utils.context.namespace import CliNameSpace
    from utils.context.context import CliContext
    from utils.context.command import CliCommand
    from utils.cmd.cmd_util import exec_command


class Doc(CliCommand):
    def description(self) -> str:
        return """
        This is a subcommand to build and view project documentation.

        Uses Doxygen to generate documentation from source code comments.
        The generated documentation will be in HTML format and can be
        viewed in a web browser.

        Examples:
            ccgo doc                # Build documentation
            ccgo doc --open         # Build and open in browser
            ccgo doc --serve        # Build and start a local web server
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
            help="Start a local web server to view documentation (requires Python's http.server)",
        )
        parser.add_argument(
            "--port",
            type=int,
            default=8000,
            help="Port for local web server (default: 8000, used with --serve)",
        )
        parser.add_argument(
            "--clean",
            action="store_true",
            help="Clean build before generating documentation",
        )
        module_name = os.path.splitext(os.path.basename(__file__))[0]
        input_argv = [x for x in sys.argv[1:] if x != module_name]
        args, unknown = parser.parse_known_args(input_argv)
        return args

    def exec(self, context: CliContext, args: CliNameSpace):
        print("Building project documentation...\n")

        # Get current working directory (project directory)
        # Save it early in case subprocess changes it
        try:
            project_dir = os.getcwd()
        except (OSError, FileNotFoundError) as e:
            # If current directory was deleted, try to use PWD environment variable
            project_dir = os.environ.get("PWD")
            if not project_dir or not os.path.exists(project_dir):
                print(f"ERROR: Current working directory no longer exists: {e}")
                print("Please navigate to your project directory and try again.")
                sys.exit(1)
            # Try to change to the saved directory
            try:
                os.chdir(project_dir)
            except (OSError, FileNotFoundError):
                print(f"ERROR: Cannot access project directory: {project_dir}")
                sys.exit(1)

        # Check if CCGO.toml exists to verify we're in a CCGO project
        config_path = None
        for subdir in os.listdir(project_dir):
            subdir_path = os.path.join(project_dir, subdir)
            if not os.path.isdir(subdir_path):
                continue
            potential_config = os.path.join(subdir_path, "CCGO.toml")
            if os.path.isfile(potential_config):
                config_path = potential_config
                project_subdir = subdir_path
                break

        # If not found in subdirectory, check current directory
        if not config_path:
            if os.path.isfile(os.path.join(project_dir, "CCGO.toml")):
                config_path = os.path.join(project_dir, "CCGO.toml")
                project_subdir = project_dir
            else:
                print("❌ ERROR: CCGO.toml not found in project directory")
                print("Please ensure you are in a CCGO project directory")
                sys.exit(1)

        # Get the build script path
        build_script_name = "build_docs"
        build_scripts_dir = os.path.join(
            os.path.dirname(os.path.dirname(__file__)), "build_scripts"
        )
        build_script_path = os.path.join(build_scripts_dir, f"{build_script_name}.py")

        if not os.path.isfile(build_script_path):
            print(f"ERROR: Build script {build_script_path} not found")
            sys.exit(1)

        print(f"Project directory: {project_subdir}")
        print(f"Build script: {build_script_path}")

        # Determine the mode
        cmd_args = []
        if args.open:
            print("Mode: Build and open in browser\n")
            cmd_args.append("--open")
        else:
            print("Mode: Build only\n")

        # Build the command
        cmd = f"cd '{project_subdir}' && python3 '{build_script_path}'"
        if cmd_args:
            cmd = f"{cmd} {' '.join(cmd_args)}"
        print(f"Execute command:")
        print(cmd)
        print()

        # Execute the build
        err_code = os.system(cmd)

        if err_code != 0:
            print("\nDocumentation build failed")
            sys.exit(err_code)

        print("\nDocumentation built successfully")

        # Get the output path
        docs_output = os.path.join(project_subdir, "cmake_build", "Docs")
        import platform

        system_name = platform.system()
        html_path = os.path.join(
            docs_output, system_name + ".out", "_html", "index.html"
        )

        if os.path.exists(html_path):
            print(f"\nDocumentation location: {html_path}")

            # Handle --serve option
            if args.serve:
                print(f"\nStarting local web server on port {args.port}...")
                docs_dir = os.path.dirname(html_path)

                try:
                    import webbrowser
                    import time
                    import subprocess

                    # Start the server in background
                    server_cmd = (
                        f"cd '{docs_dir}' && python3 -m http.server {args.port}"
                    )
                    print(f"\nServer command: {server_cmd}")
                    print(f"Documentation URL: http://localhost:{args.port}/index.html")
                    print("\nOpening browser...")

                    # Open browser after a short delay
                    import threading

                    def open_browser():
                        time.sleep(1.5)
                        webbrowser.open(f"http://localhost:{args.port}/index.html")

                    browser_thread = threading.Thread(target=open_browser)
                    browser_thread.daemon = True
                    browser_thread.start()

                    print("\nPress Ctrl+C to stop the server\n")

                    # Run the server (blocks until Ctrl+C)
                    os.system(server_cmd)

                except KeyboardInterrupt:
                    print("\n\nServer stopped")
                except Exception as e:
                    print(f"\nError starting server: {e}")
                    print(
                        f"   You can manually view the documentation at: file://{html_path}"
                    )
        else:
            print(f"\nWarning: Documentation output not found at expected location")
            print(f"   Expected: {html_path}")
