# `ambition_menu` — module map

<!-- BEGIN generated module map (scripts/measure/modules_md.py) -->

**ambition_menu** — Engine-side unified menu: the renderer-agnostic content model plus two interchangeable presentations of it.

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`backend`](src/backend.rs) | Menu backend selection vocabulary. |
| [`map`](src/map/mod.rs) | Map / minimap state AND the Map tab that renders it. |
| [`render`](src/render/mod.rs) | Renderers for a [`crate::MenuPageModel`]. |

_3 crate-root modules. Regenerate: `python scripts/measure/modules_md.py --write`._

<!-- END generated module map -->

## Notes

Use [the map browser](src/map/browser.rs) to build and navigate a map hierarchy.
Set `map_path` on an LDtk level to put the room in a group below the world.
Use `/` between nested groups. Doors from a parent room into a group supply
the group's map entrances. Use `map_title` in the world manifest for the
world's display name. Games that supply their own room geometry can set
`MapRoomNode::group_path` and `MapHierarchy` directly.
Use [the region view](src/map/view.rs) for a local map around the active room.
