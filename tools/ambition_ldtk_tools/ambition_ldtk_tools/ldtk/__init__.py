"""Small shared helpers for working with Ambition LDtk JSON.

The LDtk editor file is still represented as normal Python dictionaries.  This
package centralizes the low-level JSON mechanics so feature modules do not all
learn their own subtly-different versions of project loading, UID allocation,
entity/layer lookup, editor-style writeback, or dry-run/no-op mutation flow.
"""

from .fields import default_field_value, entity_field_value
from .ids import alloc_uid
from .io import load_project, write_project, write_project_json
from .layers import ensure_entities_layer_def, ensure_entities_layer_instance
from .paths import (
    default_sandbox_ldtk,
    path_from_ldtk,
    png_dimensions,
    rel_to_ldtk,
    repo_root_from_ldtk,
)
from .patch import (
    ApplyEntityLayerTagRule,
    MoveEntitiesToLayer,
    PatchOp,
    PatchResult,
)
from .query import (
    EntityLocation,
    entity_defs,
    find_entity_def,
    find_layer_def,
    find_layer_instance,
    find_level,
    find_tileset,
    iter_entities,
    layer_defs,
    tileset_defs,
)
from .transaction import LdtkTransaction

__all__ = [
    "ApplyEntityLayerTagRule",
    "EntityLocation",
    "LdtkTransaction",
    "MoveEntitiesToLayer",
    "PatchOp",
    "PatchResult",
    "alloc_uid",
    "default_field_value",
    "default_sandbox_ldtk",
    "entity_defs",
    "entity_field_value",
    "ensure_entities_layer_def",
    "ensure_entities_layer_instance",
    "find_entity_def",
    "find_layer_def",
    "find_layer_instance",
    "find_level",
    "find_tileset",
    "iter_entities",
    "layer_defs",
    "load_project",
    "path_from_ldtk",
    "png_dimensions",
    "rel_to_ldtk",
    "repo_root_from_ldtk",
    "tileset_defs",
    "write_project",
    "write_project_json",
]
