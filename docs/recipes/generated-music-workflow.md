---
status: current
last_verified: 2026-11-10
related_docs:
  - docs/concepts/generated-assets-audio.md
  - docs/tools/generated-audio-tools.md
  - docs/systems/audio-and-vfx.md
---

# Generated music workflow

Music source specifications and renderer code are the source of truth. Rendered
files are review/runtime artifacts produced by an explicit render or publish
step; they are not hand-edited.

## Inspect the current CLI

```bash
uv run --project tools/ambition_music_renderer \
  python -m ambition_music_renderer --help
uv run --project tools/ambition_music_renderer \
  python -m ambition_music_renderer cue --help
```

Prefer CLI help and the tool-local README over old copied flag lists.

## One-cue loop

```bash
CUE=<cue_id>

uv run --project tools/ambition_music_renderer \
  python -m ambition_music_renderer cue bundle "$CUE" \
  --backend=pretty-midi --force --zip_report
```

A report bundle is the normal review handoff: source/manifest, numeric reports,
spectral/dissonance diagnostics, and compact plots without large audio binaries.
Use a full bundle when the reviewer needs audio. Add the current explicit
publish option only when the cue should update provider runtime assets.

## Diagnose before changing runtime

1. Render only the affected cue.
2. Inspect arrangement/dissonance/stem-balance reports.
3. Listen to adjacent sections outside the game.
4. Reproduce through the provider room/encounter and capture semantic music
   director logs.
5. Change runtime transition policy only if the generated sources are already
   coherent and logs show a state/queue/gain error.

Do not use runtime gain to hide an under-authored source, clipped master, noisy
stem, or bad phrase boundary.

## Provider registration

Named cue IDs and encounter/room bindings belong to provider assets/registries.
The reusable audio crate owns App-local catalogs, state selection, loading,
mixing, web unlock, and output. Simulation emits semantic music/SFX intent; it
must not know filenames or renderer internals.

## Check the result

For a cue edit, render and listen to the affected sections, inspect its bundle
report, and hear it in the owning room or encounter. Use the level audit when
cue bindings or coverage changed:

```bash
uv run --project tools/ambition_music_renderer \
  python -m ambition_music_renderer audit levels --check
```

Run the renderer's tests when its code or format changes. Run focused audio or
content tests when their loading, registration, or transition contracts change.
Use a browser check only for web audio/device changes.
