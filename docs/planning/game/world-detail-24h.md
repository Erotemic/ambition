# World detail run (24 h, 2026-10-30)

Jon, 2026-10-30: "build out the world detail of the different maps and
dimensions, adding art, flavor, character, and interactive elements, and a bit
of chutzpah. This can also include out of town zones with enemies. Focus on pvn
dimension, science dimension, math dimension, jungle dimension, evil
dimension."

World building, not quest building (Jon, 2026-10-29): places, people, things
to touch, and a little danger outside town. No quest chains.

## How a room gets detail

- Scenery is district props drawn in room coordinates by the sprite renderer
  (`targets/props/<district>_props.py`, `_district_props`), placed by the
  room's generator (`specs/<district>/props.py`), registered in
  `authored/sprites.yaml` and `assets/data/prop_sheets.ron`. Batch small
  shapes into one mask per colour (`eureka_props.Batch`, `ashlar`): a mask per
  shape made one room's art a 13-minute render.
- Residents are chargen townsfolk (`data/townsfolk/*.chargen.yaml`), the
  catalog row, a Yarn file and the Hall of Characters wing.
- Interactive: signs, doors, lifts, pickups, switches, residents with talk;
  enemies in the out-of-town zones (EnemySpawn, existing archetypes).
- One LDtk file per dimension; rooms come from their generator, not hand
  edits.

## Backlog by dimension (measured 2026-10-30: props p, enemy spawns e)

- **Science** (21 rooms): Eureka's coil lab (p0), elevator station, climb and
  terminus (p0): the car itself has no art; the GNU-ton halls (p0); the
  station's spur/spoke (p0). Out of town: a zone past Eureka with enemies.
- **PvN** (13): the cove (p0), the Mockingbird arena and sky (p0), the dojo
  interior (p0); the port and village have props. Out of town: the sky
  lookout has enemies; the forest could carry more.
- **Math** (5): the town is dressed (p48); the gate hall is thin. Out of town:
  nothing yet, a zone with enemies.
- **Jungle** (dinosaur, 6): trail rooms have enemies and a few props each; the
  tyrant hall has none.
- **Evil** (7): Castle Godwin is dressed; the gate forest and approaches can
  carry more flavor.

## Run progress

(Newest first. Each entry: what landed, the commit, what was verified.)
