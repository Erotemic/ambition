# Current state

This is the compact active-state document for Ambition. Update it when the current architecture or active direction changes. Keep old migration plans in `docs/archive/`, not here.

**Review date:** 2026-06-10. The monolith was bisected into a layered crate
graph (Stage 20); `ambition_sandbox` is now the **machinery library**, not the
playable shell. The remaining-work survey is `docs/planning/plugin_refactor/22_monolith_breaker_survey.md`; most older `plugin_refactor/` files are historical execution notes.

## One-sentence summary

Ambition is a Bevy-native, data-driven ECS-first 2D metroidvania/platformer sandbox with reusable mechanics crates, LDtk-authored world data, generated assets, multi-platform runtime targets, and an increasingly unified actor brain/action pipeline.

## Current architectural stance

```text
Authoring / generation
  LDtk worlds, asset manifests, generated music/SFX/sprites/backgrounds,
  RON/config data where it remains useful.

Bevy ECS runtime
  Components/entities/systems/messages are the main integration language.

Crate layers (low → high; lower must never import higher):
  foundations  ambition_engine_core (geometry/collision/movement/body/player
               clusters/world), ambition_actor (unified actor system: control
               vocabulary + universal brain + character catalog; bosses are
               actors), ambition_platformer_runtime (kinematic body,
               gravity, rooms, projectile), ambition_portal, ambition_time,
               ambition_input, ambition_menu (reusable renderers), ambition_audio,
               ambition_sfx[_bank], ambition_asset_manager.
  machinery    ambition_sandbox (lib): mechanics, features (named
               actor/boss ECS world), presentation, world/LDtk, items, encounter,
               persistence, the dev STATE, the menu IR/map. Content-free
               (guard-enforced). Re-exports the foundation crates under their
               historical `crate::engine_core` / `crate::input` / … facade paths.
  content      ambition_content: named game content — quests, bosses, items
               roster, dialogue, intro, banter, portal adapters.
  app          ambition_app: Bevy assembly, host glue, ALL binaries (playable
               `ambition_sandbox` bin, headless, rl_*), the menu host stack +
               DevToolsPlugin, and the full-stack integration tests.
```

The old direction of keeping the engine backend-neutral is superseded (ADR 0002).
Gameplay subsystems are moving to a **components-as-plugins** shape: each owns its
own `Plugin` registration rather than being hand-wired in the app assembly.

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

The actor/brain unification is live, not a shadow seam: player movement/control,
melee-start gating, projectile tick/charge, enemy ranged + melee windups, and
authored boss specials all flow through it. Player ECS migration is **complete**
(2026-05-28): the player carries 18 cluster components, every path takes
`&mut PlayerClustersMut`, and `ae::Player`/`PlayerMovementAuthority`/`PlayerBody`
are deleted (tests use `PlayerClusterScratch::new_with_abilities`). Bosses ARE
actors (ADR 0016): `BossEncounterPhase` lives in `brain` and the goal is one
unified actor+brain+boss-runtime unit with only *named* boss data in
`ambition_content`. See `dev/journals/player-cluster-native-push-2026-05-28.md`.

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
