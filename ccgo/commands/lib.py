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
from utils.context.namespace import CliNameSpace
from utils.context.context import CliContext
from utils.context.command import CliCommand

class Lib(CliCommand):
    def description(self) -> str:
        return """
        This is a subcommand to create a library project. 
        """
    
    def get_target_list(self) -> list:
        return ["create"]

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
        parser.add_argument('dst_dir')
        parser.add_argument(
            "--template-url",
            action="store",
            default="https://github.com/zhlinh/ccgo-template.git",
            help="template url",
        )
        module_name = os.path.splitext(os.path.basename(__file__))[0]
        input_argv = [x for x in sys.argv[1:] if x != module_name]
        args, unknown = parser.parse_known_args(input_argv)
        return args

    def exec(self, context: CliContext, args: CliNameSpace):
        print("Creating library project, with configuration...")
        print(vars(args))
        if os.path.exists(args.dst_dir):
            # directory exists, recopy
            run_recopy(args.dst_dir, unsafe=True)
        else:
            run_copy(args.template_url, args.dst_dir, unsafe=True)

