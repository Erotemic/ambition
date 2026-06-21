"""Small shared helpers for working with Ambition LDtk JSON.

The LDtk editor file is still represented as normal Python dictionaries.  This
package centralizes the low-level JSON mechanics so feature modules do not all
learn their own subtly-different versions of project loading, UID allocation,
entity/layer lookup, and editor-style writeback.
"""

from .io import load_project, write_project, write_project_json
from .ids import alloc_uid
from .paths import repo_root_from_ldtk, rel_to_ldtk, path_from_ldtk, png_dimensions, default_sandbox_ldtk
from .query import (
    EntityLocation,
    tileset_defs,
    entity_defs,
    layer_defs,
    find_tileset,
    find_entity_def,
    find_layer_def,
    find_level,
    find_layer_instance,
    iter_entities,
)
from .fields import entity_field_value, default_field_value
from .layers import ensure_entities_layer_def, ensure_entities_layer_instance

__all__ = [
    "EntityLocation",
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
