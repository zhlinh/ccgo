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
# setup path
# >>>>>>>>>>>>>>
SCRIPT_PATH = os.path.split(os.path.realpath(__file__))[0]
PROJECT_ROOT_PATH = os.path.dirname(SCRIPT_PATH)
sys.path.append(SCRIPT_PATH)
sys.path.append(PROJECT_ROOT_PATH)
PACKAGE_NAME = os.path.basename(SCRIPT_PATH)
# <<<<<<<<<<<<<
# import this project modules
from .namespace import CliNameSpace
from .context import CliContext
from .result import CliResult


# This Command class is the interface for all commands
class CliCommand:
    def description(self) -> str:
        raise NotImplementedError

    def cli(self) -> CliNameSpace:
        raise NotImplementedError

    def exec(self, context: CliContext, args: CliNameSpace) -> CliResult:
        raise NotImplementedError

