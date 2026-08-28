
# Generated audio tools

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

### Pruning superseded renders

`generated/<track>/` keeps EVERY render ever made under `.versioned/<hash>/`,
and a single orchestral render can exceed 700 MB. Nothing prunes them
automatically, so this tree grows without bound.

```bash
python3 scripts/prune_generated_music.py                 # dry run, renders older than 14 days
python3 scripts/prune_generated_music.py --days 0        # dry run, everything except each track's newest
python3 scripts/prune_generated_music.py --days 0 --apply
```

The rule is one line: **if `latest` does not point at it and it is older than
`--days`, it goes**, along with the matching `agent/<track>_<hash>_bundle*`
files. `--days 0` is the "keep only the newest render per track" mode; the
14-day default instead keeps recent supersedes around for A/B comparison, which
is usually what you want mid-authoring.

⚠ **Dry run is the default and `--apply` is required to delete anything.** Keep
it that way. A track whose `latest` symlink is missing is skipped entirely
rather than guessed at, because a half-rendered track has no defensible "newest".

Everything it removes is gitignored and reproduced by re-rendering the score —
but note that re-rendering a cue whose score no longer exists is impossible, so
prune retired tracks knowingly. Old renders are also the only surviving record
of how a cue sounded under previous instruments; see the drift section above
before clearing them if that comparison still matters.

## SFX renderer

Location: `tools/ambition_sfx_renderer/`

Purpose: render/audit generated sound effects and banks. Some checkouts contain this as a nested tool checkout; do not delete it accidentally just because it looks self-contained.

Common entry points:

```bash
cd tools/ambition_sfx_renderer
python -m ambition_sfx_renderer --help
python -m pytest tests
```

## SFX packer

Location: `tools/ambition_sfx_pack/`

Purpose: pack sound effects into runtime banks.

## Policy

Generated audio becomes runtime input only after an explicit publish/install step. Update asset catalog docs when a cue/bank becomes a required runtime asset.
