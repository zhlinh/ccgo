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
from copier import run_recopy

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
    from ccgo.utils.template_version import resolve_template_version
except ImportError:
    from utils.context.namespace import CliNameSpace
    from utils.context.context import CliContext
    from utils.context.command import CliCommand
    from utils.template_version import resolve_template_version


class New(CliCommand):
    def description(self) -> str:
        return """
        Create a new CCGO library project in a new directory.

        Similar to 'cargo new', this command creates a new project directory
        with all necessary files and structure.

        By default, the command runs in non-interactive mode using default values.
        Use --interact to enable interactive mode with prompts.

        Version Resolution:
            - Specify X.0.0 to use the highest X.y.z version (e.g., 2.0.0 -> 2.3.5)
            - Specify X.Y.0 to use the highest X.Y.z version (e.g., 2.1.0 -> 2.1.8)
            - Specify X.Y.Z to use exact version (e.g., 2.2.3 -> 2.2.3)
            - Use --use-latest to always use the latest master/main branch

        Examples:
            ccgo new my-project
            ccgo new my-project --interact
            ccgo new my-project --template-version=2.0.0
            ccgo new my-project --template-version=2.1.0
            ccgo new my-project --use-latest
            ccgo new my-project --template-url=https://github.com/user/custom-template.git
            ccgo new my-project --data cpy_project_version=2.0.0
        """

    def cli(self) -> CliNameSpace:
        parser = argparse.ArgumentParser(
            prog="ccgo new",
            formatter_class=argparse.RawDescriptionHelpFormatter,
            description=self.description(),
        )
        parser.add_argument(
            "path", help="Directory path where the new project will be created"
        )
        parser.add_argument(
            "--template-url",
            action="store",
            default="https://github.com/zhlinh/ccgo-template.git",
            help="Template repository URL (default: official CCGO template)",
        )
        parser.add_argument(
            "--template-version",
            action="store",
            default=None,
            help="Template version to use (e.g., 2.0.0, 2.1.0). If not specified, uses latest stable tag",
        )
        parser.add_argument(
            "--use-latest",
            action="store_true",
            help="Use latest master/main branch instead of tagged version",
        )
        parser.add_argument(
            "--data",
            action="append",
            help="Template data in KEY=VALUE format (can be used multiple times)",
        )
        parser.add_argument(
            "--interact",
            action="store_true",
            help="Enable interactive mode with prompts (default is non-interactive)",
        )
        module_name = os.path.splitext(os.path.basename(__file__))[0]
        input_argv = [x for x in sys.argv[1:] if x != module_name]
        args, unknown = parser.parse_known_args(input_argv)
        return args

    def exec(self, context: CliContext, args: CliNameSpace):
        print(f"Creating new CCGO project in '{args.path}'...")
        print(f"Configuration: {vars(args)}")

        # Check if directory already exists
        if os.path.exists(args.path):
            print(f"\nDirectory '{args.path}' already exists.")
            response = input(
                "Do you want to update it with the latest template? (y/N): "
            )
            if response.lower() == "y":
                print("Updating existing project...")
                run_recopy(args.path, unsafe=True)
                print(f"\n✅ Project '{args.path}' updated successfully!")
                return
            else:
                print("Aborted.")
                sys.exit(1)

        # Resolve template version
        template_version = resolve_template_version(
            args.template_url,
            requested_version=getattr(args, "template_version", None),
            use_latest=getattr(args, "use_latest", False),
        )

        if template_version is None:
            print("Error: Failed to resolve template version")
            sys.exit(1)

        # Extract project name from destination directory for default value
        default_project_name = os.path.basename(os.path.abspath(args.path))

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

        # Use defaults for unspecified questions unless --interact is provided
        use_defaults = not (hasattr(args, "interact") and args.interact)

        print(f"\nUsing template version: {template_version}")

        run_copy(
            args.template_url,
            args.path,
            vcs_ref=template_version,
            data=data,
            unsafe=True,
            defaults=use_defaults,
            overwrite=True,
        )

        print(f"\nSuccessfully created new CCGO project: '{args.path}'")
        print(f"\nNext steps:")
        print(f"  cd {args.path}")
        print(f"  # Read README.md for more information")
