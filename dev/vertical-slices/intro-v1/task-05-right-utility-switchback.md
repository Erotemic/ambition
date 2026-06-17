# Task 05: Right utility switchback

## Purpose

Reinterpret `gate_stack_lower` as a route-choice room connecting Drain Market to mechanical infrastructure, official/private access models, and the combat branch.

## Files to read first

- `dev/vertical-slices/intro-v1/scaffold.md`
- `dev/vertical-slices/intro-v1/map-contract.md`
- `dev/vertical-slices/intro-v1/task-03-drain-market-knot.md`
- `dev/vertical-slices/intro-v1/task-04-under-town-trust-route.md`
- `tools/ambition_ldtk_tools/specs/gate_stack_lower_area.yaml`

## Files likely to change

- `tools/ambition_ldtk_tools/specs/gate_stack_lower_area.yaml`
- `crates/ambition_gameplay_core/assets/ambition/worlds/intro.ldtk`


## Design target

This room does not need to preserve Gate Stack as a final concept. Its durable
job is to show multiple access models in one space:

```text
official/easy but watched
private/trust-gated
neutral/combat route
chaotic/skill shortcut
sky/pirate tease above
```

Target graph:

```text
Drain Market -> Right Utility Switchback
Right Utility Switchback -> Combat Calibration Lab
Right Utility Switchback -> private side entrance from Alice/Bob route
Right Utility Switchback -> official shortcut / evil-lawful hook
Right Utility Switchback -> sky/pirate tease
Right Utility Switchback -> return to Drain Market
```

If renaming the level identifier is risky, keep `gate_stack_lower` and update
comments/debug labels to describe the utility-switchback function.


## Map-route contrast

This room should visibly contrast official map logic with Alice/Bob private map
logic. The official route should look cleaner, better signed, and easier to
understand. The private route should look like a quiet crack through the system:
pipe mark, shimmer mark, service gap, or hand-drawn route symbol.

Use labels if art/state is not ready:

```text
MAP_OFFICIAL: approved route / shortcut pending
MAP_PRIVATE: Alice/Bob route mark opens after Bob field survey
MAP_WATCHED: route becomes watched if private survey is reported
```

Do not make the official path obviously villainous. It should be convenient and
tempting. The cost is route exposure, not immediate cartoon evil.

## Layout sketch

```text
+------------------------------------------------------------+
| upper-left: sky chain / pirate tease / hard climb          |
| ledges, one-way platforms, visible reward                  |
|                                                            |
| upper-right: official watched door / future route          |
|                                                            |
| middle: central machine, switch, unstable ripple/crack     |
|                                                            |
| lower-left: entry from Drain Market                        |
| lower-middle: private entrance from Alice/Bob route        |
| lower-right: exit to Combat Calibration Lab                |
+------------------------------------------------------------+
```

The room should have a readable route triangle:

```text
official door = easiest-looking, watched, future evil/lawful convenience
private crack = opens with Handshake Proof / Ripple Attunement
neutral lower route = accessible path into combat lab
chaotic high route = hard climb or breakable shortcut candidate
```

## Required beats

### Arrival from Drain Market

Preserve or replace the loading zone from Drain Market. Entry should be safe and
should immediately show the central machine and at least two route possibilities.

### Central machine / switch

Place a central object, gate ring, switch, engine, or large prop that says "this
room controls routes." Existing gate machinery can remain if useful, but it
should be treated as placeholder fiction.

The switch should open a neutral route, toggle a platform, power a door, or act
as a labelled official-route hook. Do not add complex code if existing
switch/lock behavior is enough.

### Private route entrance

Connect or stub the route unlocked by Alice/Bob. This can be a LoadingZone from
Alice/Bob, a lock wall labelled as requiring Handshake Proof, or a visible
private crack/ripple. It should bypass some watched or official obstacle.

### Official/watched route

Place an obvious official route: a clean door, scanner, terminal, permit door,
or labelled shortcut. It should be tempting but not required. Do not force an
evil action in this task.

### Neutral route to combat lab

There must be a route to the combat calibration lab that does not require Alice
or reporting. It can be longer or more hazardous, but it should be fully
playable.

### Sky/pirate tease

Use upper platforms or a hard climb to show the sky/pirate promise. Do not make
it required. If `pirate_sky_arena` is not ready, use labels and props rather
than a broken LoadingZone.

## Combat and hazards

This room introduces observation and light danger, not full combat. Use one slow
scanner/search drone, timed steam/electric hazard, or simple patrol. No lock-in.

## Branch hooks

Label or implement: good/private via private map route, evil/lawful via official
terminal, neutral via lower combat route, chaotic via hard climb/breakable
shortcut, famous via scanner/alarm, private via crack/ripple path.

## Acceptance criteria

The room functions as a route-choice switchback; Drain Market entry works; route
to Combat Calibration exists or is stubbed; private map route exists or is stubbed;
official/watched hook exists; sky/pirate tease exists; hazards are light and
readable; validation is documented.



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
