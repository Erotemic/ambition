"""Procedural 2D character rigging and sprite-sheet generation."""

from .adapters import TARGETS, get_adapter
from .config import CharacterJob, load_job, save_job

__all__ = ["TARGETS", "get_adapter", "CharacterJob", "load_job", "save_job"]
__version__ = "0.3.0"
