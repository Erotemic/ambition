---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/mechanics/blink.md
  - docs/systems/blink-motion-policy.md
---

# Blink and fast-fall

Blink and fast-fall are actor capabilities composed with the shared body/action
pipeline. They are useful reference mechanics because they exercise semantic
input, gravity-relative movement, time/control affordances, collision queries,
body state, and presentation without requiring a player-only implementation.

## Blink flow

```text
semantic action
    -> ActorActionScheme/shared resolver
    -> blink capability gate and resource/cooldown policy
    -> gravity-relative direction and path policy
    -> deterministic safe-placement search
    -> atomic body relocation
    -> semantic outcome for read models/VFX/SFX
```

Precision aim may use `InputState::control_dt` so a human can steer while the
simulation is slowed. This is an input affordance, not a second body tick or a
presentation-owned time scale.

## Fast-fall flow

Fast-fall is a body/movement request expressed along local gravity. The shared
movement kernel decides whether the body is airborne, whether the capability is
available, and how the request modifies velocity/limits. It must not be a direct
world-Y velocity edit in device-input code.

## Invariants

- Both mechanics work for any capable actor/controller.
- Gravity rotation transforms direction and collision consistently.
- Blink success cannot leave a body penetrating geometry; rejection is atomic.
- Fast-fall never bypasses body mode, contact, or velocity policy.
- Cooldowns/resources/locks are authoritative simulation data.
- Presentation consumes committed outcomes and can be absent headlessly.
- Reset/snapshot/replay reconstructs active state without stale effects.

## Validation

```bash
python scripts/agent_query.py "blink fast fall control_dt"
python scripts/agent_query.py tests "blink gravity penetration"
./run_tests.sh -k blink
./run_tests.sh -k fast_fall
```

Prefer covariance, non-penetration, rejection-idempotence, and controller
symmetry tests over exact current tuning values.
