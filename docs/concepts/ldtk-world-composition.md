---
id: ldtk-world-composition
status: current
aliases:
  - LDtk
  - loading zones
  - active areas
  - room graph
  - IntGrid
  - sandbox.ldtk
implemented_by:
  - crates/ambition_actors/src/world/mod.rs
  - crates/ambition_actors/src/world/ldtk_world/mod.rs
related_adrs:
  - docs/adr/0009-world-composition-and-ldtk-authoring.md
related_docs:
  - docs/systems/ldtk-world-composition.md
  - docs/recipes/ldtk-authoring.md
  - docs/tools/index.md
last_verified: 2026-05-17
---

# LDtk world composition

LDtk is the current world/level authoring source. Old RON room manifests are historical.

## Core invariants

- Do not hand-edit `sandbox.ldtk` JSON for semantic changes.
- Use `ambition_ldtk_tools` for authoring/repair/validation.
- Preserve loading-zone graph consistency and spawn repair behavior.
- Preserve static/embedded map paths for web and Android.

## Validation

```bash
python -m ambition_ldtk_tools validate game/ambition_content/assets/worlds/sandbox.ldtk
python -m ambition_ldtk_tools repair game/ambition_content/assets/worlds/sandbox.ldtk --in-place
cargo test -p ambition_actors ldtk
```
