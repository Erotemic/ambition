---
status: current
last_verified: 2026-10-30
related_docs:
  - docs/tools/generated-visual-tools.md
  - docs/concepts/asset-management.md
---

# Asset inspector

The asset inspector is the central place to browse and review every asset of
an engine game: characters, images, music, sound effects, fonts and worlds.
Run `./scripts/asset_inspector.py` with no arguments to open it. It shows what
exists, what is missing or stale and how to make it, where the game uses each
asset, and how assets relate, including character lineage. Any asset can take a
review note. A failure in one source becomes a notice, never a refusal.

For sprites it joins owner reports for registered bundles, prepared game
identities, local products, concepts, and design history. Game use,
registration, generation state, and lineage are separate facts.
See the [tool guide](../../tools/asset_inspector/README.md) for the full interface.

Build the content and runtime sheet reader once, then reuse it after data edits:

```bash
cargo build -p ambition_content_schemas --bin ambition_content_check
python3 scripts/asset_inspector.py status --current-game --needs-generation --json
python3 scripts/asset_inspector.py inspect npc_tower_voss --animation death --json
python3 scripts/asset_inspector.py coverage --missing death --json
python3 scripts/asset_inspector.py build --open
```

The offline page opens at Characters. This page groups declared forms and parts
under their character. The flat catalog is available under All catalog entries.
Shared ancestry does not merge distinct characters. Product claims select the
history when one publisher serves several characters. Shared origins have
visual previews and links to the separate characters.
Each card opens a character page with art playback, animation coverage, forms,
parts, derivations, and a visual design lineage. Cached historical looks appear
in date order beside the current sprite on normal builds. Recorded variations
remain visible when a character has no curated eras. The History inventory view
uses the same timeline. The optional authorship scan is separate.
Previews include declared rest poses and use one fixed camera. Art below a
physical foot anchor can use the visible bottom as its preview baseline.
The framing evidence retains the physical anchor. Playback and
comparison can switch to the full animation frame. Runtime RON supplies the
geometry; the preview reports its measured framing basis.
Use `python3 scripts/asset_inspector.py serve --open` to save review notes on a
character page. The Review notes page and `notes backlog --json` expose the
same durable task queue. Each note keeps its exact art generation and source
context. See the tool guide for storage, generation changes, and resolution.
The page also has identity trees, metadata filters, bundle and tier evidence,
source/profile/runtime coverage, lineage, published action comparison, and a
player with scrubbing and death-to-corpse playback. Source Studio links remain
separate from published previews. `--no-previews` updates the report without
new strips. `--authorship` adds the optional Git blame and model attribution.

Normal queries do not draw art, decode pixels, or walk history. `--deep`
rehashes bytes. The renderer owns the status decision used by publication.
Receipts record inputs, directories, parameters, toolchains, and output digests.
Quality conversions and atlases name their source bundles. Accepted sprite bundles are immutable. One publication manifest selects their
exact members. Legacy receipts with incomplete inputs remain
unknown until a publish records complete evidence.

`capture_scene --art-snapshot` writes read-only actor and presentation facts.
Pass the file with `build --runtime-snapshot PATH` to compare live selection
with local manifests. A local scan cannot prove live state or uploaded pixels.

The inspector stores derived data. Renderer plans, compiled content, authored
worlds, and curated design history remain the authorities. Compiled bindings
that an owner does not export have no asserted reference closure.

The [implementation receipt](../planning/engine/art-inspector-and-character-coverage.md)
records the death publication correction, delivered tools, and standing limits.
