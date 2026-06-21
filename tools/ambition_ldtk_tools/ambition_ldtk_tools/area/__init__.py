"""Area authoring implementation modules.

The public CLI remains ``ambition_ldtk_tools.area_authoring``. This package is
where the monolithic area authoring flow is being split into spec loading,
normalization, patch compilation, and writeback pieces.
"""

from .spec import load_spec

__all__ = ["load_spec"]
