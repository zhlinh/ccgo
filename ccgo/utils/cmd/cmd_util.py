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

import subprocess
import time
from threading import Timer

DEFAULT_TIMEOUT_SECOND = 10


def exec_command(command, stdout=subprocess.PIPE, stderr=subprocess.STDOUT):
    # timeout is 3 hours
    return exec_command_with_timeout_second(command, 3 * 3600)


def exec_command_with_timeout_second(command, 
                                     timeout_second=DEFAULT_TIMEOUT_SECOND,
                                     stdout=subprocess.PIPE,
                                     stderr=subprocess.STDOUT):
    start_mills = int(time.time() * 1000)
    # default timeout is 10 second
    compile_popen = subprocess.Popen(
        command, shell=True, stdout=stdout, stderr=stderr,
    )
    timer = Timer(timeout_second, lambda process: process.kill(), [compile_popen])
    try:
        timer.start()
        stdout, stderr = compile_popen.communicate()
    finally:
        timer.cancel()
    err_code = compile_popen.returncode
    err_msg = bytes.decode(stdout, "UTF-8")
    if err_code == -9:
        if not err_msg:
            if stderr:
                err_msg = bytes.decode(stderr, "UTF-8")
            if not err_msg:
                use_time = int(time.time() * 1000) - start_mills
                err_msg = f"Failed for timeout({err_code}), use_time: {use_time}ms"
    return err_code, err_msg
