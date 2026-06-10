# Current state

This is the compact active-state document for Ambition. Update it when the current architecture or active direction changes. Keep old migration plans in `docs/archive/`, not here.

**Review date:** 2026-05-30. The `ambition_engine` crate was deleted on 2026-05-28; reusable mechanics now live inside `crates/ambition_engine_core/src/` or focused sandbox modules such as `crates/ambition_sandbox/src/projectile/`.

## One-sentence summary

Ambition is a Bevy-native, data-driven ECS-first 2D metroidvania/platformer sandbox with reusable mechanics crates, LDtk-authored world data, generated assets, multi-platform runtime targets, and an increasingly unified actor brain/action pipeline.

## Current architectural stance

```text
Authoring / generation
  LDtk worlds, asset manifests, generated music/SFX/sprites/backgrounds,
  RON/config data where it remains useful.

Bevy ECS runtime
  Components/entities/systems/messages are the main integration language.
  Sandbox systems adapt authored data into runtime entities and presentation.

Reusable crates
  ambition_asset_manager: asset identity, source selection, platform profiles.
  ambition_sfx / ambition_sfx_bank: generated SFX IDs and runtime banks.

Playable shell
  ambition_sandbox: Bevy app composition, LDtk runtime, input/touch/controller,
  audio, UI, debug/devtools, presentation, content, and the reusable
  mechanics modules (`src/engine_core/` for geometry/collision/movement/body
  modes/world/player clusters, plus focused sandbox-owned mechanics such as
  `src/projectile/`). The former `ambition_engine` crate was collapsed into
  this crate 2026-05-28.
```

The old direction of keeping the engine backend-neutral is superseded. See ADR 0002.

## World and data ownership

LDtk is the current world/level authoring source. The old RON room-manifest direction is historical.

Current rule:

- LDtk owns areas, collision layers, loading zones, room/world spatial data, and authored level entities.
- Bevy ECS owns the runtime representation.
- `crates/ambition_engine_core/src/` owns reusable engine semantics such as geometry, collision, movement, body modes, player clusters, and world/block policy. Focused sandbox mechanics such as projectiles live in their own crate modules.
- RON remains valid for tuning, save/settings, generated-audio specs, boss encounter specs, character catalogs, and other non-world data where it is still the best format.
- Agents must not hand-edit `sandbox.ldtk`; use `python -m ambition_ldtk_tools` and validation tools.

## Platform stance

Desktop, web, Android/mobile touch, controller, and Steam Deck are active compatibility targets. iOS is deferred until macOS test hardware exists.

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

- kinematic platformer controller, coyote time, jump buffer, dash buffer, double jump / air jump, dash charges, wall cling/jump/climb, ledge grab, fast fall, blink, pogo/rebound, glide, fly/debug mode;
- body modes and collision-safe body shape checks for crouch/crawl/slide/morph-ball style traversal;
- directional slash intents including upward and downward slash / pogo;
- projectile backend with Fireball, charged Fireball, Hadouken, and HadoukenSuper motion-input upgrades;
- shield/parry state and bubble-shield presentation;
- actors, factions, health/damage, interactions, breakables, pickups, projectiles, encounters, and boss-pattern vocabulary;
- LDtk-authored goblin encounter / encounter-style areas and transition validation;
- character catalog and Hall of Characters content flow;
- RON-authored boss encounter numeric specs with Rust behavior profiles;
- universal-brain interface in `crates/ambition_sandbox/src/brain/`: every controllable entity carries `Brain` + `ActionSet` + `ActorControl` sibling components, with `emit_brain_action_messages` producing `ActorActionMessage`s from each actor's resolved intent.

The current actor/brain migration is no longer only a shadow seam. The live code uses it for player movement/control, player melee-start gating, player projectile tick/charge input, hostile enemy ranged projectiles, hostile enemy melee windup starts, and authored boss special consumers such as apple rain / Gradient Sentinel attacks. Remaining rough edges are narrower: pogo start remains player-specific in the attack lifecycle, but both control-phase and attack-phase pogo use the centralized `BlockKind::is_pogo_target()` surface policy. As of 2026-05-28 the player ECS migration is **complete**: the player entity carries 18 cluster components (`PlayerKinematics`, `PlayerGroundState`, …, `PlayerComboTrace`); every production path takes `&mut PlayerClustersMut` natively; `PlayerMovementAuthority` + `PlayerBody` + `ae::Player` are all deleted; tests build a `PlayerClusterScratch` via `PlayerClusterScratch::new_with_abilities` (or `crate::player::primary_player_scratch` for ECS spawn sites). The engine entry points (`update_player_with_tuning_clusters`, `update_player_*_scratch`, `tick_active_ledge_grab_clusters`, `try_start_ledge_grab_clusters`, `try_change_body_mode_clusters`, `classify_safety_from_kinematics`, `detect_oob_from_kinematics`, combat `*_from_view`, …) are the only path. See `dev/journals/player-cluster-native-push-2026-05-28.md`.

Damage is functional but fragmented. The sandbox currently routes outgoing player/projectile hits through `DamageEvent`, hostile hitboxes through explicit `Hitbox` entities and `PlayerDamageEvent`, boss state through `BossDamageOutcome`, and presentation through VFX/SFX messages. There is not yet a canonical `HitSpec` / `HitInstance` / `HitResult` pipeline that carries per-hit metadata end-to-end.

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
python scripts/check_doc_links.py
```

After Rust changes, use the concept page or recipe to select focused tests before broad workspace tests.
