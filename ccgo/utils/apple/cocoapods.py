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
CocoaPods publisher for CCGO.

Handles podspec generation and publishing to CocoaPods trunk or private repos.
"""

import os
import subprocess
import shutil
from pathlib import Path
from typing import Optional, Tuple

from .config import ApplePublishConfig


class CocoaPodsPublisher:
    """Handle CocoaPods publishing workflow."""

    def __init__(self, config: ApplePublishConfig):
        """
        Initialize CocoaPods publisher.

        Args:
            config: Apple publishing configuration
        """
        self.config = config
        self.podspec_path: Optional[Path] = None

    def validate_prerequisites(self) -> Tuple[bool, str]:
        """
        Validate that all prerequisites are met.

        Returns:
            Tuple of (success, error_message)
        """
        # Check if pod CLI is available
        try:
            result = subprocess.run(
                ['pod', '--version'],
                capture_output=True,
                text=True
            )
            if result.returncode != 0:
                return False, "CocoaPods CLI not found. Install with: gem install cocoapods"
        except FileNotFoundError:
            return False, "CocoaPods CLI not found. Install with: gem install cocoapods"

        # Check if XCFramework exists
        xcframework_path = self.config.find_xcframework()
        if not xcframework_path:
            return False, (
                f"XCFramework not found for {self.config.pod_name}. "
                "Please build the project first with: ccgo build ios / ccgo build macos"
            )

        # For trunk publishing, check if authenticated
        if self.config.cocoapods.repo == 'trunk':
            # Check COCOAPODS_TRUNK_TOKEN or pod trunk me
            token = os.environ.get('COCOAPODS_TRUNK_TOKEN')
            if not token:
                try:
                    result = subprocess.run(
                        ['pod', 'trunk', 'me'],
                        capture_output=True,
                        text=True
                    )
                    if 'not yet registered' in result.stderr.lower() or result.returncode != 0:
                        return False, (
                            "Not authenticated with CocoaPods trunk. "
                            "Run: pod trunk register YOUR_EMAIL 'Your Name' --description='My Mac'"
                        )
                except Exception:
                    pass  # Ignore errors, will fail later with better message

        return True, ""

    def generate_podspec(self, output_path: Optional[Path] = None) -> Path:
        """
        Generate podspec file from configuration.

        Args:
            output_path: Optional path to write podspec. Defaults to apple/ directory.

        Returns:
            Path to generated podspec file
        """
        if output_path is None:
            # Default to apple/ directory if it exists, otherwise project root
            apple_dir = self.config.project_dir / 'apple'
            if apple_dir.exists():
                output_path = apple_dir / f'{self.config.pod_name}.podspec'
            else:
                output_path = self.config.project_dir / f'{self.config.pod_name}.podspec'

        # Ensure parent directory exists
        output_path.parent.mkdir(parents=True, exist_ok=True)

        xcframework_path = self.config.find_xcframework()

        # Build podspec content
        lines = [
            "Pod::Spec.new do |s|",
            f"  s.name             = '{self.config.pod_name}'",
            f"  s.version          = '{self.config.version}'",
            f"  s.summary          = '{self._escape_string(self.config.summary)}'",
        ]

        # Description
        if self.config.description and self.config.description != self.config.summary:
            # Multiline description
            lines.append("  s.description      = <<-DESC")
            lines.append(f"    {self._escape_string(self.config.description)}")
            lines.append("  DESC")

        # Homepage - use git URL if no homepage configured
        homepage = self.config.cocoapods.homepage
        if not homepage or homepage == 'https://github.com/user/repo':
            git_url = self._get_git_url()
            if git_url:
                # Remove .git suffix for homepage
                homepage = git_url[:-4] if git_url.endswith('.git') else git_url
            else:
                homepage = 'https://github.com/user/repo'
        lines.append(f"  s.homepage         = '{homepage}'")

        # License
        license_type = self.config.cocoapods.license
        if self.config.cocoapods.license_file:
            lines.append(f"  s.license          = {{ :type => '{license_type}', :file => '{self.config.cocoapods.license_file}' }}")
        else:
            lines.append(f"  s.license          = {{ :type => '{license_type}' }}")

        # Authors
        if self.config.cocoapods.authors:
            if len(self.config.cocoapods.authors) == 1:
                name, email = list(self.config.cocoapods.authors.items())[0]
                if email:
                    lines.append(f"  s.author           = {{ '{name}' => '{email}' }}")
                else:
                    lines.append(f"  s.author           = '{name}'")
            else:
                authors_str = ', '.join([
                    f"'{name}' => '{email}'" if email else f"'{name}' => ''"
                    for name, email in self.config.cocoapods.authors.items()
                ])
                lines.append(f"  s.authors          = {{ {authors_str} }}")

        # Source - for binary distribution
        if self.config.cocoapods.source_url:
            # Remote URL to XCFramework zip
            lines.append(f"  s.source           = {{ :http => '{self.config.cocoapods.source_url}' }}")
        else:
            # Use git tag as source (requires XCFramework to be in repo or hosted)
            git_url = self._get_git_url()
            if git_url:
                # Use 'v' prefix for tags (common convention: v1.0.0)
                lines.append(f"  s.source           = {{ :git => '{git_url}', :tag => 'v' + s.version.to_s }}")
            else:
                lines.append(f"  s.source           = {{ :http => 'https://github.com/user/repo/releases/download/v#{{s.version}}/{self.config.pod_name}.xcframework.zip' }}")

        # Platform deployment targets
        for platform in self.config.platforms:
            min_version = self.config.min_versions.get(platform, '13.0')
            lines.append(f"  s.{platform}.deployment_target = '{min_version}'")

        # Swift version (for mixed Swift/Obj-C projects)
        lines.append("  s.swift_version    = '5.0'")

        # Vendored frameworks
        if xcframework_path:
            xcframework_name = xcframework_path.name
            lines.append(f"  s.vendored_frameworks = '{xcframework_name}'")
        else:
            lines.append(f"  s.vendored_frameworks = '{self.config.pod_name}.xcframework'")

        # Libraries required for C++
        lines.append("  s.libraries        = 'c++'")

        # Pod target xcconfig for C++ support
        lines.append("  s.pod_target_xcconfig = {")
        lines.append("    'CLANG_CXX_LANGUAGE_STANDARD' => 'c++17',")
        lines.append("    'CLANG_CXX_LIBRARY' => 'libc++',")
        lines.append("  }")

        # Static framework option
        if self.config.cocoapods.static_framework:
            lines.append("  s.static_framework = true")

        # Optional metadata
        if self.config.cocoapods.social_media_url:
            lines.append(f"  s.social_media_url = '{self.config.cocoapods.social_media_url}'")
        if self.config.cocoapods.documentation_url:
            lines.append(f"  s.documentation_url = '{self.config.cocoapods.documentation_url}'")
        if self.config.cocoapods.readme:
            lines.append(f"  s.readme = '{self.config.cocoapods.readme}'")
        if self.config.cocoapods.changelog:
            lines.append(f"  s.changelog = '{self.config.cocoapods.changelog}'")

        lines.append("end")

        # Write podspec file
        podspec_content = '\n'.join(lines)
        output_path.write_text(podspec_content)
        self.podspec_path = output_path

        return output_path

    def lint_podspec(self, allow_warnings: bool = True) -> Tuple[bool, str]:
        """
        Validate podspec with pod spec lint.

        Args:
            allow_warnings: Allow warnings to pass

        Returns:
            Tuple of (success, output_message)
        """
        if not self.podspec_path:
            return False, "Podspec not generated yet"

        cmd = ['pod', 'spec', 'lint', str(self.podspec_path)]
        if allow_warnings:
            cmd.append('--allow-warnings')

        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True
            )
            output = result.stdout + result.stderr

            if result.returncode == 0:
                return True, output
            else:
                return False, f"Podspec validation failed:\n{output}"
        except Exception as e:
            return False, f"Failed to run pod spec lint: {e}"

    def publish_trunk(self, allow_warnings: bool = True, skip_lint: bool = False) -> Tuple[bool, str]:
        """
        Publish podspec to CocoaPods trunk.

        Args:
            allow_warnings: Allow warnings to pass
            skip_lint: Skip linting step

        Returns:
            Tuple of (success, output_message)
        """
        if not self.podspec_path:
            return False, "Podspec not generated yet"

        cmd = ['pod', 'trunk', 'push', str(self.podspec_path)]
        if allow_warnings:
            cmd.append('--allow-warnings')
        if skip_lint:
            cmd.append('--skip-import-validation')

        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True
            )
            output = result.stdout + result.stderr

            if result.returncode == 0:
                return True, output
            else:
                return False, f"Trunk push failed:\n{output}"
        except Exception as e:
            return False, f"Failed to run pod trunk push: {e}"

    def publish_private_repo(self, spec_repo_url: str, allow_warnings: bool = True) -> Tuple[bool, str]:
        """
        Publish podspec to a private spec repository.

        Args:
            spec_repo_url: URL to private spec repository
            allow_warnings: Allow warnings to pass

        Returns:
            Tuple of (success, output_message)
        """
        if not self.podspec_path:
            return False, "Podspec not generated yet"

        # First, add the repo if not already added
        repo_name = self._get_repo_name(spec_repo_url)

        # Check if repo exists
        try:
            result = subprocess.run(
                ['pod', 'repo', 'list'],
                capture_output=True,
                text=True
            )
            if repo_name not in result.stdout:
                # Add the repo
                add_result = subprocess.run(
                    ['pod', 'repo', 'add', repo_name, spec_repo_url],
                    capture_output=True,
                    text=True
                )
                if add_result.returncode != 0:
                    return False, f"Failed to add spec repo: {add_result.stderr}"
        except Exception as e:
            return False, f"Failed to check/add spec repo: {e}"

        # Push to private repo
        cmd = ['pod', 'repo', 'push', repo_name, str(self.podspec_path)]
        if allow_warnings:
            cmd.append('--allow-warnings')

        try:
            result = subprocess.run(
                cmd,
                capture_output=True,
                text=True
            )
            output = result.stdout + result.stderr

            if result.returncode == 0:
                return True, output
            else:
                return False, f"Repo push failed:\n{output}"
        except Exception as e:
            return False, f"Failed to run pod repo push: {e}"

    def publish(self, allow_warnings: bool = True) -> Tuple[bool, str]:
        """
        Publish podspec to configured repository.

        Args:
            allow_warnings: Allow warnings to pass

        Returns:
            Tuple of (success, output_message)
        """
        # Generate podspec if not already done
        if not self.podspec_path:
            self.generate_podspec()

        repo = self.config.cocoapods.repo

        if repo == 'trunk':
            return self.publish_trunk(allow_warnings)
        elif repo == 'private' or self.config.cocoapods.spec_repo:
            spec_repo = self.config.cocoapods.spec_repo or repo
            return self.publish_private_repo(spec_repo, allow_warnings)
        else:
            # Assume repo is a URL
            return self.publish_private_repo(repo, allow_warnings)

    def _escape_string(self, s: str) -> str:
        """Escape string for Ruby podspec."""
        return s.replace("'", "\\'").replace("\n", " ")

    def _get_git_url(self) -> Optional[str]:
        """Get git remote URL from current project."""
        try:
            result = subprocess.run(
                ['git', 'config', '--get', 'remote.origin.url'],
                capture_output=True,
                text=True,
                cwd=self.config.project_dir
            )
            if result.returncode == 0:
                url = result.stdout.strip()
                # Convert SSH to HTTPS if needed
                if url.startswith('git@github.com:'):
                    url = url.replace('git@github.com:', 'https://github.com/')
                # Ensure URL ends with .git (CocoaPods requirement)
                if not url.endswith('.git'):
                    url = url + '.git'
                return url
        except Exception:
            pass
        return None

    def _get_repo_name(self, url: str) -> str:
        """Extract repo name from URL for pod repo add."""
        # Extract name from URL like https://github.com/user/MySpecs.git
        parts = url.rstrip('/').rstrip('.git').split('/')
        return parts[-1] if parts else 'private-specs'
