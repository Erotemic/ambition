# Task 07: First system boss and return loop

## Purpose

Build a simple first system boss or boss-stub and a return shortcut back to Drain Market so the intro becomes a gridvania knot rather than a one-way corridor.

## Files to read first

- `dev/vertical-slices/intro-v1/scaffold.md`
- `dev/vertical-slices/intro-v1/task-06-combat-calibration-lab.md`
- `docs/planning/gameplay-idea-index.md`

## Files likely to change

- `crates/ambition_actors/assets/ambition/worlds/intro.ldtk`
- `tools/ambition_ldtk_tools/specs/first_system_boss_area.yaml`



## Map consequence hook

The boss reward should make the player feel that the map can remember changes.
If possible, `route_memory_received` should mark at least one route as cleared,
changed, watched, private, official, or newly reachable. If map UI is not ready,
use route-state flags and DebugLabels.

Suggested labels after boss clear:

```text
MAP_CHANGED: system route cleared
MAP_SECRET: return shortcut opened
MAP_WATCHED: official route recorded if cleared through report path
MAP_PRIVATE: quiet disable preserved hidden route
```

## Design target

Target graph:

```text
Combat Calibration Lab -> First System Boss -> return shortcut -> Drain Market
```

The boss should embody a harmful system, not a final ideological villain. Names
such as Reconciliation Machine, Audit Engine, Clockwork Warden, and Misfile Core
are placeholders. A two-pattern prototype with a vulnerability window is better
than an ambitious unfinished fight.

## Layout sketch

```text
+------------------------------------------------+
| upper platforms / safe read positions          |
|                                                |
|          boss core / machine body              |
|                                                |
| lower arena with drop-through platforms        |
|                                                |
| entry gate                         return gate |
+------------------------------------------------+
```

The arena should support jumping over horizontal attacks, dropping through or
moving between platforms, using P4 if implemented, retreating to read tells, and
a short retry loop.

## Boss behavior target

Implement the smallest viable version of this pattern set:

```text
Pattern 1: stamp slam
  vertical warning or obvious windup, then impact.

Pattern 2: sweep or arm pass
  horizontal sweep requiring jump, drop, or platform movement.

Pattern 3: misfile pulse, optional
  some platforms/zones marked unsafe for a short window; label if no mechanic.

Vulnerability window
  core opens after miss, overheat, wrong stamp, P4 interaction, or switch.
```

Only one vulnerability rule is required for v1.

## Route variants

Use route variants as small changes or labels, not separate boss fights.

Good/private: disable or expose the system without feeding it private route
records. Evil/lawful: feed classification data into the machine for an easier or
faster clear. Neutral: fight normally. Chaotic: overload the machine and open a
rough shortcut. Famous/private: public alarmed clear versus quiet disable.

## Reward

The boss should award or annotate:

```text
P5: Route Memory / System Core
```

P5 should do at least one concrete thing: open return shortcut, activate map or
route label, unlock a visible high pickup route, open a next-branch placeholder,
or mark that the world remembers the player. If only one can be implemented,
prioritize the return shortcut.

## Return shortcut

After the boss, open a shortcut back to Drain Market. Options include pipe lift,
maintenance door to Oiler, upper balcony, drain flush to under-town pipes, or a
new door through right utility switchback.

Requirements:

```text
- player recognizes an earlier space;
- shortcut is faster than walking back;
- shortcut creates one immediate pickup or route opportunity;
- route is labelled if final art/mechanics are missing.
```

## Tuning rules

Keep the first boss forgiving: short phases, clear tells, safe read positions,
recovery before fight if supported, quick restart, no instant-kill pits, and no
unclear damage volumes.

## Acceptance criteria

A boss arena or stub exists, is reachable from Combat Calibration Lab, has at
least two patterns or labelled placeholders, has a vulnerability/clear condition,
awards or annotates P5, opens/stubs a return shortcut to Drain Market, labels
route variants, and validates.



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
