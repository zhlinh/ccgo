"""
Conan configuration handler for CCGO.

Handles Conan package manager configuration from CCGO.toml and environment variables.

Configuration structure (unified field names):
    [publish.conan]
    registry = "local"          # Registry type: local/official/private
    name = "mylib"              # Package name (default: project.name)
    group_id = "myorg"          # User/organization (default: project.group_id last segment)
    version = "1.0.0"           # Package version (default: project.version)
    channel = "stable"          # Release channel (default: stable)
    description = "My library"  # Package description

Field aliases (for backward compatibility):
    - name > package_name > project.name
    - group_id > user > organization > project.group_id (last segment)
"""

import os
import re
from typing import Dict, List, Optional, Any
from dataclasses import dataclass


@dataclass
class ConanDependency:
    """Conan dependency specification."""
    name: str  # Package name (e.g., "zlib")
    version: str  # Version requirement (e.g., "1.2.13", "[>=1.2.0]")
    user: str = ""  # Optional user/organization
    channel: str = ""  # Optional channel

    def to_reference(self) -> str:
        """Convert to Conan package reference string."""
        ref = f"{self.name}/{self.version}"
        if self.user and self.channel:
            ref += f"@{self.user}/{self.channel}"
        elif self.user:
            ref += f"@{self.user}/"
        return ref


class ConanConfig:
    """Handle Conan repository configuration."""

    REGISTRY_TYPES = ['local', 'official', 'private']
    DEFAULT_CHANNEL = 'stable'

    def __init__(self, config: Dict[str, Any]):
        """
        Initialize Conan configuration.

        Args:
            config: Configuration dictionary from CCGO.toml

        Configuration structure:
            [publish.conan]
            registry = "local"
            name = "mylib"
            group_id = "myorg"
            ...
        """
        self.raw_config = config

        # Get Conan-specific publish config
        publish_config = config.get('publish', {})
        self.conan_config = publish_config.get('conan', {})

        # Parse configuration
        # Priority: registry > repository (for unified naming)
        self.registry_type = self.conan_config.get('registry',
            self.conan_config.get('repository', 'local')).lower()
        self.registry_url = self._expand_env(self.conan_config.get('url', ''))
        self.remote_name = self._expand_env(self.conan_config.get('remote_name', ''))

        # Package information
        # Priority: name > package_name > project.name
        self.package_name = self._expand_env(
            self.conan_config.get('name',
                self.conan_config.get('package_name', config.get('project', {}).get('name', 'unknown')))
        )
        self.version = self._expand_env(
            self.conan_config.get('version', config.get('project', {}).get('version', '1.0.0'))
        )
        # Priority: description > project.description
        self.description = self._expand_env(
            self.conan_config.get('description', config.get('project', {}).get('description', ''))
        )

        # User/Organization (for package reference like name/version@user/channel)
        # Priority: group_id > user > organization > project.group_id (last segment)
        # Note: If group_id is explicitly set to "" (empty string), no user/channel will be used
        self.user = self._resolve_user(config)

        # Channel (stable, testing, dev, etc.)
        self.channel = self._expand_env(
            self.conan_config.get('channel', self.DEFAULT_CHANNEL)
        )

        # Authentication
        self.auth_config = self.conan_config.get('auth', {})
        self.credentials = self._parse_credentials()

        # Build options
        self.settings = self.conan_config.get('settings', ['os', 'compiler', 'build_type', 'arch'])
        self.options = self.conan_config.get('options', {})
        self.default_options = self.conan_config.get('default_options', {})

        # Parse dependencies
        self.dependencies = self._parse_dependencies()

        # Additional metadata
        self.license = self._expand_env(
            self.conan_config.get('license', config.get('project', {}).get('license', 'MIT'))
        )
        self.author = self._expand_env(self.conan_config.get('author', ''))
        self.homepage = self._expand_env(
            self.conan_config.get('homepage',
                self.conan_config.get('url', config.get('project', {}).get('repository', '')))
        )

    def _resolve_user(self, config: Dict[str, Any]) -> str:
        """
        Resolve user/organization from configuration.

        Priority:
        1. group_id (if key exists, use value even if empty - allows explicit disable)
        2. user (legacy alias)
        3. organization (legacy alias)
        4. project.group_id last segment (fallback)

        To explicitly disable user/channel, set group_id = "" in config.
        """
        # Check if any explicit key is set (including empty string)
        if 'group_id' in self.conan_config:
            return self._expand_env(self.conan_config['group_id'])
        if 'user' in self.conan_config:
            return self._expand_env(self.conan_config['user'])
        if 'organization' in self.conan_config:
            return self._expand_env(self.conan_config['organization'])

        # Fallback to project.group_id (extract last segment)
        project_group_id = config.get('project', {}).get('group_id', '')
        if project_group_id and '.' in project_group_id:
            return project_group_id.split('.')[-1]
        elif project_group_id:
            return project_group_id
        return ''

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

    def _parse_credentials(self) -> Dict[str, str]:
        """Parse authentication credentials from config and environment."""
        credentials = {}

        # Get credentials from config
        config_creds = self.auth_config.get('credentials', {})

        # Username/password for remote repositories
        username = config_creds.get('username', '')
        password = config_creds.get('password', '')

        # Support environment variable expansion
        username = self._expand_env(username)
        password = self._expand_env(password)

        # Check direct environment variables
        if not username:
            username = os.environ.get('CONAN_LOGIN_USERNAME',
                os.environ.get('CONAN_USERNAME', ''))
        if not password:
            password = os.environ.get('CONAN_LOGIN_PASSWORD',
                os.environ.get('CONAN_PASSWORD', ''))

        credentials['username'] = username
        credentials['password'] = password

        return credentials

    def _parse_dependencies(self) -> List[ConanDependency]:
        """Parse Conan dependencies from config."""
        dependencies = []
        deps_config = self.conan_config.get('dependencies', [])

        for dep in deps_config:
            if isinstance(dep, dict):
                dependencies.append(ConanDependency(
                    name=self._expand_env(dep.get('name', '')),
                    version=self._expand_env(dep.get('version', '')),
                    user=self._expand_env(dep.get('user', '')),
                    channel=self._expand_env(dep.get('channel', '')),
                ))
            elif isinstance(dep, str):
                # Parse string format: "name/version@user/channel"
                parsed = self._parse_reference(dep)
                if parsed:
                    dependencies.append(parsed)

        return dependencies

    def _parse_reference(self, ref: str) -> Optional[ConanDependency]:
        """Parse a Conan package reference string."""
        # Format: name/version[@user/channel]
        if '/' not in ref:
            return None

        if '@' in ref:
            pkg_part, user_part = ref.split('@', 1)
        else:
            pkg_part = ref
            user_part = ''

        name, version = pkg_part.split('/', 1)
        user = ''
        channel = ''

        if user_part:
            if '/' in user_part:
                user, channel = user_part.split('/', 1)
            else:
                user = user_part

        return ConanDependency(
            name=name,
            version=version,
            user=user,
            channel=channel
        )

    def get_package_reference(self, include_user_channel: bool = True) -> str:
        """
        Get the full package reference string.

        Args:
            include_user_channel: Whether to include @user/channel suffix

        Returns:
            Package reference like "name/version" or "name/version@user/channel"
        """
        ref = f"{self.package_name}/{self.version}"
        if include_user_channel and self.user:
            ref += f"@{self.user}/{self.channel}"
        return ref

    def get_remote_name(self) -> str:
        """Get the Conan remote name for upload."""
        if self.remote_name:
            return self.remote_name
        if self.registry_type == 'official':
            return 'conancenter'
        return self.registry_type

    def get_registry_url(self) -> str:
        """Get the registry URL based on configuration."""
        if self.registry_type == 'local':
            return ''  # Local cache, no URL needed
        elif self.registry_type == 'official':
            # ConanCenter is read-only for upload, need Artifactory or similar
            if not self.registry_url:
                return 'https://center.conan.io'
            return self.registry_url
        else:  # private
            if not self.registry_url:
                raise ValueError("Private registry requires 'url' to be specified")
            return self.registry_url

    def is_local_only(self) -> bool:
        """Check if this is local-only mode (no actual remote publishing)."""
        return self.registry_type == 'local'

    def setup_conan_auth(self) -> bool:
        """
        Set up Conan authentication using credentials.

        Returns:
            True if authentication setup successful
        """
        import subprocess

        if not self.credentials.get('username') or not self.credentials.get('password'):
            return True  # No authentication needed

        remote = self.get_remote_name()

        try:
            # Login to remote
            result = subprocess.run(
                ['conan', 'remote', 'login', remote,
                 self.credentials['username'], '-p', self.credentials['password']],
                capture_output=True,
                text=True,
                check=False,
                timeout=30
            )
            return result.returncode == 0
        except Exception as e:
            print(f"Warning: Failed to setup Conan authentication: {e}")
            return False

    def validate(self) -> tuple:
        """
        Validate the configuration.

        Returns:
            Tuple of (is_valid, error_message)
        """
        if self.registry_type not in self.REGISTRY_TYPES:
            return False, f"Invalid registry type: {self.registry_type}. Must be one of {self.REGISTRY_TYPES}"

        if self.registry_type == 'private' and not self.registry_url:
            return False, "Private registry requires 'url' to be specified"

        if not self.package_name:
            return False, "Package name is required"

        if not self.version:
            return False, "Version is required"

        # Validate package name format (lowercase, alphanumeric, underscores, hyphens)
        if not re.match(r'^[a-z][a-z0-9_-]*$', self.package_name):
            return False, f"Invalid package name: {self.package_name}. Must be lowercase, start with letter, contain only alphanumeric, underscores, hyphens"

        # Validate user format if provided
        if self.user and not re.match(r'^[a-z][a-z0-9_-]*$', self.user):
            return False, f"Invalid user/organization: {self.user}. Must be lowercase, start with letter"

        # Check if authentication is needed
        if self.registry_type == 'private':
            if not self.credentials.get('username') or not self.credentials.get('password'):
                return False, "Private registry requires authentication (username/password)"

        return True, ""

    def get_config_summary(self) -> str:
        """Get a summary of the configuration for display."""
        lines = []
        lines.append(f"  Registry Type: {self.registry_type}")

        if self.registry_type == 'local':
            lines.append(f"  Location: ~/.conan2/ (local cache)")
        elif self.registry_type == 'official':
            lines.append(f"  Remote: {self.get_remote_name()}")
        else:
            lines.append(f"  Remote URL: {self.get_registry_url()}")

        # Show package reference
        lines.append(f"  Package: {self.get_package_reference()}")

        if self.user:
            lines.append(f"  User/Org: {self.user}")
            lines.append(f"  Channel: {self.channel}")

        if self.registry_type != 'local':
            lines.append(f"  Username: {'***' if self.credentials.get('username') else 'Not configured'}")
            lines.append(f"  Password: {'***' if self.credentials.get('password') else 'Not configured'}")

        # Show dependencies
        if self.dependencies:
            lines.append(f"  Dependencies: {len(self.dependencies)}")
            for dep in self.dependencies:
                lines.append(f"    - {dep.to_reference()}")

        return '\n'.join(lines)


def load_conan_config(config: Dict[str, Any]) -> ConanConfig:
    """
    Load Conan configuration from CCGO.toml config dict.

    Args:
        config: Configuration dictionary from CCGO.toml

    Returns:
        ConanConfig instance
    """
    return ConanConfig(config)
