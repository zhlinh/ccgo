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

"""
Swift Package Manager publisher for CCGO.

Handles Package.swift generation and git-based distribution.
"""

import os
import subprocess
import hashlib
import zipfile
from pathlib import Path
from typing import Optional, Tuple

from .config import ApplePublishConfig


class SPMPublisher:
    """Handle Swift Package Manager publishing workflow."""

    def __init__(self, config: ApplePublishConfig):
        """
        Initialize SPM publisher.

        Args:
            config: Apple publishing configuration
        """
        self.config = config
        self.package_swift_path: Optional[Path] = None

    def validate_prerequisites(self) -> Tuple[bool, str]:
        """
        Validate that all prerequisites are met.

        Returns:
            Tuple of (success, error_message)
        """
        # Check if swift CLI is available
        try:
            result = subprocess.run(
                ['swift', '--version'],
                capture_output=True,
                text=True
            )
            if result.returncode != 0:
                return False, "Swift CLI not found. Please install Xcode command line tools."
        except FileNotFoundError:
            return False, "Swift CLI not found. Please install Xcode command line tools."

        # Check if git is available
        try:
            result = subprocess.run(
                ['git', '--version'],
                capture_output=True,
                text=True
            )
            if result.returncode != 0:
                return False, "Git not found. Please install Git."
        except FileNotFoundError:
            return False, "Git not found. Please install Git."

        # Check if XCFramework exists
        xcframework_path = self.config.find_xcframework()
        if not xcframework_path:
            return False, (
                f"XCFramework not found for {self.config.pod_name}. "
                "Please build the project first with: ccgo build ios / ccgo build macos"
            )

        return True, ""

    def calculate_checksum(self, file_path: Path) -> str:
        """
        Calculate SHA256 checksum of a file.

        Args:
            file_path: Path to file

        Returns:
            SHA256 checksum as hex string
        """
        sha256_hash = hashlib.sha256()
        with open(file_path, "rb") as f:
            for byte_block in iter(lambda: f.read(4096), b""):
                sha256_hash.update(byte_block)
        return sha256_hash.hexdigest()

    def create_xcframework_zip(self, xcframework_path: Path, output_path: Optional[Path] = None) -> Path:
        """
        Create a zip archive of the XCFramework.

        Args:
            xcframework_path: Path to XCFramework directory
            output_path: Optional output path for zip file

        Returns:
            Path to created zip file
        """
        if output_path is None:
            output_path = xcframework_path.parent / f'{xcframework_path.stem}.xcframework.zip'

        # Create zip file
        with zipfile.ZipFile(output_path, 'w', zipfile.ZIP_DEFLATED) as zipf:
            for root, dirs, files in os.walk(xcframework_path):
                for file in files:
                    file_path = Path(root) / file
                    arcname = file_path.relative_to(xcframework_path.parent)
                    zipf.write(file_path, arcname)

        return output_path

    def generate_package_swift(self, output_path: Optional[Path] = None) -> Path:
        """
        Generate Package.swift manifest file.

        Args:
            output_path: Optional path to write Package.swift. Defaults to apple/ directory.

        Returns:
            Path to generated Package.swift file
        """
        if output_path is None:
            # Default to apple/ directory if it exists, otherwise project root
            apple_dir = self.config.project_dir / 'apple'
            if apple_dir.exists():
                output_path = apple_dir / 'Package.swift'
            else:
                output_path = self.config.project_dir / 'Package.swift'

        package_name = self.config.spm.package_name or self.config.pod_name
        library_name = self.config.spm.library_name or self.config.pod_name
        target_name = f'{self.config.pod_name}'

        xcframework_path = self.config.find_xcframework()

        # Build Package.swift content
        lines = [
            "// swift-tools-version:5.3",
            "// The swift-tools-version declares the minimum version of Swift required to build this package.",
            "",
            "import PackageDescription",
            "",
            "let package = Package(",
            f'    name: "{package_name}",',
        ]

        # Platforms
        platform_lines = []
        for platform in self.config.platforms:
            min_version = self.config.min_versions.get(platform, '13.0')
            if platform == 'ios':
                platform_lines.append(f'        .iOS("{min_version}")')
            elif platform == 'macos':
                platform_lines.append(f'        .macOS("{min_version}")')
            elif platform == 'tvos':
                platform_lines.append(f'        .tvOS("{min_version}")')
            elif platform == 'watchos':
                platform_lines.append(f'        .watchOS("{min_version}")')

        if platform_lines:
            lines.append("    platforms: [")
            lines.append(',\n'.join(platform_lines))
            lines.append("    ],")

        # Products
        lines.extend([
            "    products: [",
            f'        .library(name: "{library_name}", targets: ["{target_name}"]),',
            "    ],",
        ])

        # Targets
        lines.append("    targets: [")

        if self.config.spm.use_local_path or not self.config.spm.xcframework_url:
            # Local binary target
            if xcframework_path:
                # Calculate relative path from Package.swift location
                package_dir = output_path.parent
                try:
                    relative_path = xcframework_path.relative_to(package_dir)
                except ValueError:
                    # xcframework is not under package_dir, use path relative to project root with ../
                    relative_path = Path('..') / xcframework_path.relative_to(self.config.project_dir)
                lines.extend([
                    "        .binaryTarget(",
                    f'            name: "{target_name}",',
                    f'            path: "{relative_path}"',
                    "        ),",
                ])
            else:
                # Placeholder for local path
                lines.extend([
                    "        .binaryTarget(",
                    f'            name: "{target_name}",',
                    f'            path: "{self.config.pod_name}.xcframework"',
                    "        ),",
                ])
        else:
            # Remote binary target with URL and checksum
            url = self.config.spm.xcframework_url

            # Calculate checksum if XCFramework exists locally
            checksum = ""
            if xcframework_path:
                # Create zip if needed
                zip_path = xcframework_path.parent / f'{xcframework_path.stem}.xcframework.zip'
                if not zip_path.exists():
                    zip_path = self.create_xcframework_zip(xcframework_path)
                checksum = self.calculate_checksum(zip_path)
            else:
                checksum = "REPLACE_WITH_CHECKSUM"

            lines.extend([
                "        .binaryTarget(",
                f'            name: "{target_name}",',
                f'            url: "{url}",',
                f'            checksum: "{checksum}"',
                "        ),",
            ])

        lines.append("    ]")
        lines.append(")")

        # Write Package.swift file
        package_content = '\n'.join(lines)
        output_path.write_text(package_content)
        self.package_swift_path = output_path

        return output_path

    def create_git_tag(self, version: Optional[str] = None, push: bool = False) -> Tuple[bool, str]:
        """
        Create a git tag for the version.

        Args:
            version: Version string. Defaults to config version.
            push: Push tag to remote

        Returns:
            Tuple of (success, output_message)
        """
        if version is None:
            version = self.config.version

        tag_name = f'v{version}' if not version.startswith('v') else version

        try:
            # Check if tag exists
            result = subprocess.run(
                ['git', 'tag', '-l', tag_name],
                capture_output=True,
                text=True,
                cwd=self.config.project_dir
            )
            if tag_name in result.stdout:
                return False, f"Tag {tag_name} already exists"

            # Create tag
            result = subprocess.run(
                ['git', 'tag', '-a', tag_name, '-m', f'Release {version}'],
                capture_output=True,
                text=True,
                cwd=self.config.project_dir
            )
            if result.returncode != 0:
                return False, f"Failed to create tag: {result.stderr}"

            # Push tag if requested
            if push:
                result = subprocess.run(
                    ['git', 'push', 'origin', tag_name],
                    capture_output=True,
                    text=True,
                    cwd=self.config.project_dir
                )
                if result.returncode != 0:
                    return False, f"Failed to push tag: {result.stderr}"

            return True, f"Created tag {tag_name}"
        except Exception as e:
            return False, f"Git operation failed: {e}"

    def validate_package(self) -> Tuple[bool, str]:
        """
        Validate Package.swift with swift package describe.

        Returns:
            Tuple of (success, output_message)
        """
        if not self.package_swift_path:
            return False, "Package.swift not generated yet"

        try:
            # Run from the directory containing Package.swift
            package_dir = self.package_swift_path.parent
            result = subprocess.run(
                ['swift', 'package', 'describe'],
                capture_output=True,
                text=True,
                cwd=package_dir
            )
            if result.returncode == 0:
                return True, result.stdout
            else:
                return False, f"Package validation failed:\n{result.stderr}"
        except Exception as e:
            return False, f"Failed to validate package: {e}"

    def publish(self, push_tag: bool = True) -> Tuple[bool, str]:
        """
        Publish SPM package by generating Package.swift and creating git tag.

        Args:
            push_tag: Push git tag to remote

        Returns:
            Tuple of (success, output_message)
        """
        messages = []

        # Generate Package.swift if not already done
        if not self.package_swift_path:
            path = self.generate_package_swift()
            messages.append(f"Generated Package.swift at {path}")

        # Validate package
        valid, msg = self.validate_package()
        if not valid:
            # For binary targets, swift package describe may not work locally
            # Just warn but continue
            messages.append(f"Package validation warning: {msg}")

        # Create git tag
        success, msg = self.create_git_tag(push=push_tag)
        if success:
            messages.append(msg)
        else:
            messages.append(f"Tag creation: {msg}")

        return True, '\n'.join(messages)

    def get_package_url(self) -> Optional[str]:
        """Get the git URL for the package."""
        if self.config.spm.git_url:
            return self.config.spm.git_url

        # Try to get from git remote
        try:
            result = subprocess.run(
                ['git', 'config', '--get', 'remote.origin.url'],
                capture_output=True,
                text=True,
                cwd=self.config.project_dir
            )
            if result.returncode == 0:
                url = result.stdout.strip()
                # Convert SSH to HTTPS if needed for SPM
                if url.startswith('git@github.com:'):
                    url = url.replace('git@github.com:', 'https://github.com/')
                return url
        except Exception:
            pass
        return None
