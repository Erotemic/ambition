# Task 02: Vertical escape shaft

## Purpose

Rebuild `intro_escape_shaft` from a short horizontal corridor into the first real movement room: a vertical ascent from the wrong-list raid up into drainage, mountain air, and the edge of Drain Market.

## Files to read first

- `dev/vertical-slices/intro-v1/scaffold.md`
- `dev/vertical-slices/intro-v1/map-contract.md`
- `tools/ambition_ldtk_tools/specs/intro_escape_shaft_area.yaml`
- `tools/ambition_ldtk_tools/README.md`
- `docs/concepts/llm-spatial-authoring-discipline.md`

## Files likely to change

- `tools/ambition_ldtk_tools/specs/intro_escape_shaft_area.yaml`
- `crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk`


## Design target

The room should feel like climbing out of a hidden mountain lab. The player has
just fled the raid and lost the explanation. This should not be a dialogue room;
the environment carries the beat.

Target function:

```text
raid exit at lower/bottom edge
-> broken service shaft climb
-> optional side alcove with military/lab hint
-> visible late-gated pickup or route
-> upper drain grate exit to Drain Market
```

Suggested room size is `768 x 1408` or `1024 x 1536`. Pick the size that supports
bottom entry, safe first climb, middle hazard section, side secret alcove,
visible unreachable pickup, top exit, and fall recovery ledges.

## Layout sketch

```text
+------------------------------------------------+
| TOP EXIT: drain grate / Drain Market arrival   |
| small landing, water drip, town noise label     |
|                                                |
| high ledge with visible late pickup             |
| requires future pogo/wall/dash                  |
|                                                |
| one-way platforms around central void           |
| steam/electric hazard timing                    |
|                                                |
| side alcove: sealed hatch / military mark       |
| optional pickup or debug lore label             |
|                                                |
| safe recovery ledges under missed jumps         |
|                                                |
| BOTTOM ENTRY from raid corridor                 |
+------------------------------------------------+
```

## Required beats

### Bottom panic landing

Preserve or replace the loading zone from the raid corridor. Give the player a
safe floor and a clear upward path. The first two jumps should be easy and safe.
Add a cold-launch `PlayerStart` near the lower-left or lower-center.

### One-way platform lesson

Place three to five one-way platforms in a staggered climb. Missed jumps should
cost time, not force a full restart. Avoid hazards under the first few missed
jumps.

### Timed hazard section

Add one or two simple hazards after the player understands the climb. Prefer
steam, electric arcs, or labelled damage volumes. Provide a safe read position
before each hazard. Do not build a new hazard system in this task.

### Side alcove secret

Add a side alcove that is optional and discoverable. It can contain a pickup,
chest, prop, or DebugLabel. It should hint at the old lab, sealed service
infrastructure, or military/service connection without dumping lore.

### Visible late-gated pickup

Place one pickup or visible ledge that is not intended to be reachable with the
main story move set. It can be reachable in test mode. Label it as a later
return reward for pogo, wall tech, dash, blink, or another future power.

### Top exit

The exit should feel like emerging into public infrastructure. A top-edge exit
is ideal if supported. If the tooling prefers side exits, put the exit high on a
side wall while preserving the vertical climb fantasy.

## Combat and branch hooks

Use no combat or one optional enemy. This is a movement confidence room.

Add labels for these future hooks:

```text
Good/private: hidden memory or side alcove.
Evil/lawful: sealed hatch or recordable military/authority route.
Chaotic: cracked wall or hard jump sequence-break candidate.
Neutral: straightforward climb.
Famous/private: optional observed route only if cheap.
```

## Acceptance criteria

The room is meaningfully vertical, climbs from lower entry to upper exit, has at
least one one-way platform sequence, recoverable falls, one side alcove, one
visible later-gated reward, and reliable transition to Drain Market.



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
