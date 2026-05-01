# Progression log

This log is a compact map of major architecture and feature waves. It is not a full changelog.

## Early direction

- Ambition began as a code-first platformer/metroidvania movement sandbox.
- The first design law became: the game should be fun as raw collision boxes.
- The story premise centered on an AI-like player discovering agency, embodiment, mathematics, collaboration, and ethical compromise.

## Bevy and engine split

- The project moved from earlier prototypes into a Bevy 0.18 sandbox.
- The engine was initially treated as backend-neutral, but that constraint was later relaxed.
- Current decision: `ambition_engine` may depend on Bevy and Bevy-adjacent crates when they provide robust primitives.

## Data-driven rooms and generated audio

- Room layout, ability flags, movement tuning, audio specs, and room transitions moved toward RON.
- Generated audio shifted toward FunDSP-backed startup WAV rendering.
- The canonical sandbox asset path became `crates/ambition_sandbox/assets/ambition/sandbox.ron`.

## Input, pause, and game modes

- Leafwing Input Manager replaced bespoke keyboard polling.
- Gameplay input is converted into `ControlFrame` only when the relevant game mode allows it.
- Bevy `GameMode` states now distinguish playing, paused, dialogue, room transition, and cutscene intent.
- Presentation/debug previews were also gated so paused input does not visually leak into gameplay previews.

## Interaction/hazard/actor skeleton

- Engine-side vocabulary was added for actors, health, damage volumes, hitboxes, hurtboxes, interactables, pickups, chests, breakables, respawn policies, kinematic paths, enemy/boss placeholders, and debug/destination labels.
- Rooms now distinguish collision blocks from authored gameplay objects.

## Feature basement wave

- The central hub gained a basement feature wing.
- Focused rooms test hazards, enemies, boss patterns, breakables, treasure/pickups, and NPC dialogue hooks.
- Loading zones render destination labels for debugging.
- This is a proving ground, not final content.

## Input-feel pass

- Existing jump buffer and coyote time were preserved.
- Dash buffering and sandbox-side interaction buffering were added for more forgiving feel.
- HUD/inspector visibility improved for buffer tuning.

## Crate foundation pass

- `seldom_state` was added as the intended per-entity state-machine foundation.
- `bevy_asset_loader` was added as a conservative asset-collection/loading foundation while preserving embedded fallback startup.
- `insta` and `proptest` were added for lightweight snapshots and property tests.

## Documentation source-of-truth pass

- README was rewritten as a stable project portal.
- `CURRENT_STATE.md`, `GOAL_STATE.md`, `AGENT_HANDOFF.md`, ADRs, and story/mode notes were added so future agents can distinguish current truth from historical notes.
