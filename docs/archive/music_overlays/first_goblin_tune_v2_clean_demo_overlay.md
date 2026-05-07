# first_goblin_tune_v2 clean demo overlay

This overlay is a revision pass for the adaptive first goblin proof-of-concept.

Goals:

- keep the intro / stemmed loop sections / outro structure;
- simplify the score so wave 2 has a clear but clean intensity lift;
- remove noisy hat/crash/shaker writing from the demo cue;
- reduce fast-renderer buzz/popping in strings, winds, choir/pad, and percussion;
- stagger wave 2 spawns: first striker, second striker shortly after, brute later;
- keep the outro transition short enough for a demo.

The overlay replaces:

- `tools/audio/goblin_orchestra_renderer-v8/goblin_orchestra_renderer/examples/first_goblin_tune_v2.music.yaml`
- `tools/audio/goblin_orchestra_renderer-v8/goblin_orchestra_renderer/ambition_music_renderer/musicir_renderer.py`
- `tools/audio/install_first_goblin_tune_v2_assets.py`

Run the installer after rendering; it patches `generated_music.rs` and `encounter.rs` using the generated manifest hash.
