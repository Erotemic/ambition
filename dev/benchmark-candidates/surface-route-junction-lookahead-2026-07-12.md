# Surface route junction lookahead benchmark candidate — 2026-07-12

## Failure signature

A momentum rider reaches a tangent-continuous fork such as a floor/ramp split or
a classic loop mouth. Up/Down appears to do nothing because every outgoing
segment has the same tangent at the exact switch. In the reverse direction,
plain Left/Right can select the wrong repeated loop-mouth occurrence and create
another lap. A separate symptom is that a ramp touching a block floor still
requires jumping because the follower cannot transfer between collision owners.

## Root cause

A route is topological, not merely geometric. Scoring only the immediate tangent
cannot distinguish branches that intentionally share a tangent. Restricting a
junction to repeated vertices inside one chain also cannot express a floor that
splits into a separately authored raised chain.

## Invariant

- A route junction connects explicit chain/vertex ports; proximity alone never
  creates a branch.
- The current chain/direction is the authored default continuation.
- Directional override compares a finite lookahead heading, not only the first
  segment tangent.
- Only steering transverse to the incoming route may override the default;
  forward/back locomotion alone cannot accidentally add another loop lap.
- Near-ties keep the authored default.
- Cross-chain transfer preserves signed tangential speed and unspent distance in
  the current tick; it is not an airborne hop.
- Open-end reattachment tolerance must cover the representational endpoint
  nudge, or a rider can hover forever at a chain lip.

## Poison tests

1. Two branches share a horizontal first segment and diverge later; Up chooses
   the rising branch, Down chooses the falling branch, horizontal preserves the
   default.
2. A floor chain and ramp chain share a junction; Up transfers onto the ramp
   while horizontal stays on the floor, with no `Airborne` state.
3. Forward and reverse loop traversal with horizontal input each exits after one
   authored lap.
4. Running off an open flat chain advances past the endpoint and falls instead
   of reattaching one arc-length nudge before the lip.

Tags: `surface-momentum`, `route-topology`, `junction`, `loop`, `lookahead`,
`cross-chain`, `open-end`.
