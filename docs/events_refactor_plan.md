# Events refactor plan (ADR 0012 Phase 2)

This document is the implementation roadmap for the events refactor. ADR
0012 is the durable decision; this plan can evolve as we learn. It is the
companion to `docs/headless_simulation.md` (which describes Phase 1 and the
Phase 2/3 target shape) and ADR 0012 (which records the architectural
commitment).

## Goal

After this refactor:

- `sandbox_update` and its helpers contain zero direct calls to
  `play_sound`, `spawn_burst`, `spawn_dust`, `spawn_impact`,
  `spawn_blink_effects`, `spawn_slash_preview`, `spawn_reset_effects`, or
  `physics::spawn_debris_burst`. They emit typed events.
- `add_simulation_plugins(app)` and `add_presentation_plugins(app)` are
  honest, non-overlapping App-builder helpers.
- `bin/headless` runs the actual gameplay loop (currently it skips
  `sandbox_update`).
- `pub` widening on `SandboxRuntime` fields tightens back to `pub(crate)`.
- An integration test drives the sim with scripted `ControlFrame`
  sequences and asserts on emitted events.

## Testing strategy — minimal-plugin App + World assertions

All slice tests adopt the Bevy minimal-plugin testing pattern (see the
`feedback_bevy_testing_pattern` memory note for the canonical idiom):

```rust
#[test]
fn slash_emits_sfx_event() {
    let mut app = App::new();
    app.add_plugins(MinimalPlugins);
    app.add_plugins(AssetPlugin::default());
    app.init_state::<GameMode>();
    app.add_event::<SfxEvent>();
    app.insert_resource(/* the minimal SandboxRuntime needed */);
    app.add_systems(Update, sandbox_update);

    // Drive scripted input.
    app.world_mut().resource_mut::<ActionState<SandboxAction>>()
        .press(&SandboxAction::Attack);
    app.update();

    // Assert via World.
    let events = app.world().resource::<Events<SfxEvent>>();
    let count = events.iter_current_update_events()
        .filter(|e| matches!(e, SfxEvent::Slash { .. }))
        .count();
    assert_eq!(count, 1);
}
```

Key properties of this pattern as it applies to the events refactor:

- Tests load only `MinimalPlugins` + `AssetPlugin` + `StatesPlugin`, plus
  whatever specific systems and event types are under test. No
  rendering, audio, windowing, or input plugins. Each `app.update()` is
  fast (milliseconds).
- Inputs are injected via `app.world_mut()`-level mutations (resources,
  events, components on spawned entities) or by setting `ActionState`
  values directly. `InputManagerPlugin` is **not** needed for tests
  that bypass keyboard.
- Assertions read `app.world().resource::<T>()` and
  `app.world().get_entity(id)`. Entity IDs captured at spawn time stay
  stable across `app.update()` calls.
- Use a small test-scoped macro for repeated resource access; ownership
  rules prevent keeping `&Resource` borrows across `app.update()`:

  ```rust
  macro_rules! runtime { ($app:expr) => { $app.world().resource::<SandboxRuntime>() } }
  ```

- Each slice's call-site migration is verified by a small per-slice test
  (driving the relevant helper directly with synthetic inputs, asserting
  on the `Vec<SfxEvent>` contents). System-level tests run the full
  `sandbox_update` with a minimal App.

The integration test for the whole refactor lives at
`crates/ambition_sandbox/tests/scripted_gameplay.rs` and follows the
same pattern at a slightly larger scope: a sequence of `ControlFrame`s
applied across multiple `app.update()` calls, asserting on the full
event timeline.

This is also the testing pattern Phase 3's `bevy_rl` adapter inherits:
RL training rolls out the same minimal App with deterministic input and
reads observation/reward from the same World state.

## Threading model — Vec collector pattern

Helper functions today call presentation APIs inline through `&mut
Commands`. The migration target uses a Vec collector pattern:

```rust
fn sandbox_update(
    /* existing params */,
    mut sfx_writer: EventWriter<SfxEvent>,
    mut vfx_writer: EventWriter<VfxEvent>,
    mut debris_writer: EventWriter<DebrisBurstEvent>,
) {
    let mut sfx = Vec::new();
    let mut vfx = Vec::new();
    let mut debris = Vec::new();

    handle_player_events(&mut sfx, &mut vfx, /* ... */);
    handle_feature_events(&mut sfx, &mut vfx, &mut debris, /* ... */);
    process_attack(&mut sfx, &mut vfx, &mut debris, /* ... */);

    sfx_writer.send_batch(sfx);
    vfx_writer.send_batch(vfx);
    debris_writer.send_batch(debris);
}
```

Helpers accept `&mut Vec<SfxEvent>` (etc.); the calling system drains the
Vecs into `EventWriter`s at the end. Trade-offs:

- **Pro**: clean signatures, no borrow-checker fights through deeply
  nested helper calls.
- **Pro**: helpers become pure-ish (input → events), trivially testable
  without a Bevy World.
- **Pro**: many helpers can drop `&mut Commands` entirely.
- **Con**: per-frame allocation. Negligible; events are small structs
  and totals are bounded.
- **Con**: helper signatures change. Real churn but mechanical.

This is the **narrow, specific** version of an event pipeline (per the
design-balance principle). A wider abstraction — generic `EventBus<T>`,
trait-based `GameEvent`, a registry — would be tech debt for marginal
flexibility. Plain `Vec<SfxEvent>` and `EventWriter<SfxEvent>` are the
right shape.

## Slices

Each slice is independently mergeable. The visible binary works after
every slice; the "headless can run gameplay" goal is gated on the whole
sequence.

### Slice 1 — Audio events (~1-2 days)

**Add** `SfxEvent` in `crates/ambition_sandbox/src/audio.rs`:

```rust
#[derive(Event, Clone, Debug)]
pub enum SfxEvent {
    Jump { pos: ae::Vec2 },
    DoubleJump { pos: ae::Vec2 },
    Dash { pos: ae::Vec2 },
    Blink { precision: bool, pos: ae::Vec2 },
    Pogo { pos: ae::Vec2 },
    Slash { pos: ae::Vec2 },
    Hit { pos: ae::Vec2 },
    Death { pos: ae::Vec2 },
    Reset { pos: ae::Vec2 },
}
```

Variants mirror the existing `SoundCue`. Carrying `pos` now sets up
future spatialized audio without another refactor.

**Add** `audio_play_sfx_events` system (presentation-side) that reads
`EventReader<SfxEvent>` and calls `play_sound`.

**Migrate** ~30 call sites in `main.rs`: `handle_player_events`,
`handle_feature_events`, `handle_player_damage_events`,
`safe_respawn_player`, `apply_player_knockback`, `death_respawn_player`,
`process_attack`, `reset_sandbox`, `load_room`. Each `play_sound(...)`
becomes `events.push(SfxEvent::X { pos })`.

**Test**: per the testing-pattern section above. Module-level helper
test driving `handle_player_events` with synthetic `FrameEvents` and
asserting on the resulting `Vec<SfxEvent>` (no App needed for the
helper-level test). Plus a system-level test using a minimal App that
drives `sandbox_update` for one tick with scripted `ActionState` and
asserts the right `SfxEvent`s appear in `Events<SfxEvent>`.

**Acceptance**: `cargo test -p ambition_sandbox` (8+ tests now), `cargo
run --bin headless 30` still clean.

### Slice 2 — VFX events (~1-2 days)

**Add** `VfxEvent` in `crates/ambition_sandbox/src/fx.rs`:

```rust
#[derive(Event, Clone, Debug)]
pub enum VfxEvent {
    Burst { pos: ae::Vec2, count: u32, speed: f32, color: [f32; 4], kind: ParticleKind },
    Dust { pos: ae::Vec2, facing: f32 },
    Impact { pos: ae::Vec2 },
    BlinkEffect { from: ae::Vec2, to: ae::Vec2, precision: bool },
    SlashPreview { attack: ae::SlashHitbox },
    ResetEffects { from: ae::Vec2, to: ae::Vec2 },
}
```

**Add** `vfx_spawn_events` system (presentation-side) that reads
`EventReader<VfxEvent>` and spawns the actual particle entities.

**Migrate** ~25 call sites of `fx::spawn_*`.

**Test**: same pattern as Slice 1.

### Slice 3 — Physics-debris events (~0.5 day)

**Add** `DebrisBurstEvent` in `crates/ambition_sandbox/src/physics.rs`:

```rust
#[derive(Event, Clone, Debug)]
pub struct DebrisBurstEvent {
    pub pos: ae::Vec2,
    pub cue: PhysicsDebrisCue,
}
```

**Add** `spawn_debris_from_events` system (presentation-side, since
Avian2D debris bodies are visual/physics objects on the visible side
for now).

**Migrate** ~5 call sites of `physics::spawn_debris_burst`.

### Slice 4 — Setup split (~1 day)

Split `setup` into:

- `setup_simulation` — `LdtkWorldBundle` (with `LdtkSettings` configured
  for headless-friendliness; see "LdtkPlugin in headless" below),
  moving platform, `SandboxRuntime`, player entity with `Transform` +
  `ActionState::default()` + `InputMap::default()` + `PlayerVisual`.
- `setup_presentation` — `Camera2d`, player `Sprite`, room-visual
  `Sprite`s, HUD `Text`, generated audio library creation, and default
  music startup.

Post-split, the generated audio library is a presentation-only resource
(only the audio subscriber and pause-menu music switcher read it).
`update_hud` already reads only resources, no change needed.

### Slice 5 — App-builder split (~1 day)

```rust
// crates/ambition_sandbox/src/lib.rs
pub fn add_simulation_plugins(app: &mut App) {
    app.add_plugins(LdtkPlugin);
    app.add_plugins(AmbitionLdtkRegistrationPlugin);
    app.add_plugins(AmbitionStateMachinePlugin::default());
    app.add_plugins(AmbitionPhysicsPlugin);
    app.add_plugins(RonAssetPlugin::<data::SandboxDataSpec>::new(&["ron"]));
    app.init_state::<GameMode>();
    app.add_event::<SfxEvent>();
    app.add_event::<VfxEvent>();
    app.add_event::<DebrisBurstEvent>();
    // sim resources
    // sim startup + update systems (sandbox_update, runtime spine, etc.)
}

pub fn add_presentation_plugins(app: &mut App) {
    app.add_plugins(DefaultPlugins);
    app.add_plugins(EguiPlugin::default());
    app.add_plugins(MaterialUiPlugin);
    app.add_plugins(InputManagerPlugin::<SandboxAction>::default());
    app.add_plugins(dialog::yarn_spinner_plugin());
    // inspector plugins
    // presentation startup + update systems (event subscribers, HUD, etc.)
}
```

`main.rs` becomes ~5 lines. `run_headless` calls `add_simulation_plugins`
only.

After this slice, `bin/headless` runs `sandbox_update`. The Phase 2
goal is met.

**Test**: integration test in
`crates/ambition_sandbox/tests/scripted_gameplay.rs` that injects a
sequence of `ControlFrame`s ("jump, dash, attack"), asserts specific
`SfxEvent` and `VfxEvent` events were emitted. This becomes the
foundational test pattern Phase 3 RL adapter will reuse.

## LdtkPlugin in headless

The concern: `bevy_ecs_ldtk`'s tile-spawning pipeline depends on
`Image` assets, which need `ImagePlugin`/`RenderPlugin` for GPU upload.
Ambition's gameplay collision is entity-based (`Solid`,
`OneWayPlatform`), not tilemap-based, so tile rendering is purely
cosmetic for us — but `LdtkPlugin` will still try to spawn tile
entities if tile layers exist in the LDtk file.

**Resolution path** (to be confirmed during Slice 1 prep):

1. Configure `LdtkSettings` to skip tile-layer spawning, OR
2. Configure `bevy_ecs_ldtk` feature flags so tile rendering is
   optional, OR
3. Strip tile layers from the LDtk file's runtime use (sandbox.ldtk
   keeps them for editor authoring but they're not loaded as Bevy
   Image assets).

Whichever works: document in `docs/headless_simulation.md` and add the
config tweak to `add_simulation_plugins`. The investigation is bounded
to an afternoon.

## Headless render-to-disk (Phase 3 follow-on, not in this refactor)

`bevy_dev_tools::EasyScreenshotPlugin` (ADR 0014) supports
"render-once-to-file" with an offscreen surface. After Slice 5 lands,
adding a `bin/headless_screenshot.rs` (or a flag on the existing
binary) gives us:

1. Visual regression testing in CI ("run scripted gameplay 5s,
   screenshot, diff against baseline").
2. Debugging headless runs by visualizing what the sim believes.
3. Vision-based RL observations down the line.

This is additive and explicitly **not** in the critical path of the
events refactor. Worth doing once Slice 5 is in.

## Risks

1. **LdtkPlugin headless config** — see above.
2. **Schedule ordering** — subscribers must run after `sandbox_update`.
   Bevy's `.after(...)` is the lever; easy to get wrong, easy to fix.
3. **Compile-time impact** — three small `Event` types should be
   negligible. Verify with `cargo build --timings` after Slice 5,
   compare against the ADR 0013 baseline.
4. **Helper signature churn** — wide PR diffs. Mitigated by per-slice
   merges and a stable Vec-collector pattern that doesn't change
   between slices.

## Order of execution

| Slice | Days | Notes |
|---|---|---|
| 1. Audio events | 1-2 | Sets the threading-model precedent |
| 2. VFX events | 1-2 | Most call sites; same shape |
| 3. Physics-debris events | 0.5 | Smallest |
| 4. Setup split | 1 | Mechanical extraction |
| 5. App-builder split | 1 | Wires it all together; LdtkPlugin question gets resolved |

**~5-7 days** of focused work total.

## Cross-references

- ADR 0012 — the durable decision behind this plan.
- `docs/headless_simulation.md` — Phase 1 (landed) and the Phase 2/3
  target shape.
- ADR 0010, 0011 — events flow alongside the time-domain vocabulary;
  no conflicts.
- ADR 0013 — compile-time discipline applies; verify with
  `cargo build --timings`.
- ADR 0014 — `bevy_dev_tools` is the source of `EasyScreenshotPlugin`
  for the headless-render follow-on.
- `feedback_design_balance` memory — narrow specific events over wide
  generic payloads; no premature abstraction layers.
