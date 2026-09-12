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

The sheet table is baked AT COMPILE TIME (`ambition_sprite_sheet/build.rs`
scans `assets/sprites`), so an assetless tree compiles a binary with an EMPTY
table and fails tests that mention neither assets nor your change. **The tell is
BREADTH** — an empty oracle rejects every name at once, where a real content bug
names one or two.

| symptom | fix |
|---|---|
| one list naming EVERY move: *"no shipped FX spritesheet has a row by that name"* | `./scripts/regen/sprites.sh` |
| `expect("its manifest is baked")` on a portrait | `./scripts/regen/sprites.sh` |
| `error: couldn't read …/assets/fonts/bundled/InterDisplay-Regular.otf` | `python3 scripts/grab_font_assets.py` |
| art draws at a stale tier | `./scripts/regen/quality_variants.sh` |

A diagnostic that mutates nothing: `find crates/ambition_platformer2d_actor_monolith/assets/sprites -name '*_spritesheet.ron' | wc -l`.
Zero means the bake is empty. `./scripts/regen/sprites.sh --check-toolchain`
says whether this machine can render at all, and exits non-zero when it cannot.

⚠ Do not run a regen concurrently with a cargo build: one writes the files the
other is baking.

## ⚠ FONTS ARE NOT A CATEGORY HERE

`assets.sh` covers `backgrounds sprites variants music sfx` — **art and audio
only**. The three bundled UI fonts are a compile-time `include_bytes!`
dependency fetched by `python3 scripts/grab_font_assets.py`, so a machine that
ran the umbrella and nothing else still fails to BUILD `ambition_render`.
`scripts/setup/generated_content.sh` runs both and is the single command that
leaves nothing out.

## The umbrella, and the pieces

```bash
./scripts/regen/assets.sh                    # everything, in dependency order
./scripts/regen/assets.sh sprites variants   # just these, same order rules
```

`assets.sh` with no arguments runs `backgrounds sprites variants music sfx`.
`run_developer_setup.sh` calls exactly that, so **a full developer setup already
regenerates all of it** — see the trap below for how that can still leave a
machine with stale content.

| script | what it rebuilds | notes |
|---|---|---|
| `assets.sh` | the umbrella | takes category names; `--help` lists them |
| `backgrounds.sh` | procedural background / parallax families | |
| `sprites.sh` | the sprite suite and shared atlases | the long one |
| `quality_variants.sh` | reduced-resolution tiers of the above | `--sprites-only`, `--backgrounds-only`, `--tier`, `--target`, `--force` |
| `music.sh` | in-game music cues | needs the music renderer submodule |
| `sfx.sh` | SFX cues and the packed `.sfxbank` | needs the SFX renderer submodule |
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

`run_developer_setup.sh` now runs that check at the end and reports rather than
assuming. ⚠ Regeneration is INCREMENTAL — a sheet is rebuilt only when its
source or the generator is newer than its output — so re-running is cheap and
`--force` is the escape hatch when a freshness check is what you distrust.

Timings on `calculex` (i7-7700HQ, 6 threads): backgrounds ~3.5s, all 998 sprite
sheets ~175s.

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
