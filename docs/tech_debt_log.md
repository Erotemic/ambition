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

### Runtime / state

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

- **HIGH — Hostile NPC conversion is one-way and slightly fragile**
  - File: `crates/ambition_sandbox/src/features.rs:apply_save`
  - When an NPC's hostile flag is set, `apply_save` removes the
    `NpcRuntime` and `spawn_enemy`s a striker with the same id. If
    the conversion races with anything else mutating `npcs` /
    `enemies` in the same tick, we'd double-spawn. Today the
    conversion is the only mutator inside `apply_save` so it's safe,
    but the invariant isn't enforced — a single test would lock it
    in.
  - Hostile NPCs that *die* don't yet write a "killed" save flag, so
    they respawn on every room re-enter. Fine for the sandbox,
    wrong for a real game.

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

- **MED — Boss spec registration uses the LDtk runtime id, not a
  semantic id**
  - File: `crates/ambition_sandbox/src/boss_encounter.rs:update_boss_encounters`
  - Lazy-registers a `BossEncounterSpec` whose id matches the
    `BossRuntime.id` (which is the LDtk iid like `BossSpawn-0158`).
    Works because we currently only have one boss; with two bosses
    in different rooms we need a real "encounter id" LDtk field on
    `BossSpawn` so authoring can label each one.

- **MED — Music tracks for boss phases are placeholders**
  - File: `crates/ambition_engine/src/boss_encounter.rs:gradient_sentinel`
  - Phase 1 / 2 / Enrage all reuse existing sandbox tracks. The swap
    *mechanism* works end-to-end (see the integration test) but the
    audio identity doesn't change yet. Authoring 3–4 tracks in the
    existing RON arrangement format is the gap.

- **MED — Cutscene system has no skip-with-warning UX**
  - File: `crates/ambition_sandbox/src/cutscene.rs`
  - `CutsceneAdvanceRequest::skip_cutscene` is wired but there's no
    UI for it. We also haven't decided whether holding ESC vs.
    tapping it is the right "skip" gesture.

- **LOW — Quest log lines are inlined in the HUD format string**
  - File: `crates/ambition_sandbox/src/app.rs:update_hud`
  - Real game wants a dedicated quest panel. Right now the lines
    just get appended to the debug HUD text.

- **LOW — Map menu is text-only**
  - File: `crates/ambition_sandbox/src/map_menu.rs`
  - We track visited rooms but render them as a text list in the
    HUD. A real minimap needs sprite or gizmo rendering of room
    bounding boxes + corridors. Use the LDtk world-grid data to draw
    boxes once we have a sprite atlas.

### Tests / observability

- **MED — Hostile-NPC conversion has no integration test**
  - We test the engine's `evaluate_character_ai` and the save
    schema's flag round-trip, but the actual "strike NPC 3 times →
    spawned enemy attacks player" loop is only exercised in the
    visible binary. Add a Bevy minimal-plugin test (per the bevy
    testing pattern memory) that drives the conversion.

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

(none yet — populate as items are paid down)
