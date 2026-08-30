---
id: asset-management
aliases: []
status: current
authority: durable-concept
last_verified: 2026-08-30
implemented_by:
  - crates/ambition_asset_manager
  - crates/ambition_load
  - crates/ambition_load_presentation
  - crates/ambition_audio
  - game/ambition_content
related_docs:
  - docs/systems/asset-manager.md
  - docs/systems/ldtk-world-composition.md
  - docs/concepts/content-and-provider-boundaries.md
  - docs/planning/engine/asset-preparation-and-residency.md
---

# Asset management

Asset management turns stable logical/provider-owned identity into platform
appropriate source data, Bevy handles, readiness evidence, device-side material
and presentation.

## Stable separation

- **Providers own named assets and catalogs.** Ambition-specific worlds, sprite
  sheets, music, SFX cues, and art live with Ambition content.
- **`ambition_asset_manager` owns reusable catalog/profile/handle machinery.**
- **`ambition_load` owns headless loading plans, work states, and barriers.**
- **`ambition_load_presentation` renders unresolved load evidence.** It never
  manufactures readiness.
- **Domain crates own typed runtime use.** For example, `ambition_audio` owns
  audio catalogs/playback and render crates own image/material consumers.
- **Hosts own platform packaging and device-side source availability.**

Generated files are not runtime authority merely because they are checked in.
The generator source/spec and the provider catalog together define how to
reproduce and address them.

## Loading is a pipeline, not one event

Keep these stages conceptually distinct:

```text
logical demand
    ↓
source IO
    ↓
decode / parse
    ↓
insert prepared Bevy asset
    ↓
render extraction / device preparation
    ↓
resident drawable/usable resource
```

A handle being requested is not proof that the source is decoded. A decoded
image is not proof that its render asset is prepared. Readiness evidence must say
which boundary it actually proves.

This distinction is operationally important: recent rendered profiling found
large frame hitches in image render extraction/device preparation even though
source decode was already asynchronous.

## Demand, preparation and residency

- Raise demand from semantic knowledge as early as practical, not from the first
  frame that needs to draw the resource.
- Preparation may be paced when completion bursts are expensive; pacing policy is
  a host/presentation concern and must be validated on rendered hardware.
- A resource may be loaded but not currently required. Residency therefore needs
  explicit owners/scopes rather than accidental survival through strong handles.
- Define ownership and budgets before choosing eviction machinery. Do not begin
  with a universal LRU service.
- Quality changes preserve logical identity: changing a raster tier selects a
  different product for the same asset identity, not a different character or
  gameplay object.

## Invariants

- Logical IDs survive desktop/web/Android path differences.
- Content IDs are provider-qualified where collisions are possible.
- Required versus degradable work is explicit in load evidence.
- Preflight/preparation does not partially mutate the live session.
- Presentation never claims readiness the coordinator has not observed.
- Desktop host-path checks do not prove APK or web-served availability.
- Generated outputs are deterministic or carry enough provenance to reproduce.
- Asset preparation may change presentation latency/quality; it must not change
  simulation rules.
- A material/texture should not be marked modified every frame when its semantic
  value did not change; needless modification can force device re-preparation.

## Edit protocol

1. Classify the change: provider catalog, source/generator, reusable loader,
   readiness transaction, device preparation/residency, platform packaging, or
   presentation consumer.
2. Keep named content out of reusable crates.
3. Preserve the old live authority until replacement content is ready to commit.
4. Validate the stage the change claims to improve; do not use decode completion
   as proof of GPU readiness.
5. Validate at least one headless path for simulation/readiness semantics and a
   rendered path when the claim concerns device materialization, residency or
   raster cost.
6. Use `agent_query.py` to locate current owners rather than copying a path list
   into this page.

```bash
python scripts/agent_query.py "asset catalog loading readiness <asset kind>"
python scripts/agent_query.py crate ambition_asset_manager
python scripts/agent_query.py crate ambition_load
python scripts/agent_query.py tests "asset readiness"
./run_tests.sh -p ambition_asset_manager
```
