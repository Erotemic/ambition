---
id: asset-management
aliases: []
status: current
authority: durable-concept
last_verified: 2026-07-18
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
---

# Asset management

Asset management turns stable logical/provider-owned identity into platform
appropriate source data, Bevy handles, readiness evidence, and presentation.

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

## Invariants

- Logical IDs survive desktop/web/Android path differences.
- Content IDs are provider-qualified where collisions are possible.
- A handle being requested is not proof that an asset is ready.
- Required versus degradable work is explicit in load evidence.
- Preflight/preparation does not partially mutate the live session.
- Presentation never claims readiness the coordinator has not observed.
- Desktop host-path checks do not prove APK or web-served availability.
- Generated outputs are deterministic or carry enough provenance to reproduce.

## Edit protocol

1. Classify the change: provider catalog, source/generator, reusable loader,
   readiness transaction, platform packaging, or presentation consumer.
2. Keep named content out of reusable crates.
3. Preserve the old live authority until replacement content is ready to commit.
4. Validate at least one headless path and every affected packaging target.
5. Use `agent_query.py` to locate current owners rather than copying a path list
   into this page.

```bash
python scripts/agent_query.py "asset catalog loading readiness <asset kind>"
python scripts/agent_query.py crate ambition_asset_manager
python scripts/agent_query.py crate ambition_load
python scripts/agent_query.py tests "asset readiness"
./run_tests.sh -p ambition_asset_manager
```
