# Progression systems

Progression is the set of save-backed facts that change what the player can do or what the world presents: flags, quests, pickups, chests, NPC/enemy conversion, cutscene skip state, encounter rewards, and authored unlocks.

## Current paths

```text
crates/ambition_sandbox/src/quest.rs
crates/ambition_sandbox/src/cutscene.rs
crates/ambition_sandbox/src/save.rs
crates/ambition_content/src/quest.rs
crates/ambition_sandbox/src/features/
crates/ambition_sandbox/src/persistence/save.rs
crates/ambition_sandbox/src/dialog/
crates/ambition_sandbox/src/intro/cutscene.rs
crates/ambition_sandbox/src/presentation/cutscene.rs
crates/ambition_sandbox/src/encounter/
```

The retired root files `crates/ambition_content/src/quest.rs`, `src/cutscene.rs`, and `src/features.rs` should not be used in new docs. Compatibility re-exports may still exist at the crate root, but the owned code lives under the themed modules above.

## Current model

- Engine crates define durable vocabularies: quests, cutscenes, saves, combat/actor primitives.
- Sandbox content modules convert authored LDtk/content data into runtime entities and progression state.
- Save data stores persistent facts such as flags, defeated enemies, hostile NPC conversions, cutscene seen flags, and collected rewards.
- Presentation modules show dialogue/cutscenes and effects; they do not own persistent truth.

## Agent rules

- Add a save flag only when it represents durable player/world history.
- Keep generated/authored IDs stable; changing IDs is a save migration.
- Keep one-way conversions, such as NPC-to-hostile-enemy, tested around save/load boundaries.
- Do not use progression flags as general-purpose frame events.
- Document authored progression hooks in LDtk/tool docs when designers need to create them.

## Validation anchors

```bash
cargo test -p ambition_sandbox quest
cargo test -p ambition_sandbox save
cargo test -p ambition_sandbox content_validation
cargo test -p ambition_sandbox conversion_tests
cargo test -p ambition_sandbox encounter
```
