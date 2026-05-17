# Archived: ldtk-runtime-spine.md

Superseded migration or transition note. Preserve as historical evidence; do not treat as current procedure.

Original path: `docs/systems/ldtk-runtime-spine.md`

---

# LDtk runtime-spine migration

`ambition_sandbox` is moving from a custom LDtk JSON adapter toward
`bevy_ecs_ldtk` as the runtime spine. This file tracks promoted
categories, the parity overlay, and what's left.

See ADR 0009 (`docs/adr/0009-world-composition-and-ldtk-authoring.md`)
for the long-form rationale.

## Promoted categories

For each category we add (1) a typed component on plugin-spawned
LDtk entities, (2) a sibling per-frame index resource holding the
active-area-local view, and (3) parity diagnostics against the
JSON-derived `ae::World::blocks` collision authority.

| Category         | Component               | Index resource              | Status   |
| ---------------- | ----------------------- | --------------------------- | -------- |
| `PlayerStart`    | (covered by spine)      | `LdtkRuntimeSpineIndex`     | promoted |
| `LoadingZone`    | (covered by spine)      | `LdtkRuntimeSpineIndex`     | promoted |
| `DebugLabel`     | (covered by spine)      | `LdtkRuntimeSpineIndex`     | promoted |
| `CameraZone`     | (covered by spine)      | `LdtkRuntimeSpineIndex`     | promoted |
| `Solid`          | `LdtkSolid`             | `LdtkRuntimeSolidIndex`     | promoted (parity overlay live) |
| `OneWayPlatform` | `LdtkOneWayPlatform`    | `LdtkRuntimeOneWayIndex`    | promoted (parity overlay live) |
| `DamageVolume`   | `LdtkDamageVolume`      | `LdtkRuntimeDamageIndex`    | promoted (parity overlay live) |
| `KinematicPath`  | JSON adapter path object | `RoomSpec::kinematic_paths` | promoted; consumed by platform/NPC/enemy/hazard `path_id` |
| `BreakablePlatform` / `BreakablePogoOrb` | — | —                | later    |

The JSON adapter still owns runtime collision authority (`ae::World::blocks`).
The typed components are observers; once parity holds for several
sandbox sessions and hot-reload edits, the JSON-collision arms can
retire and the runtime-spine indices become authority. **Do not delete
the JSON adapter path until parity is proven.**

## Parity diagnostics

`LdtkRuntimeSpineParity` is a Bevy resource updated each frame by
`check_ldtk_runtime_spine_parity`. It compares the count of
`Solid` / `OneWay` / `Hazard` blocks in the JSON-derived collision
world against the sizes of the matching runtime indices.

A mismatch logs a single tracing warning at
`ambition::ldtk_runtime_spine` (deduped against the previous summary
string) so the parity bug is visible in logs without spamming every
frame. When counts converge again the warning is cleared.

The HUD/debug overlay can render `LdtkRuntimeSpineParity::summary()`
to show the live counts:

```text
solids 35/35  one-way 4/4  damage 2/2  match=true
```

## Verification gate

Authority swap (JSON → typed components) requires:

- Parity holds across a fresh boot of the sandbox.
- Parity holds after at least one LDtk hot reload.
- Parity holds after running through every authored active area at
  least once.
- A test fixture covers an LDtk file with all three promoted
  collision categories and asserts the index counts match.

When all four are green, the JSON adapter's collision arms for that
category can be deleted in a separate dedicated commit.

## Validator

`tools/validate_ambition_ldtk.py` continues to run Ambition-side
semantic validation. The current sandbox LDtk file declares
`OneWayPlatform`, `HazardBlock`, and `DamageVolume` entity
identifiers; the typed-component spawn handles all three so legacy
maps with `HazardBlock` continue to register damage volumes.

```bash
python tools/repair_ambition_ldtk.py --in-place crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
python tools/check_ldtk_editor_roundtrip.py crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
python tools/validate_ambition_ldtk.py crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
python tools/validate_ambition_ldtk.py \
  --schema tools/schemas/ldtk/JSON_SCHEMA.json \
  --require-schema \
  crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk
```
