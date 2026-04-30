# Ambition

**Ambition** is a code-first, assetless movement sandbox for a future mathematical AI-first Metroidvania/platformer.

The reusable layer is the **Ambition Engine**. The current playable binary is **Ambition: Tangent Space Sandbox**, a single room meant to test an endgame movement kit before story, art, levels, or procedural generation get layered on top.

The first design law is: **the game should be fun as raw collision boxes**.

## What is in this prototype?

- A Rust Cargo workspace.
- `ambition_engine`: deterministic gray-box platformer movement/collision logic.
- `ambition_sandbox`: a Macroquad renderer/input loop for a single generated room.
- No sprites, textures, tilemaps, imported audio, or prerendered assets.
- Generated geometry: solids, one-way shelves, hazard channels, pogo orbs, rebound pads.
- Endgame-style movement verbs: run, jump, variable jump height, wall jump, dash, pogo, slash/recoil, rebound pads.
- Debug overlay: velocity, grounded/walled state, dash availability, coyote/jump-buffer timers, combo algebra trace.
- Two chiral keyboard presets that map onto gamepad-style A/B/Y/X action semantics.
- Two bashable dummies: one infinite-health sandbag and one finite-health respawning drop dummy.

## Install requirements

Install Rust from <https://rustup.rs/>.

This project depends on Macroquad for the tiny graphics/input shell.

## Run

From this directory:

```bash
cargo run -p ambition_sandbox --release
```

The first build will download and compile Macroquad. After that, rebuilds should be much faster.

## Controls

The sandbox now treats keyboard layouts as presets that map onto gamepad-style face-button semantics.

| Preset | Movement | Face buttons | Switch |
|---|---|---|---|
| Right-hand movement | Arrow keys | `Q` = A/jump, `W` = B/dash, `E` = Y/slash, `R` = X/pogo | `F9` |
| Left-hand movement | `WASD` | `U` = A/jump, `I` = B/dash, `P` = Y/slash, `O` = X/pogo | `F10` |

Universal controls:

| Input | Action |
|---|---|
| `Escape` | Start: pause/freeze |
| `Delete` or `Backspace` | Select/sandbox reset |
| `F1` | Toggle debug overlay |
| `Tab` | Toggle slow motion |

Planned gamepad mapping:

| Gamepad control | Semantics |
|---|---|
| South / A | Jump / confirm |
| East / B | Dash / cancel |
| North / Y | Slash / attack |
| West / X | Dedicated downward/pogo slash / alternate attack |
| Start | Pause / menu |
| Select / Back | Sandbox reset |
| LB/RB, LT/RT | Reserved placeholders for future chord/stance/analog modifiers |

## Design target

The sandbox is intentionally an endgame lab, not a first level. The question it asks is:

> If the player had the full movement kit, would this still be fun after 100 hours?

The current answer is only a sketch, but the code is structured to make iteration fast.

## Next good changes

1. Add sloped collision / continuous rebound normals.
2. Add a grappling/tether primitive.
3. Add an input recorder and ghost replay.
4. Add a deterministic seed format for room generation.
5. Add procedural sound effects generated from symbolic envelopes.
6. Add a tiny internal DSL for movement theorems.
7. Add automated reachability tests for generated rooms.

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
    ai_generation_contract.md
```
