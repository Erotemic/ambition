"""The art index: every generated or authored art file in the repository, the
generator that made it, the logical asset it belongs to and where the game
uses it. `python -m art_index build` writes the offline art inspector page.

The index is derived on each build from the generators' own records and
tables (`ambition_sprite2d_renderer.provenance`, its registry, its cache) and
from the game's content data. It is never edited by hand and is never an
authority: when it disagrees with a source, the source wins.
"""

SCHEMA = "ambition.art_index.v1"
