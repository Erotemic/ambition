
# Generated visual tools

The [sprite publication guide](sprite-publication.md) describes the accepted
bundle store, source snapshots, exact membership, and publication queries.

Generated character art, terrain and backgrounds are produced from authored
sources, then selected for runtime use through the library's publication and
generated-content contracts. Edit source definitions and invoke the existing
producer; never treat generated images or metadata as authored source.

> Audio (music/SFX) lives in the same `scripts/regen/assets.sh` pipeline but is
> documented separately — see
> [generated-music-workflow.md](../recipes/generated-music-workflow.md) and the
> SFX bank notes.

## The publish pipeline (start here)

`scripts/regen/assets.sh` orchestrates the categories declared by the selected
content library and records per-category timing under `target/regen_assets/`.
Use a scoped category or target for routine iteration:

```bash
./scripts/regen/assets.sh                 # all declared producers; can be expensive
./scripts/regen/assets.sh sprites         # just one (or several) categories, in order
./scripts/regen/backgrounds.sh --help     # per-category options live on each script
./scripts/regen/sprites.sh --help
./scripts/regen/quality_variants.sh --help
```

Category scripts use the producing tool's environment from
`run_developer_setup.sh`, with documented `AMBITION_*_PYTHON` overrides. The
library's `library.json` declares its generated runtime and publication roots;
visual and audio outputs do not belong in an actor crate's authored asset tree.
Current products can be reused; `--force` rebuilds selected products.

## Sprite renderer — `tools/ambition_sprite2d_renderer/`

Registered targets define products. The selected library's
`game/ambition_content_library/authored/sprites.yaml` names its runtime sprite
targets; the renderer contains the producing definitions and generators.
Targets can use Python, rigs, SVG parts, config generators or scene graphs.

```bash
./scripts/regen/sprites.sh --plan --json
./scripts/regen/sprites.sh --target patent_clerk
./scripts/regen/sprites.sh --check
./scripts/regen/sprites.sh --audit --json
```

The [sprite publication guide](sprite-publication.md) describes source snapshots,
exact membership, immutable bundles, and atomic publication. Sprite sheets,
parts, portraits, static props, quality tiers, and shared atlases use one
publisher. Runtime loading, embedding, packaging, and inspection read its
selection. Old loose sprite files are unmanaged.

Quality variants resize isolated frames and pack new pages. Config targets can
render from source at the requested quality. Potato uses a safe source scale,
then a final reduction with the nearest-neighbour filter. Do not resize a packed
actor page to make a quality variant.

For character authoring, see [adding a character](../recipes/adding-a-character.md).
The Hall of Characters is the visual review surface for the roster.

## Background and parallax renderers

- `tools/ambition_background_renderer/`: background profile generator.
- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/backgrounds/`:
  the parallax scenes, one for each biome. They are art, so they are in the
  authoring submodule. `scenes.py` has the table and says what a view shows of
  a panel; `artkit.py` is the paint box.
- `tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/terrain/`: what
  the game lays on the blocks of a room and stands on them. `skins.py` (fill
  and trims), `decor.py` (things on the ground, and their light), `doors.py`,
  `fixtures.py` (ladders, water, blink walls, hazard spikes), `water_backdrops.py`
  (opaque scenes behind authored water volumes), `motes.py`
  (motes and ground shadows). Published to `assets/room_dressing/`. A look
  without the game: `python -m ambition_sprite2d_renderer.terrain preview OUT.png lab cave`.

`./scripts/regen/backgrounds.sh` publishes generated profile backgrounds,
parallax layers, sky parts and room dressing into the selected library's
runtime paths (`generated/runtime/full/backgrounds/` and
`generated/runtime/full/room_dressing/` in the default library).
`./scripts/regen/sprites.sh` does not produce those assets. The quality-variant
producer supplies smaller tiers where applicable.

To look at a scene without the game:

```bash
cd tools/ambition_sprite2d_renderer
python -m ambition_sprite2d_renderer.backgrounds preview /tmp/scenes.png lab cove --cameras -1,0,1
```

It draws what a 16:9 view shows with the camera at the top, the middle and the
bottom of its room.

## Promo / vanity tools

`tools/vanity_card_prep/` (when present) prepares promotional card material —
not gameplay runtime data, so it is outside the `scripts/regen/assets.sh` pipeline.

## Experimental visual tools

`tools/experimental/` holds reference or in-progress work (3D sprite
experiments, procedural-fit experiments, component-extraction prototypes).
Promote a tool out of `experimental/` and document its workflow here before
using it as a runtime asset source.

## Extending a generator

Keep new assets reproducible from tracked source definitions and the shared
producer pipeline:

- New character/entity art → register a target through the authoring family
  that best fits the asset, then run `scripts/regen/sprites.sh`. Use a YAML config only
  when the character genuinely belongs to a config-driven generator family.
  Do not fork a render script inline without registering the result.
- Never hand-edit a published `*_spritesheet.png` / `.ron` / `.yaml`,
  `*_portraits.png`, or `*_portraits.ron` under `assets/`; they are outputs and
  will be overwritten on the next regen.
- A refactor that breaks `scripts/regen/sprites.sh` / `scripts/regen/backgrounds.sh` /
  `scripts/regen/assets.sh` on a clean checkout is a regression, even if the committed
  assets still look fine.

### Sprite regeneration progress and profiling

A sprite request reports its target count and each accepted target. Current
accepted bundles can be reused, including bundles from an unpublished request.
`LINE_PROFILE=1 ./scripts/regen/sprites.sh --force` writes worker `.lprof`
databases under the renderer's `.profiles/regen-*` directory. Use
`scripts/measure/render_line_profiles.py` to make text reports.

### Fast sprite profiling and readable reports

A forced full regeneration is intentionally not the optimization inner loop.
Profile one target's sheet instead:

```bash
./profile_sprites.sh --target oiler
./profile_sprites.sh --target ninja_heavy
./profile_sprites.sh --suite representative
```

The targeted script writes `.lprof`, `.txt`, `.log`, and `profile-index.txt`
files under `tools/ambition_sprite2d_renderer/.profiles/`. It renders sheets by
default, avoiding portraits, runtime installation, review galleries, factions,
LDtk work, and unrelated postconditions. Use `--product portraits` or
`--product publish` when those paths are the subject of the profile.

For an existing profile directory:

```bash
tools/ambition_sprite2d_renderer/.venv/bin/python \
    scripts/measure/render_line_profiles.py \
    tools/ambition_sprite2d_renderer/.profiles/<profile-directory>
```
