# Ambition MusicIR Renderer

Code-only Python tool that renders Ambition MusicIR YAML scores into adaptive
OGG stems (intro / wave loops / outro, six instrument groups).

This package is the canonical audio generator. **No rendered `.ogg`, `.wav`,
or `.mid` is committed.** Generate assets locally with the repo-root script:

```bash
./generate_audio_assets.sh
```

That script renders `first_goblin_tune_v2` into
`target/generated-audio/first_goblin_tune_v2/` and installs the stable,
hash-free filenames the Rust runtime expects under
`crates/ambition_sandbox/assets/audio/music/generated/first_goblin_tune_v2/`.

## Manual rendering

```bash
cd tools/audio/music_renderer
python -m ambition_music_renderer.render_isolated \
    examples/first_goblin_tune_v2.music.yaml \
    --outdir output/first_goblin_tune_v2 \
    --backend fast \
    --simple-groups strings,choir_pad
```

For the direct-MIDI diagnostic cue (high-fidelity, slower):

```bash
python -m ambition_music_renderer.render_direct_midi_yaml \
    examples/diagnostics/goblin-short-v1.direct-midi.music.yaml \
    --outdir output/goblin-short-v1 \
    --backend fluidsynth-cli
```

## Score files

- `examples/first_goblin_tune_v2.music.yaml` - active goblin encounter cue.
- `examples/first_goblin_encounter.music.yaml` - earlier goblin score (kept for reference).
- `examples/moonlit_canal.music.yaml` - sample non-combat cue.
- `examples/diagnostics/goblin-short-v1.direct-midi.music.yaml` - mix-tuning fixture.

## Stable filenames

`render_isolated` writes hash-suffixed filenames so re-rendering does not
silently replace assets. The repo installer
(`tools/audio/install_first_goblin_tune_v2_assets.py`) copies those files into
the Bevy asset tree under stable names like
`adaptive/wave1/wave1.strings.ogg`, which the Rust loader targets directly.
