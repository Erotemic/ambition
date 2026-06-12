# ADR 0017: Rust is for behavior, RON is for content, LDtk is for space

## Status

Accepted (2026-05-24). Current guidance; historical implementation notes were folded into this summary on 2026-06-12.

## Decision

Each layer owns a single class of concern:

| Layer | Owns | Examples |
|---|---|---|
| **Rust (engine + Bevy plugins)** | Algorithms, ECS plumbing, physics, brain variants, validators, Bevy plugins | `brain::tick_melee_brute`, `step_kinematic`, `CharacterCatalogPlugin`, `BrainDriverPlugin` |
| **RON (data)** | Character catalog, brain cfgs, ActionSet specs, boss encounter numeric specs, dialogue trees, quest defs, tuning, area specs | `crates/ambition_sandbox/assets/data/character_catalog.ron`, `crates/ambition_sandbox/assets/data/boss_encounters/<id>.ron` |
| **LDtk (space)** | Level geometry, entity placement, world composition | `sandbox.ldtk`, `intro.ldtk`; `NpcSpawn` points at catalog ids |

Rule of thumb:

- A new **brain template** lives in Rust because it is behavior.
- A new **character using existing behavior** is a new row in `crates/ambition_sandbox/assets/data/character_catalog.ron`.
- A new **room layout** lives in LDtk.
- New numeric boss/encounter tuning should prefer RON if the behavior hook already exists.

## Context

Before the catalog refactor, character truth was spread across renderer registries, LDtk spawn names, brain-spawn code, and tool configs. The current architecture makes the catalog data the source of display name, sprite path, default brain preset, default action-set preset, tier, body kind, and tags. Rust owns the typed behavior variants and validation; LDtk owns where those catalog ids appear in the world.

## Consequences

- A future Ambition-powered game can swap catalog data without changing Rust when it only needs existing behavior.
- Rust-consumed config should be RON unless an upstream tool defines another format, such as LDtk JSON.
- Catalog validation should fail loudly on missing brain/action/sprite references.
- Brain variants remain typed and exhaustive in Rust.
- ADR 0016 actor unification and this ADR compose: catalog data describes what to spawn; actor/brain ECS code describes how spawned entities behave.

## Current implications for agents

- **New character or boss** → author data first. Use `crates/ambition_sandbox/assets/data/character_catalog.ron` for ordinary characters and `crates/ambition_sandbox/assets/data/boss_encounters/<id>.ron` for authored boss numeric specs.
- **New behavior** → add Rust only when existing brain/action templates cannot express the mechanic.
- **New room or arena** → edit LDtk through `ambition_ldtk_tools`, not by hand-editing LDtk JSON.
- **Tuning a number** → check `crates/ambition_sandbox/assets/data/` before adding Rust overrides.
- **Fallback discipline** → hardcoded constructors may remain as fresh-clone fallbacks, but RON is the authored source when present.
- **Validation** → add cheap schema/tool checks first, then Rust pin tests for drift against typed fallbacks.

## Cross-references

- `docs/systems/character-catalog.md`
- `docs/recipes/adding-a-character.md`
- ADR 0003 — data-driven specs and asset loading.
- ADR 0009 — LDtk world composition.
- ADR 0016 — actor unification.
