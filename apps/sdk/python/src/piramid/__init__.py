"""Python client for Piramid, an inference engine for retrieval systems."""

from .client import Piramid
from .errors import PiramidError

__all__ = ["Piramid", "PiramidError"]
__version__ = "0.2.0"
