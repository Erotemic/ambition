# HEAD status

Audited 2026-07-16 against the current source tree. This page records the live
state and current work; completed execution narratives belong in git history or
`docs/archive/`.

## Closed architecture campaign

The July 15–16 architecture campaign is complete at its stated bar:

- activation, reset, transition, restore, and LDtk reload share one App-installed
  placement-lowering authority;
- `ambition_platformer_provider` owns the typed provider preparation/activation
  lifecycle;
- `SceneEntities` is gone and sequential session teardown/activation is covered
  through the real host lifecycle;
- `ambition_sim_harness` owns the reusable reset/step/action/observation surface;
- the named content families selected for eviction now register through open,
  content-owned seams;
- boss attack execution, timing, motion locks, and effects converge on
  `MovePlayback` and moveset data;
- domain plugins own the repaired dev/dialog/encounter/menu state families;
- touch semantics compile without the presentation stack; and
- render consumes the repaired combat/dialog read-model seams.

These are foundations to preserve, not active decomposition tracks.

## Current hard work

| Workstream | Current state | What closes it |
|---|---|---|
| Encounter lifecycle convergence | **ACTIVE.** Wave and boss encounters still expose parallel lifecycle/ownership/consumer paths. | Complete E8–E13 in [`engine/encounter-orchestration.md`](engine/encounter-orchestration.md): one command/lifecycle/objective authority, explicit participant ownership, stable snapshot identity, converged consumers, and a non-boss acceptance customer. |
| Atomic active-room restoration | **OPEN.** Session isolation and same-room restore/resimulation are green, but restore deliberately refuses snapshots whose active room differs from the live room. | Preflight and transactionally reconstruct the snapshot room through canonical room staging/lowering, restore room-scoped state, and promote the portal/boss cross-room replay cases from `DIRTY` to `CLEAN`. See [`engine/netcode.md`](engine/netcode.md). |
| Super Mary-O acceptance | **PARTIAL, engine seams proven.** Pickups/equip, grown form, ranged powerup, bricks, crony stomp behavior, flag sequence, clock, tally, and cyclic restart exist. | Secret pipe/underground room, shell prop, HUD/title/results, and one deterministic scripted level-1 completion that collects and uses a real powerup. |
| Sanic acceptance | **PARTIAL, movement and host seams proven.** Provider-owned persona, standard keyboard→slot→brain→body control, ball dash, transformation, lifecycle, and route/momentum oracles exist. | Bits/drop-on-hit, at least one complete enemy/contact loop, goal/HUD/results, a complete act, and a headless high-route-versus-low-route completion oracle. |
| Fighter-brain L3 rollouts | **DESIGN CORRECTION REQUIRED.** The current proposal combines a wall-clock budget with deterministic authoritative simulation and proposes rollouts from a live snapshot despite the delayed `Perceived` contract. | Choose a deterministic work budget or recorded-input model, and define a rollout state built only from allowed perceived facts before implementation. |
| Boss animator residue | **BOUNDED.** The execution/body path is converged; remaining residue is animation vocabulary/projection (`BossAnim`→`CharacterAnim`, obsolete target mirrors where still live). | Complete the bounded animator fold. Do not reopen the already-shared body integration path. |

## Restore terminology

Two different accomplishments must not be conflated:

- **Landed:** leak-free sequential sessions and exact same-active-room
  restore/resimulation for supported registered state.
- **Open:** atomic replacement of the live room when a snapshot names a different
  active room.

`MovingPlatformSet` is a lifecycle-scoped active-session resource: it is rebuilt
from room construction, snapshot-registered, and explicitly cleared on teardown.
The type does not independently carry a session identifier; safety currently
rests on the one-live-session host contract plus teardown.

## Deferred

- The final public name for the provider crate.
- A provider-owned placement-family channel beside the closed common Tier-0 schema.
- Menu-host extraction until a second real consumer exists.
- The boss-crate carve decision. Convergence permits reassessment, but no
  concrete dependency/build/reuse boundary is currently documented; the
  maintainer ruling remains open.
- A full `features/` rename; no partial rename.

Direct maintainer confidence belongs in
[`maintainer-decisions.md`](maintainer-decisions.md), not inferred from this
status summary.
