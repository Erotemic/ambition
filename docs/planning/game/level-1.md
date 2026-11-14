# Level 1: the wrong basement and the Drain Market

**State:** OPEN. Design written 2026-11-14. Rooms 1-7 and 9 and the
checkpoint of room 8 are built (2026-11-14); queue row
[LEVEL-1](../queue.md#level-1--the-lab-raid-and-the-first-level-after-it)
holds what remains.

Jon, 2026-11-14: "redesign the entire thing so it feels like a real game.
Not just a demo, I want to start building the real intro sequences and world
structure, basically everything in that first level after the lab raid needs
to get completely redone", "really making the lab raid work is included",
and "we are basically looking to build level 1 tutorial level, meet some of
the cast, learn some mechanics, and start meeting some challenges". Then:
it is not a tutorial, only situations that make the player learn the basic
mechanics. Assume one jump, no projectiles and no ranged items; the player
can pogo, "although maybe pogo is advanced for that zone".

This page replaces the old intro chain (the stabilizer kit on the floor,
Alice's battery for Bob, the system boss's "P5 route memory", the labels
that said `P2:`, `MAP_SECRET:` and `Task 08`).

## Rules for this level

- **The place gives the obstacle.** A gap, a wrecked stair, a shut
  checkpoint, a goblin in the way. No lever that needs a story, and no sign
  that tells the player which button to press.
- **One jump, a blade, and the eyes.** Every required route is possible
  with run, one jump (it rises about 88 px; a level gap of about 110 px is
  safe), drop through a one-way floor (down + jump) and the melee strike.
  No required route uses the air jump, a wall jump, the dash, the blink,
  flight, a projectile or the pogo. A pogo can reach an extra, never a
  required way.
- **Each room asks one new thing, then asks it again harder.** The first
  time is safe (a fall costs a climb, not a life); the second time has a
  cost.
- **Levels, not shafts.** Rooms are tiered, with platforms between the
  tiers. No ladders.
- **Signs are what people in the place wrote.** Room titles and short
  in-world signs only.
- **Fia is not in this level.** No line names her, so the level stands if
  the character is reworked or cut.

### The kit

Every room of the level withholds a player's kit past run, one jump, drop
through and the strike: the level field `withheld_verbs`
(`RoomMetadata::withheld_verbs`), which the generator writes into each room
it makes (`SMALL_KIT_WITHHELD`) and the gate stack carries by hand. The
engine writes it into the player's body as a ceiling each tick
(`room.withheld_verbs`, `docs/planning/engine/capability-progression-and-world-gating.md`),
so the robot has its whole kit again past the level. The basement lab is
the exception until the rural opening replaces it: under the small kit the
playthrough's fight carried the player back over the raid's trigger, the
raid started again and shut the bay door. Withheld: the air jump,
the wall verbs and the ledge grab, the dash and the dodge, flight and the
glide, blink, the pogo (the level teaches the plain strike first), the
special verb, the shield, the grab and the morph. Witness:
`level_one_withholds_the_large_kit`, with the hub as its control.

Not yet: shots. The Hadouken is the projectile kit's, which no verb gates,
so the robot can still fire in the level (follow-up in the queue row).

## The story in one paragraph

The robot wakes in a basement lab. Its creator has just finished it. Hired
goblins break in for "one prototype, one lab of research, paid in advance";
the creator and the robot fight them off, but the creator is hurt and falls
at the foot of the stair. His last words do not finish ("I made you to ask
a question. Find out what it—"). The goblins' foreman, the Salvage Lead,
is already gone up the stair with the creator's notebooks. Up on the street
nobody is shocked: the Drain Market is busy, the news board already
prints the official version, and Oiler, a mechanic, comes over to look at
the robot's knee. The goblins took their crate east toward the gate stack,
where a checkpoint lets through only what is on a manifest; the robot is on
none. The cartographers Alice and Bob know the drains under the checkpoint.
At the far end the robot finds the goblins' freight yard and takes the
notebooks back from the Salvage Lead. The notebooks do not answer the
question; they show that the raid was sent to the wrong address.

## Rooms

Room ids that tests and other worlds already name are kept; titles change.
A room in `intro.ldtk` that this level rebuilds is written by
`game/ambition_map_assets/ambition_content/generators/intro/make_specs.py`.

| # | room id | title | what the place asks | cast |
|---|---|---|---|---|
| 1 | `intro_wake_room` (lab half) | THE BASEMENT LAB | walk, the first jump (down the cart's ramp; the creator's bench, a step of 48; a cable trench) | the Creator |
| 2 | `intro_wake_room` (raid half; `intro_raid_corridor` is merged into it) | THE LOADING BAY | the strike: two raiders, then two more, and the bay door shut until the last falls | the Creator, goblin raiders |
| 3 | `intro_escape_shaft` | THE SERVICE STAIR | single jumps up a wrecked stair in tiers; a raider on a landing; vermin that do not fight | puppy slugs |
| 4 | `drain_alley` | THE DRAIN MARKET | the street: talk, Oiler's bench (a shrine), the awnings up to a purse, the drain cover down | Oiler, the news board |
| 5 | `under_town_pipes` | THE CULVERT | gaps over the sump (a fall stings and puts you back), a raft that crosses the wide one | a goblin lookout |
| 6 | `alice_relay` | ALICE'S STATION | the survey line across the high walk; the low walk on to Bob | Alice |
| 7 | `bob_relay` | BOB'S END | the wet stretch (jumps over sump pits); the way on | Eve (listening at a pipe), Bob |
| 8 | `gate_stack_lower` | THE GATE STACK | the rings; the checkpoint that stops the robot | the Gate Janitor, the Manifest Clerk |
| 9 | `intro_freight_yard` (new) | THE FREIGHT YARD | a fight that shuts the yard; the Salvage Lead (a brute: read the windup, strike early or after) | the Salvage Lead, goblins |

Room 8 is LDtk-owned: it keeps its rings and gate network
(`data/gate_network.ron`), and the checkpoint was added to it by hand.
Every other room is written by the generator.

### 1-2. The basement lab and the loading bay (`intro_wake_room`)

One room, one floor, so that the creator can keep beside the robot (in the
old build he stopped at the head of the 128 px stair between the two
levels and died 1400 px behind the player).

- Wake on the diagnostic cart (`intro_wake`). The cart's ramp is the first
  slope; the creator's bench (a step of 48) is the first jump that is
  needed; a cable trench (96 wide, 48 deep) is the first gap, and falling in
  costs a jump.
- The raid starts at a volume where the lab opens into the loading bay
  (`intro_raid`). The goblins say why they came. The bay door at the far
  end is shut (a `LockWall` gated on the raid's last raider:
  `encounter.cleared intro_bay_raid`; the waves are in
  `data/encounters/goblin_encounter.ron`).
- The fight: two raiders in front, then two more from the bay. The creator fights
  too (`follows_closely`, and the room is `dangerous`).
- The raid stays won: its clear greens the raiders' whistle (a `Switch`
  linked to the encounter), and an encounter with no linked switch would
  start again each time the player walked back into its trigger. Blowing
  the whistle calls the raid again, for practice.
- When the bay is clear, the creator's last words play at a volume in front
  of the bay door (`creator_final_fragment`), ending on his `Fall`. He was
  hurt in the fight; he dies when it is quiet. Q173's default stands (he
  always dies here).

### 3. The service stair (`intro_escape_shaft`)

The raid wrecked the stair. The room is wide, in three tiers: broken
flights (steps of 48), a pit, a hole in the middle landing, a gap where a
girder fell. One raider waits on the middle landing. Puppy slugs live in the
rubble; they do not fight. A fall through a hole costs a climb, not a life.
The sign on the lab's door says `14 CULVERT LANE`. The top landing opens on
the market.

### 4. The Drain Market (`drain_alley`)

A street under the gate stack. Stalls with awnings (one-way ledges) make a
second level over the street. The area card names the place (the old
arrival banner is gone). Oiler comes to meet
the robot (`oiler_notices`, his `meets_the_player` brain). His bench is a
shrine (the first save point). In talk he straightens the knee, says where
the goblins went (east, with a crate, toward the checkpoint) and that the
cartographers know the drains. The news board prints the official version
and can be fought (the Easter egg is kept). Ways out: east to the gate
stack; the manhole down to the culvert; Alice's hatch (shut until her
survey is done); the freight yard's shortcut door (later).

### 5. The culvert (`under_town_pipes`)

A storm drain under the market. The sump at the bottom stings
(`DamageVolume`, one damage and a knock back out, painted murky water).
Gaps between stepping stones, each a single jump; the widest is crossed on
a raft (`MovingPlatform`). A goblin lookout stands on the far walk. A
cracked wall at the west end of the middle walk breaks to the strike and
hides a can of oil (health).

### 6-7. Alice's station and Bob's end (`alice_relay`, `bob_relay`)

Alice and Bob are surveyors in the middle of a traverse: Alice at the
instrument at one end, Bob at the staff at the other. Neither can leave
their station without losing three days of readings, and they do not shout,
because the drains carry every word up into the market. (The listener in
the drain is Eve.) Alice asks the robot to take her reading to Bob, and
gives it the second word that Bob will ask for. Bob checks the word, takes
the reading, closes the traverse and gives the robot his field survey (the
private marks of the drains, including the way under the checkpoint). The
robot can also give Bob a wrong number; Bob notices the missing word.

Facts (kept from the old chain, because the save and the quest use them):
`alice_route_note_carried` (the robot carries Alice's reading, and her
map: `map_basic_unlocked`), `bob_field_survey_received` (Bob has the
reading; the robot has the survey: `map_private_marks_unlocked`). The
survey line across Alice's high walk is a `LockWall` gated on
`bob_field_survey_received`: when the traverse is closed, the line comes
down and the high walk leads to Alice's hatch up into the market. Bob's
grate, past his station, is gated on the same fact: Bob opens it when the
traverse is closed, and the drain beyond runs under the checkpoint to the
freight yard.

### 8. The gate stack (`gate_stack_lower`)

The rings, the janitor and the manifest clerk stay. The checkpoint: a
barrier east of ring 7, before the way east (`gate_checkpoint_barrier`,
gated on `world.switch_on gate_checkpoint_release`). Unmanifested hardware
is refused. The freight yard's door comes up in the pocket behind the
barrier, where its lever is; after that the barrier is a shortcut. The
labels that said `MAP_PRIVATE`, `MAP_WATCHED` and `[official report]`, the
wall gated on Bob's survey and the "submit private route data" switch are
gone.

### 9. The freight yard (new)

The goblins' camp, where the crate waits for a gate. The Salvage Lead
speaks at the yard's west gate (`salvage_lead_yard`); an encounter shuts
the yard (`intro_freight_yard_fight`): two goblins, then the Salvage Lead
(`npc_goblin_brute`'s body: a slow, armored hammer; strike before the
windup is committed, or after the swing) with one more goblin. The inner
gate opens when it is won, and the yard bell (linked to the fight, as the
whistle is to the raid) keeps it won until it is rung. Behind it is the crate (`creator_notebooks`):
the work order names `14 CULVERT ROW`, the notebooks' last page starts "THE
QUESTION I WANT IT TO ASK IS" and the rest is torn off
(`intro_creator_notes_recovered`, the quest's last step). The yard's door
comes up behind the checkpoint.

## Dialogue and cutscenes

- `assets/data/cutscenes/intro.ron`: `intro_wake`, `intro_raid`,
  `creator_final_fragment`, `drain_market_arrival`, `oiler_notices`,
  `salvage_lead_yard`, `creator_notebooks`, `first_ripple` (unwired).
- `assets/dialogue/sandbox/intro.yarn`: the creator, the raiders, Oiler,
  the news board, Alice, Bob, Eve's cameo, the gate janitor and the manifest
  clerk.
- Kept unresolved: the creator's sentence. Allowed clues: the raid had a
  paid order and a manifest; the address was wrong; the official story
  thanks both factions.

## Validation

- The world validates at boot; the content lib tests read the intro rooms
  (`intro::tests`).
- The walked route (`a_walked_route_of_the_persistent_world`) and the
  playthrough (`a_playthrough_of_the_persistent_world`) walk the rooms with
  the player's own inputs; a room that a body cannot cross fails them.
- Single-jump reach is checked in the generator: a required step up is at
  most 64 px and a required gap at most 112 px.
