# Architectural decision records

ADRs are durable, current decisions. If an ADR becomes stale, update or supersede it; do not leave contradictory live guidance elsewhere.

Use an ADR when a decision:

- changes architecture direction,
- reverses an earlier constraint,
- selects or rejects a crate/tooling foundation,
- defines documentation/source-of-truth policy,
- affects future agent behavior.

## Index

- [0001: Use a layered repository knowledge base](0001-source-of-truth-docs.md)
- [0002: The engine must be Bevy-native](0002-engine-must-be-bevy-native.md)
- [0003: Data specs feed Bevy ECS; LDtk owns world authoring](0003-data-specs-and-asset-loading.md)
- [0004: Support multiple modes from reusable mechanics](0004-multiple-game-modes.md)
- [0005: Mark spatial/geometry code for extra review](0005-spatial-review-markers.md)
- [0006: Require explicit repo-state and patch packaging discipline](0006-repo-state-and-patch-packaging.md)
- [0007: Use Avian2D for secondary physics while keeping the player controller custom](0007-avian2d-secondary-physics.md)
- [0008: Dialogue and commerce architecture](0008-dialogue-and-commerce-architecture.md)
- [0009: LDtk is the world-composition authoring source](0009-world-composition-and-ldtk-authoring.md)
- [0010: Time domains, time-control authority, and regime policies](0010-time-domains-and-regime-policies.md)
- [0011: Per-entity proper time and the Galilean→SR ladder](0011-per-entity-proper-time-and-sr-ladder.md)
- [0012: Simulation emits gameplay messages; presentation consumes them](0012-sim-presentation-split-and-events-refactor.md)
- [0013: Compile-time hygiene as a project constraint](0013-compile-time-hygiene.md)
- [0014: Adopt Bevy dev tools where they improve iteration](0014-bevy-dev-tools-adoption.md)
- [0015: LDtk tileset rendering is presentation-only](0015-ldtk-tileset-rendering.md)
- [0016: Unify NPCs, enemies, hazards, and interactables as actor-like ECS data](0016-actor-unification.md)
- [0017: Rust holds behavior; RON authors content; LDtk authors space](0017-rust-behavior-ron-content-ldtk-space.md)
- [0018: Enemy cluster variation — per-actor jitter is mandatory at every brain-spawn site](0018-enemy-cluster-variation.md)
- [0019: Pluginized platformer runtime via same-crate proto-boundaries](0019-pluginized-platformer-runtime.md)
- [0020: Mounts and vehicles — two linked actors, control-deferral, independent hurtboxes](0020-mounts-and-vehicles.md)
