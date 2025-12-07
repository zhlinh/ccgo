"""
Maven configuration handler for CCGO.

Handles Maven repository configuration from CCGO.toml and environment variables.
"""

import os
import re
from typing import Dict, Optional, Any


class MavenConfig:
    """Handle Maven repository configuration."""

    REPO_TYPES = ['local', 'central', 'custom']

    def __init__(self, config: Dict[str, Any], platform: str = 'android'):
        """
        Initialize Maven configuration.

        Args:
            config: Configuration dictionary from CCGO.toml
            platform: Platform name (android, kmp, etc.)
        """
        self.platform = platform
        self.raw_config = config

        # Get platform-specific publish config
        publish_config = config.get('publish', {})
        self.maven_config = publish_config.get(platform, {})

        # Parse configuration
        self.repo_type = self.maven_config.get('repository', 'local').lower()
        self.repo_url = self._expand_env(self.maven_config.get('url', ''))

        # Maven coordinates
        self.group_id = self._expand_env(
            self.maven_config.get('group_id', config.get('project', {}).get('group_id', 'com.example'))
        )
        self.artifact_id = self._expand_env(
            self.maven_config.get('artifact_id', config.get('project', {}).get('name', 'unknown'))
        )
        self.version = self._expand_env(
            self.maven_config.get('version', config.get('project', {}).get('version', '1.0.0'))
        )

        # Authentication
        self.auth_config = self.maven_config.get('auth', {})
        self.credentials = self._parse_credentials()

        # Publishing options
        self.sign_artifacts = self.maven_config.get('sign', False)
        self.publish_sources = self.maven_config.get('sources', True)
        self.publish_javadoc = self.maven_config.get('javadoc', True)

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

        # Username/password for custom repos and Maven Central
        username = config_creds.get('username', '')
        password = config_creds.get('password', '')

        # Support environment variable expansion
        username = self._expand_env(username)
        password = self._expand_env(password)

        # Check direct environment variables based on repository type
        if not username or not password:
            if self.repo_type == 'central':
                # For Maven Central, prefer OSSRH variables
                if not username:
                    username = os.environ.get('OSSRH_USERNAME', os.environ.get('SONATYPE_USERNAME',
                                             os.environ.get('MAVEN_USERNAME', '')))
                if not password:
                    password = os.environ.get('OSSRH_PASSWORD', os.environ.get('SONATYPE_PASSWORD',
                                             os.environ.get('MAVEN_PASSWORD', '')))
            else:
                # For custom repos, use generic MAVEN variables
                if not username:
                    username = os.environ.get('MAVEN_USERNAME', '')
                if not password:
                    password = os.environ.get('MAVEN_PASSWORD', '')

        credentials['username'] = username
        credentials['password'] = password

        # GPG signing configuration for Maven Central
        if self.sign_artifacts:
            credentials['signing_key_id'] = self._expand_env(
                config_creds.get('signing_key_id', os.environ.get('SIGNING_KEY_ID', ''))
            )
            credentials['signing_password'] = self._expand_env(
                config_creds.get('signing_password', os.environ.get('SIGNING_PASSWORD', ''))
            )
            credentials['signing_key'] = self._expand_env(
                config_creds.get('signing_key', os.environ.get('SIGNING_KEY', ''))
            )

        return credentials

    def get_repository_url(self) -> str:
        """Get the repository URL based on configuration."""
        if self.repo_type == 'local':
            return 'mavenLocal()'
        elif self.repo_type == 'central':
            # Maven Central staging URL
            if not self.repo_url:
                return 'https://s01.oss.sonatype.org/service/local/staging/deploy/maven2/'
            return self.repo_url
        else:  # custom
            if not self.repo_url:
                raise ValueError("Custom repository requires 'url' to be specified")
            return self.repo_url

    def generate_gradle_properties(self) -> str:
        """
        Generate gradle.properties content for publishing.

        Returns:
            String content for gradle.properties file
        """
        lines = []

        # Maven coordinates
        lines.append(f"GROUP={self.group_id}")
        lines.append(f"POM_ARTIFACT_ID={self.artifact_id}")
        lines.append(f"VERSION_NAME={self.version}")

        # Repository configuration
        if self.repo_type != 'local':
            if self.repo_type == 'central':
                lines.append("RELEASE_REPOSITORY_URL=https://s01.oss.sonatype.org/service/local/staging/deploy/maven2/")
                lines.append("SNAPSHOT_REPOSITORY_URL=https://s01.oss.sonatype.org/content/repositories/snapshots/")
            elif self.repo_url:
                lines.append(f"RELEASE_REPOSITORY_URL={self.repo_url}")

                # For custom repos, use same URL for snapshots unless specified
                snapshot_url = self.maven_config.get('snapshot_url', self.repo_url)
                lines.append(f"SNAPSHOT_REPOSITORY_URL={self._expand_env(snapshot_url)}")

        # Authentication
        if self.credentials.get('username'):
            lines.append(f"OSSRH_USERNAME={self.credentials['username']}")
        if self.credentials.get('password'):
            lines.append(f"OSSRH_PASSWORD={self.credentials['password']}")

        # Signing configuration
        if self.sign_artifacts:
            if self.credentials.get('signing_key_id'):
                lines.append(f"signing.keyId={self.credentials['signing_key_id']}")
            if self.credentials.get('signing_password'):
                lines.append(f"signing.password={self.credentials['signing_password']}")
            if self.credentials.get('signing_key'):
                # Handle multiline GPG keys
                key = self.credentials['signing_key'].replace('\n', '\\n')
                lines.append(f"signing.key={key}")

        # POM metadata
        pom_name = self.maven_config.get('pom_name', self.artifact_id)
        pom_description = self.maven_config.get('pom_description', f"{self.artifact_id} library")
        pom_url = self.maven_config.get('pom_url', '')

        lines.append(f"POM_NAME={pom_name}")
        lines.append(f"POM_DESCRIPTION={pom_description}")
        if pom_url:
            lines.append(f"POM_URL={self._expand_env(pom_url)}")

        # License information
        license_name = self.maven_config.get('license_name', 'MIT License')
        license_url = self.maven_config.get('license_url', 'https://opensource.org/licenses/MIT')
        lines.append(f"POM_LICENCE_NAME={license_name}")
        lines.append(f"POM_LICENCE_URL={license_url}")
        lines.append("POM_LICENCE_DIST=repo")

        # SCM information
        scm_url = self.maven_config.get('scm_url', pom_url)
        scm_connection = self.maven_config.get('scm_connection', f"scm:git:git://github.com/OWNER/REPO.git")
        scm_dev_connection = self.maven_config.get('scm_dev_connection', f"scm:git:ssh://github.com/OWNER/REPO.git")

        if scm_url:
            lines.append(f"POM_SCM_URL={self._expand_env(scm_url)}")
            lines.append(f"POM_SCM_CONNECTION={self._expand_env(scm_connection)}")
            lines.append(f"POM_SCM_DEV_CONNECTION={self._expand_env(scm_dev_connection)}")

        # Developer information
        developer_id = self.maven_config.get('developer_id', 'developer')
        developer_name = self.maven_config.get('developer_name', 'Developer Name')
        developer_email = self.maven_config.get('developer_email', '')

        lines.append(f"POM_DEVELOPER_ID={developer_id}")
        lines.append(f"POM_DEVELOPER_NAME={developer_name}")
        if developer_email:
            lines.append(f"POM_DEVELOPER_EMAIL={self._expand_env(developer_email)}")

        return '\n'.join(lines) + '\n'

    def generate_local_properties(self) -> str:
        """
        Generate local.properties content if needed.

        Returns:
            String content for local.properties file
        """
        lines = []

        # Add SDK location if available
        android_home = os.environ.get('ANDROID_HOME', os.environ.get('ANDROID_SDK_ROOT', ''))
        if android_home:
            lines.append(f"sdk.dir={android_home}")

        # Add NDK location if available
        android_ndk = os.environ.get('ANDROID_NDK', os.environ.get('ANDROID_NDK_HOME', ''))
        if android_ndk:
            lines.append(f"ndk.dir={android_ndk}")

        return '\n'.join(lines) + '\n' if lines else ''

    def get_gradle_task(self) -> str:
        """Get the appropriate Gradle task for publishing."""
        if self.repo_type == 'local':
            return 'publishToMavenLocal'
        else:
            # For Central and custom repos
            return 'publishMainPublicationToMavenRepository'

    def validate(self) -> tuple[bool, str]:
        """
        Validate the configuration.

        Returns:
            Tuple of (is_valid, error_message)
        """
        if self.repo_type not in self.REPO_TYPES:
            return False, f"Invalid repository type: {self.repo_type}. Must be one of {self.REPO_TYPES}"

        if self.repo_type == 'custom' and not self.repo_url:
            return False, "Custom repository requires 'url' to be specified"

        if self.repo_type != 'local':
            if not self.credentials.get('username') or not self.credentials.get('password'):
                return False, f"Repository type '{self.repo_type}' requires username and password"

        if self.sign_artifacts and self.repo_type == 'central':
            if not all([
                self.credentials.get('signing_key_id'),
                self.credentials.get('signing_password'),
                self.credentials.get('signing_key')
            ]):
                return False, "Maven Central requires signing configuration (key_id, password, and key)"

        return True, ""

    def get_config_summary(self) -> str:
        """Get a summary of the configuration for display."""
        lines = []
        lines.append(f"  Repository Type: {self.repo_type}")

        if self.repo_type == 'local':
            lines.append(f"  Location: ~/.m2/repository/")
        elif self.repo_type == 'central':
            lines.append(f"  Repository: Maven Central")
            lines.append(f"  URL: {self.get_repository_url()}")
        else:
            lines.append(f"  Repository URL: {self.repo_url}")

        lines.append(f"  Group ID: {self.group_id}")
        lines.append(f"  Artifact ID: {self.artifact_id}")
        lines.append(f"  Version: {self.version}")

        if self.repo_type != 'local':
            lines.append(f"  Username: {'***' if self.credentials.get('username') else 'Not configured'}")
            lines.append(f"  Password: {'***' if self.credentials.get('password') else 'Not configured'}")

        if self.sign_artifacts:
            lines.append(f"  Signing: Enabled")
            lines.append(f"  Signing Key: {'***' if self.credentials.get('signing_key_id') else 'Not configured'}")

        return '\n'.join(lines)