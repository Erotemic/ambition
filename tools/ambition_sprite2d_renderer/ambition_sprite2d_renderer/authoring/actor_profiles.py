"""Actor-contract metadata helpers.

Metadata for characters intentionally lives with each character authoring file:
YAML adapter configs carry their own actor/body/capability blocks, single
Python tack-ons expose module-level ``ACTOR_METADATA``, and multi-target modules
attach per-target ``actor_metadata`` inside ``TARGETS``.

This module keeps only the small deep-merge helper shared by the registry and
contract builder; it is no longer a central source of character facts.
"""

from __future__ import annotations

from copy import deepcopy
from typing import Any, Mapping


def merge_actor_metadata(
    base: Mapping[str, Any] | None, overlay: Mapping[str, Any] | None
) -> dict[str, Any]:
    """Deep-merge sparse actor metadata dictionaries.

    ``base`` is copied first, then ``overlay`` wins recursively for mapping
    values. Lists/scalars from the overlay replace the base value. This is used
    only for local authoring layers, for example module defaults plus a per-target
    override in a multi-target renderer.
    """
    result: dict[str, Any] = deepcopy(dict(base or {}))
    for key, value in dict(overlay or {}).items():
        if isinstance(value, Mapping) and isinstance(result.get(key), Mapping):
            result[key] = merge_actor_metadata(result[key], value)
        else:
            result[key] = deepcopy(value)
    return result
