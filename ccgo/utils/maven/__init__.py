"""
Maven repository integration for CCGO.

This module provides functionality to publish Android/JVM artifacts to Maven repositories.
"""

from .config import MavenConfig
from .publisher import MavenPublisher

__all__ = ['MavenConfig', 'MavenPublisher']

# Version of the Maven integration module
__version__ = '1.0.0'