---
status: current
related_docs:
  - docs/mechanics/blink.md
  - docs/concepts/movement-collision.md
---

# Blink destination and post-blink motion

Blink uses the ordinary body movement state and world collision queries.
The current implementation lives in
`crates/ambition_platformer2d_core/src/movement/blink.rs`; there is no
separate general-purpose, authored `BlinkPolicy` registry.

## Destination

The caller resolves aim into a world-space direction (including the body's
facing and reference frame). `blink_destination_clusters` or
`blink_destination_to_point_clusters` resolves a destination against the
world and the body's size. Normal blink respects world bounds and blocking
surfaces. The `unrestricted_blink` ability permits the developer override.

The destination query returns a position, not a transaction containing
position, resource costs, and presentation effects. The calling movement
step is responsible for admitting and applying the complete action.

## Motion after a successful blink

`complete_blink_clusters` updates position and velocity, clears conflicting
maneuver states, arms grace and cooldown, and records a `BlinkEvent` and
movement operation. It resolves velocity in the body's local frame, so
rotating gravity does not turn a downward-speed clamp into a world-Y rule.

The current code applies fixed tangential damping factors (0.35 for
precision blink, 0.55 otherwise), while downward limits, cooldown,
distance, and grace time come from ability tuning. If a new character needs
a different response, first decide whether that dimension belongs in
existing shared tuning before inventing a second movement path.

## Change guidelines

Maintain deterministic destination selection, no new penetration for
ordinary blink, and coherent state after a rejected attempt. Keep camera
movement and particles downstream of committed simulation events.
Playtest both ordinary and precision blink in the affected gravity frame;
use a focused check for a concrete collision or state regression.
