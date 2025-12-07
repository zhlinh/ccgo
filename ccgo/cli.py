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
import importlib
import argparse

# setup path
# >>>>>>>>>>>>>>
SCRIPT_PATH = os.path.split(os.path.realpath(__file__))[0]
PROJECT_ROOT_PATH = os.path.dirname(SCRIPT_PATH)
sys.path.append(SCRIPT_PATH)
sys.path.append(PROJECT_ROOT_PATH)
PACKAGE_NAME = os.path.basename(SCRIPT_PATH)
# <<<<<<<<<<<<<<
# import this project modules
try:
    from ccgo.utils.context.namespace import CliNameSpace
    from ccgo.utils.context.context import CliContext
    from ccgo.utils.context.command import CliCommand
except ImportError:
    from utils.context.namespace import CliNameSpace
    from utils.context.context import CliContext
    from utils.context.command import CliCommand


# Root Class for Command Line Interface
class Cli(CliCommand):
    def description(self) -> str:
        return """CCGO - Cross-Platform C++ Build Tool

A unified build system for cross-platform C++ libraries supporting:
Android, iOS, macOS, Windows, Linux, OpenHarmony (OHOS), and Kotlin Multiplatform (KMP)

USAGE:
    ccgo <command> [options]

COMMANDS:
    new         Create a new library project
    init        Initialize library project in current directory
    check       Check platform dependencies
    clean       Clean build artifacts
    test        Run tests (GoogleTest)
    bench       Run benchmarks (Google Benchmark)
    doc         Build documentation (Doxygen)
    build       Build library for specific platform (use 'build all' for CI/CD)
    package     Package SDK for distribution
    publish     Publish library to repository
    help        Show detailed help information

EXAMPLES:
    ccgo new my-project              # Create new project
    ccgo build android               # Build for Android
    ccgo test                        # Run all tests
    ccgo build all --release         # CI build all platforms
    ccgo package --include-docs      # Package SDK with docs
    ccgo help                        # Show detailed help

For more information on a specific command:
    ccgo <command> --help

Project: https://github.com/zhlinh/ccgo
        """

    def get_command_list(self) -> list:
        arr = []
        for command in os.listdir(os.path.join(SCRIPT_PATH, "commands")):
            if not command.startswith("_") and command.endswith(".py"):
                arr.append(os.path.splitext(os.path.basename(command))[0])
        return arr

    def cli(self) -> CliNameSpace:
        # Check if user wants help for main command (ccgo --help or ccgo -h)
        # But NOT for subcommands (ccgo build --help)
        if len(sys.argv) == 2 and sys.argv[1] in ['--help', '-h']:
            parser = argparse.ArgumentParser(
                prog="ccgo",
                formatter_class=argparse.RawDescriptionHelpFormatter,
                description=self.description(),
            )
            parser.add_argument(
                "subcommand",
                metavar=f"{self.get_command_list()}",
                type=str,
                choices=self.get_command_list(),
            )
            parser.print_help()
            sys.exit(0)

        # Parse subcommand without automatic help handling
        parser = argparse.ArgumentParser(
            prog="ccgo",
            formatter_class=argparse.RawDescriptionHelpFormatter,
            description=self.description(),
            add_help=False,
        )
        parser.add_argument(
            "subcommand",
            metavar=f"{self.get_command_list()}",
            type=str,
            nargs='?',  # Make subcommand optional
            choices=self.get_command_list(),
        )
        # parse only known args - this will NOT consume --help if present
        args, unknown = parser.parse_known_args()
        return args

    def exec(self, context: CliContext, args: CliNameSpace):
        print(vars(args))

        # Check if subcommand is provided
        if not args.subcommand:
            print("ERROR: No command specified\n")
            # Show help
            parser = argparse.ArgumentParser(
                prog="ccgo",
                formatter_class=argparse.RawDescriptionHelpFormatter,
                description=self.description(),
            )
            parser.add_argument(
                "subcommand",
                metavar=f"{self.get_command_list()}",
                type=str,
                choices=self.get_command_list(),
            )
            parser.print_help()
            sys.exit(1)

        # get module name
        module_name = f"commands.{args.subcommand}"
        # get class name
        class_name = args.subcommand.capitalize()
        # import module
        module = importlib.import_module(module_name)
        # get class of module
        klass = getattr(module, class_name)
        # instance class
        sub_cmd = klass()
        # now execute the subcommand
        sub_cmd.exec(CliContext(), sub_cmd.cli())


def main():
    cmd = Cli()
    cmd.exec(CliContext(), cmd.cli())


if __name__ == "__main__":
    main()
