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
import subprocess
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
from utils.cmd.cmd_util import exec_command

class Build(CliCommand):
    def description(self) -> str:
        return """
        This is a subcommand to build a library. 
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
        parser.add_argument(
            "--ide-project",
            action="store",
            help="generate ide project",
        )
        parser.add_argument(
            "--arch",
            action="store",
            default="arm64-v8a",
            help="arch like armeabi-v7a, arm64-v8a, x86_64, etc",
        )
        module_name = os.path.splitext(os.path.basename(__file__))[0]
        input_argv = [x for x in sys.argv[1:] if x != module_name]
        args, unknown = parser.parse_known_args(input_argv)
        return args

    def exec(self, context: CliContext, args: CliNameSpace):
        print("Building library, with configuration...")
        print(vars(args))
        num = 2 if args.ide_project else 1
        arch = args.arch if args.target == "android" else ""
        cmd = f"python3 build_{args.target}.py {num} {arch}"
        print("\nExecute command:")
        print(cmd)
        err_code = os.system(cmd)
        sys.exit(err_code)

