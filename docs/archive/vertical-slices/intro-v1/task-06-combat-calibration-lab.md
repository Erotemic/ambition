# Task 06: Combat calibration lab

## Purpose

Build the first deliberate combat training room, formalizing or annotating the first combat-traversal powerup and preparing the player for the boss.

## Files to read first

- `dev/vertical-slices/intro-v1/scaffold.md`
- `dev/vertical-slices/intro-v1/task-05-right-utility-switchback.md`
- `docs/planning/gameplay-idea-index.md`

## Files likely to change

- `crates/ambition_actors/assets/ambition/worlds/intro.ldtk`
- `tools/ambition_ldtk_tools/specs/combat_calibration_lab_area.yaml`



## Cartography hook

If Task 04 or Task 08 has added Bob's Field Survey / private map marks, this room
should give that reward immediate gameplay value. Add at least one annotation or
label that a survey could reveal:

```text
MAP_DANGER: arena hazard cycle
MAP_SECRET: side ledge / bypass / recovery route
MAP_OFFICIAL: calibration route
```

Best case: private map marks reveal a safer side entrance, a pre-fight warning,
or a way to disable one part of the test. Minimal case: place always-visible
DebugLabels that name the intended map annotations.

## Design target

Target graph:

```text
Right Utility Switchback -> Combat Calibration Lab -> First System Boss
```

This room supports three styles:

```text
Good/private: disable or bypass part of the test through trust/private route.
Neutral: clear the encounter normally.
Evil/lawful: classify/clear all targets for an official reward/hook.
```

## Layout sketch

```text
+------------------------------------------------+
| entry ledge from utility switchback            |
|                                                |
| tutorial pocket: one simple patrol             |
|                                                |
| safe platform / small pickup / recovery node   |
|                                                |
| arena area:                                    |
|   wave 1: contact patrol                       |
|   wave 2: stationary spitter or timed hazard   |
|   wave 3: striker + hazard, optional           |
|                                                |
| side console: disable / classify / bypass hook |
|                                                |
| exit to first system boss                      |
+------------------------------------------------+
```

## Required beats

### Single enemy pocket

Before any arena lock-in, place one simple enemy in a small pocket. Use a ground
patrol/contact enemy, slow striker, or training construct. Avoid flyers and
complex shields.

### Hazard read

Add one stationary timed hazard or slow projectile source. The player should be
able to stand safely and watch the timing before committing.

### Small arena

Add a short arena or encounter zone. First pass tests readability, not endurance.
Suggested ladder: one contact patrol, then one stationary spitter/projectile,
then optionally one striker plus one simple hazard. If encounter lock-ins are
expensive, build the room spatially and leave lock-in wiring for later.

### Route-variant console

Place a console, switch, or side door that represents route variants:

```text
Good/private: disable test, spare construct, erase observation logs.
Evil/lawful: classify targets, full-clear reward, official record.
Neutral: ignore console and clear the room.
Chaotic: break or bypass console through skill route.
```

This can be a labelled placeholder until Task 08.

### P4 Combat Calibration

Choose or annotate one combat-traversal verb. Preferred if stable:

```text
P4 = downward slash / pogo calibration
```

Alternatives are parry/shield, projectile/tool, or charged slash. The chosen P4
should be useful in the boss, unlock a high pickup in Drain Market or the escape
shaft, create a future route promise, or be testable immediately in the same
room. If all abilities are already available in test mode, make P4 a story flag
or DebugLabel and place test affordances.

### Exit to boss

The exit should be visible after the combat room is cleared or after the neutral
path. It should feel like the next room is a capstone.

## Tuning rules

Use one new behavior per section. Prefer wide platforms, generous recovery
space, a health pickup after mistake-prone sections, short runback, and no pits
that make combat mistakes repeat long traversal.

## Branch hooks

Label good/private side entrance or disable console, evil/lawful classification
console, neutral clear, chaotic bypass, famous alarmed clear, and private log
wipe.

## Acceptance criteria

Combat Calibration Lab exists, is reachable from right utility switchback,
contains a simple enemy pocket, contains a hazard or projectile read, contains a
short arena or placeholder, introduces/annotates P4, exits toward the boss, and
labels route hooks.



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
