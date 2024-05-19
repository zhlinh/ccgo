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
# <<<<<<<<<<<<

class CliResult:
    def __init__(self, value=None, error=None):
        self.value = value
        self.error = error

    def is_success(self):
        return self.error is None

    def is_failure(self):
        return self.error is not None

    def get_value(self, default=None):
        if self.is_success():
            return self.value
        else:
            return default

    def get_error(self, default=None):
        if self.is_failure():
            return self.error
        else:
            return default

