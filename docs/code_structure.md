# Ambition code structure notes

> **Status (2026-05-09):** This document began as the first post-Bevy-port
> extensibility note, but several follow-up refactor passes have landed.
> `main.rs` is now a thin shim, App-builder code lives under `src/app/`,
> engine movement is a facade over `src/movement/`, and sandbox
> `encounter`, `audio`, `music`, `input`, and `trace` are now facade modules
> over focused child files. Historical sections below are kept for archaeology;
> current authoritative descriptions live in this note,
> `docs/engine_architecture.md`, and `docs/headless_simulation.md`.

This document records the first extensibility pass after the Bevy port. The goal
was not to perfect the architecture, but to remove the most obvious hard-coded
pressure points so the sandbox can keep evolving without turning into one large
file.

## Current facade-module split status (2026-05-09)

The current large-file split policy is to keep the original public module file
(`foo.rs`) as a stable facade and move implementation into `foo/*.rs`. This
avoids overlay cleanup problems from replacing `foo.rs` with `foo/mod.rs` and
keeps existing imports stable.

- `ambition_engine::movement` now splits player state, input, ops/events,
  tuning constants, and tests under `crates/ambition_engine/src/movement/`.
- `ambition_sandbox::encounter` now splits events, registry, specs, state, and
  tests under `src/encounter/`.
- `ambition_sandbox::audio` now splits runtime playback/state from procedural
  rendering and audio tests under `src/audio/`.
- `ambition_sandbox::music` now splits the adaptive catalog, channel bank,
  director state, director systems, built-in cue spec, and tests under
  `src/music/`.
- `ambition_sandbox::input` now splits actions, presets, gameplay
  `ControlFrame`, menu input repeat/scroll state, and tests under `src/input/`.
- `ambition_sandbox::trace` now splits serializable trace model, ring buffer,
  detection/event synthesis, dump writing, Bevy systems, and tests under
  `src/trace/`.

When continuing this work, compare the old facade's public imports against the
new `pub use` surface before handoff. Missing re-exports are the easiest mistake
to make in this pattern; see `dev/benchmark-candidates/rust-questions.md`.

## What changed (first pass — historical)

`ambition_sandbox/src/main.rs` was split into focused modules:

- `config.rs` — window size, z layers, grid spacing, and world-to-Bevy coordinate conversion.
- `input.rs` — generic action names, keyboard presets, gamepad semantic mapping, and `ControlFrame`.
- `dummies.rs` — compatibility re-export for engine-owned dummy/enemy simulation.
- `audio.rs` — procedural sound specs, generated lo-fi music tracks, Kira audio library construction, channel playback, and track switching helpers.
- `fx.rs` — particles, impact rings, slash previews, and reset effects.
- `rendering.rs` — render-only Bevy components, grid/block spawning, and visual state sync.
- `main.rs` — Bevy app wiring, high-level sandbox update flow, HUD text, and attack orchestration.

This kept the behavior close to the working Bevy prototype while making
future changes more local. Subsequent passes have continued in the same
direction; `main.rs` became a thin shim that invokes
`ambition_sandbox::app::run_visible`, and the App-builder implementation was
split into `src/app/` modules so LLM/human edits can load one concern at a time.

## Engine refactor pass

`ambition_engine/src/lib.rs` was split into modules after the Bevy port so the
core crate no longer lives in one large file. See `docs/engine_architecture.md`
for the module map and migration rules.

The most important gameplay migration in this pass is that dummy/enemy target
simulation moved into `ambition_engine::enemy`. The sandbox still owns rendering,
colors, debug overlays, particles, and audio feedback, but HP, stun, knockback,
death, and respawn are now backend-neutral simulation state.

## Remaining hard-coded areas

These are intentionally still simple, but they are the next things to extract
when they become painful:

1. **Room generation**
   - Room layout is now RON-backed in the sandbox crate; engine tests use small purpose-built fixture worlds.
   - Next step: represent rooms as `SandboxSpec` or `RoomSpec` data structures so the engine can load/generated multiple rooms.

2. **Movement constants**
   - `ambition_engine` has `MovementTuning`, but the default constants are still compiled in.
   - Next step: allow a tuning preset resource and hot reload from RON/TOML.

3. **Attack definitions** — RESOLVED.
   - `slash_hitbox()` and `player_slash_hitbox()` now live in
     `ambition_engine::combat`, hitbox sizes parameterized through
     `Player` state. Sandbox calls into the engine helper.

4. **Dummy behavior** — IN PROGRESS.
   - Dummy state/HP/stun/respawn now lives in `ambition_engine::enemy`.
   - The `EnemyArchetype` matrix in `crates/ambition_sandbox/src/features.rs`
     defines tunable size/aggression bands (S/M/L), and `mob_lab` exercises
     the matrix end-to-end.
   - Next step: promote archetype tuning to engine-side `EnemySpec` once
     the matrix stabilizes through more encounter use.

5. **Audio backend**
   - The audio module renders procedural frames into Kira static sound data and plays them through typed music/SFX channels.
   - Next step: move symbolic sound specs into `ambition_audio` if the sandbox audio layer becomes shared across game crates.

6. **Particle backend**
   - Particles are CPU-side sprite entities.
   - Next step: keep the public calls (`spawn_burst`, `spawn_impact`, etc.) and swap the backend to GPU particles later if needed.

7. **Input remapping persistence**
   - Presets are code-defined.
   - Next step: make presets serializable and load user bindings from a config file.

## Design rule

The sandbox should remain a backend adapter around `ambition_engine`, not a second engine.
When adding a feature, prefer this direction:

```text
symbolic/gameplay state -> ambition_engine or data spec
backend representation   -> ambition_sandbox Bevy modules
```

That means movement, collision, story-state, and generation rules should migrate
toward backend-neutral crates over time, while `ambition_sandbox` remains the
playable Bevy shell.

## Debug overlays

The Bevy port restores debug drawing through `crates/ambition_sandbox/src/debug_overlay.rs`.
This module is deliberately presentation-only: it reads `SandboxRuntime` and `ambition_engine`
state, then draws Bevy gizmos for body boxes, velocity/facing vectors, contact normals,
attack hitboxes, dummy HP bars, room bounds, and rebound impulse arrows.

Keep this layer out of `ambition_engine`. The engine should expose deterministic state and
events; the Bevy adapter decides which vectors/boxes are useful for tuning.

## Ability-system pass

`ambition_engine` now has explicit ability flags in `abilities.rs` and backend-neutral attack hitbox helpers in `combat.rs`.

The sandbox still enables every current ability by default, but tests and future story progression can construct a player with a reduced `AbilitySet`.

New engine-owned concepts:

- optional double jump
- optional dash / double dash charges
- optional wall jump
- optional wall cling
- optional wall climb
- optional attack / pogo
- optional rebound surface interaction
- generic slash hitbox computation

The Bevy layer should keep handling presentation: particles, sounds, HUD text, and debug gizmos.

See also: [Sane maximalist subset](ability_subset.md).


## Time reference / moving platform

`crates/ambition_sandbox/src/platforms.rs` contains the current moving platform reference object. It is intentionally sandbox-side and visual-only for now, used to judge bullet-time speed. Promote it into `ambition_engine` only when moving solids become real collision participants. See `docs/time_reference_platform.md`.

## Enemy collision note

Dummies now use engine-side room collision via `Dummy::update_in_world`; see `docs/enemy_collision.md`.


## Room graph and loading-zone model

See `docs/room_graph_data_model.md`. Loading zones now distinguish automatic edge exits from press-up door interactions, which is the first step toward a serializable room graph.

## Two-clock update model

See [two_clock_simulation.md](two_clock_simulation.md) for the current split
between real-time control and scaled simulation time. This exists so precision
blink can keep responsive aiming while gravity, enemies, platforms, and effects
all slow down together.

- `docs/transition_spawn_validation.md`: explains the current transition-arrival repair layer used while rooms are still sandbox-authored.
