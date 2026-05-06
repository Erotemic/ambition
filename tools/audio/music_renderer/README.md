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

Fast path, for quick preview

```bash
cd ~/code/ambition/tools/audio/music_renderer

# Song 1
python -m ambition_music_renderer.render_isolated \
  examples/first_goblin_tune_v2.music.yaml \
  --outdir output/first_goblin_tune_v2 \
  --backend fast

# Song 0
python -m ambition_music_renderer.render_isolated \
  examples/first_goblin_encounter.music.yaml \
  --outdir output/first_goblin_encounter \
  --backend fast


# Song 2
python -m ambition_music_renderer.render_isolated \
  examples/fast_paced_violin_boss.music.yaml \
  --outdir output/fast_paced_violin_boss \
  --backend fast
```

For higher fidelity, swap `--backend fast` for `--backend fluidsynth-cli`
(or `--backend pretty-midi`). Both require a General MIDI SoundFont; the
default-installed `TimGM6mb.sf2` is fine, but `FluidR3_GM.sf2` is fuller.

## Score files

- `examples/first_goblin_tune_v2.music.yaml` - active goblin encounter cue.
- `examples/first_goblin_encounter.music.yaml` - earlier goblin score (kept for reference).
- `examples/moonlit_canal.music.yaml` - sample non-combat cue.

## Stable filenames

`render_isolated` writes hash-suffixed filenames so re-rendering does not
silently replace assets. The repo installer
(`tools/audio/install_first_goblin_tune_v2_assets.py`) copies those files into
the Bevy asset tree under stable names like
`adaptive/wave1/wave1.strings.ogg`, which the Rust loader targets directly.
