# Tracks — current executable queue

This is the execution order established by the 2026-07-16 recon consensus and
Jon's decisions. Historical tracks and completion narratives are not retained
here. Focused demo/game work may proceed in parallel when it does not create a
second engine path.

## Completed prerequisite: one placement-lowering authority

`7d972b6` threaded the App-installed `PlacementLoweringRegistry` through initial
session construction, reset, and LDtk hot reload; transition and restore already
used it. The no-registry production helper was deleted, and a focused test proves
room staging uses the caller-supplied authority.

## 1. Extract and consolidate the provider protocol — COMPLETED

`ambition_platformer_provider` now owns the provider lifecycle. The substantive
preparation/activation implementation moved out of the deleted
`crates/ambition/src/provider.rs`; `ambition::provider` is a re-export of the new
crate. Typed preparation storage, exact activation, session construction, and
cleanup are consolidated into ONE shared lifecycle: a provider supplies only a
session-world source system and calls `PlatformerExperienceAuthoring::install`.
The per-provider marker generic, the duplicated prepare/activate system pairs
(Ambition, Sanic, Mary-O, Pocket), and the per-provider `PreparedPlatformerSessions`
instances are gone. Host provider registration stays explicit in `shell_host.rs`.

**Exit — met:** providers supply authoring + a world-preparation source rather
than copying the lifecycle; `ambition` is a facade again.

## 2. Session-root exclusivity and exact reconstruction

**State:** LANDED (both gates met); residual N3.1 restore debt tracked separately.

- `SceneEntities` was **removed**, not relocated: every former handle is now
  derived from a canonical marker — the home avatar from `PrimaryPlayerOnly`, the
  HUD/quest roots from their session-scoped `HudText`/`QuestPanelText` markers. No
  process-global handle bag survives.
- Moving-platform live state now has session identity and deterministic
  reconstruction. `MovingPlatformSet` is registered snapshot state (RON codec in
  `ambition_world`, `SnapshotState` in `ambition_runtime`), so a within-room
  rollback restores the advancing kinematics exactly; it is rebuilt from the room
  at every construction path and cleared on teardown.
- A provider-installed `SessionTeardownPlugin` (`ambition_actors::session::teardown`)
  resets the session-scoped resource mirrors — `MovingPlatformSet`,
  `PossessionState`, `ControlledSubject`, `EncounterRegistry`/`EncounterView`,
  `BossEncounterRegistry`, `QuestRegistry`, `SandboxSimState` — when the scope
  retires, beside the generic entity sweep. No dangling handle or stale mirror
  survives a teardown into the next activation.
- Reset and restore already lower through the same App-installed placement
  registry (`7d972b6`); this campaign added the moving-platform authority to that
  shared reconstruction path.

**Exit gates, both required — both met:**

1. **Session isolation (met):** `game/ambition_demo_sanic_app/tests/session_isolation.rs`
   drives the real host through activate A → seed the resource mirrors with A's
   live handles → tear down → activate B, and proves no entity, scope,
   resource handle, or read-model row refers to the retired scope.
2. **Exact reconstruction (met):** the `desync_canary` restore-replay oracle
   (`gap_run` clean bit-for-bit, `MovingPlatformSet` now in the state hash) plus a
   focused `restore_reconstructs_moving_platform_kinematics` snapshot test and the
   `ambition_world` codec round-trip. Boss/portal rooms remain DIRTY for restore
   for the separate, pre-existing N3.1 reasons the coverage ledger records (active
   room not yet restored sim state); that is N3.1 debt, not this campaign.

## 3. Structural content evictions — parallel-safe

**State:** COMPLETE at the campaign's exit bar. Every family the 2026-07-16
decision record named is evicted or ruled already-met at its seam; the only
open tail is the deliberately deferred engine-default asset families below.

Completed slices:

- Ambition dialogue cast names, aliases, and voice cue identities moved out of
  `ambition_dialog`/`ambition_sfx` into a content-owned registration over the
  open `DialogueVoiceCatalog`.
- The named `pirate_weapon` renderer and closed gun-sword read model were
  replaced by a generic wielded-item fact stream plus an App-local visual
  catalog populated by `ambition_content`; both light and heavy gun-sword ids
  use the content-owned art registration.
- **Projectile visual identities** (`4a0c2d5`): the closed `ProjectileVisualKind`
  enum + hardcoded `art()` table (apple/lasersword/glider asset paths) in
  `ambition_projectiles` was replaced by an open `ProjectileVisualId` component +
  an empty-by-default `ProjectileVisualCatalog` filled by
  `ambition_content::projectiles`. The foundation `EnemyProjectileSpawn.visual_tag:
  u16` opaque channel became an open `visual_id: String`.
- **Input techniques** (`1ece162`): the closed named recognizers
  (`detect_quarter_circle`/`_grace`/`_half_circle`) were deleted from
  `ambition_input`, which now offers only the generic `detect_sequence` + an open
  `MotionTechniqueCatalog`. `ambition_content::input_techniques` registers
  `qcf`/`qcf_grace`/`hcf`; the fire system resolves gestures by id.
- **Held/inventory item art** (`022200d`): the closed `ItemArt` resource +
  `GAUNTLET_PROP_IDS` + literal-id `item_sprite()` match + hardcoded
  `sprites/props/*.png` loads in `ambition_render` were replaced by the
  `HeldItemArtManifest` contribution seam (like `WorldItemArt`), filled from
  `ambition_content::items::held_visuals`.
- **Puppy-slug deep-dream presentation** (`250c1b95`): the last named-content
  module the 2026-07-16 decision record called out (`deep_dream`, beside the
  already-evicted `pirate_weapon`) left `ambition_render`. The renderer now
  exposes the positioned, session-gated `ActorOverlaySet` seam; the material,
  systems, embedded shader, and dev toggle live in
  `ambition_content::presentation`. The engine asset root and `DeveloperTools`
  carry no Ambition-named residue.

**Substantially met via existing seams (no reusable-crate edit needed to add
content):**

- *Item identities.* `install_item_catalog(ItemCatalog::from_ron(...))` already
  lets a provider re-author every one of the 24 items' identity, flavor, and
  wiring (`display_name`/`category`/`held_item_id`/`dialog_id`) as data in
  `items.ron`. The fixed 6×4 = 24-slot OoT grid and its slot enum are Jon's
  **deliberate machinery** (`crates/ambition_items` module doc), not a leak —
  the same "a closed common schema is intentional, not reopened" stance this
  track already takes for the Tier-0 world schema. Residual reusable-crate item
  content is minor and disruptive-to-move for low value: the `Item::icon_path`
  menu-grid asset paths (candidate to fold into `ItemMeta` data) and
  `OwnedItems::starter()` (a convenience constructor used by several test
  fixtures). Persistence is already string-`dialog_id`-keyed.
- *Boss sprite sheets.* The App-local `ambition_actors::boss_encounter::BossCatalog`
  is the provider seam; `ambition_sprite_sheet::boss::builtin_boss_sheets()` is a
  documented **fallback-only** layout map that "a new provider does not edit."

**Remaining, deliberately deferred (engine-default machinery, no live consumer):**

- The `EntitySprite` / `ParallaxTheme` named-sprite enums and the asset-universe
  residue (fonts, the `ambition/sandbox.ron` data id, sprite-pack tiers, the
  `ambition_ldtk_map` world manifest ids) in `ambition_sprite_sheet` /
  `ambition_asset_manager`. Unlike the evicted families (boss/weapon asset paths,
  fighting-game gestures, item props — genuine provider content a second
  platformer differs on), these are **engine-default assets**: the demo providers
  (Sanic, Mary-O) share the same UI fonts, quality-tier sprite packs, and
  generic entity tiles. There is no live second-provider that needs a *different*
  font or entity-sprite set, and they are woven into `sandbox_image_manifest`
  generation. Per "add the override seam when the use case lands" (design
  balance), these stay as reusable machinery until a provider actually differs.
  Audio — the genuinely content-heavy asset kind — is already fully evicted via
  the provider-indexed `AudioCatalogRegistry`; the `SandboxCatalogInputs` row seam
  already carries worlds / characters / bosses / music provider-side.

**Exit — met:** a second provider adds its named content without editing a
reusable engine crate. No noun scanner is part of this track. The dialogue,
wielded-item, projectile-visual, input-technique, held-item-art, actor-overlay
(deep-dream), item-identity, and boss-sheet families all meet this at their
seams; the `EntitySprite`/asset-universe manifest families remain the
deliberately deferred tail (rationale above), to be reopened only when a
provider actually differs.

## 4. Extract `ambition_sim_harness` — LANDED

**State:** LANDED (exit gate met).

`crates/ambition_sim_harness` now owns reset/step, the typed `AgentAction`
(→ `ControlFrame`), the `AgentObservation` read-model, the example `reward`
shaping, the `random_policy` fuzz driver, and `SandboxSim`. It sits below
`ambition_app` and depends only on the `ambition` facade (which does not depend
on `ambition_app`/`ambition_content`).

The single entanglement was inverted: `SandboxSim::build(options, compose)` takes
a caller-supplied composition closure. The harness owns the engine half
(`add_headless_foundation`, the fixed-tick sim-schedule choice, the time strategy,
the startup pumps) and hands the App to `compose` for the game's content + sim
plugins. `ambition_app::rl_sim` is a thin binding: it re-exports the harness and
supplies `ambition_sim_composition` (LDtk validate + world install + start-room +
`SandboxSimulationPlugin`) plus an `AmbitionSim` extension trait giving the
ergonomic `SandboxSim::new()` constructors the RL binaries and behavior/oracle
tests use. The Ambition-specific `run_headless`/`HeadlessReport` stay in
`ambition_app`.

**Exit — met:** `crates/ambition_sim_harness/tests/composes_below_the_app.rs`
builds a minimal one-room session and drives step/observation/reset through the
harness while linking only the `ambition` facade — never `ambition_app`.

## 5. Converge boss behavior onto moveset authority

**State:** CONVERGED (5cf27ae4d completed the phase/action-family fold begun in
`c618f0c`). Family-by-family:

- **Attack execution** (`c618f0c`): `MovePlayback` is the one execution
  timeline; the brain emits transient profile intent; `BossAttackState` is a
  pure projection of the live move.
- **Timing** (`5cf27ae4d`): the cycle-mode `CyclePhase` windup/active/cooldown
  machine — a second timing projection running the move's own durations in
  parallel — is DELETED. Cycle mode is pure decision policy (rest clock +
  rotation); the brain observes its live move back through
  `BossPatternContext::live_attack` and sustains/rests off what the move
  actually does.
- **Cancellation**: an abandoned windup aborts at the trigger (intent
  disappearance); a striking move is committed (the Smash convention). Bosses
  author no `Cancelable` windows today — an authoring choice on the shared
  vocabulary, not a missing mechanism.
- **Motion locks** (`5cf27ae4d`): `MoveWindow::motion_scale` +
  `MoveSpec::motion_scale_at` are the moveset's authored motion-lock primitive,
  enforced at body integration for EVERY body and controller; the brain-side
  `strike_speed_scale` damping (which possession bypassed) is deleted.
- **Semantic effects** (`c618f0c` + `5cf27ae4d`): specials ride
  `sustain_effect`/`Effect{key}`; telegraph cue/vfx (BD3), which the
  convergence had left runtime-dead, now bake as rising-edge `MoveEvent`s
  through the shared `dispatch_move_events` channel.
- **Defensive hurtboxes — no fold needed, by inspection:**
  `damageable_volumes` selects per-pose hurtbox rows (GNU-ton's head-only
  descent window) from the actor's CURRENT POSE, and the pose is picked from
  the projected `BossAttackState`, i.e. from the live move — one authority
  chain already. Wiring `WindowTag::Invuln`/`Armor` for bosses would add a
  SECOND vulnerability mechanism beside the sprite-metadata one; the per-pose
  hurtbox pipeline belongs to the actor-geometry-unification track.
- **Stays boss/encounter-owned by design:** the encounter phase machine
  (Dormant→…→Death, music, phase-invuln gate), scripted encounter mechanics
  (`CommandedMove`, `FallingHazard`), and the whole `boss_pattern` decision
  brain (scripted cursor, Select/Stance/Interrupt, macro chase/retreat,
  deterministic RNG) — decision policy is allowed to stay sophisticated.

**Exit reached — reassessment now open (Jon's call, maintainer decision #6):**
whether any coherent boss crate remains to carve. Input from the fold: what is
left boss-specific is decision policy (`boss_pattern`), encounter
orchestration, and sprite-metadata geometry derivation — no boss-specific
execution machinery survives.

## 6. Repair domain-plugin ownership

**State:** OPEN.

Audit runtime leaf-function knowledge. Domain crates install their local
messages, resources, systems, and public schedule sets. Runtime retains the
global phase graph and true cross-domain adapters.

**Exit:** runtime orders domain sets more often than it names implementation
leaf systems, and app/dev-specific setup is not hidden in the generic engine
assembly.

## 7. Split touch semantics from touch presentation

**State:** OPEN.

Separate raw touch/gesture folding and semantic `ControlFrame` production from
the visual joystick/button overlay and presentation dependencies.

## 8. Finish valuable render/read-model cleanup

**State:** OPEN, bounded. The confirmed dead `ambition_render` input/interaction/
Leafwing dependencies were removed in `7d972b6`.

Add read-model fields only for mutable simulation facts whose direct observation violates the one-way seam. Do not manufacture a
`SimView` copy of immutable authored world data merely to reduce dependency
count.

## 9. Reassess only after real consumers

- Menu-host extraction waits for Smash Siblings/Hollow Lite.
- Boss decomposition waits for track 5.
- `features/` naming remains low priority and must be coherent if attempted.
- Provider-owned placement families remain a deferred design question; the closed common Tier-0 world schema is not reopened.

## Standing execution rule

Do not create a policy/scanner task merely to accompany an architectural patch.
Use types, ownership, crate direction, visibility, and behavioral acceptance
first. A new policy test needs a concrete recurring harmful state that those
mechanisms cannot express.
