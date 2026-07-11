# Sanic visible/playable recovery

**Priority:** immediate P3 bugfix and architecture proof.

**Status (2026-07-11):** playable-persona ARCHITECTURE landed; native Sanic asset
provisioning remains open. Canonical model: the player carries a sim-owned
`ambition_characters::actor::WornCharacter(id)`; gameplay derives from it
(`apply_worn_character_gameplay`) and presentation derives from it
(`ambition_render::bind_worn_character_presentation`, used by the app AND demos).
One identity, no second authority. Slice status + evidence are per-slice below.

## Problem

The standalone Sanic shell opens a window, renders the room, and plays the
intended music, but the player is a fallback rectangle and input does not drive
the simulation.

This is not acceptable as a demo success condition. The demo exists to prove a
new game can install several playable characters and obtain input, simulation,
presentation, camera, and audio through a small public Ambition composition
surface.

## Confirmed diagnosis

### Input

The engine already has the intended input path:

```text
keyboard/gamepad -> ActionState -> ControlFrame -> fixed-tick latch
-> SlotControls -> Brain::Player -> ActorControl -> shared body tick
```

The standalone visible persona does not reliably prove that this complete path is
installed and live. Failures currently degrade to a neutral `ControlFrame`, so a
window can look healthy while ignoring every key.

Treat this as a wiring/diagnostic bug first, not a reason to invent a second input
system.

### Character presentation

`SanicDemoContentPlugin` installs a character catalog and selects `sanic`, but the
reusable presentation group does not own the complete selected-character binding
path. The main Ambition app still contains logic that resolves the selected
character's sheet and attaches the sprite, anchor, animator, baseline, and
character identity.

The temporary fallback `Sprite` made the missing player visible, but it also hid
the missing composition seam. A rectangle is a degraded diagnostic, not a
successful character install.

### Assets

The Sanic generator target exists, but the runnable shared asset tree must contain
or reproducibly generate the declared Sanic sheet image and manifest. Reused
Ambition visuals must resolve through the same content/asset registration seam.

## Design target

A game should be able to define several playable profiles without app-local
sprite or input code:

```text
Playable profile
  visual profile id
  body/movement profile
  action/capability profile
  controller assignment
  game-specific tags/rules
```

Sanic may reuse an Ambition visual while supplying different Sanic movement,
actions, and rules. Visual identity must not force Ambition-the-game behavior.

The immediate recovery may use the current catalog schema, but it must establish a
public reusable binder. A later catalog cleanup may separate visual profiles from
gameplay profiles if the multi-character proof demonstrates real duplication.

## Ordered fix plan

### S0 - observable input path — DONE

**Evidence:** `game/ambition_demo_sanic_app/tests/standard_input_path.rs`
(`--features input`): synthetic ArrowRight → leafwing `ActionState` → the standard
`populate_control_frame_from_actions` bridge → fixed-tick latch →
`SlotControls[PRIMARY]` → brain → observable movement, plus a no-input twin. Fix
landed: `ambition_host` now registers the `CursorMoved` window message
`update_active_input_kind` needs (it panicked headlessly before); the bridge's
neutral fallback is now diagnostic (`warn_once`).

Add one focused integration test that checks all checkpoints in the real windowed
composition without requiring a physical window:

1. the canonical player receives `ActionState` and `InputMap`;
2. synthetic key/gamepad input changes `ActionState`;
3. the host bridge publishes a non-neutral `ControlFrame`;
4. the fixed-tick latch publishes the frame to the player's slot;
5. one simulation tick changes player velocity or position.

Fix only the first broken checkpoint. Never add a Sanic-local keyboard system.
Make neutral fallback diagnostic when the input bridge expected exactly one
player but found zero or many.

### S1 - coherent playable persona — OPEN (nicety)

Expose one public facade/persona that means "windowed and playable" and installs
rendering, window host, input bridge, and the standard fixed-tick connection.
Downstream games should not have to discover fragile feature combinations such as
`visible` plus a separately remembered `input` feature.

Headless builds must remain renderer-, window-, and audio-device-free.

**Status:** DONE for the demo — `visible` now implies `input`, so
`cargo run --features visible ... --window` is interactive; the headless shell stays
input-free. (The CANONICAL-identity work is a separate **DONE** slice: `WornCharacter`
+ `apply_worn_character_gameplay`, proven by `ambition_actors`
`avatar::starting_character::tests` + the snapshot round-trip. Dialogue now derives
the speaker from the entity's `WornCharacter`, not `StartingCharacter` — no second
authority. Spawn + runtime share ONE `apply_worn_character_overlay`, which resolves
name + kit from identity + the body's persisted `AbilitySet` — TOTAL and
deterministic. The `default_character_id`↔hardcoded-kit coupling was replaced by an
engine-neutral `PlayableKitSource::{Authored,HostCode}` catalog field, so the content
default is free to wear its OWN authored kit; a `HostCode` protagonist rebuilds its
code kit on re-wear/restore, closing the former gap. Tests:
`runtime_rewear_to_a_host_code_protagonist_rebuilds_the_code_kit`,
`runtime_rewear_to_an_unknown_id_is_a_defined_fallback_not_stale_state`.)

### S2 - reusable selected-character presentation binder — DONE

**Evidence:** `bind_worn_character_presentation` in
`crates/ambition_render/src/rendering/actors/mod.rs`, registered in
`PresentationVisualAnimationPlugin` (app + every demo). Reads `WornCharacter`;
installs sprite/anchor/`CharacterAnimator`/`PlayerSpriteBaseline`/`PlayerSpriteCharacter`;
rebinds on change and strips stale components; no per-character branch. Tests:
`rendering::actors::worn_binder_tests`. The `ambition_app::app::scene_setup` binder
was deleted (S3); guard `app_layer_does_not_bind_the_selected_character_sprite`.

Extract the main app's selected-character binding into reusable engine/
presentation machinery. It must:

- observe the canonical playable actor and selected character/profile;
- resolve the declared visual profile/sheet;
- attach the standard sprite, anchor, animator, baseline, and visual identity;
- update cleanly when the selected/worn character changes;
- be used by both `ambition_app` and standalone demos;
- contain no Sanic-specific branch.

Remove the duplicate app-local binder when the reusable path lands.

### S3 - narrow visual asset registration — OPEN

**Status:** the binder reuses `GameAssets` cleanly (marked fallback when absent);
narrowing per-demo loading + loading the demo's own sheet is open content/tooling,
not required for the architecture proof.

Allow a game/content pack to register only the character/entity visual assets it
uses. Do not require a one-character demo to load every Ambition boss, prop, and
parallax asset.

At minimum prove composition of:

- the native Sanic visual;
- one reused Ambition character visual;
- shared generic animation machinery.

If the existing `GameAssets` resource can support this cleanly, reuse it. If it
forces all-game loading or app ownership, extract a narrower character visual
library. Do not create a second parallel asset system.

### S4 - deterministic asset provisioning — OPEN

**Status:** native Sanic sheet provisioning is the remaining content/tooling item;
the demo draws a deterministic MARKED fallback (`PlayerSpriteCharacter` records the
id), not a silent box.

Choose and document one canonical policy:

- commit generated runtime assets;
- generate them in a setup/build command;
- or ship a small checked-in acceptance fixture.

Startup must never silently render a box for a declared character. Missing
catalog row, manifest, image, or animation must produce an actionable diagnostic
naming the character and exact expected paths/generation command.

### S5 - multi-character acceptance — DONE (architecture) / OPEN (native 2nd profile)

**Evidence:** two profiles on each half of the ONE path, no engine edits —
presentation binds `robot` vs `goblin` (`worn_binder_tests`), gameplay derives
`sanic` vs `player` (`avatar::starting_character::tests`), and the full kit
(ActionSet + moveset) follows a known↔known re-wear
(`worn_kit_fully_follows_a_known_character_rewear`). The SHIPPED demo genuinely uses
Sanic GAMEPLAY, not just the name: it rides `SurfaceMomentum` + arms ball dash
(`persona_architecture::the_demo_body_rides_surface_momentum_and_arms_ball_dash`) AND
wears Sanic's authored PEACEFUL kit — no melee/ranged/special, empty moveset — proving
the visual/default identity no longer forces Ambition's protagonist combat
(`persona_architecture::the_demo_body_wears_the_authored_peaceful_kit_not_the_host_protagonist_kit`).
A native second Sanic profile (reused Ambition visual) is authoring work gated on
S4 — `OPEN`.

Sanic must define at least two playable profiles through the same public path:

1. Sanic's native visual + speedster movement + ball-dash actions;
2. a reused Ambition visual + a different Sanic movement/action profile.

Selecting either profile must change the actor's visual and gameplay data without
editing engine code. This is the proof that the shell is a game composition seam,
not a one-character special case.

### S6 - remove the temporary success mask — PARTIAL

**Status:** the fallback is now binder-owned and always records the worn identity on
`PlayerSpriteCharacter` — an explicit marked degraded state, not a silent mask. A
louder missing-sheet diagnostic rides with S4.

The fallback rectangle may remain only as an explicit degraded/error presentation
with a clear diagnostic. Tests and documentation must not count it as successful
character rendering.

## Acceptance

Running:

```bash
cargo run -p ambition_demo_sanic_app --features visible --bin sanic_demo -- --window
```

must provide:

- responsive keyboard/gamepad input through the standard host bridge;
- visible movement of the canonical actor;
- actual selected character art and animation, not a rectangle;
- camera follow;
- looping `you_are_too_slow` music;
- no missing selected-character asset errors.

Headless acceptance must prove the same selected gameplay profile and input-slot
path without renderer/window/audio dependencies.

The demo shell remains small and contains no direct sprite binding, app-local
input adapter, or dependency on `ambition_app`.

## Relationship to unified encounters

The immediate input/visual recovery is independent and should land first. Later,
Sanic becomes an acceptance customer of
[`../engine/encounter-orchestration.md`](../engine/encounter-orchestration.md):
its race/chase/timed sections and mini-boss must use the generic encounter
orchestrator rather than boss-specific machinery.
