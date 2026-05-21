# Intro v1 playtest handoff

Status: end-of-session handoff for the intro-v1 vertical slice, written at
2026-05-21 after Tasks 01–08 of the scaffold landed. This file is the single
best place for a next-iteration agent or playtester to pick the slice up.

## Playable route graph (post-Task 08)

```text
intro_wake_room  (1024x384)
  └ EdgeExit  wake_to_raid          → intro_raid_corridor

intro_raid_corridor  (1600x512)
  ├ EdgeExit  raid_from_wake        ↔ intro_wake_room
  └ EdgeExit  raid_to_escape        → intro_escape_shaft

intro_escape_shaft  (1280x1280)      ★ vertical climb, Task 02
  ├ EdgeExit  escape_from_raid      ↔ intro_raid_corridor    (lower-left)
  └ EdgeExit  escape_to_drain       → drain_alley            (upper-right)

drain_alley  (1024x1024)             ★ Drain Market knot, Task 03
  ├ EdgeExit  drain_from_escape     ↔ intro_escape_shaft     (street level)
  ├ EdgeExit  drain_to_gate_stack   ↔ gate_stack_lower       (street level)
  ├ Door      under_town_to_drain   ↔ under_town_pipes       (lower grate)
  ├ Door      drain_from_bob_return ↔ bob_relay              (mid manhole)
  └ Door      drain_from_boss_return↔ first_system_boss      (lower drain)

under_town_pipes  (1024x768)         ★ Task 04 cartography route
  ├ Door      under_town_from_drain ↔ drain_alley            (top grate)
  └ EdgeExit  under_town_to_alice   ↔ alice_relay            (right)

alice_relay  (1024x768)              ★ Task 04
  ├ EdgeExit  alice_from_under_town ↔ under_town_pipes       (left)
  └ EdgeExit  alice_to_bob          ↔ bob_relay              (right)

bob_relay  (1024x768)                ★ Task 04
  ├ EdgeExit  bob_from_alice        ↔ alice_relay            (left)
  └ EdgeExit  bob_return_to_drain   ↔ drain_alley            (right manhole)

gate_stack_lower  (1600x768)         ★ Right Utility Switchback, Task 05
  ├ EdgeExit  gate_stack_from_drain ↔ drain_alley            (left)
  ├ EdgeExit  pirate_sky_up         ↔ pirate_sky_arena       (top edge)
  ├ Door      intro_portal_zone     → central_hub_complex    (cross-world)
  └ EdgeExit  gate_to_combat_lab    → combat_calibration_lab (right)

combat_calibration_lab  (1280x768)   ★ Task 06
  ├ EdgeExit  combat_lab_from_gate  ↔ gate_stack_lower       (left)
  └ EdgeExit  combat_lab_to_boss    → first_system_boss      (right)

first_system_boss  (1280x768)        ★ Task 07
  ├ EdgeExit  boss_from_combat_lab  ↔ combat_calibration_lab (left)
  └ Door      boss_return_to_drain  ↔ drain_alley            (right)

pirate_sky_arena  (2400x1024)        existing sky tease, untouched by v1
  └ EdgeExit  sky_arrival           ↔ gate_stack_lower
```

11 levels in `intro.ldtk` total. The cartography loop closes
(drain → under_town → alice → bob → drain) and the combat loop closes
(drain → gate → combat lab → boss → drain).

## Real durable state wired (Task 08)

Pickup flags (set by walking into a `PickupSpawn` with
`kind: "flag:<id>"`):

```text
p1_stabilizer_received               (drain_alley, beside Oiler)
alice_route_note_carried             (alice_relay, on Alice's bench)
bob_field_survey_received            (bob_relay, on Bob's bench)
intro_p5_route_memory_received       (first_system_boss, on the P5 ledge)
intro_p4_combat_calibrated           (combat_calibration_lab, on P4 pad)
intro_shaft_alcove_visited           (intro_escape_shaft, in the lab alcove)
intro_shaft_late_ledge_reached       (intro_escape_shaft, late ledge — UNREACHABLE w/o future kit)
intro_drain_roof_reward_taken        (drain_alley, rooftop)
```

NPC-talked flags (auto-set by the existing interact system when the
player engages a dialogue):

```text
npc_oiler_intro_talked
npc_alice_intro_stub_talked
npc_bob_intro_stub_talked
npc_gate_janitor_ripple_talked
npc_manifest_kiosk_wrong_list_talked
npc_creator_intro_talked
npc_news_board_lab_incident_talked
… (one per dialogue_id used in any LDtk NpcSpawn)
```

Switch flags (auto-set when the player toggles a `Switch`):

```text
switch_intro_portal_switch_used
switch_combat_lab_classify_switch_used
```

Quests (auto-started at boot, all in `default_quest_specs()`):

```text
intro_p1_stabilizer        — 2 steps: talk Oiler, pick stabilizer
intro_cartography_route    — 3 steps: alice → bob → P5
first_steps                — existing tutorial
test_switch_quest          — existing test
quest_lab_visit            — existing test
pirate_treasure            — existing
```

## Route scripts

### Script A — main good/private cartography route

```text
 1. Start in intro_wake_room. Talk to Creator (optional).
 2. Walk right → intro_raid_corridor.
 3. Walk right → intro_escape_shaft, lower-left entry.
 4. Climb (lower_a..lower_e → mid_a..mid_c → upper_a..top_a) to the
    top landing. Side-alcove pickup along the way sets the
    `intro_shaft_alcove_visited` flag.
 5. Top-right EdgeExit → drain_alley street level. Talk to Oiler
    (sets `npc_oiler_intro_talked`), pick up the stabilizer
    (`p1_stabilizer_received`). `intro_p1_stabilizer` quest completes.
 6. Drop to the pipes layer, press Interact at the under-town
    grate door → under_town_pipes.
 7. Walk right through the pipe interior, read the MAP_PRIVATE
    Alice mark, avoid the skitter, exit right → alice_relay.
 8. Talk to Alice, walk onto the route-note pickup
    (`alice_route_note_carried`). Step 1 of `intro_cartography_route`
    completes.
 9. Walk right → bob_relay. Take either the low/MAP_WATCHED ground
    route or the high/MAP_PRIVATE one-way platforms.
10. Talk to Bob, walk onto the field-survey pickup
    (`bob_field_survey_received`). Quest step 2 completes.
11. Walk right → drain_alley via the manhole (`drain_from_bob_return`
    Door). The cartography loop is now closed back to town.
12. Walk right through drain → gate_stack_lower (Right Utility
    Switchback). Read the MAP_OFFICIAL / MAP_PRIVATE / MAP_WATCHED
    labels along the floor.
13. Right edge EdgeExit → combat_calibration_lab. Bounce on the P4
    ReboundPad and pick up `intro_p4_combat_calibrated`. Defeat the
    patrol and the spitter/striker pair.
14. Right edge EdgeExit → first_system_boss. Engage the
    `clockwork_warden`-brained BossSpawn. After the encounter ends
    pick up the P5 ledge reward (`intro_p5_route_memory_received`).
    Quest step 3 completes — `intro_cartography_route` is done.
15. Right-edge Door → drain_alley (lower-drain emergence).

### Script B — neutral route

```text
1-5 same as Script A.
6. Skip the under-town descent. Walk right through drain_alley to
   gate_stack_lower.
7. Take the official-looking floor route past the Gate Janitor,
   Manifest Clerk, and portal machinery.
8. Right edge → combat_calibration_lab. Clear the encounter ladder.
9. Right edge → first_system_boss. Fight the boss.
10. Right-edge Door → drain_alley. Cartography quest
    `intro_cartography_route` remains stuck at step 0 (Alice never met)
    but the slice is fully clearable.
```

### Script C — evil/lawful report route

```text
NOT YET REAL. The combat_lab_classify_switch toggles a save flag
(`switch_combat_lab_classify_switch_used`) but no system listens to
it yet, and gate_stack_lower has no "submit private route" terminal.
This script is documented for the next session: place a Switch
near the Manifest Clerk in gate_stack_lower whose activation sets
`alice_route_note_reported`, then add a quest step (or label-change
listener) that flips the MAP_PRIVATE → MAP_WATCHED label class on
the under-town grate.
```

### Script D — cartography state check

```text
 1. Run `cargo run -p ambition_sandbox --bin headless -- --start-room=intro_wake_room`
    OR launch the desktop binary with the intro entry. Open the
    in-game M menu / `MapMenuState` HUD to confirm room visits accumulate.
 2. Inspect the save file (sandbox save resource / persistence) for
    presence of the flag list in §"Real durable state wired".
 3. Confirm the `intro_cartography_route` HUD entry advances on each
    pickup.
```

## Room-by-room polish notes

These are post-build observations from spec inspection + tests. None
of the rooms have been hands-on playtested in this session; treat
the notes as a starting point for the next playtest.

### intro_wake_room
- Untouched by intro-v1. Should still work end-to-end via the
  existing v1 layout (verified via embedded_content_graph_validates).
- One-line risk: `wake_room_arrival` Door zone still targets the
  cross-world `central_hub_complex/intro_wake_door`. Confirm the
  cold-launch behavior places the player on the wake_room floor,
  not at central_hub.

### intro_raid_corridor
- Untouched. The existing creator/raid lines + EnemySpawn pair land
  as before.

### intro_escape_shaft (Task 02)
- Math-checked: every climb gap is ≤80 vertical, within single-jump
  apex (~88 px). Late ledge stays unreachable with double-jump
  (~150 px apex) and the horizontal stretch from upper_f.
- Risk to playtest: the steam HazardBlock between mid_c and upper_a
  sits at y=656 — a sloppy left-drift jump from mid_c could brush
  the bottom of the hazard. Reduce hazard width or move it lower if
  it punishes the intended right-bias climb.

### drain_alley (Task 03 + Task 04 + Task 07 patches)
- Has three Door zones (`under_town_to_drain`,
  `drain_from_bob_return`, `drain_from_boss_return`) in addition to
  the two EdgeExits. The Doors all fire on Interact. Verify the
  manhole/grate prompts are readable.
- The Oiler stabilizer PickupSpawn sits at (368, 568) — touching it
  while talking to Oiler may collect the flag before the player
  reads the dialogue. Move it 16 px right if that's confusing.

### under_town_pipes (Task 04)
- Skitter enemy uses brain `medium_striker` (taken from the raid
  corridor cell). Could be too aggressive for the intended "single
  patrol" beat — swap to `Patrol:pipe_loop` if it feels hostile.

### alice_relay (Task 04)
- LockWall `alice_private_return_lock` is non-conditional; it just
  blocks the bottom-right pocket. Players who try to use it without
  the survey see a wall, not a "locked door" affordance. Promote to
  a conditional gate (Task 08 follow-up).

### bob_relay (Task 04)
- High-route platforms are spaced for double-jump (gap from
  bob_high_a top y=480 to bob_high_b top y=384 is 96, which needs a
  double-jump). Confirm the player has double-jump available by
  this point in the intro.

### gate_stack_lower (Task 05)
- Mostly unchanged from v1. New entities are the combat-lab exit
  (right edge), the alice/bob private LockWall (bottom-center), and
  the three MAP_* labels. Verify the LockWall doesn't visually
  obstruct the Manifest Clerk approach.

### combat_calibration_lab (Task 06)
- Patrol enemy uses `brain: Patrol:lab_patrol_line` — if no such
  KinematicPath exists in the level the brain falls back to passive
  behavior. Add a `KinematicPath` entity named `lab_patrol_line` or
  switch the brain to `Patrol:` (empty) if Patrol-without-path is
  supported.
- Spitter uses `Guard:96`. Verify the leash radius produces visible
  back-and-forth motion before the player gets in melee range.
- ReboundPad `impulseY: -780` is a guess; tune if the player either
  ceilings out or barely lifts.

### first_system_boss (Task 07)
- BossSpawn brain `PhaseScript:clockwork_warden` reuses the existing
  authored boss profile. Behavior, sprites, and quest hooks all
  carry over — but the boss arena's geometry was authored for an
  unrelated room (the original gradient_sentinel arena), so phase
  transitions that depended on that geometry may not look great
  here. Confirm by entering the room.

## Real vs labelled branch hooks

```text
REAL (saves a durable flag the game can read back)
  - intro_p1_stabilizer quest progression
  - intro_cartography_route quest progression
  - Every PickupSpawn with `kind: "flag:..."`
  - Every NpcSpawn talk → npc_<dialogue_id>_talked
  - The combat_lab_classify_switch toggle → switch_<id>_used

LABELLED (DebugLabel; no behavioral effect yet)
  - MAP_PRIVATE / MAP_OFFICIAL / MAP_WATCHED / MAP_DANGER / MAP_SECRET
    placements in every Task 03–07 room.
  - alice_private_return_lock + gate_alice_private_lock LockWalls.
  - "submit private route" evil/lawful terminal (DOES NOT EXIST).
  - Forest/sky teases in drain_alley.
  - The "→ Manifest Office (TBD)" / "→ Nazi Fortress (TBD)" labels
    in gate_stack_lower.
```

## Suggested next polish order

1. Run a real hands-on playtest (Script A, then B). Note actual jump
   failures, soft locks, confusing transitions.
2. Promote the LockWalls in alice_relay + gate_stack_lower into
   conditional Door zones gated on `bob_field_survey_received`.
   Simplest approach: add a `BobSurveyGateRegistry` resource +
   system that despawns the LockWall entity when the flag flips.
3. Add a chained-flag system that emits
   `map_private_marks_unlocked` and `route_memory_received` as soon
   as `bob_field_survey_received` and
   `intro_p5_route_memory_received` (respectively) land in save.
4. Wire the evil/lawful report path: add a Switch near Manifest
   Clerk whose activation sets `alice_route_note_reported`, then a
   listener that changes the MAP_PRIVATE label class to MAP_WATCHED
   on the under-town grate.
5. Add conditional dialogue branches for Oiler (post-stabilizer),
   Alice (note-carried), Bob (low/observed vs high/private route).
6. Tile-paint variation: the current `tileset paint --map 1=0 --map
   2=28` uses two tile indices for every room. Author distinct
   tile bands per room (drain vs lab vs cave) so the biomes read.
7. Pick a single onboarding seam (drain → under_town descent) and
   add a 6–8 frame cutscene or banner so the cartography route
   feels like a deliberate beat, not a wall pass.
8. Replace the boss arena geometry (currently inherited from the
   clockwork_warden's original room) with a layout tuned to the
   intro-v1 reading positions.
9. Decide whether `pirate_sky_arena` belongs in the intro slice or
   should be moved to a later act file. The sky tease is a top-edge
   transition the player can stumble into during Task 05; either
   gate it or move it out.

## Validation results (this session)

```bash
# intro.ldtk schema + content graph
PYTHONPATH=tools/ambition_ldtk_tools python3 \
  tools/ambition_ldtk_tools/ambition_ldtk_tools/validate.py \
  --secondary-world crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk \
  crates/ambition_sandbox/assets/ambition/worlds/intro.ldtk
# → OK: 0 warnings, 11 levels.

# sandbox unit/integration tests (no UI)
cargo test -p ambition_sandbox --lib
# → 584 passed; 0 failed.

# headless smoke (full intro start, 120 ticks)
cargo run -p ambition_sandbox --bin headless -- --start-room=intro_wake_room
# → headless run completed: 120 ticks; active room intro_wake_room;
#   39 rooms loaded (intro + sandbox merge).

# pre-existing failure: `cargo fmt --check`
# → Reports drift in files NOT touched by intro-v1 (rooms.rs, etc.).
#   Not introduced by this session; safe to ignore for the slice
#   handoff, but worth a separate fmt-cleanup commit.

# pre-existing failure: `python -m ambition_ldtk_tools doctor intro.ldtk`
# → False-positives on the two cross-world LoadingZones into
#   central_hub_complex because doctor does not accept
#   --secondary-world. The underlying validate.py call (above) is
#   clean. Fix: extend doctor's CLI to forward --secondary-world.
```

## Map-contract delta

`map-contract.md` was the Task 01 snapshot before any of the Task
02–08 reshapes landed. It is now out of date in the following ways:

- `intro_escape_shaft` is 1280×1280, not 1280×512 (Task 02).
- `drain_alley` is 1024×1024, not 1024×512, with three new Doors
  (Task 03/04/07).
- Five new levels exist: `under_town_pipes`, `alice_relay`,
  `bob_relay`, `combat_calibration_lab`, `first_system_boss`.
- The two LoadingZone-graph errors from `doctor` are still the same
  cross-world `central_hub_complex` references; behavior unchanged.

A "Section 1 update" patch on `map-contract.md` is the right
follow-up; rewriting the whole audit is not needed because the
gridvania row strategy and the tooling notes are still correct.

## What changed (this session)
Tasks 01–08 of `dev/vertical-slices/intro-v1/` from scaffold to
end-to-end clearable slice. 11 levels, 2 quests, ~9 durable flags,
the `tileset add-layer` idempotent fix, the StoryFlag pickup-to-
SetFlag wiring, and the PickupSpawn-kind-to-authored-flag content
validation extension.

## What was validated
LDtk schema + content graph (0 warnings), `cargo test
-p ambition_sandbox --lib` (584/584), headless smoke for 120 ticks.

## What remains placeholder
Conditional LockWalls; report/tamper Switch + listener; chained
flags (map_private_marks_unlocked, route_memory_received);
conditional dialogue branches; combat lab brain/path tuning;
boss arena re-geometry; tile-paint biome variation; hands-on
playtest.

## What felt fun or unreadable
Not playtested in this session. The math says the escape-shaft
climb is on-rails, the cartography loop closes cleanly, and the
combat → boss → drain return chain works. The empirically-fun
question needs a real controller.

## Which room/route should be worked on next
Task 09.5 follow-ups, in priority order:

1. Hands-on Script A playtest (the spine).
2. Conditional LockWall + chained-flag system (closes Task 08's
   stretch goals).
3. Evil/lawful report Switch in gate_stack_lower (Script C).
4. Boss arena re-geometry / tuning.
