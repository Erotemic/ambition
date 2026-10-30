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

- **Jungle: the trail walks through side exits** (96d318a8c):
  `specs/primeval/side_exits.py` turns the trail's end doors into edge
  exits, opens the walls and places the rooms touching. Verified:
  the_primeval_trail, check_flat_map dinosaur 0.
- **Science: the Test Range** (cc690ce6c), out of town west of Eureka's gate
  court: a rocket (PROBABLY FINE III) and its gantry, a blast bunker, an
  observation tower; crawlers, a shock warden, mites, crash-test dummies.
- **Evil: the forest road** (876c63fb7): graveyard, mausoleum, dead oak and
  crows, a broken wheel, fog.

Backlog from the side-exit rule (Jon 2026-10-30): the GNU-ton trail
(orchard -> vector well -> conservation hall -> inversion gallery ->
approach) still joins by doors; another lane's boss route, not converted.

- **A faithful flat map** (a738acdc9, f2eed08cb). Jon: side loading zones
  connect the world (horizontal and vertical); doors only enter buildings;
  rooms placed as "a mostly faithful flat map". `specs/check_flat_map.py`
  reports edge exits whose rooms do not meet, and overlaps. Eureka, the Math
  town and PvN's port and village moved so neighbours touch with roads level
  (science 5 -> 0, math 1 -> 0, pvn 9 -> 2). Left: the village's high road
  arrives 64 px off on the forest's branch (art on both heights); the
  Mockingbird arena's ceiling exit leads to a side.
- **Math: Hylbert's Hotel** (aa3f6171a): its door in the street's passage, the
  lobby (Hylbert at the desk, the key rack that never ends, the lift whose
  dial ends in infinity), and the corridor whose east end comes back in at
  its west end: there is always one more room. Verified: hylberts_hotel.
- **Jungle: the Tyrant King's hall** (acff766c0): a sunken temple, carved
  tyrant heads, his throne with a too-small crown. Added by
  `specs/scenery_overlay.py`: the primeval generator is older than its level
  (regenerating it drops a gate terminal, water and tuned enemies).

- **Math: Zeno's Aqueduct** (3ee5622bf), the first zone out of town: the
  aqueduct's deck leaves Euclid Street west over a valley to the reservoir;
  piers close up by halves, signs count 1/2, 3/4, 7/8 of the way; The Limit
  island; dividing mites, puppy slugs, an exploding mite, an owl. Fixed on the
  way: the math generator's stale sprite path, and townsfolk talk that every
  regenerate dropped (now in the generator). Verified: the_zeno_aqueduct.
- **PvN: the Pirate Cove** (873532b44): a sea cave open on the night sea,
  lookout scaffolds, the ladder to the sky hatch, three doorways (the
  arena's scorched, feathers blown out), longboat, treasure heap, Jolly
  Roger, wanted board, parrot perch. Verified: LDtk validation, a semantic
  diff of pvn.ldtk (only the cove's props and a harbour-house re-sync).
- **Science: Tesseract Laboratories** (d5d039a27): coils with an arc between
  their toroids, gantries, Van de Graaff, Jacob's ladder, wireless bulb,
  pigeon coops, the AC/DC chalkboard.
- **Science: Eureka's tether carries its rider** (58da8bcfc): the ground car
  waits a hop over the floor; the_eureka_tether passes.
