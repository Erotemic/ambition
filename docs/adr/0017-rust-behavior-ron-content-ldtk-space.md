# ADR 0017: Rust is for behavior, RON is for content, LDtk is for space

## Status

Accepted (2026-05-24). Current guidance; updated for the Stage 20 content split
on 2026-06-17.

## Decision

Each layer owns a single class of concern:

| Layer | Owns | Examples |
|---|---|---|
| **Rust (behavior + plumbing)** | Algorithms, ECS systems, physics, brain variants, validators, Bevy plugins, generic schemas/holders | `brain::tick_melee_brute`, `step_kinematic`, `CharacterCatalogPlugin`, `BrainDriverPlugin`, generic boss/enemy registry types |
| **RON / data files (authored content and tuning)** | Character catalog, brain cfgs, ActionSet specs, boss encounter numeric specs, enemy/boss rosters, dialogue trees, quest defs, tuning, area specs | `crates/ambition_gameplay_core/assets/data/character_catalog.ron`, `crates/ambition_content/assets/data/boss_encounters/<id>.ron`, `crates/ambition_content/assets/data/enemy_archetypes.ron` |
| **LDtk (space)** | Level geometry, entity placement, world composition | `sandbox.ldtk`, `intro.ldtk`; `NpcSpawn` points at catalog ids |

Rule of thumb:

- A new **brain template** lives in Rust because it is behavior.
- A new **character using existing behavior** is authored as data, currently in
  `crates/ambition_gameplay_core/assets/data/character_catalog.ron` until the
  character catalog moves to `ambition_content`.
- A new **named boss, enemy, quest, item roster entry, dialogue sequence, intro
  hook, or banter line** belongs in `ambition_content` data/modules.
- A new **room layout** lives in LDtk.
- New numeric boss/encounter tuning should prefer RON if the behavior hook
  already exists.

## Context

Before the catalog and content refactors, character truth was spread across
renderer registries, LDtk spawn names, brain-spawn code, tool configs, and
in-lib content tables. The current architecture makes Rust own typed behavior
and generic validation while authored game-specific data moves to the content
layer. LDtk owns where authored ids appear in the world.

The boss and enemy rosters now use the target shape: `ambition_gameplay_core`
owns generic schemas/holders and no named roster content; `ambition_content`
embeds and installs the named data.

## Consequences

- A future Ambition-powered game can swap authored content data without changing
  Rust when it only needs existing behavior.
- Rust-consumed config should be RON unless an upstream tool defines another
  format, such as LDtk JSON.
- Catalog and roster validation should fail loudly on missing brain/action/sprite
  references.
- Brain variants remain typed and exhaustive in Rust.
- ADR 0016 actor unification and this ADR compose: catalog/roster data describes
  what to spawn; actor/brain ECS code describes how spawned entities behave.
- Specific game content should not accumulate in `ambition_gameplay_core`; move
  it to `ambition_content` when the generic schema/holder exists.

## Current implications for agents

- **New character** → author data first. The current character catalog file is
  `crates/ambition_gameplay_core/assets/data/character_catalog.ron`, but this is
  transitional; do not add new game-specific Rust tables in the gameplay core.
- **New boss/enemy roster data or boss encounter numbers** → use
  `crates/ambition_content/assets/data/` and install through the content crate.
- **New behavior** → add Rust only when existing brain/action templates cannot
  express the mechanic.
- **New room or arena** → edit LDtk through `ambition_ldtk_tools`, not by
  hand-editing LDtk JSON.
- **Tuning a number** → check the relevant data asset before adding Rust
  overrides.
- **Fallback discipline** → hardcoded constructors may remain as fresh-clone
  fallbacks, but authored RON/content data is the source when present.
- **Validation** → add cheap schema/tool checks first, then Rust pin tests for
  drift against typed fallbacks.

## Cross-references

- `docs/systems/character-catalog.md`
- `docs/recipes/adding-a-character.md`
- ADR 0003 — data-driven specs and asset loading.
- ADR 0009 — LDtk world composition.
- ADR 0016 — actor unification.
