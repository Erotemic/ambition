"""Small shared helpers for working with Ambition LDtk JSON.

The LDtk editor file is still represented as normal Python dictionaries.  This
package centralizes the low-level JSON mechanics so feature modules do not all
learn their own subtly-different versions of project loading, UID allocation,
entity/layer lookup, editor-style writeback, or dry-run/no-op mutation flow.
"""

from .fields import default_field_value, entity_field_value
from .ids import alloc_uid
from .issues import Issue, format_issue_lines, has_errors
from .io import load_project, write_project, write_project_json
from .layers import ensure_entities_layer_def, ensure_entities_layer_instance
from .paths import (
    default_character_catalog,
    default_content_assets_dir,
    default_hall_ldtk,
    default_sandbox_ldtk,
    default_sprite_assets_dir,
    default_worlds_dir,
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
    "Issue",
    "LdtkTransaction",
    "MoveEntitiesToLayer",
    "PatchOp",
    "PatchResult",
    "alloc_uid",
    "format_issue_lines",
    "default_character_catalog",
    "default_content_assets_dir",
    "default_field_value",
    "default_hall_ldtk",
    "default_sandbox_ldtk",
    "default_sprite_assets_dir",
    "default_worlds_dir",
    "entity_defs",
    "entity_field_value",
    "ensure_entities_layer_def",
    "ensure_entities_layer_instance",
    "find_entity_def",
    "find_layer_def",
    "find_layer_instance",
    "find_level",
    "find_tileset",
    "has_errors",
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
