# Mechanics expressibility checklist

Use this as a compact current-status map. It intentionally avoids the old 900-line wishlist, but it preserves enough future backend primitives that agents can find useful multi-hour tasks without re-reading deleted docs.

**Review date:** 2026-05-30. Reviewed against source archive `ambition-source-2026-05-30T104014-5-e721ea65c578`.

Legend: `[x]` expressible now, `[~]` scaffolded but incomplete, `[ ]` not yet reusable backend.

## Movement

- [x] Kinematic controller, coyote time, jump buffer, air jump, dash buffer, dash charges, wall cling/jump/climb, fast fall.
- [x] Blink/teleport targeting and safety checks.
- [x] Glide and fly/debug mode.
- [x] Ledge grab / mantle behavior exists in engine+sandbox code.
- [~] Ledge grab / mantle polish: action buffering, animation coverage, and edge-case tests remain incomplete.
- [~] Moving platforms: implemented path exists, but carry semantics need more validation.
- [~] Climbable / ladder body mode: available, but ladder-top passthrough and jump/dash-off polish remain.
- [~] Silksong-style jump polish: variable jump height exists; apex hang / jump sustain are not yet reusable tuning concepts.
- [ ] General input buffer for attack, pogo, projectile/tool, blink, and ledge actions.
- [ ] Sprint-jump / long-jump momentum rules.
- [ ] Grapple/tether/harpoon-dash constraints.
- [ ] Gravity columns / rotated gravity policy.
- [ ] Parametric curve riding or spline/rail movement.

## Combat and interactions

- [x] Directional slash intents, including upward slash and downward slash / pogo.
- [x] Projectile backend with Fireball, charged Fireball, Hadouken, and HadoukenSuper motion-input upgrades.
- [x] Shield/parry state and bubble-shield presentation.
- [x] Actor/faction/damage/interactable/breakable vocabulary.
- [x] Player melee-start gate emits and consumes `ActorActionMessage::Melee`.
- [x] Enemy ranged projectiles, enemy melee windup starts, and current authored boss specials have `ActorActionMessage` consumers.
- [~] Dialogue/commerce hooks: architectural seed exists, content pipeline is not final.
- [~] Boss profiles and phase machines: playable, with RON numeric specs; not fully data-authored behavior.
- [~] Combat-hit metadata: damage works, but `DamageEvent`, hostile `Hitbox`, `PlayerDamageEvent`, and boss outcomes are not unified into a canonical per-hit object.
- [ ] Canonical `HitSpec` / `HitInstance` / `HitResult` pipeline with raw damage, final damage, stagger/poise, elements/status, knockback, hitstop, VFX/SFX, and rejection reasons.
- [ ] Bubble shield dodge/roll policy.
- [ ] Falling-sand / fluid toy-room simulation.

## Actor brain / action pipeline

- [x] `Brain` + `ActionSet` + `ActorControl` sibling components exist for player, NPC, enemy, and boss entities.
- [x] `Brain::Player` translates `PlayerInputFrame` into `ActorControlFrame`.
- [x] Player movement/control phases consume `ActorControl` instead of raw `ControlFrame`.
- [x] `emit_brain_action_messages` resolves each actor's `ActionSet` into `ActorActionMessage`s.
- [x] Live consumers exist for player melee start, hostile enemy ranged, hostile enemy melee windup start, GNU-ton apple rain, and Gradient Sentinel specials.
- [x] Player projectile charging consumes `ActionRequest::PlayerProjectileTick` from the brain/action stream.
- [~] Pogo start remains player-specific, with shared target-surface policy through `BlockKind::is_pogo_target()`.
- [x] `ae::Player` ECS decomposition (2026-05-28): the player entity carries 18 cluster components (`PlayerKinematics`, `PlayerGroundState`, …, `PlayerComboTrace`); the monolithic `ae::Player` aggregate and the `PlayerMovementAuthority` wrapper are deleted.
- [ ] Possession / multiplayer input routing using arbitrary actor bodies.

## Body and traversal

- [x] Crouch/crawl/slide/body-mode vocabulary.
- [x] Collision-safe shape checks for compact traversal and morph-ball-style modes.
- [~] Authored traversal rooms for body-mode mechanics need expansion.
- [ ] Spring-ball/bomb/spider-ball-style specialized traversal.
- [ ] Ladder-through-solid or authored ladder-top passthrough rule.
- [ ] Swim/sink/iron-boots variants unified with body-mode and volume policy.

## World and authoring primitives

- [x] LDtk-authored rooms, loading zones, IntGrid lowering, and hot reload.
- [x] One-way platforms, damage volumes, climbable regions, and runtime encounter lock walls.
- [~] Stitched / side-scrolling room adjacency: schema vocabulary exists, but robust loading-zone-free traversal needs a prototype.
- [~] Generated sprites/music/backgrounds through tools: usable, but staging/publish workflow needs more clarity.
- [ ] Generic ray/shape cast query API exposed as a reusable mechanic primitive.
- [ ] Surface tangent/normal query helpers for mechanics that need slope/bounce/ledge semantics.
- [ ] Vector/scalar fields for wind, current, gravity, heat, or faction influence.
- [ ] Deterministic randomness streams for generated systems and replayable tests.

## Simulation and validation

- [x] Trace/replay/debug hooks for movement/combat validation.
- [x] Headless `SandboxSim` stepping path.
- [~] Avian2D secondary physics for debris/props; not the primary player controller.
- [~] Time-domain vocabulary is documented; full per-entity proper-time gameplay is future work.
- [ ] Headless screenshot / visual verification path.
- [ ] PyO3 or equivalent external research binding for `SandboxSim`.
- [ ] Reward-shaping examples for AI playtesting.

## Validation anchors

```bash
cargo test -p ambition_sandbox --lib combat
cargo test -p ambition_sandbox --lib projectile
cargo test -p ambition_sandbox --lib engine_core::movement
cargo test -p ambition_sandbox --lib brain::
cargo test -p ambition_sandbox projectile
cargo test -p ambition_sandbox scripted_gameplay
```

Prefer exact tests named by concepts or benchmark candidates when available. Use `TODO.md` for the centralized accepted task list; use brainstorm docs for speculative mechanic ideas that are not ready for an agent session.
