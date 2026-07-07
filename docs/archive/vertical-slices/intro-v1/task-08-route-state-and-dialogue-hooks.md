# Task 08: Route state, map layers, and dialogue hooks

## Purpose

Wire the intro-v1 map into durable route memory where practical, turning the most
important labels into flags, dialogue branches, lock states, quest state, and map
layer hooks.

This task incorporates the cartographer revision. Alice and Bob should no longer
be treated as only a handshake quest. They are unofficial cartographers, and the
main good/private route should reward the player with private map marks, field
survey annotations, and route memory.

## Files to read first

- `dev/vertical-slices/intro-v1/scaffold.md`
- `dev/vertical-slices/intro-v1/map-contract.md`
- `dev/vertical-slices/intro-v1/task-04-under-town-trust-route.md`
- `docs/planning/gameplay-idea-index.md`

## Files likely to change

- `crates/ambition_engine/src/save.rs`
- `crates/ambition_content/src/quest.rs`
- `crates/ambition_actors/src/dialog/content.rs`
- `crates/ambition_actors/src/intro/dialog.rs`
- `crates/ambition_actors/assets/ambition/worlds/intro.ldtk`
- `tools/ambition_ldtk_tools/specs/*.yaml`

## Design target

Implement route state as stored facts, not as one morality meter. Use names that
fit existing save/quest conventions, but these are the recommended facts:

```text
p1_stabilizer_received

map_basic_unlocked
map_private_marks_unlocked
bob_field_survey_received
route_memory_received

alice_route_note_carried
alice_route_note_delivered
alice_route_note_reported
alice_route_note_observed
alice_route_note_tampered

bob_survey_private
bob_survey_observed

private_route_opened
private_routes_compromised
official_map_registered
official_shortcut_opened

combat_calibration_received
first_system_boss_cleared
boss_cleared_private
boss_cleared_official
boss_cleared_chaotic
```

Do not refactor the whole dialogue, save, map, or quest system. Add the minimum
route memory hooks that make the slice iterable. If a real map layer cannot be
implemented, use durable flags plus DebugLabels/entity names that make the
intended layer clear.

## Priority 1: Oiler/Stabilizer and basic map awareness

Make the player able to trigger or receive `p1_stabilizer_received`. It should
affect at least one of:

```text
- under-town route label/lock;
- Oiler post-repair dialogue;
- repair node activation;
- basic map marker or route label;
- clear map-contract note if only annotated.
```

If map support exists, unlock or annotate:

```text
map_basic_unlocked
```

Basic map should mean visited rooms, obvious exits, repair/save nodes, and main
route names. It should not reveal private routes yet.

## Priority 2: Alice/Bob cartography loop

Wire the minimum viable cartography loop:

```text
Alice interaction -> alice_route_note_carried
Bob interaction checks alice_route_note_carried -> bob_field_survey_received
Return to Alice with bob_field_survey_received -> map_private_marks_unlocked
map_private_marks_unlocked -> private_route_opened or visible private map labels
```

If observed/private detection is easy:

```text
low/easy scanner route -> alice_route_note_observed or bob_survey_observed
high/private route -> bob_survey_private
```

If not, wire only carry, Bob survey, return, and private map unlock.

## Priority 3: private map marks

Implement at least one visible consequence of `map_private_marks_unlocked`.
Choose the easiest robust option:

```text
- hidden pipe entrance receives a DebugLabel or icon;
- private door becomes unlocked;
- route to right utility switchback opens;
- hazard/shortcut label appears near Bob's surveyed route;
- Drain Market gains a private map mark near an under-town entrance;
- map-contract.md records exact room/coordinate for private map mark.
```

Suggested debug labels:

```text
MAP_PRIVATE: Alice/Bob mark - quiet route
MAP_SECRET: Bob survey reveals return shortcut
MAP_DANGER: watched low route
MAP_UNKNOWN: not on official map
```

If conditional DebugLabels are not supported, place always-visible labels with
clear TODO text.

## Priority 4: report/tamper branch

Add one tempting evil/lawful hook, preferably in the right utility switchback or
an official-looking terminal near the utility branch:

```text
report Alice route note/private map data -> alice_route_note_reported
submit Bob field survey -> official_map_registered
```

Reward it with one small benefit:

```text
- open official shortcut;
- reveal official map label;
- disable one hazard;
- change a dialogue/label;
- add route record;
- mark a door as approved.
```

Cost should be soft at first:

```text
- Alice greeting changes;
- Bob comments that the route is now watched;
- private route label changes to MAP_WATCHED;
- private_routes_compromised is set;
- a note says later trust cost pending.
```

Do not make the first evil branch catastrophic. It should feel like convenience,
safety, maintenance, records, or map cleanup.

## Priority 5: combat and boss facts

Wire `combat_calibration_received`, `first_system_boss_cleared`, and
`route_memory_received`. If route variants are available, also wire one or two
boss outcome facts:

```text
boss_cleared_private
boss_cleared_official
boss_cleared_chaotic
```

`route_memory_received` should conceptually unlock changed/watched/cleared route
annotations even if only labels are available.

## Priority 6: visible memory

Make at least one small room-visible consequence. Examples:

```text
- Oiler post-repair line changes;
- Alice trust/suspicion line changes;
- Bob survey line changes if route was observed;
- official terminal line changes after report;
- lock wall opens;
- shortcut becomes accessible;
- MAP_WATCHED label appears after report;
- reward appears/reaches visibility;
- Drain Market route sign changes after boss clear.
```

## Dialogue guidelines

Keep lines short. Use route-function language.

```text
Oiler: repair, stabilize, pipes, fit through, do not leak.
Alice: map, carry, do not unfold, official maps find people.
Bob: field survey, wrong route, watched route, return mark.
Official/system terminal: register, record, classify, sanitize, approved map.
Boss/system: reconcile, mismatch, overheat, clear, route record.
```

Example placeholder lines:

```text
Alice: "Official maps are for people who enjoy being found."
Alice: "Carry this to Bob. Do not unfold it where the walls can read."
Bob: "Alice sent you? Good. She maps exits. I map mistakes."
Bob: "If you used the lower path, congratulations: you are on somebody else's map too."
Terminal: "Undocumented passage submitted. Shortcut approved. Privacy unresolved."
```

Avoid explicit Act 2 ideology. Early evil path should sound like convenience,
safety, maintenance, official map quality, records, and shortcuts.

## Lock and route guidelines

Use locks sparingly. Test mode may leave future-gated routes open. When a route
is intended to be gated later, label it as a story gate and keep a debug bypass
if needed for playtesting.

Prefer one real route state over five invented ones. If only one hook can be made
real, make `map_private_marks_unlocked` or `private_route_opened` real. If only
two can be made real, add `alice_route_note_reported` or `official_map_registered`.

## Acceptance criteria

At least three route/map facts are real durable state; Alice/Bob has a minimum
carry/Bob-survey/return state or a documented blocker; private map marks are
implemented or clearly annotated; one evil/lawful report or official map hook
exists; one visible consequence exists; neutral progression remains possible;
all flags/states are listed in a handoff note.

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
