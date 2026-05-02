# Architectural decision records

ADRs are the durable source of truth for decisions that may supersede older notes.

Use an ADR when a decision:

- changes the architecture direction,
- reverses an earlier constraint,
- selects a crate or rejects an alternative,
- defines a documentation/source-of-truth rule,
- affects how future agents should work.

## Index

- [0001: Maintain source-of-truth docs separately from historical notes](0001-source-of-truth-docs.md)
- [0002: The engine may be Bevy-native](0002-engine-may-be-bevy-native.md)
- [0003: Use data specs and embedded fallbacks while asset loading matures](0003-data-specs-and-asset-loading.md)
- [0004: Support multiple game modes from one reusable engine](0004-multiple-game-modes.md)
- [0005: Mark spatial/geometry code for extra review](0005-spatial-review-markers.md)
- [0006: Require explicit repo-state and patch packaging discipline](0006-repo-state-and-patch-packaging.md)
- [0007: Use Avian2D for secondary physics while keeping the player controller custom](0007-avian2d-secondary-physics.md)
- [0008: Use Yarn-oriented dialogue and a commerce architecture seed](0008-dialogue-and-commerce-architecture.md)

- [0009: Use canonical world composition with LDtk as an authoring adapter](0009-world-composition-and-ldtk-authoring.md)
- [0010: Time domains, time-control authority, and regime policies](0010-time-domains-and-regime-policies.md)
- [0011: Per-entity proper time and the Galilean→SR ladder](0011-per-entity-proper-time-and-sr-ladder.md)
- [0012: Sim/presentation split, events refactor, and bevy_rl hook target](0012-sim-presentation-split-and-events-refactor.md)
- [0013: Compile-time hygiene as a project constraint](0013-compile-time-hygiene.md)
- [0014: Adopt bevy_dev_tools for dev-time overlays and CI utilities](0014-bevy-dev-tools-adoption.md)
