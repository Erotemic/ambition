---
status: current
last_verified: 2026-10-27
related_docs:
  - docs/tools/generated-visual-tools.md
  - docs/concepts/asset-management.md
---

# Art inspector

The inspector joins owner reports for registered bundles, prepared game
identities, local products, concepts, and design history. It includes missing
art. Game use, registration, generation state, and lineage are separate facts.
See the [tool guide](../../tools/art_inspector/README.md) for the full interface.

Build the content and runtime sheet reader once, then reuse it after data edits:

```bash
cargo build -p ambition_content_schemas --bin ambition_content_check
python3 scripts/art_inspector.py status --current-game --needs-generation --json
python3 scripts/art_inspector.py inspect npc_tower_voss --animation death --json
python3 scripts/art_inspector.py coverage --missing death --json
python3 scripts/art_inspector.py build --open
```

The offline page opens at Characters. This page shows the prepared game roster.
Each card opens a character page with art playback, animation coverage, forms,
parts, derivations, and design lineage. Recorded eras appear on normal builds.
The page also has identity trees, metadata filters, bundle and tier evidence,
source/profile/runtime coverage, lineage, published action comparison, and a
player with scrubbing and death-to-corpse playback. Source Studio links remain
separate from published previews. `--no-previews` updates the report without
new strips. `--authorship` adds the optional Git blame and cached era previews.

Normal queries do not draw art, decode pixels, or walk history. `--deep`
rehashes bytes. The renderer owns the status decision used by publication.
Receipts record inputs, directories, parameters, toolchains, and output digests.
Quality conversions and atlases name their source bundles. Sprite receipts move
with staged runtime products. Legacy receipts with incomplete inputs remain
unknown until a publish records complete evidence.

`capture_scene --art-snapshot` writes read-only actor and presentation facts.
Pass the file with `build --runtime-snapshot PATH` to compare live selection
with local manifests. A local scan cannot prove live state or uploaded pixels.

The inspector stores derived data. Renderer plans, compiled content, authored
worlds, and curated design history remain the authorities. Compiled bindings
that an owner does not export have no asserted reference closure.

The [implementation receipt](../planning/engine/art-inspector-and-character-coverage.md)
records the death publication correction, delivered tools, and standing limits.
