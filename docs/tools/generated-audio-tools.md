
# Generated audio tools

The selected library declares its music and SFX producers. The root scripts
publish complete immutable bundles. Generated audio and registries are ignored
by Git. The library keeps shared SFX definitions and music naming policy in
`authored/`. Games can select no libraries.

```bash
./scripts/regen/music.sh
./scripts/regen/sfx.sh
python3 scripts/generate_library_content.py --check music
python3 scripts/generate_library_content.py --check sfx
```

These scripts are part of developer setup. A missing tool or stale product makes
setup fail. Publishing with `--skip-render` requires current renderer dependency
receipts. A failed build keeps the accepted audio. Playback retains bank asset
handles, so an accepted bank change can update the runtime without a Rust build.

Missing audio does not stop the game. Without the generated music registry,
Ambition's pack compiles with an `ungenerated-source` warning, registers no
music, and plays no music. Without the SFX bank, the bank load fails with a
logged error and the bank cues are silent. The SFX registry is authored
(`authored/runtime/audio/sfx_registry.ron`), so it is always present. To see
what is missing without a build:

```bash
python3 scripts/content_status.py --family music --family sfx
```

## Music renderer

Location: `tools/ambition_music_renderer/`

Purpose: render, publish, audit, and package generated/adaptive music cues and transition labs.

Common entry points:

```bash
cd ~/code/ambition
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer --help
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer cue bundle <cue_id> --backend=pretty-midi --force --zip
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer cue bundle <cue_id> --backend=pretty-midi --runtime_stem_gain_mode=shared --force --zip
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer cue bundle <cue_id> --backend=pretty-midi --runtime_stem_gain_mode=shared --zip_report --force
uv run --project ~/code/ambition/tools/ambition_music_renderer pytest -q tools/ambition_music_renderer/tests
```

The `cue bundle` subcommand is the preferred one-cue handoff path: it runs the renderer with retained debug stems, executes useful diagnostics, writes manifest-scoped reports/plots, prints clickable output paths, and optionally writes a full zip or compact report zip. Use `--zip_report` for small chat/agent uploads: it excludes OGG/WAV/NPY/MIDI binaries while keeping YAML, manifests, numeric reports, LLM-friendly spectral fingerprints, dissonance hotspot reports, state mix reports, and JPEG spectrograms. When plotting is available it also writes `dissonance_hotspots.md`, `plots/dissonance_timeline.<fmt>`, `plots/dissonance_layer_pairs.<fmt>`, and stem-amplitude balance/timeline/stack plots. Use `--runtime_stem_gain_mode=shared` when auditing layered dynamic music; shared gain is capped so the exporter does not hide quiet-source problems by amplifying noise floors. Generated bundles and runtime audio remain ignored by git.

Standalone report helpers are also useful while editing:

```bash
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer audit arrangement scores/active/<cue_id>.music.yaml --outdir=/tmp/<cue>_arrangement
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer audit dissonance scores/active/<cue_id>.music.yaml --outdir=/tmp/<cue>_dissonance --plots=/tmp/<cue>_dissonance/plots
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer audit spectral_localize tools/ambition_music_renderer/generated/<cue_id> --window 0 -1
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer audit reference_audio path/to/reference.mp3 --outdir=/tmp/reference_audio_audit
uv run --project ~/code/ambition/tools/ambition_music_renderer python -m ambition_music_renderer audit levels --check
```

See [`../recipes/generated-music-workflow.md`](../recipes/generated-music-workflow.md) for the current recipe.

### Which instruments a render actually used

The render cache is keyed by `(score YAML, backend)` and deliberately NOT by the
installed sample libraries — that is a multi-gigabyte tree, expensive to hash and
unstable to key on. The consequence is that **installing instruments cannot
invalidate a render**: a cue rendered before a library existed keeps its
General-MIDI audio, and `--force_render` re-renders everything rather than the
cues that would actually change.

So every render records what it resolved, in
`generated/<cue>/.versioned/<hash>/reports/instrument_fingerprint.json`, and this
compares that record against the libraries installed right now:

```bash
uv run --project tools/ambition_music_renderer python -m ambition_music_renderer audit instrument_drift
uv run --project tools/ambition_music_renderer python -m ambition_music_renderer audit instrument_drift --cue <cue_a>,<cue_b>
uv run --project tools/ambition_music_renderer python -m ambition_music_renderer audit instrument_drift --regen
```

It reports the instrument and both sides, and `--regen` re-renders only the cues
that would change:

```
DRIFTED  argand_overdrive
           solo_violin: SFZ unresolved (GM fallback) -> Violin Solo 1 Marcato.sfz
```

`unrecorded` means a render predates fingerprints — genuinely unknown, not
proven stale. `--backfill` establishes a baseline, but only for renders that
provably postdate the library tree; anything older is left alone rather than
inventing a record that would make a stale cue claim to be current. Exits
non-zero when anything has drifted, so it works as a pipeline check.

### Pruning old renders

`generated/<cue>/` keeps EVERY render ever made under `.versioned/<hash>/`, and
`agent/` keeps a full copy of each render that was bundled for a reviewer. One
orchestral render can exceed 700 MB. Nothing prunes them automatically.

```bash
python3 scripts/prune_generated_music.py            # dry run, items older than 2 days
python3 scripts/prune_generated_music.py --apply
python3 scripts/prune_generated_music.py --days 0 --cue my_cue --apply   # the drafts of one cue, now
```

An item goes when it is older than `--days` and it is one of these:

- **superseded**: a render that `latest` does not point at.
- **bundle**: `agent/<cue>_<hash>_bundle` and its zip files. The current render
  of each cue stays in `generated/`, thus its copy has no more use.
- **stems**: the raw group stems (`scratch_stems/`) in a render of a cue that
  has no score. They are most of the size of a render. The mastered audio
  stays, because a cue with no score cannot be rendered again.

Two options delete more:

- `--stems` also deletes the raw stems of the current render of each cue that
  has a score. The stem lab and the stem audits read them; render the cue again
  to get them back.
- `--orphans` deletes all renders of a cue that has no score, the mastered
  audio included. This cannot be undone.

⚠ **Dry run is the default and `--apply` is required to delete anything.** Keep
it that way. A cue whose `latest` symlink is missing keeps all its renders, and
the render that `building` points at stays. Do not use `--days 0` while a render
runs.

Everything it removes is gitignored. Old renders are also the only record of
how a cue sounded under previous instruments; see the drift section above
before you clear them if that comparison still matters.

## SFX renderer

Location: `tools/ambition_sfx_renderer/`

Purpose: render/audit generated sound effects and banks. Some checkouts contain this as a nested tool checkout; do not delete it accidentally just because it looks self-contained.

Common entry points:

```bash
cd tools/ambition_sfx_renderer
python -m ambition_sfx_renderer --help
python -m pytest tests
```

### Public samples

The SFX builder makes almost every cue. The exception is a recording that is
public and free to use, for example the Wilhelm scream (`voice.wilhelm_scream`).

- `tools/ambition_sfx_renderer/sounds/public_samples.yaml` is the one list. Each
  entry pins the URL, the sha256, the license, and the page that states the
  license.
- A cue uses an entry with a `sample` layer that has `public_sample: <id>`.
- The files are not in git. The renderer downloads an absent file into
  `tools/ambition_sfx_renderer/public_samples/` and refuses a file with a
  different sha256.
- `scripts/regen/sfx.sh` runs `fetch-samples` before it renders, so
  `scripts/setup/generated_content.sh` gets the files on a new clone. This step
  needs a network connection the first time.

To add one: add the entry, add the cue under `sounds/active/`, and run
`./scripts/regen/sfx.sh`. Add a recording only when its license permits free use
and redistribution.

## SFX packer

Location: `tools/ambition_sfx_pack/`

Purpose: pack sound effects into runtime banks.

## Policy

Generated audio becomes runtime input only after an explicit publish/install step. Update asset catalog docs when a cue/bank becomes a required runtime asset.
