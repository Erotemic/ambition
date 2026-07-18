# ADR 0017: Rust is for behavior, RON is for content, LDtk is for space

## Status

Accepted (2026-05-24). Current guidance; updated for provider-owned content and current world/tool boundaries on
2026-07-18.

## Decision

Each layer owns a single class of concern:

| Layer | Owns | Examples |
|---|---|---|
| **Rust (behavior + plumbing)** | Algorithms, ECS systems, physics, brain variants, validators, Bevy plugins, generic schemas/holders | `brain::tick_melee_brute`, `step_kinematic`, `CharacterCatalogPlugin`, `BrainDriverPlugin`, generic boss/enemy registry types |
| **RON / data files (authored content and tuning)** | Character catalog, brain cfgs, ActionSet specs, boss encounter numeric specs, enemy/boss rosters, dialogue trees, quest defs, tuning, area specs | `game/ambition_content/assets/data/character_catalog.ron`, `game/ambition_content/assets/data/boss_encounters/<id>.ron`, provider-owned encounter/roster/tuning data |
| **LDtk (space)** | Level geometry, entity placement, world composition | `sandbox.ldtk`, `intro.ldtk`; `NpcSpawn` points at catalog ids |

Rule of thumb:

- A new **brain template** lives in Rust because it is behavior.
- A new **character using existing behavior** is authored as provider data in
  `game/ambition_content/assets/data/character_catalog.ron`.
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

Named character, boss, enemy, encounter, dialogue, and progression content is
owned by the provider. Focused reusable crates own generic schemas, validation,
registries, and execution; `ambition_actors` composes the live simulation body
without owning the flagship roster.

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
- Specific game content must not accumulate in reusable engine crates; provider
  crates register it through typed App-local seams.

## Current implications for agents

- **New character** → author data first in
  `game/ambition_content/assets/data/character_catalog.ron`; do not add
  game-specific Rust tables in reusable engine crates.
- **New boss/enemy roster data or boss encounter numbers** → use
  `game/ambition_content/assets/data/` and install through the content crate.
- **New behavior** → add Rust only when existing brain/action templates cannot
  express the mechanic.
- **New room or arena** → edit LDtk through `ambition_ldtk_tools`, not by
  hand-editing LDtk JSON.
- **Tuning a number** → check the relevant data asset before adding Rust
  overrides.
- **Fallback discipline** → an empty engine/provider composition is valid;
  reusable crates do not carry hidden Ambition content fallbacks.
- **Validation** → add cheap schema/tool checks first, then Rust pin tests for
  drift against typed fallbacks.

## Cross-references

- `docs/systems/actors-brains-and-character-content.md`
- `docs/recipes/adding-a-character.md`
- ADR 0003 — data-driven specs and asset loading.
- ADR 0009 — LDtk world composition.
- ADR 0016 — actor unification.
