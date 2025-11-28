"""
OHPM configuration handler for CCGO.

Handles OHPM (OpenHarmony Package Manager) configuration from CCGO.toml and environment variables.
"""

import os
import re
import json
from typing import Dict, Optional, Any


class OhpmConfig:
    """Handle OHPM repository configuration."""

    REGISTRY_TYPES = ['official', 'private', 'local']

    def __init__(self, config: Dict[str, Any]):
        """
        Initialize OHPM configuration.

        Args:
            config: Configuration dictionary from CCGO.toml
        """
        self.raw_config = config

        # Get OHOS-specific publish config
        publish_config = config.get('publish', {})
        self.ohpm_config = publish_config.get('ohos', {})

        # Parse configuration
        self.registry_type = self.ohpm_config.get('registry', 'official').lower()
        self.registry_url = self._expand_env(self.ohpm_config.get('url', ''))

        # Package information
        self.package_name = self._expand_env(
            self.ohpm_config.get('package_name', config.get('project', {}).get('name', 'unknown'))
        )
        self.version = self._expand_env(
            self.ohpm_config.get('version', config.get('project', {}).get('version', '1.0.0'))
        )
        self.description = self._expand_env(
            self.ohpm_config.get('description', config.get('project', {}).get('description', ''))
        )

        # Organization/scope (for scoped packages like @org/package)
        self.organization = self._expand_env(self.ohpm_config.get('organization', ''))

        # Authentication
        self.auth_config = self.ohpm_config.get('auth', {})
        self.credentials = self._parse_credentials()

        # Publishing options
        self.access = self.ohpm_config.get('access', 'public')  # public or restricted
        self.tag = self.ohpm_config.get('tag', 'latest')
        self.dry_run = self.ohpm_config.get('dry_run', False)

        # oh-package.json5 configuration
        self.oh_package_config = self.ohpm_config.get('oh_package', {})

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

        # Token authentication (preferred for OHPM)
        token = config_creds.get('token', '')
        token = self._expand_env(token)

        # Check environment variables
        if not token:
            # Check OHPM-specific variables first
            token = os.environ.get('OHPM_TOKEN', os.environ.get('OHPM_ACCESS_TOKEN', ''))

        # Username/password authentication (fallback)
        username = config_creds.get('username', '')
        password = config_creds.get('password', '')

        username = self._expand_env(username)
        password = self._expand_env(password)

        # Check environment variables for username/password
        if not username:
            username = os.environ.get('OHPM_USERNAME', '')
        if not password:
            password = os.environ.get('OHPM_PASSWORD', '')

        credentials['token'] = token
        credentials['username'] = username
        credentials['password'] = password

        # Registry-specific credentials
        if self.registry_type == 'private':
            # For private registries, might need additional auth
            credentials['registry_token'] = self._expand_env(
                config_creds.get('registry_token', os.environ.get('OHPM_REGISTRY_TOKEN', ''))
            )

        return credentials

    def get_registry_url(self) -> str:
        """Get the registry URL based on configuration."""
        if self.registry_type == 'official':
            # Official OHPM registry
            if not self.registry_url:
                return 'https://ohpm.openharmony.cn/registry'
            return self.registry_url
        elif self.registry_type == 'local':
            # Local registry for testing
            if not self.registry_url:
                return 'http://localhost:4873'  # Default for local verdaccio
            return self.registry_url
        else:  # private
            if not self.registry_url:
                raise ValueError("Private registry requires 'url' to be specified")
            return self.registry_url

    def generate_oh_package_json5(self) -> str:
        """
        Generate oh-package.json5 content for publishing.

        Returns:
            String content for oh-package.json5 file
        """
        # Build the package configuration
        package_data = {
            "name": self.package_name if not self.organization else f"@{self.organization}/{self.package_name}",
            "version": self.version,
            "description": self.description,
            "main": self.oh_package_config.get('main', 'index.ets'),
            "type": self.oh_package_config.get('type', 'shared'),
            "author": self.oh_package_config.get('author', ''),
            "license": self.oh_package_config.get('license', 'MIT'),
            "dependencies": self.oh_package_config.get('dependencies', {}),
            "devDependencies": self.oh_package_config.get('devDependencies', {}),
        }

        # Add repository information if available
        if self.oh_package_config.get('repository'):
            package_data['repository'] = self.oh_package_config['repository']

        # Add keywords if available
        if self.oh_package_config.get('keywords'):
            package_data['keywords'] = self.oh_package_config['keywords']

        # Add publishConfig for private registries
        if self.registry_type != 'official':
            package_data['publishConfig'] = {
                'registry': self.get_registry_url()
            }
            if self.access:
                package_data['publishConfig']['access'] = self.access

        # Convert to JSON5 format (similar to JSON but with some relaxed rules)
        # For simplicity, we'll use JSON with proper indentation
        return json.dumps(package_data, indent=2, ensure_ascii=False)

    def get_ohpm_command_args(self) -> list:
        """Get command line arguments for ohpm publish command."""
        args = []

        # Add registry if not official
        if self.registry_type != 'official':
            args.extend(['--registry', self.get_registry_url()])

        # Add access level
        if self.access:
            args.extend(['--access', self.access])

        # Add tag
        if self.tag and self.tag != 'latest':
            args.extend(['--tag', self.tag])

        # Add dry-run if enabled
        if self.dry_run:
            args.append('--dry-run')

        return args

    def setup_ohpm_auth(self) -> bool:
        """
        Set up OHPM authentication using credentials.

        Returns:
            True if authentication setup successful
        """
        registry_url = self.get_registry_url()

        # Set authentication based on available credentials
        if self.credentials.get('token'):
            # Use token authentication
            cmd = f"ohpm config set {registry_url}:_authToken {self.credentials['token']}"
            result = os.system(cmd)
            return result == 0

        elif self.credentials.get('username') and self.credentials.get('password'):
            # Login with username/password
            import subprocess

            # Create login process
            process = subprocess.Popen(
                ['ohpm', 'login', '--registry', registry_url],
                stdin=subprocess.PIPE,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                text=True
            )

            # Provide username and password
            stdout, stderr = process.communicate(
                input=f"{self.credentials['username']}\n{self.credentials['password']}\n"
            )

            return process.returncode == 0

        # No authentication configured
        return True  # Assume public registry or already authenticated

    def validate(self) -> tuple[bool, str]:
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

        # Check if authentication is needed
        if self.registry_type == 'private':
            if not self.credentials.get('token') and not (
                self.credentials.get('username') and self.credentials.get('password')
            ):
                return False, "Private registry requires authentication (token or username/password)"

        return True, ""

    def get_config_summary(self) -> str:
        """Get a summary of the configuration for display."""
        lines = []
        lines.append(f"  Registry Type: {self.registry_type}")

        if self.registry_type == 'official':
            lines.append(f"  Registry: Official OHPM Registry")
        else:
            lines.append(f"  Registry URL: {self.get_registry_url()}")

        if self.organization:
            lines.append(f"  Package: @{self.organization}/{self.package_name}")
        else:
            lines.append(f"  Package: {self.package_name}")

        lines.append(f"  Version: {self.version}")
        lines.append(f"  Access: {self.access}")

        if self.credentials.get('token'):
            lines.append(f"  Authentication: Token (***)")
        elif self.credentials.get('username'):
            lines.append(f"  Username: {self.credentials['username']}")
            lines.append(f"  Password: ***")
        else:
            lines.append(f"  Authentication: None (public registry)")

        if self.dry_run:
            lines.append(f"  Mode: DRY RUN (no actual publishing)")

        return '\n'.join(lines)