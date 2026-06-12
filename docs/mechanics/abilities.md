# Abilities

Abilities are reusable gameplay verbs. Keep reusable engine semantics in `crates/ambition_engine_core/src/` or focused sandbox modules, and keep presentation/content/progression wiring sandbox-specific.

**Review date:** 2026-05-30. Reviewed against source archive `ambition-source-2026-05-30T104014-5-e721ea65c578`.

## Current ability families

| Family | Status | Notes |
|---|---:|---|
| Jump / air jump / coyote / jump buffer | Available | `engine_core::movement` owns coyote and jump buffering. Variable jump height exists through early-release velocity clipping. |
| Dash / dash buffer / fast fall | Available | Dash buffering exists. Fast fall is player-specific movement policy. |
| Blink | Available, partial buffering | Quick/precision blink and safe destination search exist. Holding can bridge cooldown, but there is not yet a general blink action buffer. |
| Wall cling / wall jump / wall climb | Available, high-risk | Collision correction bugs belong in trace-backed tests. Dedicated wall-coyote polish is still a candidate improvement. |
| Ledge grab / mantle | Partial but implemented | Engine and sandbox ledge behavior exist; animation/polish and ledge-action buffering remain incomplete. |
| Body modes | Available vocabulary | See body-mode doc for compact shapes and traversal constraints. |
| Combat verbs | Available, fragmented buffering | Directional slash, pogo, shield/parry, bubble shield, and projectiles exist. Pogo defaults to authored pogo orb/rebound surfaces, not plain floor/door solids, one-way platforms, or blink walls. Attack/pogo/projectile inputs do not yet share a general buffered-action system. |
| Projectiles / motion inputs | Available | Fireball charge and Hadouken/HadoukenSuper recognition are implemented in the player projectile backend. |
| Sprint jump / long jump | Not yet reusable | Horizontal-momentum-aware jump variants are not formalized as a reusable ability family. |
| Grapple / tether / harpoon dash | Not yet reusable | Needs backend constraints, targeting semantics, and authoring vocabulary. |

## Input-buffer status

The current source has real action buffers for jump and dash only. These are engine timers (`jump_buffer_timer`, `dash_buffer_timer`) filled from the control pass and consumed when the corresponding action becomes legal.

Missing or partial buffers:

- **Attack:** player melee start now listens for the player brain's `ActorActionMessage::Melee`, but a press during active/recovery is still not a general queued attack.
- **Pogo:** pogo start is still player-specific in the attack lifecycle, but valid pogo surfaces are centralized through `BlockKind::is_pogo_target()` so ground, one-way platforms, and blink walls do not become pogo targets by accident.
- **Projectile/tool:** projectile press/hold/release and motion-input recognition exist, but failed cooldown/resource windows are not a general queued tool action.
- **Blink:** held blink has cooldown bridging behavior, but there is no typed blink buffer window shared with other actions.
- **Ledge actions:** ledge climb/jump/roll/attack behavior exists, but ledge-action inputs are not buffered as a family.

When adding new Silksong-style polish, prefer a reusable `ActionBuffer` / `InputBuffer` shape over adding one more one-off timer per action.

## Combat-hit status

Combat damage now flows through the canonical `HitEvent` transport for player slash, pogo, projectile, hazard, enemy, and boss damage paths. Explicit hostile `Hitbox` entities still model enemy/boss active melee volumes, but their damage output joins the same hit-event pipeline.

The next reusable combat work should enrich `HitResult` semantics — stagger/poise, elements/status, hitstop, rejection reasons, and reward hooks — rather than adding more ad-hoc damage message variants.

## Placement rules

- Put reusable ability math and state vocabulary in `engine_core` or an explicitly focused sandbox mechanics module.
- Put player ECS state, input bridging, animation, audio, VFX, and LDtk/content-specific unlocks in `ambition_sandbox`.
- Put authored showcase-room procedures in recipes, not in this file.
- Keep status summaries in `docs/mechanics/expressibility-checklist.md`.

## Validation anchors

```bash
cargo test -p ambition_sandbox --lib engine_core::movement
cargo test -p ambition_sandbox --lib combat
cargo test -p ambition_sandbox --lib projectile
cargo test -p ambition_sandbox --lib brain::
cargo test -p ambition_sandbox projectile
cargo test -p ambition_app --test scripted_gameplay --features "rl_sim portal"
```
