"""
OHPM (OpenHarmony Package Manager) integration for CCGO.

This module provides functionality to publish OHOS/OpenHarmony artifacts to OHPM registry.
"""

from .config import OhpmConfig, OhpmDependency
from .publisher import OhpmPublisher

__all__ = ['OhpmConfig', 'OhpmDependency', 'OhpmPublisher']

# Version of the OHPM integration module
__version__ = '1.0.0'