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
from utils.cmd.cmd_util import exec_command

class Help(CliCommand):
    def description(self) -> str:
        return """
        This is a subcommand to show help. 
        """

    def cli(self) -> CliNameSpace:
        parser = argparse.ArgumentParser(
            # 获取文件名
            prog=os.path.basename(__file__),
            formatter_class = argparse.RawDescriptionHelpFormatter,
            description=self.description(),
        )
        module_name = os.path.splitext(os.path.basename(__file__))[0]
        input_argv = [x for x in sys.argv[1:] if x != module_name]
        args, unknown = parser.parse_known_args(input_argv)
        return args

    def exec(self, context: CliContext, args: CliNameSpace):
        # show help
        print("\n1. create a library project")
        print("\nccgo lib create LibName --template-url TemplateUrl")
        print("\n2. build a library")
        print("\nccgo build android --arch armeabi-v7a,arm64-v8a,x86_64")
        print("\nccgo build ios")
        print("\n")


