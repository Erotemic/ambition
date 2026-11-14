# `ambition_hosted_world` — module map

<!-- BEGIN generated module map (scripts/measure/modules_md.py) -->

**ambition_hosted_world** — A hosted world's replication: the host's bodies reach a guest App by `SimId` (NET-AUTHORITY-EXPERIMENT, `engine/hosted-world-networking-experiment.md`).

| Module | Its ONE concern (from the module's own `//!` header) |
|---|---|

_0 crate-root modules. Regenerate: `python scripts/measure/modules_md.py --write`._

<!-- END generated module map -->

## Notes

One file (`src/lib.rs`) plus the `udp` module behind the `renet` feature. The host's world is the only authority: the host writes rows (`BodyRow`, `LifeRow`, `SpawnRow`, `RoomNamed`) from its simulated components, and the guest applies them after replicon's receive. The guest sends two client messages, `GuestInput` (its seat's control frame) and `GuestRoom` (the room definition it holds). `RoomOf` is replicon's visibility filter, so a guest is sent its own room and its own seat's body. The comparison with GGRS is `docs/planning/engine/hosted-world-networking-experiment.md`.
