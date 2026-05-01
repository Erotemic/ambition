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
