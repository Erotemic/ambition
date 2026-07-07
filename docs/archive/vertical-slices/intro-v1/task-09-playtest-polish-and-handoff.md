# Task 09: Playtest polish and handoff

## Purpose

Turn the intro-v1 buildout into an iterable vertical slice through playtest scripts, room-by-room fun assessment, validation, and next-polish handoff.

## Files to read first

- `dev/vertical-slices/intro-v1/scaffold.md`
- `dev/vertical-slices/intro-v1/map-contract.md`
- `all completed task handoff notes in dev/vertical-slices/intro-v1/`

## Files likely to change

- `dev/vertical-slices/intro-v1/playtest-handoff.md`
- `dev/vertical-slices/intro-v1/map-contract.md if route graph changed`


## Required deliverable

Create or update:

```text
dev/vertical-slices/intro-v1/playtest-handoff.md
```

This file should become the single best handoff for the next iteration. Include:
current playable route graph, exact start and route instructions, known route
flags and triggers, room-by-room fun assessment, collision/platforming issues,
combat readability issues, real versus labelled branch hooks, suggested next
polish order, validation commands and results, and screenshots/map coordinates
if the repo workflow supports them.

## Playtest route scripts

Write at least three route scripts.

### Script A: main good/private route

```text
1. Start in intro_wake_room or configured intro entry.
2. Exit through raid corridor.
3. Climb vertical escape shaft.
4. Reach Drain Market and interact with Oiler.
5. Enter under-town pipes.
6. Get Alice's sealed route note.
7. Reach Bob using private or least-observed route.
8. Receive Bob's field survey / response.
9. Return to Alice and confirm private map marks or route unlock.
10. Use private map route into utility switchback.
11. Clear combat calibration lab.
12. Clear or stub first system boss.
13. Return to Drain Market through shortcut and verify changed map/route labels.
```

### Script B: neutral route

```text
1. Start normally.
2. Reach Drain Market.
3. Skip Alice/Bob.
4. Enter right utility switchback.
5. Take neutral lower route to combat lab.
6. Clear combat lab and boss by standard movement/combat.
7. Return to town.
```

### Script C: evil/lawful report route

```text
1. Start normally.
2. Reach Alice or obtain the sealed route note / Bob survey.
3. Report or submit private map information at the official/system hook.
4. Use the shortcut or benefit created by reporting.
5. Observe the cost or placeholder cost.
6. Continue to combat lab/boss.
```

If a route is not implemented, mark the blocked step and why.


### Script D: cartography state check

```text
1. Reach Drain Market before Alice/Bob and record visible map labels.
2. Complete Alice -> Bob -> Alice loop without reporting.
3. Confirm private map marks, Bob survey labels, or private route state changed.
4. Repeat or inspect report route if supported.
5. Confirm a private route becomes MAP_WATCHED, official, or otherwise compromised after reporting.
6. Confirm neutral route remains playable if Alice/Bob is skipped.
```

## Room-by-room polish checklist

Wake room: start/exit reliability, no accidental blocker, optional future-return
object visible.

Raid corridor: player understands forward motion, hazards are not confusing,
speed/Creator variant works or is labelled, shaft transition is reliable.

Vertical escape shaft: upward route readable, missed jumps recover quickly,
one-way platforms fair, hazards have safe read positions, side secret optional,
late-gated reward visible, top exit works.

Drain Market: safe decompression, Oiler visible but non-blocking, roofs/awnings
fun, high pickup tempting, under-town/right/forest/sky route promises visible,
no hostile main-street combat.

Under-town cartography route: entrance findable, route feels tighter/quieter than
town, Alice/Bob cartography route understandable, observed/private difference visible,
hazards light, return route not tedious.

Right utility switchback: official/private/neutral route differences legible,
central machine communicates route control, route to combat clear, private hook
understandable, scanner/hazard fair, sky tease not a broken required path.

Combat calibration lab: first enemy readable before damage, hazard has safe
observation point, arena is short, P4 usable or labelled, boss exit clear,
neutral path possible.

First system boss: at least one tell clear, vulnerability rule understandable,
arena supports movement, retry loop short, return shortcut opens or is stubbed,
reward/state visible.

## Tuning rules

When in doubt:

```text
- widen platforms before adding tutorials;
- shorten runbacks before reducing damage;
- remove enemies before adding more mechanics;
- label future gates instead of silently blocking;
- prefer one memorable secret over three mediocre secrets;
- preserve neutral route completion.
```

Do not make the game harder because you can clear it. The first slice should be
playtestable by someone who has not seen the map.

## Suggested next polish order

Unless playtesting reveals a blocker:

```text
1. Fix broken transitions and spawn positions.
2. Fix unreadable main-route jumps.
3. Fix long fall/runback frustration.
4. Fix combat tells and damage readability.
5. Make Drain Market traversal more fun.
6. Make Alice/Bob cartography/courier route clearer.
7. Make official/private/neutral route differences visible.
8. Add or tune first boss vulnerability rule.
9. Add small visible memory consequences.
10. Replace weak names and story labels only after gameplay route works.
```

## Acceptance criteria

`playtest-handoff.md` exists, contains at least three route scripts, identifies
real versus placeholder route hooks, includes room-by-room fun/readability notes,
lists validation results, updates `map-contract.md` if the route graph changed,
and recommends the next polish order.



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
