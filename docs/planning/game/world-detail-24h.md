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
- **Jungle** (dinosaur, 7): trail rooms have enemies and props; the tyrant
  hall is dressed; the tar pits are out past the trail.
- **Evil** (8): Castle Godwin is dressed; the gate forest and approaches can
  carry more flavor; the moor is the first zone out of town.

## Run progress
- **Jungle: the fossil beds** (0bdce43ba), down a sinkhole in the ridge
  road (vertical exit, ladder; directly under the ridge on the flat map):
  strata and their fossils, a skull under a survey scaffold; Dr Petra
  Stratton, geologist, who objects that the dinosaurs upstairs are in the
  wrong layer; compys and a raptor; a tin on the scaffold. Learned: the
  engine draws a Climbable's ladder, so art must not; the test walker cannot
  step into a 64-px hole from the floor (start below and climb up first).
- **PvN: the cellars under customs** (2f33b0b6d), down the gate plaza's
  old well against its west wall (a vertical exit and a ladder; directly
  under the plaza on the flat map): the freight customs never sees. The
  forger's desk, a cursed idol under a sheet, undeclared chickens, the still;
  Fennimore Slipp, import consultant; three tariff-averse colleagues; a
  strongbox on the barrels. Verified: the_cellars_under_customs.
- **Map (Jon's shift, 10-30)**: the LDtk Map page, labels, windowing, pins,
  a game-owned presentation; the engine's panel and minimap on the same
  model. Fixed on the way: null Bools made bevy_ecs_ldtk refuse pvn,
  sandbox, noodle and dinosaur (no tiles painted).

(Newest first. Each entry: what landed, the commit, what was verified.)

- **Science composed: the steam plant under the Eureka quad** (down a
  manhole in the street; catwalks, boilers, one gauge in the red labelled
  FINE; east into the Collider, up its stair: a loop) and **Lab 0** behind a
  bricked door (a breakable wall). Verified: the_eureka_tether (8), capture.
- **Jungle composed: the treetops** over the trail, up the great tree's
  rope ladder in the run, down a gap in the leaves into the rookery
  (6ba334330). Verified: the_tar_pits treetops test, captures.
- **Ladders seen; secrets hidden (Jon 10-30: "ladders ... no ladder
  props/sprites")**: climbables drew under the room's scenery; now over it,
  under the cast. A secret_ zone draws no doorway. Evil: the catacombs'
  cracked wall (the first breakable_wall) over the ossuary of the other
  founders (6a6213768). Verified: captures, the_searchlight_moor.
- **PvN composed**: the Pirate Port's smugglers' tunnel (cellars -> drain
  in the waterfront's street, grate opened by a lever below; the hoard
  behind a plank wall) (79f4130e3); the Ninja Village's hidden way (a loose
  board in the street -> the shrine's slab, lever below) (b018ae4a1).
  Verified: the_wreckers_reef (6), the_ninja_village (3).
- **Evil outdoors composed (Jon 10-30: dense regions, not strings)**:
  the crypt and the catacombs under the moor and the allotments (in by the
  crypt stair, up under plot 13); the Searchlight Ridge over them, up the
  moor's watchtower; Gallows Hill over Lower Godwin, up its bell tower (the
  gibbet TEMPORARILY UNOCCUPIED, the paupers' graves, the founder's
  billboard), across a plank bridge onto the ridge, dropped through onto the
  greenhouse. A surface row with a loop under and a loop over (91fb598ad,
  130060b47). Verified: the_searchlight_moor (9), check_flat_map evil 0.
- **Math (subagents, Jon's asks)**: Monty Carlo's game plays an honest
  round, a won car opens the prize case (13188065e); the Tower of Hanoi's
  64 disks and three needles are props, laid out as move 555,039 of the
  solution (97d95bac3).
- **Science: the Eureka Collider**, down a stair in one of the lecture
  hall's arches on the quad: the ring's tunnel, its beam pipe and magnets
  (BEAM ON: do not lick the pipe), the staff's bicycles, the detector end on
  (rings in rings) with its gallery up a ladder (a chest: a spare quark,
  charm); Dr. Ilse Quark (chargen, Hall wing); crawlers and a shock warden
  loose in the tunnel. Verified: the_eureka_tether (7).
- **PvN: the sail loft**, in at a door added to the net loft on the
  waterfront: nets on the rafters, bolts of canvas, a sail spread half sewn,
  and flags to order (skulls with an eyepatch, a monocle, a moustache, a
  party hat; a smiling one sent back: ships kept sailing up to say hello);
  Tamsin Bolt the sailmaker (chargen, Hall wing). Verified: the_wreckers_reef
  loft test, the_pirate_port.
- **Evil: Crumb's bakery inside**, by its door in Lower Godwin: the
  castle's black iron oven (the crest on its door) and the hamlet's glowing
  brick one, shelves of both breads, the counter; behind the flour sacks,
  the press of the Lower Godwin Gazette, its sheets drying on a line; Silas
  Prentice the printer (chargen, Hall wing), whose Gazette goes up to the
  castle wrapped round its bread. Verified: the_searchlight_moor (6).
- **Jungle: the ranger station**, inside the trailhead's ranger hut (its
  deck now walked on, a door added): the trail's big map (you are here:
  please stay here), the radio, specimen jars (ferns, eggs, compys rescued
  from tar, very annoyed), a cot, the lost-property box (a boot, a monocle,
  an umbrella); Ranger Dlamini on the radio (chargen, Hall wing). The first
  chargen seed failed the publish's own check (a walk frame's draws did not
  reproduce its render: chargen's lane); the next seed published. Verified:
  the_tar_pits station test, the_primeval_trail (5).
- **Math: the Tower of Hanoi**, west of the gate hall by a side exit (the
  hall's "<- Manifest Office" sign points that way): three diamond needles,
  golden disks part moved, each disk a one-way platform so each stack is a
  stair, a chest on the tallest; the monks' tally on the wall; Brother Lucas
  keeps the count (chargen, Hall wing); "Moves left: 18,446,744,073,708,
  503,040". Verified: the_zeno_aqueduct Hanoi test, the_gate_network (11),
  check_flat_map math 0. Seen on the way, NOT this lane's: the_consignment_
  errand fails since 697058efa (10-24) renamed Georg Canter's choice; its
  test still asks for "Please count the lenses."
- **Science: Ottoline Hum busks** in the middle arch of the gate court's
  arcade: a theremin on its stand, an open case of coins from forty
  dimensions (one technically a button) (chargen, Hall wing). Verified:
  the_eureka_tether busker test.
- **PvN: Matsuda referees the Bamboo Pass**: the referee by his tally
  board at the way in (the day's bouts in 正, the visitors' column a nought),
  before the first bout (chargen, Hall wing). Verified: the_bamboo_pass (3).
- **Evil: Lower Godwin**, the hamlet under the castle, west of the
  allotments by a side exit: crooked thatched cottages, the bakery, the
  well, the stocks (FOR COMPLAINING), the decree board (DECREE 47: smiling is
  compulsory on Tuesdays), a barred cottage and a turnip cart; Barnaby
  Bellows the town crier on his platform, who reads the decrees to the
  well, and Prudence Crumb the baker, who bakes two breads (chargen, Hall
  wing). Verified: the_searchlight_moor (5), check_flat_map evil 0.
- **Jungle: rewards for the climb**: a chest on the highest ledge of the
  trailhead, the run, the ridge and the canopy (a ranger's lost lunch, a nest
  of something large, a fossil hunter's cache, an egg the size of a chest),
  and a sign in each; `scenery_overlay.py` learns chests. Verified:
  the_primeval_trail (5).
- **Math: the Mobius Mine**, under the Fractal Woods: a ladder down a hole
  in their floor (a vertical exit). A mine with one side: at its far end
  the half twist (a FlipGravity switch) turns it over, the way back is the
  ceiling where the coins are, and the switch's twin on the ceiling turns
  it back. Leaning timber sets, ore veins, rails that twist up the back
  wall, a cart on the floor and one hung on the ceiling; Felix Kleinbottle
  the miner (chargen, Hall wing); dividing mites. Verified:
  the_zeno_aqueduct (5: the walker climbs up and down the shaft),
  check_flat_map math 0.
- **Science: the spoke's tube** (one drawing in two frames, no noise
  across the seam): hull ribs, portholes on the stars, conduits and lamps
  overhead, the tram trench's rails and hazard edges, the lift's shaft at
  the far end, its own door hatches, a bench, a plant, a mural of the
  planet (`scenery_overlay.py`). Verified: capture.
- **PvN: the Lighthouse Point**, out past the Wreckers' Reef by a side
  exit at its east rocks (floors level): the honest light against the
  wreckers' false one. The tower in red and white, cut away to its stair of
  landings up to the gallery and the lamp (a chest there), its beams; the
  keeper's cottage, garden, upturned boat; Ezra Lamplugh the keeper
  (chargen, Hall wing); wreckers come round the point. Verified:
  the_wreckers_reef (3), check_flat_map pvn (only the Mockingbird finding).
- **Evil: the Prototype Workshop**, down a door added to the barracks by
  its failed prototypes: where they come from. Tool wall (RETURN THE BIG
  SPANNER), a walking toaster half built, the copper boiler in the red,
  blueprints (one REJECTED), the battle kettle (MARK 47: almost), the red
  button (DO NOT), a crate of spare parts; Dr. Gideon Sprocket the chief
  engineer (chargen, Hall wing). Verified: the_castle_kitchens (3).
- **Jungle: the rookery's birdwatcher**: Mildred Twitchell's hide on the
  raised ground (canvas and palm fronds, a long lens through the slit, her
  stool, notebook and thermos), and Mildred, who has stopped asking why the
  penguins are here (chargen, Hall wing; `scenery_overlay.py`). Verified:
  the_tar_pits birdwatcher test, the_primeval_trail (5).
- **Math: Zeno Halfway** on the aqueduct's deck, between the 7/8 and 3/4
  piers: set out for the reservoir at dawn, some years ago, always halfway;
  his arrow is still in the air (chargen, Hall wing). Verified:
  the_zeno_aqueduct (4).
- **Science: the tether's climb**: things passed, each at its sign's
  altitude: a weather balloon (12 km), a hoop round the ribbon and a dotted
  line across the room at the Karman line (space, technically), a
  satellite with a lost glove and a turning spanner (low orbit, and its
  traffic), a navigation satellite (20,000 km). Verified: captures.
- **PvN: Ume's tea house inside**, by a door under the noren in the village
  street (Ume steps aside to her bench): the tea room (tatami, the sunken
  hearth and its kettle, low tables, the alcove), the step chest up to the
  loft's two guest rooms, where the family who left before daylight forgot
  a child's sandals under the futon (Ume's talk) and left a thank-you on
  the pillow (a chest); Tsuru the travelling medicine seller staying
  (chargen, Hall wing). Verified: the_ninja_village (2).
- **Evil: the Castle Allotments**, out of town west of the Searchlight Moor
  by a side exit (floor level, rooms touching): raised beds of the castle's
  vegetables (pumpkins that come up scowling, cabbages with teeth,
  mandrakes not to be pulled), a trellis of nightshade, a steaming compost
  heap, the scarecrow in the castle's helmet, the Malevolent Marrow on the
  show bench; the greenhouse's roof climbed by the water butt to a chest;
  Hester Crabbe the head gardener behind her fence (chargen, Hall wing);
  zombies, a hound and a failed prototype in the beds. Verified:
  the_searchlight_moor (4), check_flat_map evil 0.
- **Jungle: the mammoth's skull**: the block of a skull is redrawn as a
  mammoth's (a high domed crown, the eye's socket, the tusks' sheath, the
  tusks in a spiral, clipped at the ground); curved ribs, the spine's
  vertebrae. Verified: capture.
- **Math: Monty Carlo hosts the ballroom**: the three doors' host (chargen,
  Hall wing), who explains why switching wins and sends a goat to each
  mathematician who writes in. Verified: hylberts_hotel (4).
- **Science: the tether terminus** (station end of the space elevator):
  hull hall, the ribbon through collars in floor and vault, a window on the
  planet's limb, the down car's lift doors and the bay's bulkhead, the
  arrivals board (every car ON TIME: it is a ribbon), and Dobbs's souvenir
  kiosk: shirts, snow globes with the planet in them, genuine orbit air
  (chargen, Hall wing). Verified: the_eureka_tether (5).
- **PvN: the charcoal burner's camp** in the cedar forest: an earth kiln
  smoking at its vents, seasoning billets, a bark lean-to against a cedar
  with a kettle on, charcoal bales; Moriyama (chargen) at his kiln, who
  tells of the crows' hoard on the fourth branch (a chest there); Hall wing.
  Verified: the_bamboo_pass (2), the_ninja_village.
- **Fixed: murky water never landed** (5b4f72f66). MEASURED: the moor's bog,
  the tar pits and the Bamboo Pass's stream had no Water cells; `area
  create` built no Water layer, so each paint was refused. A made level now
  carries every IntGrid layer its project defines (test); the three zones
  have their water (160, 153, 124 cells). Verified: the zones' tests (9).
  Open for Jon: murky water looks the same in every biome by design
  (`terrain/fixtures.py`), so the tar pits' tar reads dark teal, not black.
- **Evil: the Complaints Department** behind Castle Godwin's kitchens, by a
  door at their east end: two windows both CLOSED, NOW SERVING 3, a ticket
  machine, a padlocked suggestions box, and Agnes Mudge (chargen) holding
  ticket 4,117; Hall wing. Verified: the_castle_kitchens (2).
- **PvN flat map: the village's high road** (09ccc3977) meets the forest's
  first branch: the branches step down from y 448 (six now), art and
  generator. check_flat_map pvn: only the Mockingbird arena's ceiling exit
  (another lane) remains.
- **Science: the service deck's clutter** (4567821b4): lockers, a vending
  machine, the robots' charging dock, DAYS WITHOUT INCIDENT: 0.
- **Evil: the founder's crypt** (a4cc9e181) under the moor, by a gate
  beside his plinth: bones, pointing busts, his medals on the sarcophagus;
  a spectral wolf and conscripts. Verified: the_searchlight_moor (3).
- **Jungle: Professor Tock keeps the meteor watch** (68ed25bf4) on the
  mountain's rim; the test walker climbs the mountain to talk to him.
- **Math: Ivo keeps the Fractal Woods** (811f00ea7), a new Hall wing for
  the Math town's people.
- **Residents for the out-of-town zones**: Range Officer Pryce at the Test
  Range (739b79790), Nell Brine on the Wreckers' Reef's shore (5f9ce8bd5).
  Verified: their talk tests.
- **Science: the gate court furnished** (998423f11) and **the observation
  deck's window** (be6cf39c3).
- **Jungle: the Smoking Mountain** (24d9f7cd4), past the tar pits: basalt
  shelves under a volcano's cone, lava pits and crater (DamageVolumes), the
  meteor watch on the rim. Split frames now overlap by 16 and carry no
  noise across the seam (a frame's edge column is filtered soft).
- **Math: the Fractal Woods** (0b661a7f3), out of town east of the summit:
  the great fractal tree climbed by its limbs to a chest, a Koch hedge, a
  Sierpinski rock, a computed Mandelbrot topiary; puppy slugs, dividing
  mites. The buildings' insides moved to y 2400. Verified: Math, hotel,
  Zeno, gate-network tests (18).
- **Science: Eureka's quad filled in** (5df747b7d): the raised walk's
  arcade, the Uncertain Cup's front, the lecture hall's colonnade, a Klein
  bottle fountain, Tesseract Labs' front with a sparking coil. A still
  platform with no picture is not ground the test walker's navigation
  knows: the quad's walks stay ledges.
- **PvN: the Wreckers' Reef** (67823e0c6), out of the port east of the
  docks: the GOOD SHIP PROBABLY broken in two on the reef, the wreckers'
  false lighthouse, the captain's strongbox; Cutlass Vipers, the
  Quartermaster, a Lookout. Verified: the_wreckers_reef, the_pirate_port.
- **Evil: the Crooked Lantern** (c9de81e52), an inn at the moor's far west
  end: Gudrun and Tobias the trapper (who knows the searchlights' blind
  spots), WANTED posters with the robot's face. Verified:
  the_searchlight_moor (2).
- **Chests at the top of each out-of-town climb** (177b78090): moor
  watchtower, bamboo lookout, tar pits scaffold, Test Range tower.
- **Math: Hylbert's ballroom** (06fe2abc1): a game show of three lit doors
  (stay 1/3, switch 2/3); behind them three closets: a goat, a car with a
  bow and money, a goat. Verified: hylberts_hotel (4).
- **Science: the coil lab's anti-gravity prototype** (21c2d7a4f): DO NOT
  PRESS flips the lab's gravity (this room only); its twin on the ceiling
  (TOLD YOU.) flips it back. Dmitri knows. Verified: the_eureka_tether (3).
- **Jungle: the trail's people** (36bc19547): Ranger Brandvold at the
  trailhead by her new hut, Dr Feld at the tar pits' dig camp (now a quiet
  pocket). `scenery_overlay.py` adds residents and each piece by its own
  name. Verified: the_tar_pits (2).
- **Math: Hylbert's guests**: the Paradox Barber, Joseph Furrier and
  Python Goras at doors of the infinite corridor, with hotel talk
  (`math.yarn` `hotel_*`). Verified: hylberts_hotel (3).
- **Science: the damaged spur looks damaged** (21f654b00): a hull breach on
  the stars, torn deck and severed cables arcing over the live pits, the
  buckled bulkhead, red emergency lamps, the crew's mess.
- **PvN: the Bamboo Pass** (22096138a), out of the village east of the
  cedar forest: the Shadow Clan's sparring ground; grove, rope bridge over a
  gorge, lookout, waterfall and jizo; Ninja Swordsmen and the Oni Leader
  (the plain duelist and the heavy author no locomotion and cannot be
  placed as enemies). Verified: the_bamboo_pass, the_ninja_village.
- **Evil: the castle kitchens and the castle's staff** (f94f1f248): the
  first residents of the Evil dimension (chargen, crew family): Mrs
  Hargreave the cook, Osgood the night porter, Brigid Dunmore of the
  Henchpersons' Guild; talk, catalog, Hall wing; music doves_of_the_commons.
  Verified: the_castle_kitchens.
- **Jungle: the Tar Pits** (16e495c39), past the end of the trail, east of
  the canopy by a side exit (`specs/primeval/tar_pits.py`): two tar pools
  (murky water drawn as tar), a mammoth half sunk (its spine a still walk),
  a sabre-tooth skull, the paleontologists' dig camp and scaffold; raptors,
  a triceratops, a dilophosaurus, compys, a vulture. Verified: the_tar_pits,
  check_flat_map dinosaur 0. The new edge first took the id of the canopy's
  door; only the game refused it, so `validate` now refuses two zones of one
  id in an area.
- **Math: the gate hall's arrivals furniture** (6d519ac43): the Manifest
  Office's booth, the customs desk for axioms on the west gallery, the
  departures board (infinity cancelled), a porter's cart, polyhedral lamps,
  banners. Verified: math tests (5), captures.
- **PvN: the Pirate Sky Lookout's deck** (d3014245e): the lookout's floors
  were invisible over open sky; now a deck on a hull band, crow's nest and
  Jolly Roger, the captain's perch, the companionway house, the sharks'
  hitching rail, a spyglass, an anti-shark gun, four signs
  (`scenery_overlay.py` adds signs too).
- **Science: the tether's ground station hall and car** (c2d4e105f): a hall
  of steel and glass round the ribbon, galleries, console, departures board,
  altimeter, capsule tube; the cars drawn as `eureka_tether_car` (a
  platform look). Engine: moving platforms now draw over a room's scenery
  (a one-picture room hid every platform). Verified: the_eureka_tether,
  render platform tests.
- **Evil: the Searchlight Moor** (392716c7a), out of town west of the gate
  forest by a side exit (`specs/evil_moor/make_specs.py`): a ruined
  watchtower to climb, its searchlight's beam; a murky bog (IntGrid murky
  water, its surface in the art) with a drowned cart; the founder toppled
  face down off his plinth, his back walked on (a still one-way platform
  with no picture); barbed wire; hounds, failed prototypes, risen
  conscripts. Verified: the_searchlight_moor, check_flat_map evil 0, room
  captures. Note: a hidden `SurfaceChain` did not hold a spawned body here.
- **Math: Hylbert's lift only goes up** (995743d24): its top comes back in at
  its bottom. Verified: hylberts_hotel (two laps).
- **Jungle: trail dressing** (9ca12eaba): ferns, fallen logs, nests (one egg
  hatching), a toppled idol head, a half-sunk ribcage in five trail rooms.
- **PvN: the hall of the Shadow Clan** (7d76d7249): shoji, rafters as
  balconies, racks, gong, bonsai shrine, lanterns, and the scroll the
  Mockingbird "corrected".
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
  (science 5 -> 0, math 1 -> 0, pvn 9 -> 2; the village's high road fixed
  since, 09ccc3977). Left: the Mockingbird arena's ceiling exit leads to a
  side (another lane).
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
