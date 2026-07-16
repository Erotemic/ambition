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

**State:** PARTIAL and divisible into small patches.

Completed slices:

- Ambition dialogue cast names, aliases, and voice cue identities moved out of
  `ambition_dialog`/`ambition_sfx` into a content-owned registration over the
  open `DialogueVoiceCatalog`.
- The named `pirate_weapon` renderer and closed gun-sword read model were
  replaced by a generic wielded-item fact stream plus an App-local visual
  catalog populated by `ambition_content`; both light and heavy gun-sword ids
  use the content-owned art registration.

Next prioritize the closed item catalog, remaining named render/art bindings,
asset universe, projectile identities, and input techniques. Each patch must
install the correct provider-owned catalog, registration, or presentation seam
and delete the engine-owned closed content.

**Exit:** a second provider adds its named content without editing a reusable
engine crate. No noun scanner is part of this track.

## 4. Extract `ambition_sim_harness`

**State:** OPEN.

Move reset/step, typed actions, observations, reward/termination plumbing, and
programmatic composition below `ambition_app`. The harness accepts plugin/provider
composition rather than importing the flagship app.

**Exit:** a demo or test can run through the harness without linking Ambition's
product shell.

## 5. Converge boss behavior onto moveset authority

**State:** PARTIAL. The brain no longer owns a second attack timing projection or
direct special resolver; it emits transient profile intent and `MovePlayback` is
the execution authority. Remaining work is the broader phase/action-family fold.

Keep boss decision policy sophisticated, but make attack execution, timing,
cancellation, motion locks, and semantic effects use the shared move/action
lifecycle. Delete each superseded boss-specific path when its family migrates.

**Exit:** only then reassess whether any coherent boss crate remains.

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
