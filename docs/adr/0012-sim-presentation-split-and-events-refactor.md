# ADR 0012: Sim/presentation split, events refactor, and bevy_rl hook target

## Status

Accepted.

## Context

The sandbox's gameplay loop (`sandbox_update` and its helpers) directly
calls audio, particle, and physics-debris APIs from inside simulation
systems. `play_sound`, `spawn_burst`, `spawn_dust`, `spawn_impact`,
`spawn_blink_effects`, `physics::spawn_debris_burst` are all invoked
inline as gameplay events fire. This means the simulation cannot run on
machines without a display, audio device, or GPU — `Assets<AudioSource>`
isn't registered without `AudioPlugin`, particle entities require Sprite
plugins, etc.

This blocks three goals simultaneously:

1. **Headless / CI testing** — running the game on a no-display VM panics.
2. **RL agent integration** — RL drivers need deterministic stepping
   without rendering/audio/window plugins. `bevy_rl` is the canonical
   ecosystem crate for this and expects a clean simulation/presentation
   boundary.
3. **Multi-player support** *(future)* — server-authoritative simulations
   shouldn't render, and clients shouldn't simulate authoritatively. The
   same boundary that enables headless enables this.

Phase 1 of the headless work landed the library/binary split: `lib.rs`
owns the module graph, `bin/headless.rs` calls a `run_headless` that
builds an App with `MinimalPlugins` and ticks runtime-spine systems
without panicking. Phase 1 deliberately skips `sandbox_update` because
the simulation is still presentation-coupled. This ADR records the Phase
2/3 plan and the bevy_rl hook target.

This ADR also reinforces the "use existing professional packages"
principle (per the user-feedback memory): Phase 3 RL adapter targets
`bevy_rl` rather than a custom RL API. The ADR exists in part to ensure
Phase 2 produces a shape that `bevy_rl` can plug into cleanly.

## Decision

### Sim/presentation boundary

Simulation systems must not directly call presentation APIs. Side effects
flow through typed Bevy `Event`s. Presentation systems (audio, VFX, HUD,
debug overlays) subscribe via `EventReader`. Headless builds omit the
subscriber plugins; events accumulate harmlessly and are drained per tick
(or are handled by RL observation extractors).

```text
Simulation
  - reads ControlFrame, World, Player, ProperTime, Resources
  - writes Resources, Components
  - emits typed Events (SfxEvent, VfxEvent, PhysicsBurstEvent, etc.)
  - never calls audio/render/spawn APIs directly

Presentation (visible binary only)
  - subscribes to typed Events
  - performs the actual audio/VFX/physics-debris/HUD work
  - reads sim Resources to render

Headless (RL, CI, replay)
  - subscribes to no presentation events (or to a typed-observation
    aggregator)
  - sim runs identically; events are drained or observed
```

### Phase 2 — events refactor (the bulk of this ADR's implementation work)

Migrate every direct presentation call inside simulation systems to event
emission:

- `play_sound(commands, bank, SoundCue::X)` →
  `events.send(SfxEvent::X { pos })`. An audio plugin reads `SfxEvent`
  and performs the actual playback.
- `spawn_burst(...)`, `spawn_dust(...)`, `spawn_impact(...)`,
  `spawn_blink_effects(...)`, `spawn_slash_preview(...)` →
  `VfxEvent` variants consumed by the fx plugin.
- `physics::spawn_debris_burst(...)` → `DebrisBurstEvent`; the Avian2D
  debris plugin handles spawning bodies.
- `update_hud` stays in presentation as it already only reads resources.
- `setup`'s presentation responsibilities (Camera2d, player Sprite, room
  visuals, HUD Text, SoundBank, ambience playback) split into a
  `setup_presentation` startup system that runs only in the visible
  binary.

After Phase 2, `add_simulation_plugins(app)` and `add_presentation_plugins(app)`
become honest, non-overlapping App-builder helpers. The visible binary
calls both; `bin/headless` calls only the simulation half and gets a
working gameplay loop.

### Phase 3 — bevy_rl integration

Once Phase 2 lands, `bevy_rl` integration becomes a thin layer:

- **Action injection** — `bevy_rl` provides the action space; an adapter
  produces `ControlFrame` per tick from the action vector. The existing
  `ControlFrame` boundary already supports this; no engine change needed.
- **Observation extraction** — typed Rust struct constructed from the
  same Resources presentation reads (`SandboxRuntime`, `GameWorld`,
  `LdtkRuntimeSolidIndex`, etc.). A Phase 1 `HeadlessReport` is the
  proto-shape; Phase 3 expands it with player position, velocity,
  health, room-relative coordinates of nearby entities, etc.
- **Reward signal** — typed events from the sim (damage taken, room
  transition, boss defeated) are exactly the right shape for reward
  computation. The events refactor produces these for free.
- **Determinism** — under the `RLDeterministic` regime (ADR 0010), no
  time-scale requests are granted; with seeded RNG and fixed timestep,
  the sim is reproducible.

`bevy_rl` itself is **not adopted in this ADR**. This is a hooks-now,
deep-later commitment: Phase 2 produces the right shape; the actual
`bevy_rl` evaluation and integration is a future patch. Documenting the
target here ensures Phase 2 work doesn't accidentally close the door.

### bevy_dev_tools

`bevy_dev_tools` (Bevy ecosystem dev-time overlays, FPS counter, etc.)
is a candidate for adoption alongside the visible binary's existing
inspector/debug tools. Evaluation deferred — this ADR notes the target,
not the integration.

## Consequences

- Phase 2 is a substantial multi-file refactor. Every direct
  presentation call inside simulation needs an event variant and a
  subscriber. The work is mechanical but broad.
- After Phase 2, `cargo run --bin headless` exercises the actual gameplay
  loop. Tests can drive the sim with scripted `ControlFrame` sequences
  and assert on observation/event streams — real integration testing
  without GUI scaffolding.
- The `pub` widening on `SandboxRuntime` fields done in Phase 1 can
  tighten back: Phase 2 moves the gameplay-update reads into
  library-internal systems, so binary-side access is no longer needed.
- The "events as gameplay vocabulary" pattern composes with ADR 0010's
  `ClockScaleRequest` model: time-control is *itself* a typed request
  flowing through the same machinery.
- bevy_rl integration becomes a small additive patch when ready, not a
  large refactor.
- Compile-time consideration (per the compile-time discipline memory):
  events add types but not heavy macros. Each event is a small struct;
  the cost is bounded.

## Initial implementation target

Phase 2, in slices to keep each PR shippable:

1. **Audio events.** Define `SfxEvent` and an audio plugin that subscribes.
   Migrate every `play_sound(...)` call site to `events.send(...)`.
   The audio plugin is added only in the visible binary.
2. **VFX events.** Define `VfxEvent` (Burst, Dust, Impact, Blink,
   SlashPreview, ResetEffects). Migrate `fx::spawn_*` call sites.
   Presentation-side fx plugin subscribes.
3. **Physics-debris events.** Define `DebrisBurstEvent`. Migrate
   `physics::spawn_debris_burst` callers. Avian2D debris plugin
   subscribes (visible binary only).
4. **Setup split.** Extract `setup_simulation` (resources + LdtkWorldBundle
   spawn) from `setup_presentation` (Camera2d, Sprite, HUD, SoundBank,
   ambience).
5. **App-builder split.** `add_simulation_plugins(app)` and
   `add_presentation_plugins(app)` registered uniformly. `run_headless`
   uses only the simulation side; `main.rs` (visible) uses both.

After all five slices, `run_headless` invokes the actual gameplay loop
and the Phase 2 commitment is met.

## Non-goals for the first implementation

- bevy_rl adoption. Documented as the target; integration is a future
  patch.
- bevy_dev_tools adoption. Same.
- Network synchronization, server-authoritative architecture, lockstep
  determinism. The events boundary is multi-player-friendly, but
  building the multi-player driver is out of scope.
- Migrating every Bevy `Time::delta_secs()` read in gameplay code to a
  domain-tagged accessor. The discipline starts with mutations (ADR 0010);
  reads migrate as systems evolve.
- Replacing the existing custom player controller with anything driven
  by Avian2D. The boundary lets that be a future decision; this ADR
  doesn't make it.

## Review notes

- Each Phase 2 slice should land with at least one new `cargo test` that
  exercises the sim path headless (e.g., a test that ticks the app for
  N frames and asserts on emitted events). This is the regression check
  that the slice didn't accidentally re-couple presentation.
- `cargo build --timings` after each slice; events themselves are cheap
  but adding many small Event types could surprise compile time. Audit
  if it does.
- Cross-references: ADR 0010 (regime policies — events flow through the
  policy machinery), ADR 0011 (per-entity proper time — sim systems
  consuming proper time read it like any other component), and the
  headless-simulation doc (`docs/headless_simulation.md`) which already
  describes Phase 2 informally.
- Use `AMBITION_REVIEW(spatial)` near any event whose payload encodes
  positions/velocities — these are easy to get wrong relative to the
  active room's coordinate frame.
