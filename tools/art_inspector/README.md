# Art inspector

One offline page over all of Ambition's art: each file, the generator that
made it, the asset it belongs to and where the game uses it.

```bash
python3 scripts/art_inspector.py build --open   # first build ~1.5 min, then ~25 s
```

The build writes `.agent/art_inspector/` (git-ignored): `index.html`,
`app.js`, `app.css`, `catalog.js` and the previews. Open `index.html` from
disk; it needs no server and no network. `--no-previews` rebuilds only the
index, `--prune` deletes previews that the index no longer names, `--jobs N`
sets the preview processes (at most 6), `--no-lineage` skips the
lineages and the authorship (`git blame`; about 10 s when its cache is warm,
about 2 min the first time).

## Views

- **Gallery**: one card per asset (a character with all its files and
  quality tiers, a parallax theme, a terrain skin, an image). Search
  (`/`), facets for family, generator link and use, sort by frames, size,
  placements or findings.
- **Asset**: an animation player and a pose grid for a sheet, a layer
  viewer with a camera for a parallax theme, portraits, part tracks and body
  rig joints. The generator (source file, drawing function with its line,
  the regen command, the record and the files the render read), where the
  game uses it (content rows with their line, LDtk placements per level),
  every file with its confidence, and the findings.
- **Lineage and authorship** (on a character's asset page): each lineage
  of the design history that claims the character, with its eras (the
  commit, who made it, who wrote the generator's code and art at that
  commit, and the still when the design-history cache has it), and who wrote
  the lines of the character's generator files now. See
  `tools/ambition_sprite2d_renderer/design_history/README.md`.
- **Generators**, **Findings** (copy as CSV), **Coverage** (how many files
  of each root link to a generator).

## How each file links to its generator

Each file gets one link and one confidence:

| confidence | evidence |
|---|---|
| verified | a provenance record or `.cache/sheets` has the file's current digest |
| declared | a record or the renderer registry names the file; nothing vouches for its bytes |
| derived | a quality tier copy of another file (`generate_visual_quality_variants.py`) |
| inferred | only a name points at a generator |
| reference | authored or imported art under `assets/` |
| unknown | no generator found: a finding |

The evidence comes from the generators, not from this tool:

- the provenance records that each publish writes into
  `crates/ambition_platformer2d_actor_monolith/asset_provenance/`
  (`ambition_sprite2d_renderer/provenance.py`; `python -m
  ambition_sprite2d_renderer.provenance backfill` records older art);
- the renderer registry, `ambition-sprite2d-renderer list --json`;
- the renderer cache, `tools/ambition_sprite2d_renderer/.cache/sheets/`.

Uses come from the tracked content data (`game/*/assets/**/*.ron`), the LDtk
worlds (`character_id`, prop looks, boss brains, `parallax_theme`) and Rust
string literals (a heuristic). The index is derived on each build and is
never an authority.

## Layout

- `art_index/`: the indexer (`scan`, `sheets`, `generators`, `assets`,
  `usage`, `diagnostics`, `thumbs`, `build`). Pillow and PyYAML;
  `scripts/art_inspector.py` runs it in the sprite renderer's interpreter
  when the current one lacks them.
- `web/`: the page, plain JavaScript with no libraries.
- `tests/`: a synthetic repository; `./run_tests.sh --tool-tests --only-job
  'art inspector tests'`.
