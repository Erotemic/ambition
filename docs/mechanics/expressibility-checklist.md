---
status: current
last_verified: 2026-07-18
---

# Mechanic expressibility checklist

Use this before adding a mechanic or declaring the engine unable to express one.
The goal is to identify the smallest missing reusable primitive, not to create a
content-specific exception.

## Actor and control

- Can any actor body own the required capability?
- Can human, brain, RL, replay, and scripted controllers request it through the
  same semantic action seam?
- Does `ActorActionScheme` describe the live meaning of the slot?
- Do prompt generation and execution consume the same resolution result?
- Is any proposed `Player*`, `Enemy*`, or `Boss*` state duplicating an existing
  body/action path?

## Space and movement

- Is intent expressed in actor/gravity space rather than hard-coded world axes?
- Are collision, casts, safe placement, surface normals/tangents, portals, and
  moving geometry available through shared world/query vocabulary?
- Does the mechanic require a genuinely new primitive such as a scalar/vector
  field, deterministic random stream, constraint, or shape cast?
- Can body shape/mode changes validate atomically?

## Combat and interaction

- Does one canonical hit/interaction fact carry source, target, geometry,
  faction, payload, and outcome?
- Are costs, cooldowns, windup/active/recovery, cancellation, armor, and hurtbox
  changes composable rather than bespoke?
- Is there one authoritative spawn/emission site?

## Content and providers

- Is named content owned by the provider?
- Can reusable crates stay free of Ambition character/room/item names?
- Is registration App-local, deterministic, and validated before activation?
- Can another provider select different tuning, assets, and rules?

## Lifecycle and time

- Which session/room/actor scope owns spawned state?
- Does cleanup use lifecycle scopes rather than ad hoc entity lists?
- Can reset, room replacement, save/load, and snapshot restore reconstruct it?
- Does authoritative behavior depend only on explicit simulation time and stable
  ordering—not wall clock, rendering, audio, or allocator order?

## Observation and proof

- Can the mechanic run in the headless runtime?
- Is authoritative state visible through a stable read model/trace rather than
  renderer internals?
- Are the strongest tests invariant/property tests (symmetry, covariance,
  conservation, non-penetration, idempotence) rather than exact unpolished tuning?
- Can `agent_query.py` identify an owning crate and narrow test surface?

When the answer is “no,” record the missing primitive in planning. Do not hide it
behind a provider-specific bridge that creates a second execution path.
