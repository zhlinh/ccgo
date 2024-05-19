#
# Copyright 2024 ccgo Project. All rights reserved.
# Use of this source code is governed by a MIT-style
# license that can be found at
#
# https://opensource.org/license/MIT
#
# The above copyright notice and this permission
# notice shall be included in all copies or
# substantial portions of the Software.

import os

from setuptools import setup, find_packages

ALL_PROGRAM_ENTRIES = ['ccgo = ccgo.main:main']

with open("README.md", "r") as f:
    long_description = f.read()

setup(
    name='ccgo',
    version='1.0.0',
    description='A C++ cross-platform build system.',
    long_description=long_description,
    author='zhlinh',
    author_email='zhlinhng@gmail.com',
    url='https://github.com/zhlinh/ccgo',
    package_dir={"./": "ccgo"},
    packages=find_packages(),
    include_package_data = True,
    install_requires=[
        "copier>=9.2.0",
        "copier-templates-extensions>=0.3.0",
    ],
    classifiers=[
        'Development Status :: 3 - Alpha',
        'Intended Audience :: Developers',
        'License :: OSI Approved :: MIT License',
        'Programming Language :: Python :: 3',
        'Programming Language :: Python :: 3.6',
        'Programming Language :: Python :: 3.7',
        'Programming Language :: Python :: 3.8',
        'Programming Language :: Python :: 3.9',
        'Programming Language :: Python :: Implementation :: CPython',
        "Operating System :: POSIX :: Linux",
        "Operating System :: MacOS :: MacOS X",
        "Operating System :: Microsoft :: Windows"
    ],
    zip_safe=False,
    entry_points = {
        'console_scripts': ALL_PROGRAM_ENTRIES
    }
)
