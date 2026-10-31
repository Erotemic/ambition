# World layers and travel

**State:** OPEN (15-hour run from 2026-10-31; see "Run progress").

Jon, 2026-10-31: "Maybe the world is divided into a surface, underground,
sky, and space for planet like dimensions, and then the topological
dimensions can do whatever they want. The trick is that forces a sort of
horizontal linearity I don't like. Fast travel is a thing, and we will need
forms of that. Portals help a lot here. For lore, portals should generally be
within dimension, whereas dimension gates are a different thing. Portals maybe
even have some lore-like range."

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
| dinosaur (Jungle) | planet | 12 | 25792 x 1888 | 13.7 | one line: the trail, a sinkhole to the fossil beds |
| math | topological | 19 | 16960 x 4144 | 4.1 | free |
| noodle | topological | 7 | 12960 x 1408 | 9.2 | free |

## Plan (in order)

1. **Travel portals** (engine + game): the ring `kind` (gate or portal),
   the portal's `reach`, the dial rule (a portal reaches only portals of its
   own dimension within reach; refusals `other_dimension`, `out_of_reach`),
   attunement by visit, the terminal's destination list, the arch art, tests.
2. **The Jungle folded into layers**: the trail is the surface; the fossil
   beds, tar pits and amber vault go under it; the canopy and treetops go
   over it; the Smoking Mountain's peak tops it. Arches at the trailhead,
   the canopy and the deep.
3. **Magical gets a sky** and an arch web (garden, castle, the void).
4. Arches for the other planet-like dimensions where a trip is long.
5. Each step: the flat map checked (`specs/check_flat_map.py`), the walks
   tested, the ratio in the census updated.

## Run progress

(Newest first.)
