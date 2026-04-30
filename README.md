# Ambition

**Ambition** is a code-first, assetless movement sandbox for a future mathematical AI-first Metroidvania/platformer.

The reusable layer is the **Ambition Engine**. The current playable binary is **Ambition: Tangent Space Sandbox**, a single room meant to test an endgame movement kit before story, art, levels, or procedural generation get layered on top.

The first design law is: **the game should be fun as raw collision boxes**.

## What is in this prototype?

- A Rust Cargo workspace.
- `ambition_engine`: deterministic gray-box platformer movement/collision logic.
- `ambition_sandbox`: a Macroquad renderer/input/audio loop for a single generated room.
- No sprites, textures, tilemaps, imported audio, or prerendered assets.
- A larger generated room than v0.1: solids, one-way shelves, hazard channels, pogo orbs, rebound pads.
- Endgame-style movement verbs: run, jump, double jump, variable jump height, wall jump, dash, pogo, slash/recoil, rebound pads.
- Debug overlay: velocity, grounded/walled state, dash and air-jump availability, coyote/jump-buffer timers, combo algebra trace.
- Four keyboard presets with `F9`/`F10` preset cycling.
- Two easy-to-reach bashable dummies near spawn: one infinite-health sandbag and one finite-health respawning drop dummy.
- Feedback: generated sound effects, hitstop, dummy hit-stun, hit flash, impact rings, and a small procedural particle system.
- Full sandbox restart: reset now restores player, enemies, particles, hitstop, slash preview, and transient effects.

## Install requirements

Install Rust from <https://rustup.rs/>.

This project depends on Macroquad for the tiny graphics/input/audio shell.

## Run

From this directory:

```bash
cargo run -p ambition_sandbox --release
```

The first build will download and compile Macroquad. After that, rebuilds should be much faster.

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

## Sound and particles

All sound effects are generated at startup from compact synth recipes and loaded through Macroquad audio. There are no audio files in the repo. The approach mirrors the earlier pygame experiment: symbolic sound specs render to PCM/WAV data, then the runtime plays the cached/generated sound object.

The particle system is deliberately tiny and code-first. It uses draw primitives only: sparks, dust, shards, and rings. It is not a reason to upgrade frameworks yet; Macroquad is enough for this prototype. If Ambition later needs thousands of GPU particles, collision-aware particle fields, or shader-driven effects, that would be the point to consider a Bevy/wgpu renderer.

## Movement tuning

Movement constants now live behind `MovementTuning` / `DEFAULT_TUNING` in `ambition_engine`. The public `update_player` function uses the default tuning, while `update_player_with_tuning` exists for later per-character, per-room, or experimental tuning passes.

This pass is a little snappier than the previous one: stronger acceleration/deceleration, slightly faster dash, one air jump, pogo/rebound refreshes, and impact hitstop when attacks land.

## Design target

The sandbox is intentionally an endgame lab, not a first level. The question it asks is:

> If the player had the full movement kit, would this still be fun after 100 hours?

The current answer is only a sketch, but the code is structured to make iteration fast.

## Next good changes

1. Add real user-editable keybinding config, likely RON/TOML.
2. Add gamepad input once the keyboard presets feel right.
3. Add sloped collision / continuous rebound normals.
4. Add user-visible tuning sliders/hotkeys for gravity, run speed, dash speed, and jump speed.
5. Add a grappling/tether primitive.
6. Add an input recorder and ghost replay.
7. Add a deterministic seed format for room generation.
8. Add a procedural music layer using the same generated-audio pattern as SFX.
9. Add automated reachability tests for generated rooms.

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
    input_model.md
    audio_particles.md
    ai_generation_contract.md
```

## Audio note

The sandbox enables Macroquad's optional `audio` feature so generated SFX play at runtime. If you see `macroquad's "audio" feature disabled`, check `crates/ambition_sandbox/Cargo.toml` and ensure Macroquad is declared with `features = ["audio"]`.
