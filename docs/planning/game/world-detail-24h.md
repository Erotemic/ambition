# World detail run (24 h, 2026-10-30)

**Tracking:** [WORLD-LAYERS](../queue.md#world-layers--fold-the-remaining-long-dimensions-into-connected-regions)
owns the next topology and room-detail selection. The counts below are a dated
baseline. Recheck current generators before repeating any completed work.

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

## Coverage and next selections by dimension

The October 30 baseline counts and `p0` labels were measured *before* the
same-day world-detail run. They are not current defects. The later authored
regions are listed under **Completed October 30 content run** below. Select
new work only after inspecting the present generators, checking that each
room is reachable and legible, and identifying a concrete missing interaction
or authored detail. `WORLD-LAYERS` owns selection.

- **Science:** The run added the Eureka Collider, Test Range, steam plant,
  GNU-ton orchard, tether car/terminus and station/spoke dressing. Check
  reachability and coverage rather than re-requesting these as empty rooms.
- **PvN:** Pirate Cove, Wreckers' Reef, Lighthouse Point, Bamboo Pass,
  Wreckers' camp and other out-of-town content landed. Verify whether the
  Mockingbird arena/sky, dojo and forest need further art or interactions.
- **Math:** Zeno's Aqueduct, Fractal Woods, Mobius Mine and Tower of Hanoi
  provide out-of-town locations, so the old “none yet” item is complete.
  Check the current gate hall and connecting routes before choosing more.
- **Jungle:** The Tar Pits, Smoking Mountain, fossil beds, ranger station,
  rookery and treetop route work landed. Check current traversal and enemy
  encounter density before extending trails.
- **Evil:** Searchlight Moor, Lower Godwin, castle kitchens, allotments,
  founder's crypt and forest road landed. Verify any remaining empty approach
  or underused room by inspection, not the old eight-room count.

Keep the world-building scope: authored places, residents, discoverable
interactions and occasional danger; do not turn this into a quest-chain plan.

## Completed October 30 content run

This was a dated **implementation log**, not a live task queue. The rooms,
people, route links and interactions below were authored during the run.
The source generators, content pack and map assets now own their details;
Git history contains the change-by-change reports. Revalidate any room
coverage before redoing or extending it.

- **Jungle:** the fossil beds; the ranger station; rewards for the climb; the rookery's birdwatcher; the mammoth's skull; Professor Tock keeps the meteor watch; the Smoking Mountain; the trail's people; the Tar Pits; trail dressing; the trail walks through side exits; the Tyrant King's hall.
- **PvN:** the cellars under customs; the sail loft; Matsuda referees the Bamboo Pass; the Lighthouse Point; Ume's tea house inside; the charcoal burner's camp; the Wreckers' Reef; the Bamboo Pass; the Pirate Sky Lookout's deck; the hall of the Shadow Clan; the Pirate Cove.
- **Other:** Map (Jon's shift, 10-30); The Magical dimension (Mary-O's), begun; Enemy patterns (Jon's shift, 10-30); Secrets in every region; PvN composed; Math (subagents, Jon's asks); Residents for the out-of-town zones; Chests at the top of each out-of-town climb; A faithful flat map.
- **Magical:** more of Mary-O's cast at home; the Unlit, where the void is hostile; the Void.
- **Math:** two keepers; the Tower of Hanoi; the Mobius Mine; Zeno Halfway; Monty Carlo hosts the ballroom; Ivo keeps the Fractal Woods; the Fractal Woods; Hylbert's ballroom; Hylbert's guests; the gate hall's arrivals furniture; Hylbert's lift only goes up; Hylbert's Hotel; Zeno's Aqueduct.
- **Evil:** the guild meeting; Crumb's bakery inside; Lower Godwin; the Prototype Workshop; the Castle Allotments; the Complaints Department; the founder's crypt; the Crooked Lantern; the castle kitchens and the castle's staff; the Searchlight Moor; the forest road.
- **Science:** the vector well furnished; the rest of GNU-ton's works; GNU-ton's orchard and its apples; the Eureka Collider; Ottoline Hum busks; the spoke's tube; the tether's climb; the tether terminus; the service deck's clutter; the gate court furnished; Eureka's quad filled in; the coil lab's anti-gravity prototype; the damaged spur looks damaged; the tether's ground station hall and car; the Test Range; Tesseract Laboratories; Eureka's tether carries its rider.
- **The Magical dimension:** dressed, and the Void Castle.
- **Math composed:** the roots.
- **Science composed:** the steam plant under the Eureka quad.
- **Jungle composed:** the treetops.
- **Evil outdoors composed:** dense connected regions instead of isolated strings of rooms.
- **Fixed:** murky water never landed.
- **PvN flat map:** the village's high road.

## Unresolved observations from that run

- **Consignment errand:** the October 30 note said
  `the_consignment_errand` still expected “Please count the lenses” after
  Georg Canter's choice text changed in `697058efa` (2026-10-24).
  Re-run the test against current source before calling this a live defect.
- **Tar/murky-water presentation:** there was a maintainer question about
  whether tar should be black rather than the shared dark-teal murky water.
  The run later reports a rendering repair; verify the current appearance
  before proposing another change.
- **World coverage:** the prior dated counts were superseded by the run.
  `WORLD-LAYERS` owns new selections after a current-source inventory.

