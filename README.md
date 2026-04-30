# Ambition

**Ambition** is a code-first, assetless movement sandbox for a future mathematical AI-first Metroidvania/platformer.

The reusable layer is the **Ambition Engine**. The current playable binary is **Ambition: Tangent Space Sandbox**, a single room meant to test an endgame movement kit before story, art, levels, or procedural generation get layered on top.

The first design law is: **the game should be fun as raw collision boxes**.

## What is in this prototype?

- A Rust Cargo workspace.
- `ambition_engine`: deterministic gray-box platformer movement/collision logic.
- `ambition_sandbox`: a Macroquad renderer/input loop for a single generated room.
- No sprites, textures, tilemaps, imported audio, or prerendered assets.
- A larger generated room than v0.1: solids, one-way shelves, hazard channels, pogo orbs, rebound pads.
- Endgame-style movement verbs: run, jump, variable jump height, wall jump, dash, pogo, slash/recoil, rebound pads.
- Debug overlay: velocity, grounded/walled state, dash availability, coyote/jump-buffer timers, combo algebra trace.
- Four keyboard presets with `F9`/`F10` preset cycling.
- Two easy-to-reach bashable dummies near spawn: one infinite-health sandbag and one finite-health respawning drop dummy.

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

The sandbox treats keyboard layouts as presets that map onto semantic actions and a future console/gamepad layout.

`F9` cycles to the previous preset. `F10` cycles to the next preset.

| Preset | Movement | Jump | Attack | Dash | Pogo |
|---|---|---|---|---|---|
| Hollow Knight default | Arrow keys | `Z` | `X` | `C` | Down + `X` |
| Custom PC | `WASD` | `Space` | `J` | `K` | Down + `J` |
| Chirality A | Arrow keys | `Q` | `E` | `W` | `R` or Down + `E` |
| Chirality B | `WASD` | `U` | `P` | `I` | `O` or Down + `P` |

Universal controls:

| Input | Action |
|---|---|
| `Escape` | Start: pause/freeze |
| `Delete` or `Backspace` | Select/sandbox reset |
| `F1` | Toggle debug overlay |
| `F2` | Toggle slow motion |
| `F9` / `F10` | Previous / next control preset |

Planned gamepad mapping:

| Gamepad control | Semantics |
|---|---|
| L-stick / D-pad | Movement |
| A / Cross | Jump / confirm |
| X / Square | Attack / slash; Down+Attack is pogo |
| RT / R2 | Dash |
| B / Circle | Focus / cast placeholder |
| RB / R1 | Quick cast placeholder |
| LT / L2 | Super dash placeholder |
| Y / Triangle | Dream nail placeholder |
| LB / L1 | Quick map placeholder |
| Back / Touchpad | Inventory/select; sandbox reset for now |
| Start / Options | Pause / menu |

## Design target

The sandbox is intentionally an endgame lab, not a first level. The question it asks is:

> If the player had the full movement kit, would this still be fun after 100 hours?

The current answer is only a sketch, but the code is structured to make iteration fast.

## Next good changes

1. Add real user-editable keybinding config, likely RON/TOML.
2. Add gamepad input once the keyboard presets feel right.
3. Add sloped collision / continuous rebound normals.
4. Add a grappling/tether primitive.
5. Add an input recorder and ghost replay.
6. Add a deterministic seed format for room generation.
7. Add procedural sound effects generated from symbolic envelopes.
8. Add automated reachability tests for generated rooms.

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
