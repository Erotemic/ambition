# Abilities

Abilities are reusable gameplay verbs. Keep ability implementation split between reusable mechanics in `ambition_engine` and sandbox-specific input, presentation, and content wiring in `ambition_sandbox`.

## Current ability families

| Family | Status | Notes |
|---|---:|---|
| Jump / air jump / coyote / buffer | Available | Core platformer feel. |
| Dash / fast fall | Available | Input policy and repeat/hysteresis live in sandbox settings/input. |
| Blink | Available | Has a separate motion-policy doc because destination safety is high-risk. |
| Wall cling / wall jump / wall climb | Available, high-risk | Collision correction bugs belong in trace-backed tests. |
| Ledge grab / mantle | Partial | Behavior exists; polish and animation coverage need work. |
| Body modes | Available vocabulary | See body-mode doc for compact shapes and traversal constraints. |
| Combat verbs | Available | Directional slash, pogo, shield/parry, bubble shield, projectiles. |
| Grapple/tether | Not yet reusable | Needs backend constraints and authoring semantics. |

## Placement rules

- Put reusable ability math and state vocabulary in `ambition_engine`.
- Put player ECS state, input bridging, animation, audio, VFX, and LDtk/content-specific unlocks in `ambition_sandbox`.
- Put authored showcase-room procedures in recipes, not in this file.
- Keep status summaries in `docs/mechanics/expressibility-checklist.md`.

## Validation anchors

```bash
cargo test -p ambition_engine movement
cargo test -p ambition_engine combat
cargo test -p ambition_sandbox scripted_gameplay
```
