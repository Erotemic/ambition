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
| Encounter lifecycle convergence | **DONE (2026-07-16).** One command/lifecycle/objective authority (`EncounterLifecycle` + reducer + `EncounterCommand` ingress); ownership/policy-driven cleanup; `SimId::encounter` + snapshot-registered relations; consumers derive from lifecycle + staging policy; the Noether attunement is the shipped non-boss customer. E8–E13 all closed with exit tests in [`engine/encounter-orchestration.md`](engine/encounter-orchestration.md). | — closed; residual boss-owned pieces (outro-gated persistence, reward anchors, adaptive music) recorded there as actor-local/authored policy. |
| Atomic active-room restoration | **DONE + CLOSED OUT (2026-07-16).** `restore` stages a differing snapshot room through canonical `RoomStaging`, and every currently preflightable refusal occurs before mutation. The closeout now includes complete room-content staging, no hollow room-backed respawns, exact session plus prepared-world routing identity (`SessionMismatch` / `WorldMismatch`), post-staging stable-id validation, blob-rebuildable projectiles with restored room/session lifetime scope, coordinated same-room reconstruction of content-staged actor batches, and registered fight-aged Smash/aggression/disposition/melee state. Exit oracles: `portal_lab` CLEAN, cross-room duel suffix equality, same-room missing-duelist batch reconstruction, and dynamic lifetime-shell reconstruction. See [`engine/netcode.md`](engine/netcode.md) N3.2b. | Closed for supported same-build rollback. Remaining boundaries: unanchored dead dynamic families need domain spawn recipes; room-presence mismatch refuses; `PlayerProjectileState` and broader N3.3 content fingerprint/versioning remain later work. |
| Super Mary-O acceptance | **PARTIAL, engine seams proven.** Pickups/equip, grown form, ranged powerup, bricks, crony stomp behavior, flag sequence, clock, tally, and cyclic restart exist. | Secret pipe/underground room, shell prop, HUD/title/results, and one deterministic scripted level-1 completion that collects and uses a real powerup. |
| Sanic acceptance | **PARTIAL, movement and host seams proven.** Provider-owned persona, standard keyboard→slot→brain→body control, ball dash, transformation, lifecycle, and route/momentum oracles exist. | Bits/drop-on-hit, at least one complete enemy/contact loop, goal/HUD/results, a complete act, and a headless high-route-versus-low-route completion oracle. |
| Fighter-brain L3 rollouts | **DESIGN CORRECTION REQUIRED.** The current proposal combines a wall-clock budget with deterministic authoritative simulation and proposes rollouts from a live snapshot despite the delayed `Perceived` contract. | Choose a deterministic work budget or recorded-input model, and define a rollout state built only from allowed perceived facts before implementation. |
| Boss animator residue | **BOUNDED.** The execution/body path is converged; remaining residue is animation vocabulary/projection (`BossAnim`→`CharacterAnim`, obsolete target mirrors where still live). | Complete the bounded animator fold. Do not reopen the already-shared body integration path. |

## Restore terminology

Two different accomplishments, both landed (2026-07-16):

- leak-free sequential sessions and exact same-active-room
  restore/resimulation for supported registered state;
- atomic replacement of the live room when a snapshot names a different
  active room (the N3.2b staged transaction).

`MovingPlatformSet` is a lifecycle-scoped active-session resource: it is
installed exclusively by construction (session setup, transition, sandbox
reset, hot-reload, restore staging — the visual sync owns no reset),
snapshot-registered, and explicitly cleared on teardown. The type does not
independently carry a session identifier; safety currently rests on the
one-live-session host contract plus teardown.

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
