# World layers and travel

**State:** remaining topology is selected in
[WORLD-LAYERS](../queue.md#world-layers--fold-the-remaining-long-dimensions-into-connected-regions).
Travel portal mechanics are built (`c818c0801`); Magical's sky and arches
are authored (MG-1, MG-2, 2026-11-14); the Jungle is folded (JL-1) and
waits for its arches (JL-2). Their counts and bounds were rechecked on 2026-10-31.

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
| magical (2026-11-14) | planet | 15 | 17408 x 3456 | 5.0 | surface row, hollow under, void under the castle; the Starlight Heights over the square and the road's west end |
| art (2026-11-03) | planet | 15 | 12400 x 4960 | 2.5 | stacks, plaza and garden surface; amphitheatre, cliffs and reach over; caverns and workshop deeps under; seven buildings |
| dinosaur (Jungle, 2026-11-14) | planet | 12 | 25792 x 2240 (13216 x 2080 without the tyrant hall and ranger station, placed apart) | 11.5 (6.4) | surface trail to the ridge; treetops, canopy and the Smoking Mountain over it; fossil beds, amber vault and tar pits under it |
| math | topological | 19 | 16960 x 4144 | 4.1 | free |
| noodle | topological | 7 | 12960 x 1408 | 9.2 | free |
| history (2026-11-03) | topological | 36 | 22400 x 8208 | 2.7 | free: a grid, east is later and each floor is a region of the world; ladders join floors where the regions met; seven buildings under it |

## Remaining authored work (WORLD-LAYERS queue owner)

Portal kinds, same-dimension reach checks, attunement and shared gate transit
already ship (`ambition_platformer2d_world::rooms::gate_network`). The
Ambition catalog `game/ambition_content/assets/data/gate_network.ron`
holds Magical's three arches (MG-2), its first `kind: Portal` rings. Add **authored regions and arch
routes**, not another travel engine. Take one packet per change, in order;
delete each packet when its acceptance passes.

### JL-1 — Jungle geometry and walking loops (done 2026-11-14)

The rooms stay LDtk-owned and were edited in place (`entity add`,
`set-field`, `delete`, `intgrid paint`/`erase`; the three moves with the
tools' own writer, a load and write of the unchanged file being byte for
byte the same). The surface is the trail, the trailhead to the ridge road.
Over it: the treetops (as before), the canopy now over the ridge's east end
up a vine (`ridge_vine`/`canopy_vine`, a one-way branch half way up), and
the Smoking Mountain east of the canopy and higher, by a side exit
(`primeval_canopy_mountain`/`mountain_west`). Under it: the amber vault and
the fossil beds (as before), and the tar pits east of the beds by a side
exit (`fossils_east`/`tar_west`), whose seep climbs back to the ridge
(`tar_shaft`/`ridge_tar_shaft`; a hatch over the ridge's hole catches a
body that climbs out, as Magical's holes have). Two different
interlayer joins (the sinkhole, a drop and ladder; the seep, a climb)
make the underground loop ridge, beds, pits, ridge with no portal. The
ridge's and the canopy's old side exits are walled; the tar pits' east
exit is gone. Room IDs, secrets (the amber vault's cracked wall) and every
placement stay; the tyrant hall door stays on the canopy.

`the_tar_pits.rs` walks each join by keys: the loop both ways round, the
vine up and down, the mountain to the canopy and back. Two placements were
moved for the keyboard walker, which aims at the surface nearest an exit:
the seep stands west of the mammoth's back (a ledge beside it first had
made the walk to the paleontologist end on the dig camp's deck), and the
vine has a one-way branch half way up, nearer its top than the chest shelf. `check_flat_map`:
0 findings. The tar pits' origin (30008, 512) is off the 16 px grid, as
the fossil beds' and the amber vault's already were (a validator warning).

### JL-2 — Jungle arch travel (waits for an arch prop)

**Held 2026-11-14:** the only ring art is the 288 px gate ring. Magical's
open rooms took it (MG-2); the Jungle's do not: the trailhead's east end,
the treetops and the canopy carry one-way branches and a chest shelf
through the 288 px band over the floor wherever a surface arch would stand,
so each ring would be drawn through a platform. A planned set that fits the
reach rule: the trailhead (about x 700), the treetops, the canopy's west
end and the tar pits' east end; the trailhead cannot reach the canopy
(about 8400 px), so the trip is a chain. Make the arch prop (smaller than a
gate ring, drawn as an arch), then place these.


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

### MG-1 — Magical sky and region loop (done 2026-11-14)

`magical_starlight_heights` (generator `magical/make_specs.py`) is the
clouds over Starlight Square and the Dune Road's west end, 3840 wide, joined
two different ways: the beanstalk beside the square's well (a climb, both
ways), and on the road the star spring hanging over the road (jump on: it
throws a body up through the clouds' floor) with a gap in the clouds east of
it to drop back down. With the square-to-road side exit that is a loop; the
hollow joins the square and the garden as before. No room ID changed and the
gate's arrival is untouched. `the_magical_dimension.rs`
`the_heights_join_the_square_and_the_road_two_different_ways` walks it by
keys: square, beanstalk, heights, gap, road, spring, heights, beanstalk,
square, road. The spring hangs 128 px over the road because on the floor it
threw every body walking east to the castle into the sky.

Left: the heights have no prop art of their own (clouds are the one-way
platforms; the bg is the sky colour) and no authored shortcut, since the
loop needed none. A cloud prop sheet and the beanstalk's art go with the
next Magical prop publish.

### MG-2 — Magical arch travel (done 2026-11-14)

Three portal rings (`kind: Portal`, each its own facility in
`gate_network.ron`, `Dialed` in `gate_portals.ron`): STEADY on the heights'
west end, CASTLE at the Void Keep's east end, CANDLE in the Heart of the
Void. The heights' arch reaches the keep's (about 7700 px on the flat map)
and not the heart's (about 9200, past `DEFAULT_PORTAL_REACH`), so the
heights to the heart is two hops by way of the keep. The heights' terminal
posts nothing, the keep's posts STEADY (back toward town), the heart's
CASTLE (the way out); under the `Discovered` rule an arch is listed once its
address is learned. `the_gate_network.rs`
`magicals_arches_hop_within_their_reach_and_chain_past_it` plays it: from
the heights a gate address is `wrong_kind` and CANDLE `out_of_reach`;
CASTLE crosses to the keep, and from there CANDLE (a redial of the keep's
open arch) crosses to the heart. `ambition_content`
`magicals_arches_list_only_the_arches_they_reach` holds the lists and the
discovery rule. Two gate-era tests now say they cover gates: a gate's list
holds only gates, and an arch is not held to posting a home address (no
home ring answers an arch; its way home is the walk to its world's gate).
A void star that stood on the keep's arrival moved west: the press that
carries a traveller through the arch started its conversation.

Left: the arches use the round gate ring's art; an arch's own drawing goes
with the next prop publish.

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
