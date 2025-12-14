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
class CocoaPodsDependency:
    """CocoaPods dependency specification."""
    name: str
    version: str = ""  # Version requirement (e.g., "~> 1.0", ">= 1.0.0")
    git: str = ""  # Git URL for git-based dependencies
    branch: str = ""  # Git branch
    tag: str = ""  # Git tag
    commit: str = ""  # Git commit


@dataclass
class SPMDependency:
    """SPM dependency specification."""
    name: str
    url: str = ""  # Git URL for remote packages
    path: str = ""  # Local path for local packages
    from_version: str = ""  # Minimum version (from: "1.0.0")
    up_to_next_major: str = ""  # Up to next major version
    up_to_next_minor: str = ""  # Up to next minor version
    exact: str = ""  # Exact version
    branch: str = ""  # Git branch
    revision: str = ""  # Git revision/commit


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
    dependencies: List[CocoaPodsDependency] = field(default_factory=list)


@dataclass
class SPMSettings:
    """Swift Package Manager configuration."""
    enabled: bool = True
    git_url: str = ""  # Package git URL
    package_name: str = ""  # Override package name
    library_name: str = ""  # Override library name
    xcframework_url: str = ""  # URL to hosted XCFramework zip
    use_local_path: bool = False  # Use local path instead of URL
    dependencies: List[SPMDependency] = field(default_factory=list)


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

        # Parse dependencies
        dependencies = []
        deps_config = cocoa_config.get('dependencies', [])
        for dep in deps_config:
            if isinstance(dep, dict):
                dependencies.append(CocoaPodsDependency(
                    name=self._expand_env(dep.get('name', '')),
                    version=self._expand_env(dep.get('version', '')),
                    git=self._expand_env(dep.get('git', '')),
                    branch=self._expand_env(dep.get('branch', '')),
                    tag=self._expand_env(dep.get('tag', '')),
                    commit=self._expand_env(dep.get('commit', '')),
                ))

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
            dependencies=dependencies,
        )

    def _parse_spm_settings(self) -> SPMSettings:
        """Parse SPM-specific settings from config."""
        spm_config = self.apple_config.get('spm', {})

        # Parse dependencies
        dependencies = []
        deps_config = spm_config.get('dependencies', [])
        for dep in deps_config:
            if isinstance(dep, dict):
                dependencies.append(SPMDependency(
                    name=self._expand_env(dep.get('name', '')),
                    url=self._expand_env(dep.get('url', '')),
                    path=self._expand_env(dep.get('path', '')),
                    from_version=self._expand_env(dep.get('from', '')),
                    up_to_next_major=self._expand_env(dep.get('up_to_next_major', '')),
                    up_to_next_minor=self._expand_env(dep.get('up_to_next_minor', '')),
                    exact=self._expand_env(dep.get('exact', '')),
                    branch=self._expand_env(dep.get('branch', '')),
                    revision=self._expand_env(dep.get('revision', '')),
                ))

        return SPMSettings(
            enabled=spm_config.get('enabled', True),
            git_url=self._expand_env(spm_config.get('git_url', '')),
            package_name=self._expand_env(spm_config.get('package_name', self.pod_name)),
            library_name=self._expand_env(spm_config.get('library_name', self.pod_name)),
            xcframework_url=self._expand_env(spm_config.get('xcframework_url', '')),
            use_local_path=spm_config.get('use_local_path', False),
            dependencies=dependencies,
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

        # Check various possible locations - all platforms now use same structure
        possible_paths = [
            # cmake_build directory - new unified structure: {static|shared}/out/
            self.project_dir / 'cmake_build' / cmake_platform / 'static' / 'out' / f'{self.pod_name}.xcframework',
            self.project_dir / 'cmake_build' / cmake_platform / 'static' / 'out' / f'{self.pod_name}.framework',
            # cmake_build directory - legacy out/static/ structure (for backward compatibility)
            self.project_dir / 'cmake_build' / cmake_platform / 'out' / 'static' / f'{self.pod_name}.xcframework',
            self.project_dir / 'cmake_build' / cmake_platform / 'out' / 'static' / f'{self.pod_name}.framework',
            self.project_dir / 'cmake_build' / cmake_platform / 'out' / f'{self.pod_name}.xcframework',
            self.project_dir / 'cmake_build' / cmake_platform / 'out' / f'{self.pod_name}.framework',
            # Legacy Darwin.out structure (for backward compatibility)
            self.project_dir / 'cmake_build' / cmake_platform / 'Darwin.out' / 'static' / f'{self.pod_name}.xcframework',
            self.project_dir / 'cmake_build' / cmake_platform / 'Darwin.out' / 'static' / f'{self.pod_name}.framework',
            self.project_dir / 'cmake_build' / cmake_platform / 'Darwin.out' / f'{self.pod_name}.xcframework',
            self.project_dir / 'cmake_build' / cmake_platform / 'Darwin.out' / f'{self.pod_name}.framework',
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
