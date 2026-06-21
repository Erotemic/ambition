"""The small, portable rendering core.

Everything in ``ambition_sprite2d_renderer.core`` must import with **only
Pillow + the Python standard library** — no ``yaml``, ``rich``, ``PySide6``,
or ``numpy``. The point (see docs/planning/sprite-renderer-refactor.md) is that
a chatbot agent with just ``pip install Pillow`` can author and render a sprite.

Heavy/optional dependencies live at the edges:
  * ``yaml``    — reading author-facing configs (an edge module), never here.
  * ``rich``    — CLI prettiness only.
  * ``PySide6`` — the rig editor GUI only.

The core's job is the one render spine every authoring style flows through:

    FrameSet  ──▶  supersample → downsample → crop → measure → assemble → emit

The seam between *authoring* (plural: drawers, imperative generators, YAML
adapters, rig docs) and the spine is :class:`~.frameset.FrameSet`. The output
contract the game reads is mirrored by :mod:`~.manifest` (written as RON, no
YAML in the write path).

This package is being grown incrementally; today it holds the seam types. The
spine implementation lands as the existing emitters are consolidated onto it.
"""

from .frameset import (  # noqa: F401
    AnimationSpec,
    FrameSet,
    FrameSpec,
    frameset_from_states,
)

__all__ = ["FrameSpec", "AnimationSpec", "FrameSet", "frameset_from_states"]
