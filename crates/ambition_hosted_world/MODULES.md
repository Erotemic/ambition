# `ambition_hosted_world` — module map

<!-- BEGIN generated module map (scripts/measure/modules_md.py) -->

**ambition_hosted_world** — A hosted world's replication: the host's bodies reach a guest App by `SimId` (NET-AUTHORITY-EXPERIMENT, `engine/hosted-world-networking-experiment.md`).

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|
| [`input_clock`](src/input_clock.rs) | Input sample order is independent of the estimated host tick. |
| [`room_rollback`](src/room_rollback.rs) | A host that keeps a guest's room correct by rollback (SIMULATION-ISLANDS). |

_2 crate-root modules. Regenerate: `python scripts/measure/modules_md.py --write`._

<!-- END generated module map -->

## Notes

One file (`src/lib.rs`) plus the `udp` module behind the `renet` feature. The host's world is the only authority: the host writes rows (`BodyRow`, `LifeRow`, `SpawnRow`, `RoomNamed`) from its simulated components, and the guest applies them after replicon's receive. The guest sends two client messages, `GuestInput` (its seat's control frame) and `GuestRoom` (the room definition it holds). `RoomOf` is replicon's visibility filter, so a guest is sent its own room and its own seat's body. The comparison with GGRS is `docs/planning/engine/hosted-world-networking-experiment.md`.

## Input ownership

`GuestInput.sequence` orders render samples within one connection. The tick is
estimated from the replicated host clock, elapsed time, and the fixed step.
Samples for one tick use the latest held controls and combine button edges.
Prediction keeps held controls and clears edges. Input ticks must be within
`REWIND_HORIZON` of the host's past or `MAX_INPUT_LEAD` of its future.

A disconnected seat loses its arrived, used, and local input history, sequence
counter, and current controls. A new owner starts at its own assignment tick and
cannot rewind the previous owner's frames. A guest without a body can send its
join press through the normal control path.

Automatic allocation reserves admitted session participants and connected guest
seats. Authorized clients without a seat remain eligible on later updates.
The UDP protocol id is `ambhost2`; both ends must use this input format.
