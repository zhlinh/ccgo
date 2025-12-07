"""
Canon platform integration for CCGO.

This module provides functionality to publish build artifacts to Canon platform.
"""

from .client import CanonClient
from .auth import CanonAuth
from .uploader import CanonUploader

__all__ = ['CanonClient', 'CanonAuth', 'CanonUploader']

# Version of the Canon integration module
__version__ = '1.0.0'