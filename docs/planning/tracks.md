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

**State:** OPEN / N3.2 campaign.

- Eliminate stale process-global mirrors, beginning with an audit of whether each `SceneEntities` handle should exist.
- Give moving-platform live state mechanical session identity and deterministic reconstruction.
- Align reset and restore around the same room/session construction services.

**Exit gates, both required:**

1. Activate A, exercise it, tear it down, activate B (or A with a fresh scope), and prove no entity, relationship, cache, read model, or raw handle refers to the old scope.
2. Reset and restore reconstruct equivalent room-derived state through the same authorities and produce the expected canonical snapshot/observation result.

## 3. Structural content evictions — parallel-safe

**State:** OPEN and divisible into small patches.

Prioritize the closed item catalog, named render modules/art bindings, asset
universe, projectile identities, input techniques, and dialogue/audio cast data.
Each patch must install the correct provider-owned catalog, registration, or
presentation seam and delete the engine-owned closed content.

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
