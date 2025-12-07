"""
File uploader for Canon platform.

Supports:
- Large file uploads with progress tracking
- Chunked uploads for better reliability
- Resume capability for interrupted uploads
- Parallel uploads for multiple files
"""

import os
import json
import hashlib
import time
from pathlib import Path
from typing import Dict, List, Optional, Callable
from concurrent.futures import ThreadPoolExecutor, as_completed
import requests
from requests.adapters import HTTPAdapter
from urllib3.util.retry import Retry


class CanonUploader:
    """Handle file uploads to Canon platform."""

    # Default chunk size: 5MB
    DEFAULT_CHUNK_SIZE = 5 * 1024 * 1024

    # Maximum parallel uploads
    MAX_PARALLEL_UPLOADS = 3

    def __init__(self, registry: str, headers: Dict[str, str], verbose: bool = False):
        """
        Initialize Canon uploader.

        Args:
            registry: Canon registry URL
            headers: Authentication headers
            verbose: Enable verbose output
        """
        self.registry = registry.rstrip('/')
        self.headers = headers
        self.verbose = verbose

        # Create session with retry strategy
        self.session = self._create_session()

    def _create_session(self) -> requests.Session:
        """Create HTTP session with retry strategy."""
        session = requests.Session()

        # Configure retry strategy
        retry = Retry(
            total=3,
            read=3,
            connect=3,
            backoff_factor=0.3,
            status_forcelist=(500, 502, 503, 504)
        )

        adapter = HTTPAdapter(max_retries=retry)
        session.mount('http://', adapter)
        session.mount('https://', adapter)

        # Set default headers
        session.headers.update(self.headers)

        return session

    def upload_file(self,
                   file_path: str,
                   artifact_path: str,
                   metadata: Optional[Dict] = None,
                   progress_callback: Optional[Callable[[int, int], None]] = None) -> bool:
        """
        Upload a single file to Canon.

        Args:
            file_path: Local file path to upload
            artifact_path: Remote artifact path (e.g., com/company/lib/1.0.0/lib.jar)
            metadata: Optional metadata for the artifact
            progress_callback: Callback for progress updates (bytes_uploaded, total_bytes)

        Returns:
            True if upload successful
        """
        file_path = Path(file_path)
        if not file_path.exists():
            raise FileNotFoundError(f"File not found: {file_path}")

        file_size = file_path.stat().st_size

        # Calculate checksums
        checksums = self._calculate_checksums(file_path)

        if self.verbose:
            print(f"Uploading: {file_path.name}")
            print(f"  Size: {self._format_size(file_size)}")
            print(f"  MD5: {checksums['md5']}")
            print(f"  SHA256: {checksums['sha256']}")

        # Decide upload strategy based on file size
        if file_size < 10 * 1024 * 1024:  # < 10MB
            return self._upload_simple(file_path, artifact_path, checksums, metadata, progress_callback)
        else:
            return self._upload_chunked(file_path, artifact_path, checksums, metadata, progress_callback)

    def _upload_simple(self,
                      file_path: Path,
                      artifact_path: str,
                      checksums: Dict[str, str],
                      metadata: Optional[Dict],
                      progress_callback: Optional[Callable]) -> bool:
        """Upload small file in a single request."""
        url = f"{self.registry}/api/v1/artifacts/{artifact_path}"

        with open(file_path, 'rb') as f:
            files = {'file': (file_path.name, f, 'application/octet-stream')}

            # Prepare form data
            data = {
                'md5': checksums['md5'],
                'sha256': checksums['sha256']
            }

            if metadata:
                data['metadata'] = json.dumps(metadata)

            # Upload with progress tracking
            if progress_callback:
                # Use a custom iterator for progress tracking
                encoder = MultipartEncoderMonitor(
                    files=files,
                    fields=data,
                    callback=lambda monitor: progress_callback(monitor.bytes_read, monitor.len)
                )
                response = self.session.put(url, data=encoder)
            else:
                response = self.session.put(url, files=files, data=data)

        if response.status_code in (200, 201):
            if self.verbose:
                print(f"✓ Successfully uploaded: {file_path.name}")
            return True
        else:
            print(f"✗ Upload failed: {response.status_code} - {response.text}")
            return False

    def _upload_chunked(self,
                       file_path: Path,
                       artifact_path: str,
                       checksums: Dict[str, str],
                       metadata: Optional[Dict],
                       progress_callback: Optional[Callable]) -> bool:
        """Upload large file in chunks."""
        file_size = file_path.stat().st_size
        chunk_size = self.DEFAULT_CHUNK_SIZE

        # Initialize multipart upload
        init_url = f"{self.registry}/api/v1/artifacts/{artifact_path}/multipart"
        init_data = {
            'filename': file_path.name,
            'size': file_size,
            'md5': checksums['md5'],
            'sha256': checksums['sha256'],
            'chunk_size': chunk_size
        }

        if metadata:
            init_data['metadata'] = metadata

        response = self.session.post(init_url, json=init_data)
        if response.status_code != 200:
            print(f"Failed to initialize multipart upload: {response.text}")
            return False

        upload_id = response.json()['upload_id']
        total_chunks = (file_size + chunk_size - 1) // chunk_size

        if self.verbose:
            print(f"Uploading in {total_chunks} chunks...")

        # Upload chunks
        bytes_uploaded = 0
        with open(file_path, 'rb') as f:
            for chunk_num in range(total_chunks):
                chunk_data = f.read(chunk_size)
                chunk_md5 = hashlib.md5(chunk_data).hexdigest()

                chunk_url = f"{self.registry}/api/v1/artifacts/{artifact_path}/multipart/{upload_id}/chunks/{chunk_num}"

                response = self.session.put(
                    chunk_url,
                    data=chunk_data,
                    headers={'Content-MD5': chunk_md5}
                )

                if response.status_code != 200:
                    print(f"Failed to upload chunk {chunk_num}: {response.text}")
                    # Abort multipart upload
                    self._abort_multipart_upload(artifact_path, upload_id)
                    return False

                bytes_uploaded += len(chunk_data)
                if progress_callback:
                    progress_callback(bytes_uploaded, file_size)

                if self.verbose:
                    progress = (chunk_num + 1) / total_chunks * 100
                    print(f"  Chunk {chunk_num + 1}/{total_chunks} uploaded ({progress:.1f}%)")

        # Complete multipart upload
        complete_url = f"{self.registry}/api/v1/artifacts/{artifact_path}/multipart/{upload_id}/complete"
        response = self.session.post(complete_url)

        if response.status_code == 200:
            if self.verbose:
                print(f"✓ Successfully uploaded: {file_path.name}")
            return True
        else:
            print(f"Failed to complete multipart upload: {response.text}")
            return False

    def _abort_multipart_upload(self, artifact_path: str, upload_id: str):
        """Abort a multipart upload."""
        abort_url = f"{self.registry}/api/v1/artifacts/{artifact_path}/multipart/{upload_id}/abort"
        self.session.delete(abort_url)

    def upload_multiple(self,
                       files: List[Dict[str, str]],
                       progress_callback: Optional[Callable] = None) -> Dict[str, bool]:
        """
        Upload multiple files in parallel.

        Args:
            files: List of dicts with 'local_path' and 'artifact_path' keys
            progress_callback: Overall progress callback

        Returns:
            Dictionary mapping file paths to upload success status
        """
        results = {}
        total_size = sum(Path(f['local_path']).stat().st_size for f in files if Path(f['local_path']).exists())
        bytes_uploaded = 0

        with ThreadPoolExecutor(max_workers=self.MAX_PARALLEL_UPLOADS) as executor:
            futures = {}

            for file_info in files:
                def file_progress(uploaded, total, path=file_info['local_path']):
                    nonlocal bytes_uploaded
                    if progress_callback:
                        progress_callback(bytes_uploaded + uploaded, total_size)

                future = executor.submit(
                    self.upload_file,
                    file_info['local_path'],
                    file_info['artifact_path'],
                    file_info.get('metadata'),
                    file_progress if progress_callback else None
                )
                futures[future] = file_info['local_path']

            for future in as_completed(futures):
                file_path = futures[future]
                try:
                    results[file_path] = future.result()
                    if results[file_path]:
                        bytes_uploaded += Path(file_path).stat().st_size
                except Exception as e:
                    print(f"Error uploading {file_path}: {e}")
                    results[file_path] = False

        return results

    def _calculate_checksums(self, file_path: Path) -> Dict[str, str]:
        """Calculate MD5 and SHA256 checksums for a file."""
        md5_hash = hashlib.md5()
        sha256_hash = hashlib.sha256()

        with open(file_path, 'rb') as f:
            for chunk in iter(lambda: f.read(8192), b''):
                md5_hash.update(chunk)
                sha256_hash.update(chunk)

        return {
            'md5': md5_hash.hexdigest(),
            'sha256': sha256_hash.hexdigest()
        }

    def _format_size(self, size: int) -> str:
        """Format file size in human-readable format."""
        for unit in ['B', 'KB', 'MB', 'GB']:
            if size < 1024.0:
                return f"{size:.2f} {unit}"
            size /= 1024.0
        return f"{size:.2f} TB"

    def verify_upload(self, artifact_path: str, local_file: str) -> bool:
        """
        Verify that an uploaded artifact matches the local file.

        Args:
            artifact_path: Remote artifact path
            local_file: Local file path to compare

        Returns:
            True if verification successful
        """
        # Get artifact metadata from Canon
        url = f"{self.registry}/api/v1/artifacts/{artifact_path}/metadata"
        response = self.session.get(url)

        if response.status_code != 200:
            print(f"Failed to get artifact metadata: {response.text}")
            return False

        remote_meta = response.json()

        # Calculate local checksums
        local_checksums = self._calculate_checksums(Path(local_file))

        # Compare checksums
        if remote_meta.get('md5') != local_checksums['md5']:
            print(f"MD5 mismatch: local={local_checksums['md5']}, remote={remote_meta.get('md5')}")
            return False

        if remote_meta.get('sha256') != local_checksums['sha256']:
            print(f"SHA256 mismatch: local={local_checksums['sha256']}, remote={remote_meta.get('sha256')}")
            return False

        if self.verbose:
            print(f"✓ Verification successful: {artifact_path}")

        return True