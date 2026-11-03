# World layers and travel

**State:** remaining topology is selected in
[WORLD-LAYERS](../queue.md#world-layers--fold-the-remaining-long-dimensions-into-connected-regions).
Travel portal mechanics are built (`c818c0801`); Jungle and Magical still need
the layout pass. Their counts and bounds were rechecked on 2026-10-31.

Still binding: dense interconnected regions with loops, shortcuts and
secrets (`world-composition.md`); side exits join the world and doors enter
buildings; a mostly faithful flat map; one LDtk world per dimension, rooms
from their generators.

## Two kinds of dimension

- **Planet-like** (Jungle, Evil, PvN, Science, Magical, the hub): a ground
  with things under it and over it. They have **layers**: surface,
  underground, sky and (where the story reaches it) space.
- **Topological** (Math; the Noodly Temple; anything whose joke is its
  shape): the layout follows the idea (a Mobius mine, Hylbert's hotel) and
  owes the layers nothing.

## Layers, without a horizontal line

A layer is not a corridor. The rules:

1. **Each layer is regions, not a row.** A layer is one or more dense
   regions (`world-composition.md`), each about as tall as it is wide. A
   layer never strings rooms east to west to its end.
2. **Layers stack on the flat map.** Underground is under the surface it is
   under, sky over it, space over the sky. The flat map stays faithful: a
   well in the surface opens into the room under it.
3. **Every layer joins the next in at least two places**, far apart (a well
   and a sinkhole; a tower and a cliff; the elevator and a wreck that fell).
   So a trip is a 2D route, up and across and down, never one line.
4. **A dimension's bounding box is no more than about 3:1** (wide:tall). A
   world that runs 13:1 (the Jungle today) is a line; fold it into layers.
5. **Each layer has its own look** (biome): the surface's, a cave's, the
   sky's, space's. The player knows the layer by its look.

## Travel: walk, portal, gate

| tier | goes | lore |
| --- | --- | --- |
| **walk** | within a region and between neighbours | the default; every place is reachable on foot |
| **portal** | within ONE dimension, as far as its **reach** | a portal folds a world onto itself |
| **gate** | between dimensions, any distance | a gate is a door between worlds |

- **Portals are within a dimension.** A portal can not reach another world:
  folding a world onto itself is one thing, opening a door into another is
  the gate's (the gate network, `gate-network.md`).
- **A portal has a reach** (lore: a fold is only so deep). A portal arch
  reaches another arch within its reach on the flat map, measured between the
  two arches. Beyond it, travel chains arches, walks, or takes the long way
  (the space elevator is the way to the station: no surface arch reaches
  orbit). So the reach is what makes layers matter and keeps each region's
  web of arches local.
- **A portal arch is attuned by visiting it** (it lists only arches the
  traveller has stood at), as a gate's known addresses are learned. Like the
  gates, development knows every arch (`start_known: All`).
- **Gates and portals share the machinery**: a ring (here an arch) is a zone
  a live link opens; the far end is reached by the ordinary room transition;
  the opening is the portal presentation (portals are never painted). The
  difference is data: a ring's `kind`, a portal's `reach`, and the dial rule.
- Gates keep their rules (no reach limit). Two gates in one dimension may
  still dial each other ("generally"), but new same-dimension travel is
  portals.

## Census (2026-10-31, rooms and bounding box)

| dimension | kind | rooms | box (px) | ratio | layers today |
| --- | --- | --- | --- | --- | --- |
| science | planet | 25 | 17472 x 12224 | 1.4 | all four: Eureka surface, collider/plant under, elevator sky, station in space |
| evil | planet | 22 | 19280 x 8064 | 2.4 | surface moor/hamlet, castle floors, crypt/catacombs under, ridge over |
| pvn | planet | 22 | 11136 x 16656 | 0.7 | port/village surface, cellars/tunnel under, sky lookout; boss arenas apart |
| magical | planet | 14 | 17408 x 2688 | 6.5 | surface row, hollow under, void under the castle; no sky |
| art (2026-11-02) | planet | 13 | 12400 x 3564 | 3.5 | stacks, plaza and garden surface; amphitheatre, cliffs and reach over; caverns and workshop deeps under; five buildings |
| dinosaur (Jungle) | planet | 12 | 25792 x 1888 | 13.7 | one line: the trail, a sinkhole to the fossil beds |
| math | topological | 19 | 16960 x 4144 | 4.1 | free |
| noodle | topological | 7 | 12960 x 1408 | 9.2 | free |

## Remaining authored work (WORLD-LAYERS queue owner)

Portal kinds, same-dimension reach checks, attunement and shared gate transit
already ship (`ambition_platformer2d_world::rooms::gate_network`). The
Ambition catalog `game/ambition_content/assets/data/gate_network.ron`
currently has no `kind: Portal` rings. Add **authored regions and arch
routes**, not another travel engine. Take one packet per change, in order;
delete each packet when its acceptance passes.

### JL-1 — Jungle geometry and walking loops

**Owner:** `tools/ambition_ldtk_tools/specs/primeval/make_specs.py`,
`treetops.py`, `side_exits.py` and adjacent primeval `.ron` rooms. Fold the
existing trail to the surface, fossil beds/tar pits/amber vault underneath,
canopy/treetops above and Smoking Mountain higher still. Reuse rooms and
preserve secret exits, permanent breaks and existing placements. Make at least
two **different traversable** interlayer joins, creating a loop without
requiring a portal.

**Pass:** Flat-map adjacency checks find no new unmatched exits, a real player
can walk each new connection, and `game/ambition_app/tests/the_tar_pits.rs`
(or another focused `app_it` test) exercises the loop and save-persistent
changes. Compare edited LDtk placements with the generator's source; don't
silently overwrite manual edits on regeneration.

### JL-2 — Jungle arch travel (after JL-1)

**Owner:** Jungle arch zone authoring and
`game/ambition_content/assets/data/gate_network.ron`. Add trailhead, canopy
and deep arches using existing portal `kind`, `reach`, visiting/attunement,
terminal and room-transition semantics. Avoid cross-dimension portals or a
single arch bypassing the whole region.

**Pass:** The player visits/attunes a destination, then traverses a local
arch; an out-of-range endpoint is rejected; an unvisited endpoint is absent
from the known destination list under discovery policy; the same-dimension
chain works from a live Jungle room. Extend `the_gate_network.rs` or a
focused Jungle integration fixture. Do not change the interdimensional gates.

### MG-1 — Magical sky and region loop

**Owner:** `tools/ambition_ldtk_tools/specs/magical/make_specs.py`,
existing garden/square/hollow/void `.ron` rooms, plus newly authored sky
rooms. Add a *playable* sky layer and at least two distinct ways to ascend or
descend, so the ground, high route and hollow/void are connected without a
one-way corridor.

**Pass:** Extend `game/ambition_app/tests/the_magical_dimension.rs`: player
walks/climbs both connections, shortcuts work after their authored triggers,
and a return path remains. Preserve existing room IDs and authored gate entry.

### MG-2 — Magical arch travel (after MG-1)

Author a reachable local arch web among garden, high route/castle and void
using the same portal rules as JL-2. The existing dimension gate remains the
interdimensional arrival. **Pass:** real portal hop, out-of-reach refusal and
valid multi-hop chain in `the_magical_dimension.rs` or `the_gate_network.rs`.

**Every geometry packet:** run
`python tools/ambition_ldtk_tools/specs/check_flat_map.py <world.ldtk>`
with the edited world file and review the semantic LDtk diff. Preserve
manually placed rooms (`--keep-position` or generator equivalent); deliberately
re-layout only the selected region. Re-measure and update the dimension
census after the edits. Approximately 3:1 world extent is a design review
heuristic, *not* a CI gate or reason to reject otherwise good topology.
After these packets, add arches in other planet-like worlds **only** where
traversal distance warrants them. Topological dimensions are not required
to use the layer model.
