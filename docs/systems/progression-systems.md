# Progression systems

Progression is the set of save-backed facts that change what the player can do or what the world presents: flags, quests, pickups, chests, NPC/enemy conversion, cutscene skip state, encounter rewards, and authored unlocks.

## Current paths

```text
crates/ambition_actors/src/quest/                 # generic quest runtime and state
game/ambition_content/src/quest.rs               # shipped quest specs and payouts
game/ambition_content/src/quests/                # authored quest registration/content
crates/ambition_actors/src/persistence/save.rs    # durable save facts
crates/ambition_actors/src/dialog/                # dialogue runtime/read model
crates/ambition_actors/src/encounter/             # encounter progression hooks
crates/ambition_render/src/cutscene/
crates/ambition_actors/src/intro/cutscene.rs
```

The sandbox owns reusable progression machinery and durable state. The content crate owns named authored quest/content definitions. Root-level compatibility re-exports may exist, but new docs should point at the owning modules above.

## Current model

- Engine/sandbox crates define durable vocabularies: quests, cutscenes, saves, combat/actor primitives.
- `ambition_content` registers shipped quests, rewards, dialogue/intro hooks, and named content.
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
cargo test -p ambition_actors quest
cargo test -p ambition_actors save
cargo test -p ambition_content quest
cargo test -p ambition_actors encounter
```
