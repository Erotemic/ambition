# Level 1: Hobford

**State:** OPEN. Rebuilt 2026-11-14 around a rural gate settlement. The
eight rooms of the level are built by the generator
(`generators/intro/make_specs.py`): the workshop and its raid, the lane and
Alice, the meadow, the gate clearing and its ring, the freight siding fight,
the millstream, the village and Bob in the woods. The rooms are drawn with
the level's own looks (`hobford`, `hobford_woods`, `creator_workshop`);
props and villagers are not made yet. A new game starts in the sandbox hub
(D3). Queue row
[LEVEL-1](../queue.md#level-1--the-lab-raid-and-the-first-level-after-it)
holds what remains.

Jon, 2026-11-14, first: "redesign the entire thing so it feels like a real
game", "really making the lab raid work is included", and "we are basically
looking to build level 1 tutorial level, meet some of the cast, learn some
mechanics, and start meeting some challenges"; it is not a tutorial, only
situations that make the player learn the basic mechanics, with one jump,
no projectiles and no ranged items. Then the brief that this page follows:
a rural region newly joined to the gate network, the robot named Scout, the
raid as an interruption, an escape into open country, Alice as the first
friendly character, and three directions the player chooses between. On
decisions: "For any decisions you need input on, make a reversible decision
with your best judgement." Each one is under **Provisional decisions**.
None of it is canon (`docs/storylines/cannon.md` is not touched).

This page replaces the basement lab, the service stair and the drain level
(the Drain Market and the culvert stay, as town rooms past the ring).

## Canon this level stands on

- "Alice and Bob are cartographers. They gate the map system. They provide
  quests."
- "The military and other factions control various larger
  'dimension-gates'. There happens to be one nearby the creator's lab."
- "Oiler is the lead mechanic in the largest town." The largest town is not
  in this level, so Oiler is not either (D4).

### The kit

Every room of the level withholds a player's kit past run, one jump, drop
through and the strike: the level field `withheld_verbs`
(`RoomMetadata::withheld_verbs`), which the generator writes into each room
of the level (`SMALL_KIT_WITHHELD`). The engine writes it into the player's
body as a ceiling each tick (`room.withheld_verbs`,
`docs/planning/engine/capability-progression-and-world-gating.md`), so the
robot has its whole kit again past the level. Withheld: the air jump, the
wall verbs and the ledge grab, the dash and the dodge, flight and the
glide, blink, the pogo, the special verb, the shield, the grab and the
morph. Witness: `level_one_withholds_the_large_kit`, with the hub as its
control. The town rooms (the gate stack, the Drain Market, the culvert) are
past the level and keep the whole kit.

The shot is withheld too (2026-11-14): `ranged` is a permission every kit
has (`AbilitySet::ranged`, true in `NONE` and every preset, since a shot is
already gated by being authored) and exists to be withheld. Without it the
action scheme has no projectile slot, so the prompt shows no Shot, and the
control gate clears a charging body's charge stream, so the Hadouken does
not fire. Level 1's `SMALL_KIT_WITHHELD` names it. Witnesses:
`level_one_withholds_the_large_kit` (with the hub as control) and
`an_authored_charging_character_keeps_its_projectile_press`'s withheld
gunner.

## The level in one paragraph

Hobford is a farming settlement in hill country that was connected to the
gate network last year. The old stone ring in the clearing above the village
has new steel on it: a freight siding, a guard post, cables, a timetable on
the notice board. The Creator rents a barn outside the village as a robotics
workshop. He is testing a new fine-tuning method on a robot he calls Scout.
Scout wakes on the charging cart, he runs ordinary checks, they talk, and
goblin raiders break in partway through. The goblins were hired to rob a lab
from a list that had the wrong address. They take what they can carry anyway.
The Creator is hurt in the fight and dies before he finishes his last
sentence. Scout leaves the barn into open country. Alice, a cartographer
whose survey station is on the slope above the barn, comes down because she
heard the fight. She tells Scout what she saw if asked, warns that following
the goblins is dangerous, and asks Scout to take a message to Bob, who is
camped in the woods on the goblins' route. Scout can do that, go after the
goblins (they are waiting at the freight siding for the next freight window),
or walk around the country. Every way is open from the start.

## Rules

- **The place gives the obstacle.** A stream, a hedge bank, a loft, a
  shut barrier, a goblin. No sign tells the player which button to press.
- **The small kit.** Run, one jump (rises about 88 px), drop through a
  one-way floor, the strike. Every room carries `withheld_verbs`
  (`SMALL_KIT_WITHHELD`). Swimming is not withheld: water slows, it does not
  kill.
- **No route is gated on a story fact.** A door changes because a lock, a
  lever or an obstruction changes. One shortcut is the exception (D21). Knowing a route (Alice's map, Bob's marks)
  helps find it; every passage is walkable without it.
- **Any order.** The three directions (Alice's request, the goblins, the
  country) are reachable from the first exterior room, and each can be done
  first, last or never. Later lines read what happened (world facts and
  items), so returning anywhere never contradicts.
- **Every required route is walked by the census** with the level's own
  kit (`walked_route_census`, the robot's abilities intersected with
  `AbilitySet::all_but_listed(SMALL_KIT_WITHHELD)`), not only checked by
  `check_reach`, which compares heights and nothing else.
- **Writing** follows `docs/authoring/anti-llmism-style-guide.md`. The
  Creator is a practical engineer: no destiny speech. Gem, Dil and Harper
  are not in the opening and are not named in it.

## Map

```
[THE WORKSHOP]--[HOBFORD LANE]                                          [THE HIGH MEADOW]--[THE GATE CLEARING]
 intro_wake_room  intro_hobford_lane                                     intro_high_meadow   intro_gate_clearing
                        \ the bank path down             the hill path up /                   \ the freight track down
                         [THE MILLSTREAM]--[HOBFORD]--[HOBFORD WOODS]                             [THE FREIGHT SIDING]--[THE FELL]
                          intro_millstream  intro_hobford intro_woods (Bob's camp)                 intro_freight_yard    intro_fell
```

One line of rooms, west to east: the barn on the west hill, the valley, the
gate on the east hill, and past the siding the fell, the mountain beside
the gate's hill. Each join is an edge at one height; the lane, the
meadow and the clearing are hill rooms that climb between the hilltops'
height and the valley's on slopes. The design and its reasons are in
[Hobford polish](hobford-polish.md) ("The land").

Ids: `intro_wake_room` and `intro_freight_yard` are kept (tests, the gate
network and saves name them). The rest are new. Removed from the intro
world: `intro_escape_shaft`, `alice_relay`, `bob_relay`. The Drain Market
(`drain_alley`) and the culvert (`under_town_pipes`) stay as town rooms
under the gate stack, outside level 1 (D5).

| room | title | what the place asks | who is there |
|---|---|---|---|
| `intro_wake_room` | THE WORKSHOP | walk, the first jump (the test plinth), up to the parts loft and back down (drop through) | the Creator |
| `intro_hobford_lane` | HOBFORD LANE | the first open air, high on the west hill; the bank path down to the mill | Alice, hens |
| `intro_high_meadow` | THE HIGH MEADOW | the hill path up from the woods, the shepherd's terrace, a dry-stone wall | a farm dog |
| `intro_gate_clearing` | THE GATE CLEARING | the gate on the hilltop, the guard post, the barrier and its lever, the freight track down | the gatekeeper (`npc_valley_gatekeeper`, D20) |
| `intro_freight_yard` | THE FREIGHT SIDING | the goblins' camp: a fight in tiers of stacked freight | goblin raiders, their foreman |
| `intro_fell` | THE FELL | the climb: the drove road up slopes, the sheepfold's wall, the scree, two crags, the top and its cairn | the shepherd (`npc_valley_shepherd`) |
| `intro_millstream` | THE MILLSTREAM | stepping stones, the mill wheel, the pond (swim), a cracked wall behind the wheel | the miller (`npc_valley_miller`) |
| `intro_hobford` | HOBFORD | the street, roofs and lofts, the notice board, the well | the trader and the innkeeper (`npc_valley_trader`, `npc_valley_innkeeper`) |
| `intro_woods` | HOBFORD WOODS | roots and fallen trunks, the goblins' trail, a hollow tree | Bob |
| `intro_salt_run` | THE SALT RUN | under the valley, dark (75%): the old mill race's steps and channel, the foot of the well, boards up the oak's roots | puppy slugs |

## Scene by scene

What is built. Ids of dialogue nodes are in
`assets/dialogue/sandbox/intro.yarn`, cutscenes in
`assets/data/cutscenes/intro.ron`, fights in
`assets/data/encounters/goblin_encounter.ron`.

### The workshop (`intro_wake_room`)

A converted barn, one floor, and its yard: the loft at the west end (a
crate up to its one-way boards, the torque gauge at the far end), the test
plinth (a step of 48), the charging cart and its ramp, the Creator at his
bench, the barred barn door, and the yard outside it to the lane. The cart
stands beside the Creator, so the first view holds Scout waking and him at
his bench; the door is in the same view when they talk, and the yard is out
of it (staging rules: [level design guide](../../authoring/level-design-guide.md)).

1. `intro_wake` (entry cutscene): fade up on the cart. The Creator: "Hey
   you, you're finally awake." Then "Don't sit up fast. The left knee servo
   is new and I haven't set its torque yet."
2. `creator_intro`: he asks for the torque gauge, which is up on the loft
   ("Use the crate."). Scout can ask who he is and where this is, go for the
   gauge, or ask for the questions first. The gauge is a pickup that writes
   `intro_torque_gauge_found` (D15); his next talk reads it, reads both
   knees ("Twelve. They match.") and sets `intro_joints_checked`.
3. `creator_intro__questions`: the evaluation of his method. Scout's name
   (`scout_gave_its_name` when it answers "Scout."), the wrench and the
   hammer, then "Is there anything you want to ask me?". His answer lands
   first; then "Hang on. Was that a cart?" calls the raid
   (`intro_raiders_coming`: the room's `while_live` line starts it while
   that flag is set). The raiders come up the yard out of shot. "The door's
   barred. It'll hold." Then the bar breaks (`intro_barn_door_broken`, the
   barn door's `gated_by`) and they walk in through the door, in view.
4. `creator_intro__interrupted`: the first raider within earshot breaks into
   the conversation (`conversation.join npc_lab_raider`). One reads the job
   sheet: "It says a lab with a big sign. This is a barn."
   (`wrong_list_clue_1`). The Creator: "Who sent you? Put that d—". The
   conversation ends and the fight is on.
5. The raid is called the other way too: Scout goes to the barn door
   without talking to him (`intro_raid_at_the_door`, a cutscene volume in
   front of it, sets both flags a beat apart). The goblins come either way.
6. The raid: three raiders in one wave, up the yard and in at the door. The
   yard gate (a `LockWall` with no `gated_by`, the raid's own wall) stands
   while the fight is on. Its clear greens a raider's dropped whistle (a
   `Switch`, `ResetEncounter`); blowing it calls the raid back. A won raid
   spawns finished, so the room's line does not start it again.
7. Out in the yard, once the raid is won (`after_encounter` on the trigger),
   `creator_final_fragment`: "Scout. Over here. Are you damaged?", the case
   taken off the bench with Scout's logs in it, "I have to sit down for a
   minute.", "You answered like yourself, all the way through. I want to
   write that d—". He falls (`Fall npc_creator`, `intro_creator_dead`).
   Q173's default (D6).

### Hobford Lane (`intro_hobford_lane`)

The first open air: the lane outside the barn, high on the west hill, hens,
and the valley below with the gate far off on the east hill. Alice's survey
line is pegged across the lane west of her station (`alice_survey_line`, a
24 px `LockWall` on `bob_field_survey_received`): a step over, until Bob's
survey is in. Past her the bank path goes down to the mill in two pitches
with a landing between; the lane's east edge is at the stream's height.

**Alice** (`meets_the_player`), `alice_lane`:

- First meeting (`alice_met_scout`): "Hello? I heard shouting from the barn.
  Is he...?" Scout answers "He's dead. Goblins came in." or "Who are you?".
- `alice_lane__topics` (optional, any order): what she saw (five goblins,
  a handcart with a crate, up the track toward the ring; they will sit at the
  siding until the freight window; "One of them had a hammer as long as I
  am. I'd leave them alone.", `alice_told_goblins`); what she is measuring;
  her request; or go.
- `alice_lane__ask`: Bob is camped in the woods past the village, on the
  trail from the siding, and does not know the goblins are here. "I'll tell
  him." sets `alice_route_note_carried` (the flag chain adds
  `map_basic_unlocked`); "Not now." sets nothing.
- Later lines read what happened: Bob told (`alice_lane__bob_told` sets
  `alice_heard_bob_moved`, and thanks Scout differently if she never asked),
  the case recovered (`intro_creator_notes_recovered`), the siding fight won
  (`encounter.cleared intro_freight_yard_fight`).

### The high meadow (`intro_high_meadow`)

The hill path climbs out of the woods in two pitches. Halfway up is the
shepherd's terrace, with his hut (a step of 64), the oak and the farm dog
(`companion_dog_roam`). At the top a dry-stone wall to hop and the road on
to the clearing.

### The gate clearing (`intro_gate_clearing`)

Standing stones, the guard post, the barrier (`hobford_barrier`, a
`LockWall` on `world.flag_set hobford_barrier_lifted`) and its lever beside
it on the road's side. The lever sets the flag and nothing clears it, so
the barrier stays up once lifted. Behind it the gate of the new facility
`hobford_ring` (address TILLAGE, D18), its dialer and terminal. Past the
gate the freight track goes down the hill to the siding.

### The freight siding (`intro_freight_yard`)

Stacked freight and pallets on the platform. `foreman_siding` at the foot of
the freight track ("That's the robot from the barn. It walked here."). The fight
(`intro_freight_yard_fight`): two raiders, then the brute and another
raider. The platform bell is its linked switch. The work order on the
goblins' crate (`work_order`): "THE WORKSHOP AT THE OLD MILL, HOBFORD", not
the barn (`wrong_list_clue_2`, D17). The crate holds the Creator's case
(`intro_creator_notes_recovered`, a story-flag reward, D8); the quest
`intro_creator_case` reads it. It is a dead end below the gate; the way
out is back up the track.

### The millstream (`intro_millstream`)

A west bank with a hollow under it behind a cracked wall (breaks to the
strike; coins inside, and a hatch in its floor down to the salt run), the
stream with stepping stones flush with the bank
(a miss drops Scout in the water, which slows and costs a climb out), the
mill's porch, upper floor and loft window (coins). East is Hobford.

### Hobford (`intro_hobford`)

Cottage, smithy and inn with porches and roofs to climb (coins in the
smithy loft), the well, signs for the gate goods shed and the inn, the
notice board (the freight window, a lost hen, strangers at the ring). West
the millstream, east the woods.

### The salt run (`intro_salt_run`)

The smugglers' passage under the valley (2026-11-16, [TN2](../engine/dark-rooms-and-tunnels.md)):
salt came this way and not through the gate's toll (the smithy sells it as
gate goods). From the hatch in the millstream's hollow the old mill race
steps down to its channel (ankle-deep water, two puppy slugs), climbs out
under the village past the smugglers' chalk tally ("112 SACKS. 0 TOLLS."),
reaches the foot of the well under the village's well (its shaft open
above, a bucket of coins: "SOMEBODY UP THERE THREW A WISH DOWN HERE"), and
runs on to a note ("IF YOU CAN READ THIS, YOU ARE NOT WORTH YOUR SALT. TURN
BACK.") and the boards the smugglers wedged between the oak's roots, up to
the shelf under the oak and the way up into the woods. It is a way, not a
gate: the street above is always open. Dark at 75% (DR1); the
smugglers' lanterns light it from the hatch to the well, and daylight falls
faintly down the well's shaft (at High, DR2's light map); past the well it
is dark, and the light is the one you carry: a spare lantern hangs by the
hatch (the Lantern, a key item that widens your light). Its ways out are
ladders up to a trapdoor at each end.

### Hobford woods (`intro_woods`)

Roots, a fallen trunk, a hollow oak split open with three shelves inside
(a health pickup at the top) and a gap between its roots down to the salt
run,
the lost hen, a goblin lookout at the far end. **Bob**, `bob_woods`:

- Without Alice's message: "Just walking through." (the noise on the
  trail), "What are you doing out here?", "There are goblins at the
  siding." (he moves anyway), or leave.
- With it: "Alice says move your camp off the trail. There are goblins at
  the siding."
- `bob_woods__warned`: he moves up by the hollow oak, gives his survey sheet
  (`fieldsurvey`) and sets `bob_field_survey_received` (the flag chain adds
  `map_private_marks_unlocked`). That takes up Alice's line across the lane.

East, the goblins' trail out of the trees and up the hill path.

### Choices

From the lane the player can, in any order or not at all: talk to Alice and
take her message (`intro_cartography_route`, "A Message for Bob": Alice,
Bob, Alice again); go after the goblins at the siding (the fight, the work
order, the Creator's case); walk the country (the meadow, the clearing and
its barrier, the millstream's hollow, the village roofs, the woods' hollow
oak). Bob can be warned without Alice's message. One way changes on a
story fact: Alice's line across the lane comes up when Bob's survey is
closed (D21). It is a step over, so no way waits on it.

## Systems used

- `withheld_verbs` on every level-1 room (`SMALL_KIT_WITHHELD`).
- Encounters with waves, each with a linked switch, so a won fight stays
  won (the whistle, the bell).
- A room's `while_live` line held back by `while_live_when` (a condition
  line): the raid starts when the conversation sets its flag. Dialogue
  cannot start an encounter itself (it cannot name an occurrence).
- Multi-party conversation: `conversation.join` brings the raiders into
  `creator_intro__interrupted` (the coordinator's PRIORITY 2, landed).
- Gated `LockWall`s on `encounter.cleared`, `world.switch_on` and
  `world.flag_set`.
- Story-flag pickups and chest rewards (`flag:` kinds); content validation
  counts a chest's story-flag reward as a writer of its flag.
- The gate network: a facility and ring row in `gate_network.ron`, the zone
  in `gate_portals.ron`.

Requests to the engine, none needed for the level to work:

1. **Item custody for stolen property** (a raider who spawns carrying an
   item and drops it where it falls). Coordinator: not now; the case is in
   the goblins' crate (D8).
2. **A looter's goal** (a raider who leaves with an item). Not now.
3. Multi-party conversation: landed and used.
4. A "take" verb in dialogue: not needed. The gauge is a story flag, so the
   Creator reads it and nothing leaves the bag.

## Art

Drawn in `tools/ambition_sprite2d_renderer` and registered in `SCENES`
(`backgrounds/hobford.py`):

- `hobford`: hill country on a clear morning. Cumulus, far hills, fields
  and hedgerows, the village roofs, the mill and its wheel, the ring on its
  hill with scaffold and a crane, cable poles, a fence, an oak and hedge in
  front.
- `hobford_gateside`: the same country without the ring and its steel, behind
  the gate clearing and the siding (level field `backdrop`).
- `hobford_woods`: trunks, canopy and light shafts.
- `creator_workshop`: the barn's plank wall, trusses, windows, shelves and
  a tool board.

Each look has a terrain skin (turf over soil; the woods' darker soil; barn
floorboards), motes, a door and six decor pieces (`terrain/skins.py`,
`motes.py`, `doors.py`, `decor.py`), and its rows in
`game/ambition_content_library/authored/catalog.json`
(`background.parallax.<look>.*`, `room.dressing.<look>.*`; without them a
room draws the default grid). Published with
`scripts/regen/backgrounds.sh --theme <look> --skin <look>`; the generator's
`WORKSHOP_LOOK`, `COUNTRY_LOOK` and `WOODS_LOOK` name them.

The props are `targets/props/hill_village_props.py` (nine pieces with
name-neutral `hill_*` ids), published, placed and captured
([Hobford polish](hobford-polish.md) row 8). The villagers are the valley's
residents (row 9 there).

## Provisional decisions

| # | decision | how to undo it |
|---|---|---|
| D1 | The settlement is **Hobford** (a hob is a household sprite in English folklore; the village had no goblins until the gate opened). The region has no name of its own yet. | rename titles and lines; ids carry `hobford`, so rename the rooms too if the name goes |
| D2 | **Scout** is the robot's name in the dialogue only. The protagonist row keeps `display_name` "Player Robot v3", which engine crates' tests pin (`ambition_platformer2d_actor_monolith`). | set the row's `display_name` and update those tests |
| D3 | A new game starts in the sandbox hub (`worlds.ron` `entry_room: "central_hub_complex"`). Jon confirmed this on 2026-11-17. The hub's door leads to the workshop. Tests of the hub and the robot's full kit name the hub explicitly. | Change `entry_room` when the maintainer requests a different start. |
| D4 | Oiler is not in level 1. He stays at the Drain Market under the gate stack (canon: the largest town's mechanic). His lines name Hobford. | put his `NpcSpawn` back in a level-1 room |
| D5 | `drain_alley` and `under_town_pipes` stay as town rooms outside level 1; their ways to the removed rooms are walled. The culvert's raft walk (`across_the_raft`) is deleted with its crossing. | regenerate the old rooms from the previous generator in git history |
| D6 | The Creator dies (Q173's default). Open: whether something the player does could keep him alive. | a branch in `creator_final_fragment` on a world fact |
| D7 | The raid starts at a line of the conversation (`encounter.start`), and the raiders join it at `creator_intro__interrupted`. | move the start back to a trigger volume |
| D8 | Until item custody can carry it, the Creator's case is a story flag (`intro_creator_notes_recovered`) in the goblins' crate at the siding, with its own quest (`intro_creator_case`). | move it into a raider's hands when custody lands |
| D9 | Alice's message: move the camp off the goblins' trail. Bob returns her chain himself. | rewrite her node |
| D10 | The clearing's ring is a ring of a new facility (`hobford_ring`, dimension `hobford`); its barrier and lever stand in front of it. | delete the facility and ring rows; wall the clearing |
| D11 | Music: `something_worth_building` (the workshop), `first_goblin_tune_v2_radio` (the siding), `a_possible_morning` (the country), `doves_of_the_commons` (Hobford). | change the room fields |
| D12 | Wildlife from the existing cast: hens and the farm dog. No puppy slugs in the country. | add rows to the rooms |
| D13 | `intro_wake_room` and `intro_freight_yard` keep their ids with new contents; the other level-1 rooms get new ids. | none needed: ids are internal |
| D14 | The town rooms (the gate stack, the Drain Market, the culvert) do not withhold the kit: they are past the level. The gate stack's old checkpoint (barrier, release switch, the door from the yard) is removed: it shut the combat lab's edge. | write `withheld_verbs` into those rooms |
| D15 | The torque gauge is a story-flag pickup (`intro_torque_gauge_found`) and the Creator reads the flag; no take verb. | an item and a take verb |
| D16 | Eve's cameo in the Drain Market is cut (her room went with the drains). | place `npc_eve` in a town room |
| D17 | The goblins' work order names "THE WORKSHOP AT THE OLD MILL, HOBFORD": they hit the barn because it had a robot in it. The mill in the millstream room is the address they meant. | change the work order's text |
| D18 | The Hobford ring's address is TILLAGE (inscription letters only). Its posted list is LEGAL; the note: "Freight window daily at noon. Persons by appointment." | change the ring row |
| D19 | The raid also starts when Scout walks to the barn door without talking to the Creator (its trigger is between him and the door), so the barn cannot be a dead end. The sim's agent action has no menu select, so the walked route takes this start and the playthrough the conversation. | put the trigger out of reach again (the raid then needs the talk) |
| D20 | Built 2026-11-14 ([Hobford polish](hobford-polish.md) row 9): the gatekeeper, the miller, the innkeeper and the trader, new chargen residents with their own Yarn nodes. | more villagers as the village grows |
| D21 | Alice's survey line across the lane is a low `LockWall` (a step over) on `bob_field_survey_received`, against the brief's "no story-fact gates": the Alice/Bob acceptance tests (`the_note_reaches_bob_and_opens_alices_return`, `the_hand_over_opens_the_wall_in_the_other_participants_room`) pin a wall that opens on Bob's survey. It shuts nothing. | a lever or a cut for the line, and port the two tests to another fact |
