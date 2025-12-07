"""
Canon platform client for publishing artifacts.

Main interface for publishing build artifacts to Canon platform.
Handles authentication, uploading, and metadata management.
"""

import os
import json
import time
from pathlib import Path
from typing import Dict, List, Optional, Any
from urllib.parse import urljoin

import requests
from .auth import CanonAuth
from .uploader import CanonUploader


class CanonClient:
    """Main client for interacting with Canon platform."""

    def __init__(self, config: Dict[str, Any], verbose: bool = False):
        """
        Initialize Canon client.

        Args:
            config: Configuration dictionary containing:
                - registry: Canon registry URL
                - auth: Authentication configuration
                - publish: Publishing configuration
            verbose: Enable verbose output
        """
        self.registry = config.get('registry', '').rstrip('/')
        self.verbose = verbose

        # Initialize authentication
        auth_config = {
            'registry': self.registry,
            'method': config.get('auth', {}).get('method', 'token'),
            'credentials': config.get('auth', {}).get('credentials', {})
        }
        self.auth = CanonAuth(auth_config)

        # Initialize uploader
        headers = self.auth.get_headers()
        self.uploader = CanonUploader(self.registry, headers, verbose)

        # Publishing configuration
        self.publish_config = config.get('publish', {})

        # Create session for API requests
        self.session = requests.Session()
        self.session.headers.update(headers)

    def publish_artifact(self,
                         file_path: str,
                         group_id: str,
                         artifact_id: str,
                         version: str,
                         classifier: Optional[str] = None,
                         packaging: str = 'jar',
                         metadata: Optional[Dict] = None) -> bool:
        """
        Publish a single artifact to Canon.

        Args:
            file_path: Path to the artifact file
            group_id: Group ID (e.g., 'com.example')
            artifact_id: Artifact ID (e.g., 'mylib')
            version: Version string (e.g., '1.0.0')
            classifier: Optional classifier (e.g., 'sources', 'javadoc')
            packaging: Package type (default: 'jar')
            metadata: Additional metadata

        Returns:
            True if publishing successful
        """
        # Construct artifact path
        group_path = group_id.replace('.', '/')

        if classifier:
            filename = f"{artifact_id}-{version}-{classifier}.{packaging}"
        else:
            filename = f"{artifact_id}-{version}.{packaging}"

        artifact_path = f"{group_path}/{artifact_id}/{version}/{filename}"

        if self.verbose:
            print(f"Publishing artifact: {artifact_path}")

        # Prepare metadata
        full_metadata = {
            'groupId': group_id,
            'artifactId': artifact_id,
            'version': version,
            'packaging': packaging,
            'timestamp': int(time.time()),
        }

        if classifier:
            full_metadata['classifier'] = classifier

        if metadata:
            full_metadata.update(metadata)

        # Upload the artifact
        success = self.uploader.upload_file(
            file_path,
            artifact_path,
            full_metadata
        )

        if success:
            # Generate and upload checksums
            self._upload_checksums(file_path, artifact_path)

            # Generate and upload POM if it's a JAR
            if packaging == 'jar' and not classifier:
                self._generate_and_upload_pom(
                    group_id, artifact_id, version, artifact_path
                )

        return success

    def publish_library(self,
                       library_path: str,
                       group_id: str,
                       artifact_id: str,
                       version: str,
                       sources_path: Optional[str] = None,
                       javadoc_path: Optional[str] = None,
                       metadata: Optional[Dict] = None) -> bool:
        """
        Publish a library with optional sources and javadoc.

        Args:
            library_path: Path to the main library file
            group_id: Group ID
            artifact_id: Artifact ID
            version: Version string
            sources_path: Optional path to sources JAR
            javadoc_path: Optional path to javadoc JAR
            metadata: Additional metadata

        Returns:
            True if all artifacts published successfully
        """
        results = []

        # Publish main artifact
        results.append(self.publish_artifact(
            library_path,
            group_id,
            artifact_id,
            version,
            metadata=metadata
        ))

        # Publish sources if provided
        if sources_path and Path(sources_path).exists():
            results.append(self.publish_artifact(
                sources_path,
                group_id,
                artifact_id,
                version,
                classifier='sources',
                metadata=metadata
            ))

        # Publish javadoc if provided
        if javadoc_path and Path(javadoc_path).exists():
            results.append(self.publish_artifact(
                javadoc_path,
                group_id,
                artifact_id,
                version,
                classifier='javadoc',
                metadata=metadata
            ))

        return all(results)

    def publish_android_aar(self,
                           aar_path: str,
                           group_id: str,
                           artifact_id: str,
                           version: str,
                           pom_path: Optional[str] = None,
                           sources_jar: Optional[str] = None) -> bool:
        """
        Publish Android AAR package.

        Args:
            aar_path: Path to AAR file
            group_id: Group ID
            artifact_id: Artifact ID
            version: Version string
            pom_path: Optional POM file path
            sources_jar: Optional sources JAR path

        Returns:
            True if publishing successful
        """
        results = []

        # Publish AAR
        results.append(self.publish_artifact(
            aar_path,
            group_id,
            artifact_id,
            version,
            packaging='aar'
        ))

        # Publish POM if provided
        if pom_path and Path(pom_path).exists():
            pom_artifact_path = f"{group_id.replace('.', '/')}/{artifact_id}/{version}/{artifact_id}-{version}.pom"
            results.append(self.uploader.upload_file(
                pom_path,
                pom_artifact_path
            ))

        # Publish sources if provided
        if sources_jar and Path(sources_jar).exists():
            results.append(self.publish_artifact(
                sources_jar,
                group_id,
                artifact_id,
                version,
                classifier='sources',
                packaging='jar'
            ))

        return all(results)

    def publish_kmp_artifacts(self,
                            artifacts: List[Dict[str, str]],
                            group_id: str,
                            artifact_id: str,
                            version: str) -> bool:
        """
        Publish Kotlin Multiplatform artifacts.

        Args:
            artifacts: List of artifact dictionaries with:
                - path: File path
                - platform: Target platform (jvm, android, ios, etc.)
                - type: Artifact type (jar, aar, klib, etc.)
            group_id: Group ID
            artifact_id: Artifact ID
            version: Version string

        Returns:
            True if all artifacts published successfully
        """
        results = []

        for artifact in artifacts:
            platform = artifact.get('platform', '')
            artifact_type = artifact.get('type', 'jar')

            # Construct classifier from platform
            classifier = f"{platform}" if platform else None

            results.append(self.publish_artifact(
                artifact['path'],
                group_id,
                artifact_id,
                version,
                classifier=classifier,
                packaging=artifact_type,
                metadata={'platform': platform}
            ))

        return all(results)

    def _upload_checksums(self, file_path: str, artifact_path: str):
        """Upload MD5 and SHA1 checksum files."""
        checksums = self.uploader._calculate_checksums(Path(file_path))

        # Upload MD5
        md5_path = f"{artifact_path}.md5"
        self._upload_text(checksums['md5'], md5_path)

        # Upload SHA256
        sha256_path = f"{artifact_path}.sha256"
        self._upload_text(checksums['sha256'], sha256_path)

    def _upload_text(self, content: str, artifact_path: str) -> bool:
        """Upload text content as an artifact."""
        url = f"{self.registry}/api/v1/artifacts/{artifact_path}"
        response = self.session.put(
            url,
            data=content.encode('utf-8'),
            headers={'Content-Type': 'text/plain'}
        )
        return response.status_code in (200, 201)

    def _generate_and_upload_pom(self,
                                group_id: str,
                                artifact_id: str,
                                version: str,
                                base_artifact_path: str) -> bool:
        """Generate and upload a minimal POM file."""
        pom_content = f"""<?xml version="1.0" encoding="UTF-8"?>
<project xmlns="http://maven.apache.org/POM/4.0.0"
         xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0
         http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>

    <groupId>{group_id}</groupId>
    <artifactId>{artifact_id}</artifactId>
    <version>{version}</version>

    <name>{artifact_id}</name>
    <description>Published by CCGO</description>
</project>
"""

        # Upload POM
        pom_path = base_artifact_path.replace('.jar', '.pom')
        return self._upload_text(pom_content, pom_path)

    def search_artifacts(self, query: str, limit: int = 20) -> List[Dict]:
        """
        Search for artifacts in Canon repository.

        Args:
            query: Search query string
            limit: Maximum number of results

        Returns:
            List of artifact metadata dictionaries
        """
        url = f"{self.registry}/api/v1/search"
        params = {'q': query, 'limit': limit}

        response = self.session.get(url, params=params)
        if response.status_code == 200:
            return response.json().get('artifacts', [])
        else:
            print(f"Search failed: {response.text}")
            return []

    def get_artifact_metadata(self,
                             group_id: str,
                             artifact_id: str,
                             version: str) -> Optional[Dict]:
        """
        Get metadata for a specific artifact.

        Args:
            group_id: Group ID
            artifact_id: Artifact ID
            version: Version string

        Returns:
            Artifact metadata dictionary or None if not found
        """
        group_path = group_id.replace('.', '/')
        artifact_path = f"{group_path}/{artifact_id}/{version}"

        url = f"{self.registry}/api/v1/artifacts/{artifact_path}/metadata"
        response = self.session.get(url)

        if response.status_code == 200:
            return response.json()
        else:
            if self.verbose:
                print(f"Failed to get metadata: {response.text}")
            return None

    def delete_artifact(self,
                       group_id: str,
                       artifact_id: str,
                       version: str) -> bool:
        """
        Delete an artifact from Canon repository.

        Args:
            group_id: Group ID
            artifact_id: Artifact ID
            version: Version string

        Returns:
            True if deletion successful
        """
        group_path = group_id.replace('.', '/')
        artifact_path = f"{group_path}/{artifact_id}/{version}"

        url = f"{self.registry}/api/v1/artifacts/{artifact_path}"
        response = self.session.delete(url)

        if response.status_code in (200, 204):
            if self.verbose:
                print(f"✓ Deleted: {artifact_path}")
            return True
        else:
            print(f"✗ Deletion failed: {response.text}")
            return False

    def validate_auth(self) -> bool:
        """
        Validate authentication configuration.

        Returns:
            True if authentication is valid
        """
        return self.auth.validate()

    @classmethod
    def from_config_file(cls, config_path: str, verbose: bool = False) -> 'CanonClient':
        """
        Create CanonClient from configuration file.

        Args:
            config_path: Path to CCGO.toml configuration file
            verbose: Enable verbose output

        Returns:
            CanonClient instance
        """
        with open(config_path, 'r') as f:
            if config_path.endswith('.toml'):
                import tomli
                config = tomli.load(f)
            elif config_path.endswith('.json'):
                config = json.load(f)
            else:
                raise ValueError(f"Unsupported config format: {config_path}")

        canon_config = config.get('publish', {}).get('canon', {})
        return cls(canon_config, verbose)