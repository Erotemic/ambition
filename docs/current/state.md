# Current state

This is the compact active-state document for Ambition. Update it when the current architecture or active direction changes. Keep old migration plans in `docs/archive/`, not here.

## One-sentence summary

Ambition is a Bevy-native, data-driven ECS-first 2D metroidvania/platformer sandbox with reusable mechanics crates, LDtk-authored world data, generated assets, and multi-platform runtime targets.

## Current architectural stance

```text
Authoring / generation
  LDtk worlds, asset manifests, generated music/SFX/sprites/backgrounds,
  RON/config data where it remains useful.

Bevy ECS runtime
  Components/entities/systems/messages are the main integration language.
  Sandbox systems adapt authored data into runtime entities and presentation.

Reusable crates
  ambition_engine: mechanics, geometry, collision, body modes, combat,
    projectiles, actors, state vocabularies, and tests.
  ambition_asset_manager: asset identity, source selection, platform profiles.
  ambition_sfx / ambition_sfx_bank: generated SFX IDs and runtime banks.

Playable shell
  ambition_sandbox: Bevy app composition, LDtk runtime, input/touch/controller,
  audio, UI, debug/devtools, presentation, and platform feature sets.
```

The old direction of keeping the engine backend-neutral is superseded. See ADR 0002.

## World and data ownership

LDtk is the current world/level authoring source. The old RON room-manifest direction is historical.

Current rule:

- LDtk owns areas, collision layers, loading zones, room/world spatial data, and authored level entities.
- Bevy ECS owns the runtime representation.
- `ambition_engine` owns reusable mechanics semantics.
- RON remains valid for tuning, save/settings, generated-audio specs, and other non-world data where it is still the best format.
- Agents must not hand-edit `sandbox.ldtk`; use `python -m ambition_ldtk_tools` and validation tools.

## Platform stance

Desktop, web, Android/mobile touch, controller, and Steam Deck are all active compatibility targets. iOS is deferred until macOS test hardware exists.

Platform feature work should preserve:

- keyboard/mouse and controller input,
- touch controls when the platform supports touch,
- web build constraints,
- Android APK packaging constraints,
- Steam Deck asset-root and controller behavior,
- headless/minimal test paths.

See `docs/concepts/platform-targets.md`.

## Gameplay state

Landed or scaffolded mechanics include:

- kinematic platformer controller, coyote/buffered jump, dash, double dash, wall cling/jump/climb, fast fall, blink, pogo/rebound, glide, fly/debug mode;
- body modes and collision-safe body shape checks for crouch/crawl/slide/morph-ball style traversal;
- directional slash intents including upward and downward slash / pogo;
- projectile backend with Fireball and Hadouken-style motion-input upgrade;
- shield/parry state and bubble-shield presentation;
- actors, health/damage, interactions, breakables, pickups, projectiles, encounters, and boss-pattern vocabulary;
- LDtk-authored goblin encounter / encounter-style areas and transition validation.

The mechanics are still sandbox-grade. Treat mechanics docs as expressibility and validation guides, not as promises of final tuning or animation polish.

## Documentation maintenance

- ADRs must stay modern. If an ADR name or decision is stale, rewrite it instead of adding a contradictory note elsewhere.
- `docs/concepts/` should hold stable terms and edit protocols.
- `docs/systems/` should describe current systems only.
- `docs/recipes/` should describe current procedures only.
- `docs/archive/` holds superseded migrations, old handoffs, and historical evidence.
- `docs/brainstorms/` remains active idea incubation.
- `dev/` remains active engineering memory for lessons and benchmark traps.

## Current validation habit

After doc moves or concept changes:

```bash
python scripts/generate_agent_index.py
python scripts/check_agent_kb.py
```

After Rust changes, use the concept page or recipe to select focused tests before broad workspace tests.
