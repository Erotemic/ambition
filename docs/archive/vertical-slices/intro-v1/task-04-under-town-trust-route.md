# Task 04: Under-town cartography route

## Purpose

Build the first good/private route: under-town pipes leading to Alice and Bob's
unofficial cartography loop. The player carries Alice's sealed route note to Bob,
finds Bob in an inconvenient field-survey situation, returns with Bob's survey or
response, and unlocks private map marks / a private route.

This task replaces the older framing of Alice/Bob as only a trust or crypto
handshake. Trust and verification still matter, but the gameplay reward is now
cartographic: map layers, hidden route annotations, dangerous shortcut notes, and
route memory hooks.

## Files to read first

- `dev/vertical-slices/intro-v1/scaffold.md`
- `dev/vertical-slices/intro-v1/map-contract.md`
- `dev/vertical-slices/intro-v1/task-03-drain-market-knot.md`
- `docs/planning/gameplay-idea-index.md`

## Files likely to change

- `crates/ambition_actors/assets/ambition/worlds/intro.ldtk`
- `tools/ambition_ldtk_tools/specs/under_town_pipes_area.yaml`
- `tools/ambition_ldtk_tools/specs/alice_relay_area.yaml`
- `tools/ambition_ldtk_tools/specs/bob_relay_area.yaml`

## Design target

Target graph:

```text
Drain Market Main
  -> under_town_pipes
    -> alice_relay / Alice Map Alcove
      -> bob_relay / Bob Field Survey traverse
        -> return shortcut to alice_relay or under_town_pipes
          -> private map marks unlock
          -> private route unlock toward right utility switchback
```

If three rooms is too much for one session, create a two-room version:

```text
under_town_pipes
alice_bob_cartography_relay_combined
```

The route must be playable in test mode even if final story gates are labels.
Use debug/map labels if map UI, quest flags, or lock states are not ready.

## Core fiction for implementation

Alice and Bob are unofficial cartographers. They do not merely keep secrets; they
make maps that protect people.

Official maps show permitted routes, watched doors, authorized passages, and
sanitized hazards. Alice/Bob maps show how people actually survive: pipes,
roofs, quiet shortcuts, unsafe rooms, one-way passages, hidden doors, and routes
that become dangerous if reported.

Use this contrast:

```text
Official map: where you are allowed to go.
Alice/Bob map: where people actually go.
```

Alice is cautious and local. Bob is the field cartographer who keeps getting
found in strange places. In later content Bob can make jokes like "crypt-
tographer" if found in a crypt. For this task, add only a placeholder if it fits
naturally; do not force a crypt into Act 1 unless the room already wants that
shape.

## Room 1: under_town_pipes

Function: first hidden/private route below Drain Market. It should feel tighter,
wetter, lower, quieter, and more route-readable than the market.

Layout sketch:

```text
+------------------------------------------------+
| entry from Drain Market grate                  |
| crawl/compact tunnel -> valve pocket -> duct   |
|                         |                      |
|                  suspicious dead end           |
|                         |                      |
|         Alice map mark / private door ->       |
| lower water/steam hazard with skitter patrol   |
| blocked old mountain/military hatch below      |
+------------------------------------------------+
```

Required elements:

```text
- loading zone from Drain Market;
- PlayerStart for cold testing;
- CameraZone if the project uses it;
- solid boundaries and recoverable geometry;
- one simple patrol or pipe hazard;
- one suspicious dead end;
- one old mountain/military-service hint;
- one visible Alice/Bob map mark;
- exit to Alice Relay.
```

The map mark should look like a route clue, not just graffiti. Use a placeholder
label if art is not available:

```text
MAP_PRIVATE: Alice mark - pipe route, do not report
```

If switches/lock walls are easy, add a valve route choice. Otherwise use layout
and labels. The route should teach that unofficial maps are spatial, not merely
textual.

## Room 2: alice_relay / Alice Map Alcove

Function: Alice gives the sealed route note and frames map knowledge as something
that can be protected or exposed.

Required elements:

```text
- Alice NPC or placeholder terminal.
- DebugLabel naming P2: Alice's Sealed Route Note.
- DebugLabel naming map reward: Private Map Marks.
- Door/lock/label for private route that opens after Bob proof/survey.
- Exit toward Bob traverse.
```

If feasible, Alice interaction sets `alice_route_note_carried`. If flags are not
easy, place a pickup or label named `alice_sealed_route_note`.

Use short placeholder lines such as:

```text
Alice: "Official maps are for people who enjoy being found."
Alice: "Carry this to Bob. Do not unfold it where the walls can read."
Alice: "If someone asks you to register the route, they are asking you to kill it."
```

Do not polish dialogue heavily in this task. The lines only need to support room
function.

## Room 3: bob_relay / Bob Field Survey traverse

Function: courier movement challenge. The player carries Alice's route note to
Bob and returns with Bob's field survey or response. The route introduces
observed versus private traversal and makes map knowledge feel earned.

Layout sketch:

```text
+------------------------------------------------+
| entry from Alice                               |
|                                                |
| low route: easier, watched by scanner/label    |
|                                                |
| high route: harder jumps, private              |
|                                                |
|        Bob alcove: field survey in progress    |
|                    return shortcut opens       |
+------------------------------------------------+
```

Required elements:

```text
- two visible routes if possible;
- low/easy route marked observed or MAP_WATCHED;
- high/harder route marked private or MAP_PRIVATE;
- Bob NPC or terminal;
- Bob Field Survey label or pickup;
- return shortcut back to Alice or pipes;
- one annotation showing a newly revealed map mark.
```

Bob should be physically in a mapping situation, not standing in a generic room.
Examples:

```text
- stuck behind a collapsed pipe while drawing an exit arrow;
- perched above a hazard room with half-finished danger marks;
- hiding in a survey alcove after an official route became watched;
- stranded near a one-way gate and pretending this was intentional.
```

Possible placeholder lines:

```text
Bob: "Alice sent you? Good. She maps exits. I map mistakes."
Bob: "Give her this survey. Tell her the low route is watched now."
Bob: "If you used the lower path, congratulations: you are on somebody else's map too."
```

Optional later gag, only if appropriate:

```text
Bob: "If this were a crypt, you could call me a crypt-tographer. Sadly, this is drainage."
```

## Map unlock reward

The desired reward is not just `private_route_opened`. It is a map-layer moment.
Implement whichever version is feasible:

### Best case

Alice/Bob loop unlocks a real or semi-real map layer:

```text
map_private_marks_unlocked
```

Effects:

```text
- one hidden pipe entrance becomes marked;
- one private door/route becomes available;
- one hazard/shortcut annotation appears;
- one future route is labelled MAP_PRIVATE or MAP_SECRET.
```

### Practical first pass

If map UI is not ready, use debug labels and route state:

```text
alice_route_note_delivered
bob_field_survey_received
private_map_marks_unlocked
private_route_opened
```

Add visible labels near the opened route:

```text
MAP_PRIVATE: Bob survey reveals this quiet route.
MAP_DANGER: low passage watched by scanner.
MAP_SECRET: return shortcut to Alice.
```

### Minimal pass

If no state wiring is feasible, build the rooms and annotate the intended flags
so Task 08 can wire them later.

## Flags and route tags

If feasible, wire or stub:

```text
alice_route_note_carried
alice_route_note_delivered
alice_route_note_reported
alice_route_note_tampered optional
bob_field_survey_received
bob_survey_observed
bob_survey_private
map_private_marks_unlocked
private_route_opened
private_routes_compromised optional
```

If these flags cannot be implemented yet, label the route outcomes. Task 08 can
wire state later.

## Branch hooks

Good/private means carrying Alice's route note to Bob and returning with Bob's
survey without reporting it. Good/famous means completing the route but using an
observed path or public route. Evil/lawful means reporting Alice/Bob map data to
an official/system hook in a later room. Chaotic means opening/tampering with the
sealed route note, reaching Bob through an unmapped break, or discovering the
private route before the map marks unlock. Neutral means ignoring the
cartography loop and continuing through the standard utility route.

The first evil/lawful hook should be tempting, not monstrous:

```text
Submit private route for safety review.
Register undocumented passage.
Upload field survey to official map.
```

Reward it with convenience later. Cost it with watched/private-route changes
later.

## Combat and hazards

Keep combat light: one simple skitter/contact patrol, one stationary pipe
spitter or steam hazard, or one scanner/observation beam if already supported.
No hard combat lock-in.

The main challenge is route reading: safe/easy/observed versus harder/private.
The player should understand the choice spatially before dialogue explains it.

## Acceptance criteria

A playable under-town cartography route exists or is clearly stubbed; Alice and
Bob relay points exist; the route contains at least one movement choice; hazards
are light; Alice's Sealed Route Note and Bob's Field Survey are implemented or
annotated; private map marks and private-route unlock are implemented or
annotated; and branch hooks are labelled.

## Validation baseline

For LDtk edits, run from the repository root:

```bash
PYTHONPATH=tools/ambition_ldtk_tools python -m ambition_ldtk_tools doctor \
  crates/ambition_actors/assets/ambition/worlds/intro.ldtk
```

If the area spec changed, use the relevant `area create --dry-run` command first
when the current tool supports it, then apply the edit through the LDtk tooling
and inspect the diff. If the current tooling only documents `sandbox.ldtk`, adapt
the command to `intro.ldtk` for this slice and document any mismatch.

When code or dialogue changes are made, also run the narrowest relevant checks:

```bash
cargo fmt --check
cargo test -p ambition_actors --lib
cargo run -p ambition_actors --bin headless
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
