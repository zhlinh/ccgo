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
from typing import Dict, Optional, Tuple

from .config import ApplePublishConfig


class CocoaPodsPublisher:
    """Handle CocoaPods publishing workflow."""

    # CocoaPods uses different platform names than our internal names
    # Our internal: ios, macos, tvos, watchos
    # CocoaPods: ios, osx, tvos, watchos
    PLATFORM_TO_COCOAPODS = {
        'ios': 'ios',
        'macos': 'osx',
        'tvos': 'tvos',
        'watchos': 'watchos',
    }

    def __init__(self, config: ApplePublishConfig):
        """
        Initialize CocoaPods publisher.

        Args:
            config: Apple publishing configuration
        """
        self.config = config
        self.podspec_path: Optional[Path] = None
        # Track framework types per platform (set by create_cocoapods_zip)
        # Maps platform -> 'xcframework' or 'framework'
        self.platform_framework_types: Dict[str, str] = {}

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

    def create_cocoapods_zip(self, output_dir: Optional[Path] = None) -> Optional[Path]:
        """
        Create a CocoaPods-compatible xcframework zip from build output.

        The SDK zip structure is:
            PROJECTNAME_IOS_SDK-x.x.x.zip/lib/ios/static/project.xcframework/
            PROJECTNAME_MACOS_SDK-x.x.x.zip/lib/macos/static/project.xcframework/

        CocoaPods expects (platform-specific):
            project.xcframework.zip/
                ios/project.xcframework/
                macos/project.xcframework/
                LICENSE

        Args:
            output_dir: Output directory for the zip. Defaults to apple/ directory.

        Returns:
            Path to created zip file, or None if failed
        """
        import zipfile
        import tempfile

        if output_dir is None:
            apple_dir = self.config.project_dir / 'apple'
            if apple_dir.exists():
                output_dir = apple_dir
            else:
                output_dir = self.config.project_dir

        target_dir = self.config.project_dir / 'target'

        # Platform name mapping for SDK zip files
        platform_sdk_names = {
            'ios': 'IOS',
            'macos': 'MACOS',
            'tvos': 'TVOS',
            'watchos': 'WATCHOS',
        }

        # Find SDK zips for each configured platform
        found_platforms = {}
        for build_type in ['release', 'debug']:
            for platform in self.config.platforms:
                if platform in found_platforms:
                    continue
                platform_dir = target_dir / build_type / platform
                if platform_dir.exists():
                    sdk_name = platform_sdk_names.get(platform, platform.upper())
                    # Find SDK zip (e.g., CCGONOWDEP_IOS_SDK-1.0.0-release.zip)
                    for f in platform_dir.glob(f'*_{sdk_name}_SDK-*.zip'):
                        if 'SYMBOLS' not in f.name:
                            found_platforms[platform] = f
                            break

        if not found_platforms:
            print(f"ERROR: SDK zip not found in {target_dir}")
            return None

        print(f"[CocoaPods] Found SDK zips for platforms: {', '.join(found_platforms.keys())}")

        # Create CocoaPods-compatible zip with platform-specific directories
        cocoapods_zip_path = output_dir / f'{self.config.pod_name}.xcframework.zip'

        with tempfile.TemporaryDirectory() as temp_dir:
            temp_path = Path(temp_dir)

            with zipfile.ZipFile(cocoapods_zip_path, 'w', zipfile.ZIP_DEFLATED) as cocoapods_zip:
                # Process each platform's SDK zip
                for platform, sdk_zip_path in found_platforms.items():
                    print(f"[CocoaPods] Processing {platform}: {sdk_zip_path.name}")

                    # Extract SDK zip to temp directory
                    platform_temp = temp_path / platform
                    platform_temp.mkdir(parents=True, exist_ok=True)

                    with zipfile.ZipFile(sdk_zip_path, 'r') as sdk_zip:
                        sdk_zip.extractall(platform_temp)

                    # Find xcframework or framework in extracted content (prefer static xcframework)
                    framework_path = None
                    is_xcframework = False

                    # First try to find xcframework (preferred)
                    for xcfw in platform_temp.rglob('*.xcframework'):
                        # Prefer static over shared
                        if 'static' in str(xcfw):
                            framework_path = xcfw
                            is_xcframework = True
                            break
                        elif framework_path is None:
                            framework_path = xcfw
                            is_xcframework = True

                    # If no xcframework, try to find regular framework
                    if not framework_path:
                        for fw in platform_temp.rglob('*.framework'):
                            # Prefer static over shared
                            if 'static' in str(fw):
                                framework_path = fw
                                break
                            elif framework_path is None:
                                framework_path = fw

                    if not framework_path:
                        print(f"[CocoaPods] Warning: framework not found for {platform}")
                        continue

                    # Determine output framework name based on type
                    if is_xcframework:
                        out_name = f'{self.config.pod_name}.xcframework'
                        self.platform_framework_types[platform] = 'xcframework'
                    else:
                        out_name = f'{self.config.pod_name}.framework'
                        self.platform_framework_types[platform] = 'framework'

                    # Add framework to zip under platform directory
                    for root, dirs, files in os.walk(framework_path):
                        for file in files:
                            file_path = Path(root) / file
                            # Structure: platform/pod_name.xcframework/... or platform/pod_name.framework/...
                            arcname = f'{platform}/{out_name}/{file_path.relative_to(framework_path)}'
                            cocoapods_zip.write(file_path, arcname)

                # Add LICENSE file if exists
                license_path = self.config.project_dir / 'LICENSE'
                if license_path.exists():
                    cocoapods_zip.write(license_path, 'LICENSE')
                else:
                    # Check parent directory
                    parent_license = self.config.project_dir.parent / 'LICENSE'
                    if parent_license.exists():
                        cocoapods_zip.write(parent_license, 'LICENSE')

        print(f"[CocoaPods] Created: {cocoapods_zip_path}")
        return cocoapods_zip_path

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
        license_file = self.config.cocoapods.license_file
        # Auto-detect LICENSE file if not configured
        if not license_file:
            for name in ['LICENSE', 'LICENSE.md', 'LICENSE.txt', 'License']:
                if (self.config.project_dir / name).exists():
                    license_file = name
                    break
                # Check parent directory (for mono-repo structure)
                if (self.config.project_dir.parent / name).exists():
                    license_file = name
                    break
        if license_file:
            lines.append(f"  s.license          = {{ :type => '{license_type}', :file => '{license_file}' }}")
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

        # Source - prefer binary distribution via HTTP
        if self.config.cocoapods.source_url:
            # Remote URL to XCFramework zip (configured)
            lines.append(f"  s.source           = {{ :http => '{self.config.cocoapods.source_url}' }}")
        else:
            # Generate HTTP source URL from git remote for binary distribution
            git_url = self._get_git_url()
            if git_url:
                # Convert git URL to release download URL
                # https://github.com/user/repo.git -> https://github.com/user/repo
                http_base = git_url[:-4] if git_url.endswith('.git') else git_url
                # Binary distribution: download xcframework zip from GitHub releases
                lines.append("  # Binary distribution: download xcframework zip from GitHub releases")
                lines.append(f"  s.source           = {{ :http => '{http_base}/releases/download/v' + s.version.to_s + '/{self.config.pod_name}.xcframework.zip' }}")
                # Add commented git source for reference
                lines.append("  # For git source (requires xcframework committed to repo):")
                lines.append(f"  # s.source           = {{ :git => '{git_url}', :tag => 'v' + s.version.to_s }}")
            else:
                lines.append(f"  s.source           = {{ :http => 'https://github.com/user/repo/releases/download/v' + s.version.to_s + '/{self.config.pod_name}.xcframework.zip' }}")

        # Platform deployment targets
        for platform in self.config.platforms:
            cocoapods_platform = self.PLATFORM_TO_COCOAPODS.get(platform, platform)
            min_version = self.config.min_versions.get(platform, '13.0')
            lines.append(f"  s.{cocoapods_platform}.deployment_target = '{min_version}'")

        # Swift version (for mixed Swift/Obj-C projects)
        lines.append("  s.swift_version    = '5.0'")

        # Vendored frameworks - platform-specific paths
        # Each platform has its own xcframework/framework in the zip
        for platform in self.config.platforms:
            cocoapods_platform = self.PLATFORM_TO_COCOAPODS.get(platform, platform)
            # Use tracked framework type if available, default to xcframework
            fw_type = self.platform_framework_types.get(platform, 'xcframework')
            lines.append(f"  s.{cocoapods_platform}.vendored_frameworks = '{platform}/{self.config.pod_name}.{fw_type}'")

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

        # Dependencies
        if self.config.cocoapods.dependencies:
            lines.append("")
            lines.append("  # Dependencies")
            for dep in self.config.cocoapods.dependencies:
                if dep.git:
                    # Git-based dependency
                    git_options = [f":git => '{dep.git}'"]
                    if dep.branch:
                        git_options.append(f":branch => '{dep.branch}'")
                    elif dep.tag:
                        git_options.append(f":tag => '{dep.tag}'")
                    elif dep.commit:
                        git_options.append(f":commit => '{dep.commit}'")
                    lines.append(f"  s.dependency '{dep.name}', {{ {', '.join(git_options)} }}")
                elif dep.version:
                    # Version-based dependency
                    lines.append(f"  s.dependency '{dep.name}', '{dep.version}'")
                else:
                    # No version specified
                    lines.append(f"  s.dependency '{dep.name}'")

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

    def upload_to_github_release(self, zip_path: Path, version: Optional[str] = None) -> Tuple[bool, str]:
        """
        Upload xcframework.zip to GitHub releases using gh CLI.

        Args:
            zip_path: Path to the xcframework.zip file
            version: Version tag (e.g., "1.0.0"). Defaults to config version.

        Returns:
            Tuple of (success, message)
        """
        if not zip_path.exists():
            return False, f"File not found: {zip_path}"

        version = version or self.config.version
        tag = f"v{version}"

        # Check if gh CLI is available
        try:
            result = subprocess.run(
                ['gh', '--version'],
                capture_output=True,
                text=True
            )
            if result.returncode != 0:
                return False, "GitHub CLI (gh) not found. Install from: https://cli.github.com/"
        except FileNotFoundError:
            return False, "GitHub CLI (gh) not found. Install from: https://cli.github.com/"

        # Check if authenticated
        try:
            result = subprocess.run(
                ['gh', 'auth', 'status'],
                capture_output=True,
                text=True,
                cwd=self.config.project_dir
            )
            if result.returncode != 0:
                return False, "Not authenticated with GitHub CLI. Run: gh auth login"
        except Exception as e:
            return False, f"Failed to check gh auth status: {e}"

        # Create git tag if not exists
        try:
            # Check if tag exists locally
            result = subprocess.run(
                ['git', 'tag', '-l', tag],
                capture_output=True,
                text=True,
                cwd=self.config.project_dir
            )
            if tag not in result.stdout:
                # Create tag
                subprocess.run(
                    ['git', 'tag', tag],
                    capture_output=True,
                    text=True,
                    cwd=self.config.project_dir
                )
                print(f"[GitHub] Created tag: {tag}")

            # Push tag to remote
            result = subprocess.run(
                ['git', 'push', 'origin', tag],
                capture_output=True,
                text=True,
                cwd=self.config.project_dir
            )
            if result.returncode != 0 and 'already exists' not in result.stderr:
                # Tag might already exist on remote, try to continue
                pass
        except Exception as e:
            return False, f"Failed to create/push tag: {e}"

        # Check if release exists, if not create it
        try:
            result = subprocess.run(
                ['gh', 'release', 'view', tag],
                capture_output=True,
                text=True,
                cwd=self.config.project_dir
            )
            if result.returncode != 0:
                # Create release
                print(f"[GitHub] Creating release for {tag}...")
                result = subprocess.run(
                    ['gh', 'release', 'create', tag,
                     '--title', f"Release {version}",
                     '--notes', f"Release {version}\n\nGenerated by ccgo publish apple"],
                    capture_output=True,
                    text=True,
                    cwd=self.config.project_dir
                )
                if result.returncode != 0:
                    return False, f"Failed to create release: {result.stderr}"
                print(f"[GitHub] Created release: {tag}")
        except Exception as e:
            return False, f"Failed to check/create release: {e}"

        # Upload file to release (delete existing first if present)
        try:
            # Try to delete existing asset first
            subprocess.run(
                ['gh', 'release', 'delete-asset', tag, zip_path.name, '-y'],
                capture_output=True,
                text=True,
                cwd=self.config.project_dir
            )
        except:
            pass  # Ignore if asset doesn't exist

        # Upload new asset
        try:
            print(f"[GitHub] Uploading {zip_path.name} to release {tag}...")
            result = subprocess.run(
                ['gh', 'release', 'upload', tag, str(zip_path), '--clobber'],
                capture_output=True,
                text=True,
                cwd=self.config.project_dir
            )
            if result.returncode != 0:
                return False, f"Failed to upload: {result.stderr}"

            # Get the download URL
            git_url = self._get_git_url()
            if git_url:
                http_base = git_url[:-4] if git_url.endswith('.git') else git_url
                download_url = f"{http_base}/releases/download/{tag}/{zip_path.name}"
                return True, f"Uploaded successfully!\nDownload URL: {download_url}"
            return True, "Uploaded successfully!"

        except Exception as e:
            return False, f"Failed to upload: {e}"
