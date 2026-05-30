# Abilities

Abilities are reusable gameplay verbs. Keep ability implementation split between reusable mechanics in `ambition_engine` and sandbox-specific input, presentation, content wiring, and authored unlocks in `ambition_sandbox`.

**Review date:** 2026-05-27. Reviewed against source archive `ambition-source-2026-05-26T222032-5-3e93516618a5`.

## Current ability families

| Family | Status | Notes |
|---|---:|---|
| Jump / air jump / coyote / jump buffer | Available | `ambition_engine` owns coyote and jump buffering. Variable jump height exists through early-release velocity clipping. |
| Dash / dash buffer / fast fall | Available | Dash buffering exists. Fast fall is player-specific movement policy. |
| Blink | Available, partial buffering | Quick/precision blink and safe destination search exist. Holding can bridge cooldown, but there is not yet a general blink action buffer. |
| Wall cling / wall jump / wall climb | Available, high-risk | Collision correction bugs belong in trace-backed tests. Dedicated wall-coyote polish is still a candidate improvement. |
| Ledge grab / mantle | Partial but implemented | Engine and sandbox ledge behavior exist; animation/polish and ledge-action buffering remain incomplete. |
| Body modes | Available vocabulary | See body-mode doc for compact shapes and traversal constraints. |
| Combat verbs | Available, fragmented buffering | Directional slash, pogo, shield/parry, bubble shield, and projectiles exist. Pogo defaults to authored pogo surfaces, not plain floor/door solids. Attack/pogo/projectile inputs do not yet share a general buffered-action system. |
| Projectiles / motion inputs | Available | Fireball charge and Hadouken/HadoukenSuper recognition are implemented in the player projectile backend. |
| Sprint jump / long jump | Not yet reusable | Horizontal-momentum-aware jump variants are not formalized as a reusable ability family. |
| Grapple / tether / harpoon dash | Not yet reusable | Needs backend constraints, targeting semantics, and authoring vocabulary. |

## Input-buffer status

The current source has real action buffers for jump and dash only. These are engine timers (`jump_buffer_timer`, `dash_buffer_timer`) filled from the control pass and consumed when the corresponding action becomes legal.

Missing or partial buffers:

- **Attack:** player melee start now listens for the player brain's `ActorActionMessage::Melee`, but a press during active/recovery is still not a general queued attack.
- **Pogo:** pogo is still a player-specific raw input path layered on top of the attack system.
- **Projectile/tool:** projectile press/hold/release and motion-input recognition exist, but failed cooldown/resource windows are not a general queued tool action.
- **Blink:** held blink has cooldown bridging behavior, but there is no typed blink buffer window shared with other actions.
- **Ledge actions:** ledge climb/jump/roll/attack behavior exists, but ledge-action inputs are not buffered as a family.

When adding new Silksong-style polish, prefer a reusable `ActionBuffer` / `InputBuffer` shape over adding one more one-off timer per action.

## Combat-hit status

Damage exists, but a canonical per-hit payload does not. Current combat uses several payloads:

- `DamageEvent` for outgoing player slash / projectile hits against feature targets;
- `PogoBounceEvent` for pogo-refresh breakables;
- explicit hostile `Hitbox` entities for enemy/boss active melee volumes;
- `PlayerDamageEvent` for hazards/enemies/bosses damaging the player;
- boss encounter outcomes for boss HP / invulnerability / kill handling.

The next reusable combat primitive should be `HitSpec` -> `HitInstance` -> `HitResult`, not more ad-hoc damage message variants.

## Placement rules

- Put reusable ability math and state vocabulary in `ambition_engine`.
- Put player ECS state, input bridging, animation, audio, VFX, and LDtk/content-specific unlocks in `ambition_sandbox`.
- Put authored showcase-room procedures in recipes, not in this file.
- Keep status summaries in `docs/mechanics/expressibility-checklist.md`.

## Validation anchors

```bash
cargo test -p ambition_engine movement
cargo test -p ambition_engine combat
cargo test -p ambition_engine projectile
cargo test -p ambition_sandbox --lib brain::
cargo test -p ambition_sandbox projectile
cargo test -p ambition_sandbox scripted_gameplay
```
