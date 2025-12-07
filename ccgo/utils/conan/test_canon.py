#!/usr/bin/env python3
"""
Tests for Canon platform integration.

Run with: python3 -m pytest test_canon.py
"""

import os
import json
import tempfile
import unittest
from unittest.mock import Mock, patch, MagicMock
from pathlib import Path

# Mock the requests module before importing Canon modules
import sys
sys.modules['requests'] = MagicMock()

from auth import CanonAuth
from uploader import CanonUploader
from client import CanonClient


class TestCanonAuth(unittest.TestCase):
    """Test Canon authentication module."""

    def test_token_auth_from_config(self):
        """Test token authentication from configuration."""
        config = {
            'method': 'token',
            'registry': 'https://canon.example.com',
            'credentials': {
                'token': 'test-token-123'
            }
        }

        auth = CanonAuth(config)
        headers = auth.get_headers()

        self.assertEqual(headers['Authorization'], 'Bearer test-token-123')
        self.assertEqual(headers['Content-Type'], 'application/json')

    def test_token_auth_from_env(self):
        """Test token authentication from environment variable."""
        config = {
            'method': 'token',
            'registry': 'https://canon.example.com',
            'credentials': {}
        }

        with patch.dict(os.environ, {'CANON_TOKEN': 'env-token-456'}):
            auth = CanonAuth(config)
            headers = auth.get_headers()

            self.assertEqual(headers['Authorization'], 'Bearer env-token-456')

    def test_basic_auth(self):
        """Test basic authentication."""
        config = {
            'method': 'basic',
            'registry': 'https://canon.example.com',
            'credentials': {
                'username': 'testuser',
                'password': 'testpass'
            }
        }

        auth = CanonAuth(config)
        headers = auth.get_headers()

        # Basic auth should be base64 encoded
        import base64
        expected = base64.b64encode(b'testuser:testpass').decode('ascii')
        self.assertEqual(headers['Authorization'], f'Basic {expected}')

    def test_unsupported_auth_method(self):
        """Test that unsupported auth method raises error."""
        config = {
            'method': 'unsupported',
            'registry': 'https://canon.example.com',
            'credentials': {}
        }

        with self.assertRaises(ValueError) as context:
            CanonAuth(config)

        self.assertIn('Unsupported auth method', str(context.exception))


class TestCanonUploader(unittest.TestCase):
    """Test Canon uploader module."""

    def setUp(self):
        """Set up test fixtures."""
        self.registry = 'https://canon.example.com'
        self.headers = {'Authorization': 'Bearer test-token'}
        self.uploader = CanonUploader(self.registry, self.headers, verbose=False)

    def test_calculate_checksums(self):
        """Test checksum calculation."""
        # Create a temporary file with known content
        with tempfile.NamedTemporaryFile(delete=False) as f:
            f.write(b'Hello, Canon!')
            temp_path = f.name

        try:
            checksums = self.uploader._calculate_checksums(Path(temp_path))

            # Verify MD5 and SHA256 are present
            self.assertIn('md5', checksums)
            self.assertIn('sha256', checksums)

            # Verify they are hex strings
            self.assertEqual(len(checksums['md5']), 32)
            self.assertEqual(len(checksums['sha256']), 64)
        finally:
            os.unlink(temp_path)

    def test_format_size(self):
        """Test human-readable size formatting."""
        self.assertEqual(self.uploader._format_size(100), '100.00 B')
        self.assertEqual(self.uploader._format_size(1024), '1.00 KB')
        self.assertEqual(self.uploader._format_size(1024 * 1024), '1.00 MB')
        self.assertEqual(self.uploader._format_size(1024 * 1024 * 1024), '1.00 GB')

    @patch('uploader.requests.Session')
    def test_upload_small_file(self, mock_session_class):
        """Test uploading a small file (< 10MB)."""
        # Create mock session
        mock_session = Mock()
        mock_session_class.return_value = mock_session

        # Mock successful response
        mock_response = Mock()
        mock_response.status_code = 201
        mock_session.put.return_value = mock_response

        # Create uploader with mocked session
        uploader = CanonUploader(self.registry, self.headers, verbose=False)
        uploader.session = mock_session

        # Create a small test file
        with tempfile.NamedTemporaryFile(delete=False) as f:
            f.write(b'Small file content')
            temp_path = f.name

        try:
            # Upload the file
            result = uploader.upload_file(
                temp_path,
                'com/example/test/1.0.0/test.jar'
            )

            # Verify upload was successful
            self.assertTrue(result)

            # Verify PUT was called
            mock_session.put.assert_called_once()
        finally:
            os.unlink(temp_path)


class TestCanonClient(unittest.TestCase):
    """Test Canon client module."""

    def setUp(self):
        """Set up test fixtures."""
        self.config = {
            'registry': 'https://canon.example.com',
            'auth': {
                'method': 'token',
                'credentials': {
                    'token': 'test-token'
                }
            }
        }

    @patch('client.CanonUploader')
    @patch('client.CanonAuth')
    def test_client_initialization(self, mock_auth_class, mock_uploader_class):
        """Test client initialization."""
        # Create mocks
        mock_auth = Mock()
        mock_auth.get_headers.return_value = {'Authorization': 'Bearer test'}
        mock_auth_class.return_value = mock_auth

        mock_uploader = Mock()
        mock_uploader_class.return_value = mock_uploader

        # Initialize client
        client = CanonClient(self.config, verbose=True)

        # Verify auth was initialized correctly
        mock_auth_class.assert_called_once()

        # Verify uploader was initialized
        mock_uploader_class.assert_called_once_with(
            'https://canon.example.com',
            {'Authorization': 'Bearer test'},
            True
        )

        # Verify client attributes
        self.assertEqual(client.registry, 'https://canon.example.com')
        self.assertTrue(client.verbose)

    @patch('client.CanonUploader')
    @patch('client.CanonAuth')
    def test_publish_artifact(self, mock_auth_class, mock_uploader_class):
        """Test publishing a single artifact."""
        # Setup mocks
        mock_auth = Mock()
        mock_auth.get_headers.return_value = {'Authorization': 'Bearer test'}
        mock_auth_class.return_value = mock_auth

        mock_uploader = Mock()
        mock_uploader.upload_file.return_value = True
        mock_uploader._calculate_checksums.return_value = {
            'md5': 'test-md5',
            'sha256': 'test-sha256'
        }
        mock_uploader_class.return_value = mock_uploader

        # Initialize client
        client = CanonClient(self.config, verbose=False)

        # Mock session for checksum uploads
        mock_session = Mock()
        mock_response = Mock()
        mock_response.status_code = 201
        mock_session.put.return_value = mock_response
        client.session = mock_session

        # Create a temporary file to publish
        with tempfile.NamedTemporaryFile(suffix='.jar', delete=False) as f:
            f.write(b'Test JAR content')
            temp_path = f.name

        try:
            # Publish artifact
            result = client.publish_artifact(
                temp_path,
                'com.example',
                'testlib',
                '1.0.0'
            )

            # Verify success
            self.assertTrue(result)

            # Verify uploader was called
            mock_uploader.upload_file.assert_called_once_with(
                temp_path,
                'com/example/testlib/1.0.0/testlib-1.0.0.jar',
                {
                    'groupId': 'com.example',
                    'artifactId': 'testlib',
                    'version': '1.0.0',
                    'packaging': 'jar',
                    'timestamp': unittest.mock.ANY
                }
            )
        finally:
            os.unlink(temp_path)

    @patch('client.CanonUploader')
    @patch('client.CanonAuth')
    def test_validate_auth(self, mock_auth_class, mock_uploader_class):
        """Test authentication validation."""
        # Setup mocks
        mock_auth = Mock()
        mock_auth.validate.return_value = True
        mock_auth.get_headers.return_value = {'Authorization': 'Bearer test'}
        mock_auth_class.return_value = mock_auth

        mock_uploader = Mock()
        mock_uploader_class.return_value = mock_uploader

        # Initialize client
        client = CanonClient(self.config, verbose=False)

        # Test validation
        result = client.validate_auth()

        # Verify auth.validate was called
        mock_auth.validate.assert_called_once()
        self.assertTrue(result)


class TestCanonIntegration(unittest.TestCase):
    """Integration tests for Canon platform."""

    @patch('client.requests.Session')
    @patch('uploader.requests.Session')
    def test_end_to_end_publish(self, mock_uploader_session, mock_client_session):
        """Test end-to-end publishing workflow."""
        # Setup mock sessions
        mock_upload_sess = Mock()
        mock_upload_resp = Mock()
        mock_upload_resp.status_code = 201
        mock_upload_sess.put.return_value = mock_upload_resp
        mock_upload_sess.post.return_value = mock_upload_resp
        mock_upload_sess.get.return_value = mock_upload_resp
        mock_uploader_session.return_value = mock_upload_sess

        mock_client_sess = Mock()
        mock_client_resp = Mock()
        mock_client_resp.status_code = 200
        mock_client_sess.get.return_value = mock_client_resp
        mock_client_sess.put.return_value = mock_client_resp
        mock_client_session.return_value = mock_client_sess

        # Create configuration
        config = {
            'registry': 'https://canon.example.com',
            'auth': {
                'method': 'token',
                'credentials': {
                    'token': 'integration-test-token'
                }
            },
            'publish': {
                'group_id': 'com.test',
                'artifact_id': 'integration'
            }
        }

        # Initialize client
        client = CanonClient(config, verbose=False)

        # Create a test file
        with tempfile.NamedTemporaryFile(suffix='.jar', delete=False) as f:
            f.write(b'Integration test content')
            temp_path = f.name

        try:
            # Publish the artifact
            result = client.publish_artifact(
                temp_path,
                'com.test',
                'integration',
                '1.0.0-test'
            )

            # Verify success
            self.assertTrue(result)

            # Verify upload was attempted
            self.assertTrue(mock_upload_sess.put.called or mock_upload_sess.post.called)
        finally:
            os.unlink(temp_path)


def run_tests():
    """Run all tests."""
    unittest.main(argv=[''], exit=False, verbosity=2)


if __name__ == '__main__':
    run_tests()