---
id: ldtk-world-composition
aliases:
  - active area
  - LoadingZone
  - LDtk runtime spine
  - sandbox.ldtk
  - editor roundtrip
implemented_by:
  - crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
  - crates/ambition_sandbox/src/world.rs
  - tools/ambition_ldtk_tools/ambition_ldtk_tools/area_authoring.py
  - tools/ambition_ldtk_tools
related_adrs:
  - docs/adr/0009-world-composition-and-ldtk-authoring.md
related_docs:
  - docs/systems/ldtk-world-composition.md
  - docs/recipes/ldtk-authoring.md
  - docs/current/state.md
related_memory:
  - dev/journals/lessons_learned.md
  - dev/benchmark-candidates/ldtk-runtime-collision-questions.md
last_verified: 2026-05-17
---

# LDtk world composition

## Definition

LDtk is the current sandbox level-editor adapter. Ambition still treats typed runtime data as the canonical game vocabulary, but the checked-in sandbox world is authored in `sandbox.ldtk` and projected into runtime rooms, solids, loading zones, metadata, and feature entities.

## Core invariants

- `LoadingZone.target_room` targets an `activeArea` id, not necessarily the LDtk level id.
- Multiple LDtk levels may stitch into one runtime active area.
- Do not hand-edit `sandbox.ldtk`; use Ambition LDtk tooling and validation.
- LDtk editor metadata must roundtrip cleanly through LDtk 1.5.x.
- Runtime projection must preserve physical reachability: arrivals should not start outside the target active area or inside authored solids.

## Edit protocol

1. Read `docs/recipes/ldtk-authoring.md` and ADR 0009 for world-authoring direction.
2. Search `dev/` for the exact LDtk symptom or field name.
3. Use `tools/ambition_ldtk_tools/ambition_ldtk_tools/area_authoring.py`, `tools/ambition_ldtk_tools/ambition_ldtk_tools/repair.py`, or `ambition_ldtk_tools` rather than direct JSON surgery.
4. Run doctor/roundtrip validation after map mutations.
5. Update this concept or focused LDtk docs if an authoring rule changes.

## Validation

```bash
python -m pytest tools/ambition_ldtk_tools/tests/test_area_authoring_features.py
python tools/check_ldtk_editor_roundtrip.py crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
cargo run -p ambition_sandbox --bin headless
```

Use the exact tool command documented by the touched LDtk workflow when it differs from these examples.
