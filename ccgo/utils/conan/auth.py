"""
Authentication module for Canon platform.

Supports multiple authentication methods:
- Token-based authentication
- OAuth2 authentication
- Basic authentication
"""

import os
import base64
import json
import time
from typing import Dict, Optional, Tuple
from urllib.parse import urljoin
import requests


class CanonAuth:
    """Handle authentication for Canon platform."""

    AUTH_METHODS = ['token', 'oauth2', 'basic']

    def __init__(self, config: Dict):
        """
        Initialize Canon authentication.

        Args:
            config: Authentication configuration dictionary containing:
                - method: Authentication method (token/oauth2/basic)
                - registry: Canon registry URL
                - credentials: Method-specific credentials
        """
        self.method = config.get('method', 'token')
        self.registry = config.get('registry', '')
        self.credentials = config.get('credentials', {})

        if self.method not in self.AUTH_METHODS:
            raise ValueError(f"Unsupported auth method: {self.method}. "
                           f"Supported: {', '.join(self.AUTH_METHODS)}")

        # Cache for OAuth2 tokens
        self._token_cache = None
        self._token_expiry = 0

    def get_headers(self) -> Dict[str, str]:
        """
        Get authentication headers based on configured method.

        Returns:
            Dictionary of HTTP headers for authentication
        """
        if self.method == 'token':
            return self._get_token_headers()
        elif self.method == 'oauth2':
            return self._get_oauth2_headers()
        elif self.method == 'basic':
            return self._get_basic_headers()
        else:
            return {}

    def _get_token_headers(self) -> Dict[str, str]:
        """Get headers for token authentication."""
        # Try environment variable first
        token = os.environ.get('CANON_TOKEN')

        if not token:
            # Try credentials config
            token = self.credentials.get('token')

        if not token:
            # Try reading from file
            token_file = self.credentials.get('token_file')
            if token_file and os.path.exists(token_file):
                with open(token_file, 'r') as f:
                    token = f.read().strip()

        if not token:
            raise ValueError("No Canon token found. Set CANON_TOKEN environment variable "
                           "or configure in CCGO.toml")

        return {
            'Authorization': f'Bearer {token}',
            'Content-Type': 'application/json'
        }

    def _get_oauth2_headers(self) -> Dict[str, str]:
        """Get headers for OAuth2 authentication."""
        # Check if we have a valid cached token
        if self._token_cache and time.time() < self._token_expiry:
            return {
                'Authorization': f'Bearer {self._token_cache}',
                'Content-Type': 'application/json'
            }

        # Get new token
        token, expires_in = self._get_oauth2_token()

        # Cache the token
        self._token_cache = token
        self._token_expiry = time.time() + expires_in - 60  # Refresh 1 minute early

        return {
            'Authorization': f'Bearer {token}',
            'Content-Type': 'application/json'
        }

    def _get_oauth2_token(self) -> Tuple[str, int]:
        """
        Get OAuth2 token using client credentials grant.

        Returns:
            Tuple of (token, expires_in_seconds)
        """
        # Get OAuth2 credentials
        client_id = os.environ.get('CANON_CLIENT_ID') or self.credentials.get('client_id')
        client_secret = os.environ.get('CANON_CLIENT_SECRET') or self.credentials.get('client_secret')

        if not client_id or not client_secret:
            raise ValueError("OAuth2 requires CANON_CLIENT_ID and CANON_CLIENT_SECRET")

        # Token endpoint
        token_url = self.credentials.get('token_url')
        if not token_url:
            token_url = urljoin(self.registry, '/oauth/token')

        # Request token
        data = {
            'grant_type': 'client_credentials',
            'client_id': client_id,
            'client_secret': client_secret,
            'scope': self.credentials.get('scope', 'publish')
        }

        response = requests.post(token_url, data=data)
        response.raise_for_status()

        token_data = response.json()
        return token_data['access_token'], token_data.get('expires_in', 3600)

    def _get_basic_headers(self) -> Dict[str, str]:
        """Get headers for basic authentication."""
        # Get credentials
        username = os.environ.get('CANON_USERNAME') or self.credentials.get('username')
        password = os.environ.get('CANON_PASSWORD') or self.credentials.get('password')

        if not username or not password:
            raise ValueError("Basic auth requires CANON_USERNAME and CANON_PASSWORD")

        # Encode credentials
        credentials = f"{username}:{password}"
        encoded = base64.b64encode(credentials.encode()).decode('ascii')

        return {
            'Authorization': f'Basic {encoded}',
            'Content-Type': 'application/json'
        }

    def validate(self) -> bool:
        """
        Validate authentication by making a test request.

        Returns:
            True if authentication is valid
        """
        try:
            # Try to get user info or perform a simple authenticated request
            test_url = urljoin(self.registry, '/api/v1/user')
            headers = self.get_headers()

            response = requests.get(test_url, headers=headers, timeout=10)
            return response.status_code == 200
        except Exception as e:
            print(f"Authentication validation failed: {e}")
            return False

    @classmethod
    def from_config_file(cls, config_path: str) -> 'CanonAuth':
        """
        Create CanonAuth instance from configuration file.

        Args:
            config_path: Path to configuration file

        Returns:
            CanonAuth instance
        """
        with open(config_path, 'r') as f:
            if config_path.endswith('.json'):
                config = json.load(f)
            elif config_path.endswith('.toml'):
                import tomli
                config = tomli.load(f)
            else:
                raise ValueError(f"Unsupported config format: {config_path}")

        return cls(config.get('publish', {}).get('canon', {}))