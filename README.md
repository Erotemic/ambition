# Ambition

**Ambition** is a code-first, assetless movement sandbox for a future mathematical AI-first Metroidvania/platformer.

The reusable layer is the **Ambition Engine**. The current playable binary is **Ambition: Tangent Space Sandbox**, a single room meant to test an endgame movement kit before story, art, levels, or procedural generation get layered on top.

The first design law is: **the game should be fun as raw collision boxes**.

## What is in this prototype?

- A Rust Cargo workspace.
- `ambition_engine`: backend-neutral simulation logic: math, AABB collision, generated room blocks, player movement, combo traces, and sandbox dummy/enemy state.
- `ambition_sandbox`: a Bevy 0.18 ECS app for a single generated room.
- No sprites, textures, tilemaps, imported audio, or prerendered assets.
- A generated room: solids, one-way shelves, hazard channels, pogo orbs, and rebound/impulse pads.
- Endgame-style movement verbs: run, jump, double jump, variable jump height, wall jump, dash, pogo, slash/recoil, rebound pads.
- Debug overlay: velocity, grounded/walled state, dash and air-jump availability, coyote/jump-buffer timers, combo algebra trace.
- Four keyboard presets with `F9`/`F10` preset cycling.
- Two easy-to-reach bashable dummies near spawn: one infinite-health sandbag and one finite-health respawning drop dummy.
- Feedback: generated sound effects, hitstop, dummy hit-stun, hit flash, impact rings, and a small procedural particle system.
- Full sandbox restart: reset restores player, enemies, particles, hitstop, slash previews, and transient effects.
- Draft storyline documentation under `docs/storylines/`, with the current primary arc focused on AI agency, embodiment, mathematics, collaboration, and ethical compromise.

## Install requirements

Install Rust from <https://rustup.rs/>.

This project depends on Bevy for the graphics/input/audio shell. The movement/collision core remains in `ambition_engine`.

## Run

From this directory:

```bash
cargo run -p ambition_sandbox --release
```

The first build will download and compile Bevy. After that, rebuilds should be much faster.

## Controls

The sandbox treats keyboard layouts as presets that map onto semantic actions and a future console/gamepad layout.

`F9` cycles to the previous preset. `F10` cycles to the next preset.

| Preset | Movement | Jump | Attack | Dash | Pogo |
|---|---|---|---|---|---|
| Classic action | Arrow keys | `Z` | `X` | `C` | Down + `X` |
| Custom PC | `WASD` | `Space` | `J` | `K` | Down + `J` |
| Chirality A | Arrow keys | `Q` | `E` | `W` | `R` or Down + `E` |
| Chirality B | `WASD` | `U` | `P` | `I` | `O` or Down + `P` |

Universal controls:

| Input | Action |
|---|---|
| `Escape` | Start: pause/freeze |
| `Delete` or `Backspace` | Select/full sandbox restart |
| `F1` | Toggle debug overlay |
| `F2` | Toggle slow motion |
| `F9` / `F10` | Previous / next control preset |

Planned gamepad mapping:

| Gamepad control | Semantics |
|---|---|
| L-stick / D-pad | Movement |
| A / Cross | Jump / confirm |
| X / Square | Primary attack; Down+Attack is pogo |
| RT / R2 | Dash |
| B / Circle | Secondary action placeholder |
| RB / R1 | Quick action placeholder |
| LT / L2 | Modifier placeholder |
| Y / Triangle | Utility action placeholder |
| LB / L1 | Map placeholder |
| Back / Touchpad | Inventory/select; sandbox restart for now |
| Start / Options | Pause / menu |

## Storyline drafts

Narrative notes live in `docs/storylines/`. This is intentional: Ambition may support multiple storylines, and the Ambition Engine should remain generic enough to express different arcs through data, world-state transforms, generated rooms, dialogue/events, and ability framing.

The current primary draft is `docs/storylines/primary_ai_agency.md`. It preserves the core concept: the player is an AI-like entity discovering agency through movement, embodiment, human collaboration, mathematical theorems, and ethical funding choices.

## Sound and particles

All sound effects are generated at startup from compact synth recipes and registered as Bevy `AudioSource` assets. There are no audio files in the repo. Playback spawns `AudioPlayer` entities with `PlaybackSettings::DESPAWN` for one-shot cleanup.

The particle system is deliberately tiny and code-first. Each particle is a Bevy sprite entity with position, velocity, lifetime, color, gravity, drag, and kind. This is enough for movement feedback. If Ambition later needs massive GPU particle fields or shader-driven visual effects, the next step is a Bevy GPU particle backend or custom wgpu pipeline.

## Movement tuning

Movement constants live behind `MovementTuning` / `DEFAULT_TUNING` in `ambition_engine`. The public `update_player` function uses the default tuning, while `update_player_with_tuning` exists for later per-character, per-room, or experimental tuning passes.

## Bevy port notes

This version ports the sandbox shell from Macroquad to Bevy 0.18.1. The architecture is now split across Bevy resources, components, and systems:

- `GameWorld`: generated room data from Ambition Engine.
- `SandboxRuntime`: player, enemies, presets, pause/slowmo/debug state, hitstop.
- Sprite entities: blocks, player, dummies, particles, impacts, slash previews.
- Bevy audio assets: generated WAV bytes stored as `AudioSource` handles.
- UI text: fixed debug HUD.

The movement engine intentionally stays backend-neutral.

## Design target

The sandbox is intentionally an endgame lab, not a first level. The question it asks is:

> If the player had the full movement kit, would this still be fun after 100 hours?

## Next good changes

1. Add user-editable keybinding config, likely RON/TOML.
2. Add gamepad input now that Bevy is the backend.
3. Add sloped collision / continuous rebound normals.
4. Add user-visible tuning sliders/hotkeys for gravity, run speed, dash speed, and jump speed.
5. Add a grappling/tether primitive.
6. Add an input recorder and ghost replay.
7. Add a deterministic seed format for room generation.
8. Add procedural music using the same generated-audio pattern as SFX.
9. Add a backend-neutral story/world-state event log.
10. Add automated reachability tests for generated rooms.

## Workspace layout

```text
ambition/
  Cargo.toml
  crates/
    ambition_engine/
      src/lib.rs
    ambition_sandbox/
      src/main.rs
  docs/
    endgame_sandbox.md
    ability_system.md
    testing_strategy.md
    input_model.md
    audio_particles.md
    ai_generation_contract.md
    storylines/
      README.md
      primary_ai_agency.md
```

## Architecture notes

- `docs/code_structure.md` tracks the current module split and remaining hard-coded areas.
- `docs/ability_system.md` describes optional movement/combat upgrades.
- `docs/testing_strategy.md` describes the intended automated-test layers.

## Engine architecture

The Bevy executable is the presentation/runtime layer. `ambition_engine` is the backend-neutral simulation core. See `docs/engine_architecture.md` for the current module split and migration rules.

## Current mechanic notes

- `docs/blink_and_fastfall.md` documents the current precision blink, blink-through wall, bullet-time ramp, and double-tap fast-fall behavior.
