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
from utils.context.namespace import CliNameSpace
from utils.context.context import CliContext
from utils.context.command import CliCommand

# Root Class for Command Line Interface
class Cli(CliCommand):
    def description(self) -> str:
        return """
        This is the CCGO Build System. 
        """
    
    def get_command_list(self) -> list:
        arr = []
        for command in os.listdir(os.path.join(SCRIPT_PATH, "commands")):
            if not command.startswith("_") and command.endswith(".py"):
                arr.append(os.path.splitext(os.path.basename(command))[0])
        return arr

    def cli(self) -> CliNameSpace:
        parser = argparse.ArgumentParser(
                prog="CCGO",
                formatter_class = argparse.RawDescriptionHelpFormatter,
                description=self.description(),
        )
        parser.add_argument(
            'subcommand', metavar=f"{self.get_command_list()}",
            type=str, choices=self.get_command_list(),
        )
        # parse only known args
        args, unknown = parser.parse_known_args()
        return args

    def exec(self, context: CliContext, args: CliNameSpace):
        print(vars(args))
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
