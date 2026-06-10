---
id: bevy-native-data-driven-ecs
status: current
aliases:
  - Bevy-native
  - data-driven ECS
  - code-driven versus data-driven
  - RON room migration
  - LDtk runtime projection
related_adrs:
  - docs/adr/0002-engine-must-be-bevy-native.md
  - docs/adr/0003-data-specs-and-asset-loading.md
  - docs/adr/0009-world-composition-and-ldtk-authoring.md
implemented_by:
  - crates/ambition_sandbox/src/app/mod.rs
  - crates/ambition_sandbox/src/world/mod.rs
  - crates/ambition_sandbox/src/world/ldtk_world/mod.rs
related_docs:
  - docs/systems/architecture.md
  - docs/systems/ldtk-world-composition.md
last_verified: 2026-05-17
---

# Bevy-native data-driven ECS

## Definition

Ambition should lean into Bevy. Authored/generated data should feed Bevy components/entities/systems instead of being mirrored in parallel code-owned structures unless a focused test seam requires it.

## Current rule

- Bevy-native is the default, not an exception.
- LDtk owns world/level authoring.
- RON room manifests are historical.
- RON remains fine for tuning, save/settings, generated-audio specs, and compact non-world data.
- Reusable mechanics live in `crates/ambition_engine_core/src/`; runtime integration lives in Bevy ECS. (The standalone `ambition_engine` crate was collapsed into `engine_core/` on 2026-05-28.)

## Common failure modes

- Reintroducing backend-neutral abstractions that make Bevy integration harder.
- Treating old RON room docs as current.
- Duplicating authored data into a second runtime structure without tests proving parity.
- Putting presentation details into engine types.

## Validation

For world/data changes, run the relevant LDtk tool/tests and at least one headless or minimal sandbox test. For docs-only changes, run:

```bash
python scripts/generate_agent_index.py
python scripts/check_agent_kb.py
```
