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
from copier import run_copy

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


class Init(CliCommand):
    def description(self) -> str:
        return """
        Initialize a CCGO library project in the current directory.

        Similar to 'cargo init', this command initializes the current directory
        as a CCGO project without creating a new subdirectory.

        WARNING: This will add/overwrite files in the current directory!

        Examples:
            ccgo init
            ccgo init --defaults
            ccgo init --template-url=https://github.com/user/custom-template.git
            ccgo init --data cpy_project_version=2.0.0
        """

    def cli(self) -> CliNameSpace:
        parser = argparse.ArgumentParser(
            prog="ccgo init",
            formatter_class=argparse.RawDescriptionHelpFormatter,
            description=self.description(),
        )
        parser.add_argument(
            "--template-url",
            action="store",
            default="https://github.com/zhlinh/ccgo-template.git",
            help="Template repository URL (default: official CCGO template)",
        )
        parser.add_argument(
            "--data",
            action="append",
            help="Template data in KEY=VALUE format (can be used multiple times)",
        )
        parser.add_argument(
            "--defaults",
            action="store_true",
            help="Use default values for all questions not provided via --data",
        )
        parser.add_argument(
            "--force",
            action="store_true",
            help="Skip confirmation prompt and initialize immediately",
        )
        module_name = os.path.splitext(os.path.basename(__file__))[0]
        input_argv = [x for x in sys.argv[1:] if x != module_name]
        args, unknown = parser.parse_known_args(input_argv)
        return args

    def exec(self, context: CliContext, args: CliNameSpace):
        current_dir = os.getcwd()
        dir_name = os.path.basename(current_dir)

        print(f"Initializing CCGO project in current directory: '{current_dir}'")
        print(f"Configuration: {vars(args)}")

        # Check if directory is not empty
        if os.listdir(current_dir):
            print(f"\n⚠️  WARNING: Current directory is not empty!")
            print("This operation will add/overwrite files in the current directory.")

            if not args.force:
                response = input("\nDo you want to continue? (y/N): ")
                if response.lower() != "y":
                    print("Aborted.")
                    sys.exit(0)

        # Use current directory name as default project name
        default_project_name = dir_name

        # Provide default value for project name question
        data = {"cpy_project_name": default_project_name}

        # Parse --data arguments if provided
        if hasattr(args, "data") and args.data:
            for item in args.data:
                if "=" in item:
                    key, value = item.split("=", 1)
                    # Convert string boolean values to actual booleans
                    if value.lower() == "true":
                        value = True
                    elif value.lower() == "false":
                        value = False
                    data[key] = value

        # Use defaults for unspecified questions if --defaults is provided
        use_defaults = hasattr(args, "defaults") and args.defaults

        print(f"\nInitializing project from template: {args.template_url}")

        run_copy(
            args.template_url,
            current_dir,
            data=data,
            unsafe=True,
            defaults=use_defaults,
            overwrite=True,
        )

        print(f"\n✅ Successfully initialized CCGO project in current directory!")
        print(f"\nNext steps:")
        print(f"  # Read README.md for more information")
        print(f"  # Start coding!")
