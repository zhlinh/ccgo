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
Apple publishing configuration handler for CCGO.

Handles CocoaPods and SPM configuration from CCGO.toml and environment variables.
"""

import os
import re
from pathlib import Path
from typing import Dict, List, Optional, Any, Tuple
from dataclasses import dataclass, field


@dataclass
class CocoaPodsSettings:
    """CocoaPods-specific configuration."""
    enabled: bool = True
    repo: str = "trunk"  # "trunk" or private repo URL
    spec_repo: str = ""  # For private spec repos
    license: str = "MIT"
    license_file: str = ""
    homepage: str = ""
    source_url: str = ""  # URL to hosted XCFramework
    authors: Dict[str, str] = field(default_factory=dict)
    social_media_url: str = ""
    readme: str = ""
    changelog: str = ""
    documentation_url: str = ""
    static_framework: bool = True


@dataclass
class SPMSettings:
    """Swift Package Manager configuration."""
    enabled: bool = True
    git_url: str = ""  # Package git URL
    package_name: str = ""  # Override package name
    library_name: str = ""  # Override library name
    xcframework_url: str = ""  # URL to hosted XCFramework zip
    use_local_path: bool = False  # Use local path instead of URL


class ApplePublishConfig:
    """Handle Apple platform publishing configuration."""

    SUPPORTED_PLATFORMS = ['ios', 'macos', 'tvos', 'watchos']

    DEFAULT_MIN_VERSIONS = {
        'ios': '13.0',
        'macos': '10.15',
        'tvos': '13.0',
        'watchos': '6.0',
    }

    def __init__(self, config: Dict[str, Any], project_dir: str = "."):
        """
        Initialize Apple publishing configuration.

        Args:
            config: Configuration dictionary from CCGO.toml
            project_dir: Project directory path
        """
        self.raw_config = config
        self.project_dir = Path(project_dir).resolve()

        # Get apple-specific publish config
        publish_config = config.get('publish', {})
        self.apple_config = publish_config.get('apple', {})

        # Project information
        project_config = config.get('project', {})
        self.pod_name = self._expand_env(
            self.apple_config.get('pod_name', project_config.get('name', 'Unknown'))
        )
        self.version = self._expand_env(
            self.apple_config.get('version', project_config.get('version', '1.0.0'))
        )
        self.summary = self._expand_env(
            self.apple_config.get('summary', project_config.get('description', f'{self.pod_name} library'))
        )
        self.description = self._expand_env(
            self.apple_config.get('description', self.summary)
        )

        # Platform configuration
        platforms_config = self.apple_config.get('platforms', ['ios', 'macos'])
        if platforms_config == 'all':
            self.platforms = self.SUPPORTED_PLATFORMS.copy()
        elif isinstance(platforms_config, str):
            self.platforms = [p.strip().lower() for p in platforms_config.split(',')]
        else:
            self.platforms = [p.lower() for p in platforms_config]

        # Minimum versions
        self.min_versions = {}
        for platform in self.SUPPORTED_PLATFORMS:
            version_key = f'min_{platform}_version'
            self.min_versions[platform] = self._expand_env(
                self.apple_config.get(version_key, self.DEFAULT_MIN_VERSIONS.get(platform, '13.0'))
            )

        # Parse CocoaPods settings
        self.cocoapods = self._parse_cocoapods_settings()

        # Parse SPM settings
        self.spm = self._parse_spm_settings()

    def _expand_env(self, value: str) -> str:
        """
        Expand environment variables in configuration values.

        Supports ${VAR_NAME} and $VAR_NAME syntax.
        """
        if not isinstance(value, str):
            return value

        # Pattern for ${VAR_NAME}
        pattern1 = re.compile(r'\$\{([^}]+)\}')
        value = pattern1.sub(lambda m: os.environ.get(m.group(1), m.group(0)), value)

        # Pattern for $VAR_NAME
        pattern2 = re.compile(r'\$([A-Za-z_][A-Za-z0-9_]*)')
        value = pattern2.sub(lambda m: os.environ.get(m.group(1), m.group(0)), value)

        return value

    def _parse_cocoapods_settings(self) -> CocoaPodsSettings:
        """Parse CocoaPods-specific settings from config."""
        cocoa_config = self.apple_config.get('cocoapods', {})

        # Get author information
        authors = cocoa_config.get('authors', {})
        if not authors:
            # Try to get from git config or project config
            author_name = self._expand_env(
                self.raw_config.get('project', {}).get('author', os.environ.get('USER', 'Developer'))
            )
            author_email = self._expand_env(
                self.raw_config.get('project', {}).get('email', '')
            )
            if author_name:
                authors = {author_name: author_email}

        return CocoaPodsSettings(
            enabled=cocoa_config.get('enabled', True),
            repo=self._expand_env(cocoa_config.get('repo', 'trunk')),
            spec_repo=self._expand_env(cocoa_config.get('spec_repo', '')),
            license=self._expand_env(cocoa_config.get('license', 'MIT')),
            license_file=self._expand_env(cocoa_config.get('license_file', '')),
            homepage=self._expand_env(cocoa_config.get('homepage', '')),
            source_url=self._expand_env(cocoa_config.get('source_url', '')),
            authors=authors,
            social_media_url=self._expand_env(cocoa_config.get('social_media_url', '')),
            readme=self._expand_env(cocoa_config.get('readme', '')),
            changelog=self._expand_env(cocoa_config.get('changelog', '')),
            documentation_url=self._expand_env(cocoa_config.get('documentation_url', '')),
            static_framework=cocoa_config.get('static_framework', True),
        )

    def _parse_spm_settings(self) -> SPMSettings:
        """Parse SPM-specific settings from config."""
        spm_config = self.apple_config.get('spm', {})

        return SPMSettings(
            enabled=spm_config.get('enabled', True),
            git_url=self._expand_env(spm_config.get('git_url', '')),
            package_name=self._expand_env(spm_config.get('package_name', self.pod_name)),
            library_name=self._expand_env(spm_config.get('library_name', self.pod_name)),
            xcframework_url=self._expand_env(spm_config.get('xcframework_url', '')),
            use_local_path=spm_config.get('use_local_path', False),
        )

    def get_xcframework_path(self, platform: str = 'ios') -> Optional[Path]:
        """
        Get the path to XCFramework for the specified platform.

        Args:
            platform: Platform name (ios, macos, tvos, watchos)

        Returns:
            Path to XCFramework or None if not found
        """
        # Normalize platform name
        platform_lower = platform.lower()

        # Map platform to cmake_build directory name
        cmake_platform_map = {
            'ios': 'iOS',
            'macos': 'macOS',
            'tvos': 'tvOS',
            'watchos': 'watchOS',
        }
        cmake_platform = cmake_platform_map.get(platform_lower, platform.capitalize())

        # Check various possible locations
        possible_paths = [
            # cmake_build directory (primary build output)
            self.project_dir / 'cmake_build' / cmake_platform / 'Darwin.out' / f'{self.pod_name}.xcframework',
            # target directory (CI build output) - debug
            self.project_dir / 'target' / 'debug' / platform_lower / f'{self.pod_name}.xcframework',
            # target directory (CI build output) - release
            self.project_dir / 'target' / 'release' / platform_lower / f'{self.pod_name}.xcframework',
            # Legacy target directory (flat structure)
            self.project_dir / 'target' / platform_lower / f'{self.pod_name}.xcframework',
            # Direct xcframework in project root
            self.project_dir / f'{self.pod_name}.xcframework',
        ]

        # For iOS, also check iOS-specific paths
        if platform_lower == 'ios':
            possible_paths.extend([
                self.project_dir / 'target' / 'debug' / 'ios' / f'{self.pod_name.upper()}_IOS_SDK' / f'{self.pod_name}.xcframework',
                self.project_dir / 'target' / 'release' / 'ios' / f'{self.pod_name.upper()}_IOS_SDK' / f'{self.pod_name}.xcframework',
            ])

        # For macOS
        if platform_lower == 'macos':
            possible_paths.extend([
                self.project_dir / 'target' / 'debug' / 'macos' / f'{self.pod_name.upper()}_MACOS_SDK' / f'{self.pod_name}.xcframework',
                self.project_dir / 'target' / 'release' / 'macos' / f'{self.pod_name.upper()}_MACOS_SDK' / f'{self.pod_name}.xcframework',
            ])

        # For tvOS
        if platform_lower == 'tvos':
            possible_paths.extend([
                self.project_dir / 'target' / 'debug' / 'tvos' / f'{self.pod_name.upper()}_TVOS_SDK' / f'{self.pod_name}.xcframework',
                self.project_dir / 'target' / 'release' / 'tvos' / f'{self.pod_name.upper()}_TVOS_SDK' / f'{self.pod_name}.xcframework',
            ])

        # For watchOS
        if platform_lower == 'watchos':
            possible_paths.extend([
                self.project_dir / 'target' / 'debug' / 'watchos' / f'{self.pod_name.upper()}_WATCHOS_SDK' / f'{self.pod_name}.xcframework',
                self.project_dir / 'target' / 'release' / 'watchos' / f'{self.pod_name.upper()}_WATCHOS_SDK' / f'{self.pod_name}.xcframework',
            ])

        for path in possible_paths:
            if path.exists():
                return path

        return None

    def find_xcframework(self) -> Optional[Path]:
        """
        Find the XCFramework file in the project.

        Returns:
            Path to XCFramework or None if not found
        """
        # Try each platform in order of priority
        for platform in ['ios', 'macos', 'tvos', 'watchos']:
            if platform in self.platforms:
                path = self.get_xcframework_path(platform)
                if path:
                    return path

        # Search cmake_build directory
        cmake_build_dir = self.project_dir / 'cmake_build'
        if cmake_build_dir.exists():
            for xcfw in cmake_build_dir.rglob('*.xcframework'):
                return xcfw

        # Also search target directory recursively
        target_dir = self.project_dir / 'target'
        if target_dir.exists():
            for xcfw in target_dir.rglob('*.xcframework'):
                return xcfw

        return None

    def validate(self) -> Tuple[bool, List[str]]:
        """
        Validate the configuration.

        Returns:
            Tuple of (is_valid, list of error messages)
        """
        errors = []

        # Check required fields
        if not self.pod_name:
            errors.append("pod_name is required")

        if not self.version:
            errors.append("version is required")

        # Validate platforms
        for platform in self.platforms:
            if platform not in self.SUPPORTED_PLATFORMS:
                errors.append(f"Unsupported platform: {platform}")

        # Validate CocoaPods settings if enabled
        if self.cocoapods.enabled:
            if self.cocoapods.repo not in ['trunk', 'private'] and not self.cocoapods.repo.startswith('http'):
                if self.cocoapods.repo != 'trunk':
                    errors.append(f"Invalid CocoaPods repo: {self.cocoapods.repo}")

            if self.cocoapods.repo == 'private' and not self.cocoapods.spec_repo:
                errors.append("Private CocoaPods repo requires spec_repo URL")

        # Validate SPM settings if enabled
        if self.spm.enabled:
            if not self.spm.use_local_path and not self.spm.xcframework_url:
                # SPM remote distribution requires URL
                pass  # This is OK, can use local path

        return len(errors) == 0, errors

    def get_config_summary(self) -> str:
        """Get a summary of the configuration for display."""
        lines = [
            f"  Pod Name: {self.pod_name}",
            f"  Version: {self.version}",
            f"  Platforms: {', '.join(self.platforms)}",
        ]

        # Platform versions
        for platform in self.platforms:
            lines.append(f"    {platform}: {self.min_versions.get(platform, 'N/A')}")

        # CocoaPods settings
        if self.cocoapods.enabled:
            lines.append(f"  CocoaPods: Enabled")
            lines.append(f"    Repo: {self.cocoapods.repo}")
            if self.cocoapods.homepage:
                lines.append(f"    Homepage: {self.cocoapods.homepage}")
        else:
            lines.append(f"  CocoaPods: Disabled")

        # SPM settings
        if self.spm.enabled:
            lines.append(f"  SPM: Enabled")
            if self.spm.git_url:
                lines.append(f"    Git URL: {self.spm.git_url}")
            if self.spm.xcframework_url:
                lines.append(f"    XCFramework URL: {self.spm.xcframework_url}")
        else:
            lines.append(f"  SPM: Disabled")

        return '\n'.join(lines)
