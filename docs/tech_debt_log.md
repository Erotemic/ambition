# Tech debt log

Running tally of jank, shortcuts, and "good enough for now" decisions
that should be revisited as the project matures. New items go at the
top. Each entry tags severity (LOW / MED / HIGH), a category, and
(when known) the file:line that holds the smell. Resolved items move
to the bottom under "Closed" with the commit that fixed them.

> **Note for the agent**: when you take a shortcut, write it here in
> the same commit that introduced it. When you fix something on this
> list, move it to the Closed section with the fix commit.

## Open

### Simulation-order debt

- **HIGH — Player-owned mechanics run *after* `sandbox_update` and
  bypass main movement/trace assumptions**
  - Files: `crates/ambition_sandbox/src/ledge_grab.rs`,
    `crates/ambition_sandbox/src/swim.rs`,
    `crates/ambition_sandbox/src/app.rs` (progression chain after
    `sandbox_update`).
  - `update_ledge_grab` and `update_swim` mutate
    `runtime.player.pos / vel / on_ground / on_wall / wall_clinging`
    *outside* the main `movement.rs` step. Trace recording fires
    after them so the snapshot is consistent, but any invariant
    enforced inside `movement.rs` (collision repair, ground/wall
    flag derivation, locomotion-state transitions) does not run a
    second time after these systems write. Today this is benign —
    swim only damps velocity inside water, ledge grab snaps the
    player to a ledge anchor that the engine probe vetted — but
    the next post-update mechanic that, say, repositions the
    player horizontally would need to re-validate against
    `World::blocks` itself or accept that the next frame's
    collision step has to fix it up.
  - Same shape covers the F3 stats editor's writes
    (`mana_current`, `slash_damage`, `invincible`): they're a
    sandbox-side veneer that the engine doesn't know about.
  - **Future target**: move player-owned mechanics toward explicit
    Player component / state-machine ownership inside the engine
    (one unified "player mechanics" phase, not N post-update
    bolt-ons). Until then: don't add a new post-update player
    mutator without writing the ordering decision down here, and
    prefer extending the engine state machine over adding another
    system to the chain.
  - **Multiplayer caveat**: this isn't a "make it MP-ready" item.
    `SandboxRuntime` is a global SP-only resource by construction;
    the per-player split is its own follow-up. This entry is
    purely about simulation-order coherence within SP.

- **MED — `SandboxRuntime` collected new per-player fields without
  per-player ownership**
  - File: `crates/ambition_sandbox/src/lib.rs`
  - Recent additions: `mana_current`, `mana_max`, `slash_damage`,
    `invincible`, `ledge_grab`, `player_died_pending`. Each one
    shipped on `SandboxRuntime` rather than on `Player` because
    the surrounding F3 / encounter / ledge-grab features needed a
    place to put state and `SandboxRuntime` was the lowest-friction
    answer. The cost is that the long-planned `Player`-as-entity
    refactor now has more fields to relocate.
  - **Rule of thumb**: if a new field is conceptually per-player,
    open a tech-debt entry instead of treating "stick it on
    `SandboxRuntime`" as zero-cost. The list is the budget the
    eventual per-player split has to spend.

### Runtime / state

- **HIGH — Wall-cling on the mob_lab lock wall teleports player onto
  the arena ceiling**
  - Trace: `debug_traces/ambition_trace_1777995217-972692847-000000_20578d15h33m37s`.
  - Repro shape: enter `mob_lab`, the encounter starts, the runtime-
    inserted `lockwall:mob_lab` block (LDtk px (480, 400) size
    (224, 208)) materializes between the hallway and arena. While
    wall-clinging on the lock wall's right edge (player x=718,
    lock_wall.right=704), the next y-sweep snaps the player from
    `(718, 434.1)` to `(718, -23)` — exactly `arena_ceiling.top - half_height`
    (`0 - 23`). The player then ping-pongs between `y=-23` (sitting
    on the ceiling, AABB.bottom = 0) and `y=423` (re-clinging on
    the lock wall) every time they press Jump. The auto-OOB
    detector does not fire because the 46 px excursion above the
    world envelope is within `OOB_MARGIN = 96.0`.
  - Same *shape* as the resolved "Wall-cling y-sweep teleports
    player to wall's far edge" lesson in
    `docs/lessons_learned.md`: an unconditional
    `pos.y += hit.block.aabb.top() - body.bottom()` snap when the
    swept hit returns `time_of_impact = 0`. The `body_is_side_contact`
    predicate added by that fix correctly skips the LOCK WALL (body
    y-range is nested inside lockwall y-range 400–608) — so the
    offending hit must be on a different block. The most likely
    suspect from the trace's "nearby collision" report is
    `arena_ceiling` (top at y=0, body.bottom at y=457 → snap delta
    -457 ≈ matches the observed -457.1 px correction).
  - Hypothesis to test: parry `cast_shapes(stop_at_penetration=true)`
    is returning a `time_of_impact = 0` hit on `arena_ceiling`
    when the body is starting from an exact-edge-touching configuration
    against the LOCK WALL. The vertical sweep then snaps to the
    ceiling's top instead of the (correctly-skipped) lock wall's
    top. This is one degree removed from the original lesson — the
    skip predicate filters the wall the body is touching, not the
    bogus far-block hit.
  - Repro target for the fix: an integration test in
    `crates/ambition_sandbox/tests/repro_walls.rs` that places the
    player wall-clinging on a lock-wall-shaped block with an
    arena_ceiling-shaped block above and asserts no >100 px y-snap
    after one update. Mirror of the existing
    `square_arena_wall_cling_full_world_does_not_teleport`.
  - Bumping `OOB_MARGIN` is NOT the fix — it would just hide the
    teleport. The right fix is rejecting `time_of_impact = 0` hits
    that snap the body MORE than the velocity budget for the
    frame, which is also what the `CollisionCorrection` event in
    the trace measures (`unexplained delta 457.1px (vel-budget 16.1px)`).
  - Until fixed, runtime-inserted blocks (lock walls, future
    encounter geometry) live alongside this bug. Document any new
    runtime block insertion with this caveat.

- **MED — `EnemyRuntime` movement still pre-computes `desired_x` from
  brain enums, then overrides via `ai_mode`**
  - File: `crates/ambition_sandbox/src/features.rs:EnemyRuntime::update`
  - The post-refactor pattern is: evaluate ai_mode → branch movement
    on it. The match-on-brain still has its own arms because the
    chase-speed and aggro-radius logic varies per `EnemyBrain`
    variant. The right shape is to push that into the brain trait
    (or into `EnemyArchetype`) so the AI evaluator's output can drive
    everything; today the brain match is duplicated under the
    `ai_mode` branch.
  - Leaving for the same reason as the broader state-machine refactor
    — it's a meaningful surgery on a tested system, schedule a
    dedicated pass.

- **HIGH — `SandboxRuntime` is one global god-resource**
  - File: `crates/ambition_sandbox/src/lib.rs`
  - Holds player state, feature runtimes, dialogue, physics tuning,
    timers, mana, slash damage, invincible flag, ledge-grab state.
    Per the architecture targets memory, per-player state should live
    on a Player entity / component. The architecture-targets doc is
    explicit that the global-resource shape doesn't extend to
    multi-player or per-player input feel.
  - The crate split (`docs/crate_split_plan.md`) renames it but keeps
    the global shape; the per-player split is a separate later
    refactor.

- **MED — Hostile NPC conversion is one-way; conversion path now
  tested but the race-with-other-mutators invariant isn't enforced**
  - File: `crates/ambition_sandbox/src/features.rs:apply_save`
  - When an NPC's hostile flag is set, `apply_save` removes the
    `NpcRuntime` and `spawn_enemy`s a striker with the same id. The
    `enemy_<id>_dead` save flag now suppresses the respawn loop
    (closed by commit `75ebfcb`). Four tests in
    `features::conversion_tests` lock the conversion behaviors in.
    Remaining smell: the invariant "only `apply_save` mutates
    `npcs`/`enemies` during the convert window" isn't enforced by
    types — a future system could violate it.

- **HIGH — `EnemyRuntime` and `BossRuntime` carry their own ad-hoc
  state machines**
  - File: `crates/ambition_sandbox/src/features.rs`
  - We just added `ai_mode: CharacterAiMode` snapshot but the actual
    movement / attack code still uses the timer-fields directly.
    Real refactor: swap those branches over to `evaluate_character_ai`
    + a small `tick(snapshot) → events` API so all combatants share
    one state machine. Boss patterns then layer on top via
    `BossPatternStep`.

- **MED — `SandboxRuntime::ledge_grab` shoves new player state
  outside `Player`**
  - File: `crates/ambition_sandbox/src/lib.rs`
  - Done deliberately to avoid touching the dense `movement.rs`. The
    cost is a small split-brain: gravity / wall-cling state lives on
    `Player`, ledge-grab state on `SandboxRuntime`. When the
    character state machine refactor lands, fold the ledge-grab state
    into the unified player state.

- **MED — `mana_current` / `mana_max` live on `SandboxRuntime`, not
  `Player`**
  - File: `crates/ambition_sandbox/src/lib.rs`
  - Same root cause as above. No engine ability consumes mana yet,
    so it's a debug field for the F3 inspector. Promote to engine
    `Player` (ideally as a `ResourceMeter`) when the first mana-cost
    ability lands.

### Encounter

- **MED — Encounter chest reward is hard-coded to a small heal**
  - File: `crates/ambition_sandbox/src/encounter.rs:update_encounters_from_world`
  - When an encounter clears we drop a chest with
    `PickupKind::Health { amount: 2 }`. Real encounters want
    per-encounter reward authoring (an `EncounterSpec::reward` field
    pointing at a `PickupKind` or a chest spec id).

- **MED — `runtime.player_died_pending` is a side-channel boolean**
  - File: `crates/ambition_sandbox/src/lib.rs`
  - The cleanest shape would be a Bevy event (`PlayerDiedEvent`) the
    encounter system reads. It's a boolean on `SandboxRuntime` for
    the same reason ledge-grab is — adding a Message channel is one
    more piece of plumbing to set up. Promote when we move
    `SandboxRuntime` per-player.

- **LOW — Camera ease snaps in overview mode**
  - File: `crates/ambition_sandbox/src/rendering.rs:camera_follow`
  - Overview camera (F5) sets the live scale directly so the
    debug toggle is instant. If we ever want a smooth dev-only
    transition, parameterize the rate.

### Architecture

- **MED — `feature_runtime_phase` runs every system in `sandbox_update`
  inline**
  - File: `crates/ambition_sandbox/src/app.rs`
  - The system has 16+ params and we already had to bundle them in
    `SandboxQueues` once. Each new feature pushes the limit. Splitting
    into multiple smaller systems with `apply_deferred` between them
    would be cleaner, but sequencing is tricky because phase helpers
    mutate `runtime.player` in series and rely on contiguous control
    flow. Plan: when the `Player`-as-entity refactor lands, those
    phase helpers become per-entity systems and the giant
    `sandbox_update` evaporates.

- **MED — Two parallel chains in the Update schedule**
  - File: `crates/ambition_sandbox/src/app.rs`
  - The sim chain is split into "main" and "progression" because the
    macro tuple-arity caps out around 20. The split is mechanical
    rather than meaningful; consumers shouldn't care. Long-term this
    goes away with explicit `SystemSet`s and proper ordering between
    them.

- **MED — `FeatureEventBus` is a workaround for the param-count cap**
  - File: `crates/ambition_sandbox/src/features.rs`
  - We fan events out through a resource because `sandbox_update`
    can't accept more `ResMut`s. Once the crate split lands and
    sandbox_update is replaced by per-entity systems, the bus may
    not be necessary at all.

- **LOW — `ProgressionResources` and `SandboxQueues` SystemParam
  bundles are ad-hoc**
  - File: `crates/ambition_sandbox/src/app.rs`
  - Pure pragmatism (16-param cap). Not jank exactly, but the bundles
    are growing. When the crate split lands, take a moment to make
    each bundle a public type with a docstring.

### Content / authoring

- **LOW — Boss spec id derives from name; explicit LDtk field still
  open**
  - File: `crates/ambition_sandbox/src/boss_encounter.rs:encounter_id_from_name`
  - The encounter id now normalizes the LDtk `BossSpawn::name` field
    (closed by commit `75ebfcb`). A purpose-built `encounter_id`
    LDtk field would still be cleaner — the name doubles as both the
    HUD display string and the save key.

- **MED — Music tracks for boss phases are placeholders**
  - File: `crates/ambition_engine/src/boss_encounter.rs:gradient_sentinel`
  - Phase 1 / 2 / Enrage all reuse existing sandbox tracks. The swap
    *mechanism* works end-to-end (see the integration test) but the
    audio identity doesn't change yet. Authoring 3–4 tracks in the
    existing RON arrangement format is the gap.

- ~~**MED — Cutscene system has no skip-with-warning UX**~~
  *(Closed — `CutsceneAdvanceRequest::skip_hold_seconds` accumulates
  while the player holds `Reset` (Backspace/Delete/pad-Select); the
  HUD shows a progress bar above `SKIP_HOLD_THRESHOLD_SECS = 1.2`,
  and the input layer flips `skip_cutscene = true` once the hold
  passes the threshold. Reset was chosen over Start so the pause
  toggle is unaffected during cutscenes.)*

- **LOW — Quest log lines are inlined in the HUD format string**
  - File: `crates/ambition_sandbox/src/app.rs:update_hud`
  - Real game wants a dedicated quest panel. Right now the lines
    just get appended to the debug HUD text.

- ~~**LOW — Map menu is text-only**~~
  *(Closed by commit `75ebfcb` — `MapMenuState::rooms` is populated
  from `LdtkProject::levels`, and `sync_map_menu` paints each room
  as a Bevy UI rectangle on a full-screen panel + corner minimap,
  with the active room highlighted. Despawn-and-respawn each tick is
  fine for <20 rooms; switch to per-room entities + change detection
  if room count grows.)*

- **LOW — Map UI repaints rectangles every frame**
  - File: `crates/ambition_sandbox/src/map_menu.rs:sync_map_menu`
  - Despawn + respawn pattern is cheap for the current room count
    but isn't ideal. Switch to per-room entities with change
    detection if the map ever holds many rooms.

### Tests / observability

- ~~**MED — Hostile-NPC conversion has no integration test**~~
  *(Closed by commit on 2026-05-05 — `features::conversion_tests`
  now exercises strike-flips-hostile, apply_save replaces NPC with
  enemy, dead flag suppresses the respawn, and authored EnemySpawn
  enemies are marked dead from the save flag. Doesn't yet drive a
  full Bevy app; pure-data tests covered the four scenarios.)*

- **MED — Boss music swap requests aren't asserted in tests**
  - The integration test
    `boss_encounter::tests::full_encounter_progression_intro_to_death`
    checks that "some" music change fired but doesn't assert the
    specific track sequence. Tighten the assertion when the real
    boss tracks land.

- **LOW — Ledge grab probe doesn't test the "diagonal corner" case**
  - File: `crates/ambition_engine/src/ledge_grab.rs`
  - We test wall-on-right + clear-above and reject blocked-above; we
    don't test what happens when two adjacent solid blocks form a
    long wall and the ledge is at the corner of the upper one. Edge
    case for an authoring scenario we haven't built yet.

### Build / repo

- **LOW — Sandbox `headless` feature build is broken**
  - File: `crates/ambition_sandbox/src/ldtk_world/bevy_runtime.rs`
  - Per the LDtk runtime spine memory, the headless feature gate
    doesn't fully cfg-out the LDtk plugin yet. The default visible
    build is fine; `cargo check --features headless` errors on
    `register_ldtk_entity` / `init_collection`. Tracked separately in
    `docs/headless_simulation.md`.

- **LOW — Stale tools directory + AppImage at repo root**
  - Files: `LDtk 1.5.3 installer.AppImage`, `tmp-config`, `todo.txt`,
    various `tools/...` artifacts.
  - Many are .gitignored or untracked but the repo root is starting
    to look noisy. Periodic cleanup.

## Closed

- **HIGH — Hostile NPC death wasn't persisted (respawn loop)** —
  closed by commit `75ebfcb` (2026-05-05). `apply_player_attack`
  writes `enemy_<id>_dead` flag for non-encounter, non-sandbag
  enemies on death; `apply_save` honors the flag for both NPC
  conversions and authored EnemySpawn entries.
- **MED — Boss spec registration used LDtk iid, not a semantic id** —
  closed by commit `75ebfcb`. `encounter_id_from_name` normalizes
  the BossSpawn `name` field; runtime_id link table still maps to
  the iid for combat damage routing.
- **MED — Hostile-NPC conversion had no test** — closed by
  `features::conversion_tests` (4 tests).
- **LOW — Map menu was text-only** — closed by commit `75ebfcb`. Real
  Bevy UI panel + minimap with room rectangles drawn from LDtk
  worldX/worldY.
