# `scripts/regen/` — content regeneration

Every script here rebuilds generated content from its authoring source. They are
all independently runnable, they all take `--help`, and they are all composed by
one umbrella so a fresh machine gets the lot without knowing any of their names.

## Why these outputs are gitignored

⭐⭐ **The megabytes are DERIVED; git carries the small authored sources.** A
renderer target is a few KB of Python or config; its published sheet is a PNG
plus metadata. A cue is a `.music.yaml`; its render is audio. Keeping the inputs
under version control means they diff, review and merge like code, while the
large outputs stay reproducible rather than committed — so "the asset is
missing" is never data loss, it is a build that has not run here yet.

⇒ That also fixes where to make a change: **edit the source and re-render.**
Hand-editing a published `.png`/`.ron`/audio file produces work the next regen
silently discards.

## ⛔ If a test fails in a crate you did not touch, check for missing assets FIRST

The sheet table comes from the accepted art publication. A development build
reads its immutable files at runtime. A static build embeds the same selected
metadata. See [sprite publication](../../docs/tools/sprite-publication.md) for
planning, generation, exact membership, and unmanaged files.

| symptom | fix |
|---|---|
| one list naming EVERY move: *"no shipped FX spritesheet has a row by that name"* | `./scripts/regen/sprites.sh` |
| `expect("its manifest is baked")` on a portrait | `./scripts/regen/sprites.sh` |
| `error: couldn't read …/assets/fonts/bundled/InterDisplay-Regular.otf` | `python3 scripts/grab_font_assets.py` |
| art draws at a stale tier | `./scripts/regen/quality_variants.sh` |

Read the selected sprite products with `./scripts/regen/sprites.sh --check`.
Use `--plan --json` to list definitions and `--audit --json` to inspect old loose
files. `./scripts/regen/sprites.sh --check-toolchain` checks the renderer setup.

⚠ Do not run a regen concurrently with a `static_content` build or with a
running test: one writes the files the other is reading.

## Selected library and accepted products

The `producers` table in `game/ambition_content_library/library.json` selects
fonts, backgrounds, sprites, quality variants, music, and SFX. Set
`AMBITION_CONTENT_LIBRARY` to use another library manifest.

Each producer writes to a private staging root. The existing immutable store
accepts a complete result. A failed build keeps the accepted selection. Runtime
readers and packages use that selection. Old loose outputs do not define runtime
membership within a published producer's declared roots.

```bash
python3 scripts/generate_library_content.py --check        # all declared media
python3 scripts/generate_library_content.py --check music  # one family
./scripts/regen/sprites.sh --check                         # sprite bundles
python3 scripts/check_quality_variants_are_fresh.py        # quality products
```

Freshness includes source files, generator code, tool versions, and the audio
renderer's dependency identities. Audio generation can reuse current renders.
`--skip-render` refuses stale audio. Review reports and conversion receipts stay
in review roots. They are not runtime package members.

## The umbrella, and the pieces

```bash
./scripts/regen/assets.sh                    # everything, in dependency order
./scripts/regen/assets.sh sprites variants   # just these, same order rules
```

`assets.sh` with no arguments runs every declared producer in dependency order.
`run_developer_setup.sh` calls `scripts/setup/generated_content.sh`, which builds
the validator, runs the producers, and checks accepted products. A required
producer or freshness failure makes setup fail.

| script | what it rebuilds | notes |
|---|---|---|
| `assets.sh` | the umbrella | takes category names; `--help` lists them |
| `../grab_font_assets.py` | declared fonts and licenses | archive hashes are pinned in library policy |
| `backgrounds.sh` | procedural background / parallax families | |
| `sprites.sh` | the sprite suite and shared atlases | the long one |
| `quality_variants.sh` | reduced-resolution tiers of the above | `--sprites-only`, `--backgrounds-only`, `--tier`, `--target`, `--force` |
| `music.sh` | in-game music cues | needs the music renderer submodule |
| `sfx.sh` | SFX cues and the packed `.sfxbank` | needs the SFX renderer submodule; downloads the public samples that are absent |
| `source_navigation.sh` | the generated `.agent/` navigation index | not content; regenerates agent aids |

## ⛔ The trap: "the setup ran" is not "the content is current"

Measured on `calculex`, 2026-08-29: **998 sprite sheets ~11 days stale**, and
`backgrounds/parallax_layers_0_5x` / `_0_25x` / `_potato` did not exist at all.
The game therefore drew OLD art at Low/Medium and NO parallax art at all — so the
tiers meant for weak hardware were the ones most likely to be broken, on the
machines least able to notice.

⭐ **The checker already existed and nothing called it.**

```bash
python3 scripts/check_quality_variants_are_fresh.py   # says exactly what is stale
./scripts/regen/quality_variants.sh                   # fixes it, incrementally
```

`run_developer_setup.sh` runs that check at the end. A stale result makes setup
fail. ⚠ Regeneration is INCREMENTAL — a sprite bundle is rebuilt when its
recipe or an accepted member has changed — so re-running is cheap and
`--force` is the escape hatch when a freshness check is what you distrust.

Timings on `calculex` (i7-7700HQ, 6 threads): backgrounds ~3.5s, all 998 sprite
sheets ~175s.

## Coverage checks

Run `python3 scripts/check_regeneration_coverage.py` to check the sprite publish
plan, the music roster and files, the SFX bank, and generated scenery files.
Use `--family sprites` (or `music`, `sfx`, `backgrounds`) for one family.
Use `--plans-only` when runtime assets are absent. This mode checks source
coverage, not published file presence. The repository tooling tests run this
mode. The asset umbrella checks published files after regeneration. The music
script also checks its published files and registry.

Active music scores define registry entries even before a render exists. A
missing render must fail the coverage check; it must not remove the track from
the registry.

## Adding one

Keep the shape the others have, because it is what makes them composable:

1. `--help` prints the header comment (the `print_help` awk idiom the others use
   — the file's own comment block is the usage text, so they cannot drift apart).
2. Compute `repo_root` two levels up — ⚠ these live in `scripts/regen/`, not at
   the repo root, and a script that gets this wrong writes its output into
   `scripts/regen/` and silently produces nothing the game will load.
3. Be incremental by default and offer `--force`.
4. Register it in `assets.sh`'s category list if a fresh machine needs it, and
   in the table above either way.
