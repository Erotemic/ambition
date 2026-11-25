---
status: current
last_verified: 2026-11-25
related_docs:
  - docs/tools/generated-visual-tools.md
  - docs/concepts/asset-management.md
---

# Asset inspector

`scripts/asset_inspector.py` provides an inventory of authored and published
assets, source and generation evidence, known game uses, and asset reviews.
It helps locate missing or stale products; it does not publish them.

```bash
python3 scripts/asset_inspector.py --help
python3 scripts/asset_inspector.py status --current-game --needs-generation --json
python3 scripts/asset_inspector.py inspect npc_tower_voss --animation death --json
python3 scripts/asset_inspector.py coverage --missing death --json
python3 scripts/asset_inspector.py build --open
```

`build` opens an offline view of character forms, parts, animation coverage,
lineage, publication tiers and available previews. A character's page starts
with its canonical image in a zoomable viewer: the vector design from its
bundle's review files when the target writes one (a chargen character's
five-view turnaround, sharp at any zoom), and the canonical image (one pose
at the sheet's frame size). A character's animation player loops by default
and scales up to 8x. `serve --open` can also save
reviews of every asset kind, in the music tool's review format: a score from 1
to 10, notes and issue tags for one exact version. `reviews backlog --json`
lists the reviews that ask for work.
See [the editor tool README](../../tools/ambition_editor/README.md) for storage
and current command details.

Maintained lineage previews use current authored targets and configs. Draw them
with `./scripts/regen/lineages.sh --all`, then build the inspector again. They
appear separately from archive reference images. Normal lineage rendering does
not read Git history. Use `--archive-reference` only for archive research.
See [maintained lineage variants](generated-visual-tools.md#maintained-character-lineage-variants).
Survey attribution labels are not verified by an ordinary inspector build.
Unsigned-commit and date-selected model labels are inferences.

Ordinary status queries rely on manifests and receipts without rerendering.
`--deep` rechecks bytes, `--no-previews` avoids generating preview strips, and
`--authorship` adds an optional Git history scan.

`capture_scene --art-snapshot` exports actor/presentation facts from a running
scene. Pass the file to `build --runtime-snapshot PATH` to compare them with
local manifests. A local inventory cannot prove what a running game displayed.

The renderer owns generation status; the publisher owns accepted bundles.
Source definitions, authored content, and curated art history remain their
respective authorities. See [sprite publication](sprite-publication.md).
