# Intro v1 vertical slice scaffold

Status: planning and implementation scaffold for long-running autonomous agents.

This directory breaks the Act 1 intro rebuild into manageable overnight tasks.
Each task is intended to be handed to an agent independently. Agents should read
this scaffold first, then read exactly the task file they were assigned, plus the
repo files named in that task.

This v2 scaffold incorporates the cartographer revision: Alice and Bob are not
only trust/cryptography placeholders. They are unofficial cartographers whose
sealed messages and field surveys unlock map knowledge, private route marks, and
route memory. This changes the reward spine more than the room grid.

The work here is intentionally map-first. Names such as Gate Stack, Manifest
Clerk, Ledger, Audit Engine, and similar concepts are placeholders unless a task
explicitly says otherwise. The durable goal is to build a playable first
gridvania knot with strong traversal, readable branch promises, tunable combat,
map-layer progression, and persistent route hooks. Lore concepts can evolve
while the room functions remain useful.

## Existing repo anchors

The current intro world lives at:

```text
crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk
```

The current intro levels are:

```text
intro_wake_room       1024 x 384   world (0, 0)
intro_raid_corridor   1600 x 512   world (1024, 0)
intro_escape_shaft    1280 x 512   world (2624, 0)
drain_alley           1024 x 512   world (3904, 0)
gate_stack_lower      1600 x 768   world (4928, 0)
pirate_sky_arena      2400 x 1024  world (108000, -1024)
```

Existing LDtk area specs are under:

```text
tools/ambition_ldtk_tools/specs/
```

The intro-related specs are:

```text
tools/ambition_ldtk_tools/specs/intro_wake_room_area.yaml
tools/ambition_ldtk_tools/specs/intro_raid_corridor_area.yaml
tools/ambition_ldtk_tools/specs/intro_escape_shaft_area.yaml
tools/ambition_ldtk_tools/specs/drain_alley_area.yaml
tools/ambition_ldtk_tools/specs/gate_stack_lower_area.yaml
```

The LDtk tooling entry point is:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools <subcommand> ...
```

Important repo rule: do not hand-edit LDtk JSON when placing rooms, entities,
gates, walls, hitboxes, or authored level geometry. Prefer existing area specs
and LDtk tooling. If tooling cannot perform a needed edit, either extend the
tooling in a focused way or leave clear implementation notes rather than making
unstructured JSON edits.

Focused files agents should consider:

```text
AGENTS.md
dev/README.md
docs/concepts/llm-spatial-authoring-discipline.md
docs/planning/story-gameplay-progression-draft.md
docs/planning/gameplay-idea-index.md
docs/mechanics/body-modes.md
docs/mechanics/blink.md
docs/recipes/ldtk-authoring.md
docs/tools/ldtk-tools.md
tools/ambition_ldtk_tools/README.md
```

Do not read the entire docs tree. Read focused files for the assigned task.

## Design north star

The Act 1 intro should become a playable, inspectable platforming and combat
slice. Story is present, but it is not the unit of implementation. The unit of
implementation is room function.

The intro should move through these experiences:

```text
wake and move
-> flee a wrong-list raid
-> climb upward through broken infrastructure
-> emerge into a small layered drain market
-> discover a private under-town cartography route
-> learn that map knowledge can be protected, traded, reported, or discovered
-> see at least two future Act 1 promises
-> enter a mechanical utility/combat branch
-> clear a first system encounter
-> return to town through a shortcut with new route options
```

The first slice should not over-invest in any one named faction. The player
should feel that authority, records, official maps, private maps, repair culture,
unstable routes, and hidden cartographer networks exist. The exact names can
change later.

Design rule:

```text
Names are cheap. Traversal grammar is expensive. Build traversal grammar first.
```

Cartography rule:

```text
The map is not neutral.
Official maps classify.
Alice/Bob maps protect.
Chaotic play discovers unmapped space.
Evil/lawful play reports private routes and turns them into watched routes.
```

## Alice and Bob: unofficial cartographers

Alice and Bob should now be treated as map-progression characters, not only as
cryptography or trust abstractions.

Alice is the careful local cartographer. She maps private routes, repair culture,
pipe passages, dangerous shortcuts, and places authority does not recognize. She
does not ask the player to declare an identity. She asks them to prove they can
carry route knowledge without leaking it.

Bob is the field cartographer. He is usually found in inconvenient places:
stranded in a pipe, trapped behind a one-way route, sketching a hazard room,
hiding inside a crypt, dangling near a sky chain, or pretending not to be lost.
He can be warmer and more comic than Alice. One later gag can be:

```text
"So you found me in a crypt. Maybe that makes me a crypt-tographer."
```

Do not force that specific joke into Act 1 unless it naturally fits. The
important motif is reusable:

```text
Find Alice.
Alice gives a sealed route note or survey request.
Find Bob in a dangerous or inconvenient mapping situation.
Bob gives a field survey, response, or map correction.
Return or complete the loop.
Unlock a map layer, route annotation, or private shortcut.
```

This makes Alice/Bob immediately useful to gameplay. The reward is not only
friendship or lore. The reward is a better, more dangerous, more political map.

## Future motif: the Silent Cartographer

Reserve a Silent Cartographer nod for later. Do not spend Act 1 implementation
time on it. It can become one of these later-act payoffs:

```text
- a legendary cartographer who never speaks;
- a silent map-machine that updates only through room geometry;
- a dead or missing cartographer whose map still changes;
- a dialogue-free route where only marks, symbols, and map updates communicate;
- a major discovery room where the player realizes the map itself is a character.
```

For intro-v1, only add this as a scaffold note. Do not add a room, quest, or
asset dependency for it.

## Macro gridvania shape

Target structure:

```text
                         [Sky Chain / Pirate Tease]
                                  |
                         [Upper Drain Market Roofs]
                                  |
[Forest Edge Tease] -- [Drain Market Main] -- [Right Utility Switchback]
          |                    |                     |
          |              [Under-Town Pipes] -- [Alice Map Relay]
          |                    |                     |
          |              [Old Mountain Drain] -- [Bob Field Survey]
          |                    |                     |
          |              [Blocked Military Hatch] [Combat Lab]
          |                                          |
          |                                  [First System Boss]
          |
     [Later Forest / Ninja Route]

Linear intro feed:

[Wake Room] -> [Raid Corridor] -> [Vertical Escape Shaft] -> [Drain Market Main]
```

The current room identifiers can remain if renaming is expensive, but their
functions should shift:

```text
intro_wake_room      = embodiment room
intro_raid_corridor  = pressure corridor
intro_escape_shaft   = vertical movement ascent
drain_alley          = Drain Market main knot
gate_stack_lower     = right utility switchback / official infrastructure branch
pirate_sky_arena     = future sky/pirate promise, not required for intro-v1
```

New rooms or reinterpreted areas may be added as needed:

```text
under_town_pipes
alice_relay
bob_relay
combat_calibration_lab
first_system_boss
```

## Main route for this slice

The main route for this scaffold is the good/private cartography route. It should
be fully playable even if branch consequences are only debug labels and save
flags at first.

```text
1. Wake in lab.
2. Flee the raid corridor.
3. Climb the vertical escape shaft.
4. Reach Drain Market and meet Oiler.
5. Receive or annotate the Stabilizer / Compact Calibration.
6. Enter under-town pipes.
7. Find Alice or an Alice map mark.
8. Carry Alice's sealed route note to Bob.
9. Reach Bob using a private or least-observed route.
10. Receive Bob's field survey or response.
11. Return with the survey without reporting or tampering.
12. Unlock private map marks and a private route through the right utility switchback.
13. Enter the combat calibration lab.
14. Gain or annotate a combat traversal verb.
15. Clear a first system boss or boss stub.
16. Return to Drain Market through a shortcut.
17. See new next-route options: forest, sky, military/service, deeper cartography.
```

This route is not the only valid route. It is the route used to make sure the map
has a coherent spine.

## Branch axes

Do not model branch identity as one morality meter. Treat branch identity as
route tags that can combine.

### Good / private

Protect private information, carry sealed map notes, avoid observation, disable
harmful systems without feeding them more route records, preserve hidden paths.

Gameplay feel:

```text
slower
spatial
secretive
map-mark heavy
trust-door heavy
less immediately rewarding
warmer long-term NPC reactions
```

Map consequence:

```text
private map marks deepen
unofficial paths become more readable
Alice/Bob reveal dangerous shortcuts
official map remains incomplete or misleading
```

### Evil / lawful

Report private routes, classify uncertain entities, submit Alice/Bob map data,
use official doors for convenience, make rooms cleaner and safer-looking but
colder, trade trust for access.

Gameplay feel:

```text
faster access
more official map data
official shortcuts
stronger combat permission
more surveillance
NPC fear or silence
```

Map consequence:

```text
private routes become watched
official annotations appear earlier
some hidden marks disappear or become suspect
Alice/Bob greetings change
```

This route should be tempting, not cartoonishly evil. Early choices should feel
like small compromises: "submit this route for safety", "register this hidden
passage", "sanitize unknown map data", "open shortcut now".

### Neutral

Progress through movement and combat while avoiding strong commitments. Neutral
players should not be blocked from finishing the slice.

Gameplay feel:

```text
standard platformer route
basic map fills in through exploration
fewer special doors
less trust
less official power
fewest story-specific consequences
```

### Lawful / chaotic

Lawful means using switches, signs, terminals, intended doors, official maps,
and documented mechanisms. Chaotic means breaking walls, finding pipes, using
hard jumps, sequence-breaking, and taking risky unmapped shortcuts. Both can be
good, evil, or neutral.

### Famous / private

Famous means being observed: alarms, public boss defeats, helping crowds, using
main streets. Private means avoiding scanners, using pipes, carrying sealed map
notes, and leaving fewer route records. Both can be good, evil, or neutral.

## Progression and powerup scaffold

Progression should be implemented as actual mechanics where they already exist
and as explicit annotations/flags where enforcement would require too much new
backend work. Test mode may still expose all abilities; that is acceptable as
long as intended story-mode gates are marked.

Suggested progression:

```text
P0. Body Online
    Source: intro_wake_room
    Function: movement, jump, interact

P1. Oiler Stabilizer / Compact Calibration
    Source: Drain Market
    Function: unlock or justify under-town pipes, crawl/compact routes,
              repair nodes, or reliable movement

P2. Alice's Sealed Route Note
    Source: Alice Map Relay
    Function: map payload that must be carried to Bob without leaking it

P3. Bob's Field Survey / Private Map Marks
    Source: Bob -> Alice return
    Function: unlock private map layer, route annotations, shimmer door,
              hidden shortcut, or unofficial path markers

P4. Combat Calibration
    Source: combat_calibration_lab
    Function: formalize pogo, parry, projectile, or another combat verb

P5. Route Memory / System Core
    Source: first_system_boss
    Function: open return shortcut, mark changed rooms, and enable next Act 1 branches
```

If only one new playable ability can be formalized in this slice, prefer a verb
that connects combat and platforming. Pogo/downslash is ideal if stable; parry
or projectile is acceptable if that is better supported by the current code.

## Map layer scaffold

Even if the UI cannot render all layers yet, agents should label and design
rooms as though these layers exist.

```text
map_basic_unlocked
  shows visited rooms, obvious exits, repair/save nodes, and main route names.

map_private_marks_unlocked
  shows Alice/Bob symbols, pipe entrances, hidden doors, quiet routes, and
  routes that should not be reported.

bob_field_survey_received
  adds danger marks, hazard notes, boss/arena warning, high-value pickups, and
  shortcut candidates.

route_memory_received
  marks changed, watched, cleared, blocked, or compromised routes after major
  choices and boss/system outcomes.
```

Suggested debug/map labels:

```text
MAP_PRIVATE
MAP_OFFICIAL
MAP_DANGER
MAP_SECRET
MAP_UNKNOWN
MAP_CHANGED
MAP_WATCHED
MAP_SEQUENCE_BREAK
MAP_REPORTABLE
MAP_UNMAPPED
```

Use these labels as implementation handles, not final UI copy.

## Encounter and puzzle ladder

Use one readable element at a time. Avoid enemy soup.

Recommended early ladder:

```text
1. Empty movement and interact.
2. Pressure movement with one avoidable enemy or hazard.
3. Vertical climb with one-way platforms and recoverable falls.
4. Stationary timed hazard.
5. Simple ground patrol.
6. Scanner or observed route.
7. Slow projectile / pipe spitter.
8. Melee windup enemy.
9. Two-enemy combination in a safe combat lab.
10. First boss with two or three readable patterns.
```

Puzzle ladder:

```text
1. See an unreachable pickup.
2. Learn one-way platforms.
3. Notice a map mark before understanding it.
4. Choose high/private route versus low/observed route.
5. Carry a sealed route note across a movement challenge.
6. Receive Bob's survey and reveal or annotate a map layer.
7. Use a route flag to open a private shortcut.
8. Use a combat/traversal verb to expose a boss vulnerability or return route.
```

## Implementation order

The suggested implementation order is:

1. `task-01-map-contract-and-tooling-audit.md`
2. `task-02-vertical-escape-shaft.md`
3. `task-03-drain-market-knot.md`
4. `task-04-under-town-trust-route.md` - now the under-town cartography route
5. `task-05-right-utility-switchback.md`
6. `task-06-combat-calibration-lab.md`
7. `task-07-first-system-boss-and-return-loop.md`
8. `task-08-route-state-and-dialogue-hooks.md` - now includes map layers/state
9. `task-09-playtest-polish-and-handoff.md`

The tasks are ordered so each one can be useful even if the next task never
happens. The first three tasks create the playable spatial spine. Task four adds
Alice/Bob cartography and private map progression. Tasks five through seven add
route choice, combat, and capstone progression. Tasks eight and nine turn the
slice from a collection of rooms into an iterable vertical slice with memory,
map-layer hooks, validation, and polish handles.

## Validation baseline

For LDtk edits, run from the repository root:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor \
  crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk
```

If the area spec changed, use the relevant `area create --dry-run` command first
when the current tool supports it, then apply the edit through the LDtk tooling
and inspect the diff. If the current tooling only documents `sandbox.ldtk`, adapt
the command to `intro.ldtk` for this slice and document any mismatch.

When code or dialogue changes are made, also run the narrowest relevant checks:

```bash
cargo fmt --check
cargo test -p ambition_gameplay_core --lib
cargo run -p ambition_gameplay_core --bin headless
```

If a command fails for a known pre-existing reason, record the exact command and
the short error summary in the task handoff instead of hiding the failure.

## Required handoff note

End the task with a short handoff note in the changed doc, commit message, or a
new note beside the task. Include:

```text
what changed
what was validated
what remains placeholder
what felt fun or unreadable
which room/route should be worked on next
```
