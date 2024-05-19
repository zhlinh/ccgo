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
from utils.context.namespace import CliNameSpace
from utils.context.context import CliContext
from utils.context.command import CliCommand
from utils.cmd.cmd_util import exec_command

class Publish(CliCommand):
    def description(self) -> str:
        return """
        This is a subcommand to publish the library to maven repository.
        """
    
    def get_target_list(self) -> list:
        return [
            "android", "ios", "windows",
            "linux", "macos",
            "tests", "benches"
        ]

    def cli(self) -> CliNameSpace:
        parser = argparse.ArgumentParser(
            # 获取文件名
            prog=os.path.basename(__file__),
            formatter_class = argparse.RawDescriptionHelpFormatter,
            description=self.description(),
        )
        parser.add_argument(
            'target',
            metavar=f"{self.get_target_list()}",
            type=str,
            choices=self.get_target_list(),
        )
        module_name = os.path.splitext(os.path.basename(__file__))[0]
        input_argv = [x for x in sys.argv[1:] if x != module_name]
        args, unknown = parser.parse_known_args(input_argv)
        return args

    def exec(self, context: CliContext, args: CliNameSpace):
        print("Publishing library project, with configuration...")
        print(vars(args))
        if args.target != "android":
            print("\nPublishing only support maven of android now")
            sys.exit(1)
        # do publish
        cmd = f"./gradlew publishMainPublicationToMavenRepository"
        err_code, err_msg = exec_command(cmd)
        if err_code != 0:
            print("\nEnd with error:")
            print(err_msg)

