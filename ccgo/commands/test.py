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


class Test(CliCommand):
    def description(self) -> str:
        return """
        This is a subcommand to build and run tests.

        Uses GoogleTest framework to run unit tests for the project.
        Tests are built with CMake and executed automatically.

        Examples:
            ccgo test                      # Build and run all tests
            ccgo test --build-only         # Only build tests without running
            ccgo test --filter "MyTest*"   # Run only tests matching filter
            ccgo test --ide-project        # Generate IDE project for tests
        """

    def cli(self) -> CliNameSpace:
        parser = argparse.ArgumentParser(
            prog="ccgo test",
            formatter_class=argparse.RawDescriptionHelpFormatter,
            description=self.description(),
        )
        parser.add_argument(
            "--build-only",
            action="store_true",
            help="Only build tests without running them",
        )
        parser.add_argument(
            "--run-only",
            action="store_true",
            help="Only run tests without building (assumes tests are already built)",
        )
        parser.add_argument(
            "--filter",
            type=str,
            help="GoogleTest filter pattern (e.g., 'MyTest*' or 'MyTest.MyCase')",
        )
        parser.add_argument(
            "--ide-project",
            action="store_true",
            help="Generate IDE project for tests (Xcode on macOS, Visual Studio on Windows, CodeLite on Linux)",
        )
        parser.add_argument(
            "--gtest-args",
            type=str,
            help="Additional GoogleTest arguments (e.g., '--gtest_repeat=3 --gtest_shuffle')",
        )
        module_name = os.path.splitext(os.path.basename(__file__))[0]
        input_argv = [x for x in sys.argv[1:] if x != module_name]
        args, unknown = parser.parse_known_args(input_argv)

        # Store unknown args as additional gtest args
        if unknown:
            if args.gtest_args:
                args.gtest_args = f"{args.gtest_args} {' '.join(unknown)}"
            else:
                args.gtest_args = " ".join(unknown)

        return args

    def exec(self, context: CliContext, args: CliNameSpace):
        print("Running project tests...\n")

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
                print("\n❌ ERROR: CCGO.toml not found in project directory")
                print("Please ensure you are in a CCGO project directory")
                sys.exit(1)

        # Get the build script path
        build_script_name = "build_tests"
        build_scripts_dir = os.path.join(
            os.path.dirname(os.path.dirname(__file__)), "build_scripts"
        )
        build_script_path = os.path.join(build_scripts_dir, f"{build_script_name}.py")

        if not os.path.isfile(build_script_path):
            print(f"ERROR: Build script {build_script_path} not found")
            sys.exit(1)

        print(f"Project directory: {project_subdir}")
        print(f"Build script: {build_script_path}")

        # Determine the mode and build filter arguments
        cmd_args = []
        gtest_filter = ""

        if args.ide_project:
            print("Mode: Generate IDE project\n")
            cmd_args.append("--ide-project")
        elif args.build_only:
            print("Mode: Build tests only\n")
            # Default mode, no flag needed
        elif args.run_only:
            print("Mode: Run tests only\n")
            cmd_args.append("--run-only")
            gtest_filter = self._build_filter_args(args)
        else:
            print("Mode: Build and run tests\n")
            cmd_args.append("--run")
            gtest_filter = self._build_filter_args(args)

        # Build the command
        cmd = f"cd '{project_subdir}' && python3 '{build_script_path}'"

        # Add argparse flags
        if cmd_args:
            cmd = f"{cmd} {' '.join(cmd_args)}"

        # Add filter arguments if any
        if gtest_filter:
            cmd = f"{cmd} {gtest_filter}"

        print(f"Execute command:")
        print(cmd)
        print()

        # Execute the tests
        err_code = os.system(cmd)

        if err_code != 0:
            print("\nTests failed")
            sys.exit(err_code)

        if args.run_only or (not args.build_only and not args.ide_project):
            print("\nAll tests passed")
        elif args.build_only:
            print("\nTests built successfully")
        elif args.ide_project:
            print("\nIDE project generated successfully")

    def _build_filter_args(self, args):
        """Build GoogleTest filter arguments"""
        filter_args = []

        # Add user-specified filter
        if args.filter:
            # Process filter: add '*' at the end if not already present
            filters = args.filter.split(",")
            processed_filters = []
            for f in filters:
                f = f.strip()
                if f and not f.startswith("-"):
                    if "." not in f and not f.endswith("*"):
                        f = f + "*"
                    processed_filters.append(f)

            if processed_filters:
                filter_args.append(f'--gtest_filter={":".join(processed_filters)}')

        # Add additional gtest arguments
        if args.gtest_args:
            filter_args.append(args.gtest_args)

        return " ".join(filter_args)
