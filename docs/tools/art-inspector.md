---
status: current
last_verified: 2026-10-24
related_docs:
  - docs/tools/generated-visual-tools.md
  - docs/concepts/asset-management.md
---

# Art inspector

`python3 scripts/art_inspector.py build --open` writes one offline page,
`.agent/art_inspector/index.html`. The page lists each art file, the
generator that made it, the asset it belongs to and where the game uses it.
See [`tools/art_inspector/README.md`](../../tools/art_inspector/README.md).

Each file links to its generator with a confidence (verified, declared,
derived, inferred, reference, unknown). A file with no known generator is a
finding. The links come from the generators' own provenance records:

- Each sprite target install, `backgrounds draw`/`parts` and `terrain draw`
  writes a record per unit into
  `crates/ambition_platformer2d_actor_monolith/asset_provenance/`
  (`tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/provenance.py`).
  The directory is beside the runtime asset root, not in it, because the
  asset roots are packaged whole.
- `scripts/regen/sprites.sh` sets the record directory for its staged
  publish and backfills a record for each target that it skipped as current.
- `python -m ambition_sprite2d_renderer.provenance backfill` records art
  published before the records existed.

A character's asset page also shows its lineage and authorship from the
sprite renderer's design history
(`tools/ambition_sprite2d_renderer/design_history/`): each era of the
character (a commit, with its still when the design-history cache has one),
who made that commit, and who wrote the lines of the generator's code and
art, by `git blame`. A model is named only by evidence (a resource-ledger
session, a `Co-Authored-By` trailer or the commit message); a commit from
Jon's account that names no model is shown as that, and no evidence is
`unknown`. Run `python -m design_history fetch` in the renderer once so that
the eras before the active renderer repository have authorship too.

The inspector is read-only. It does not regenerate art and it does not
replace a registry: the renderer registry, the content packs and the LDtk
worlds stay the authorities.
