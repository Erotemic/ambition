# Generated goblin encounter music overlay

This overlay wires the Python-generated first goblin encounter cue into the
Rust sandbox as rendered OGG assets. The integration is deliberately runtime-only:
Python/YAML remain the authoring/build pipeline; Rust loads OGG pieces from
`crates/ambition_sandbox/assets/audio/music/generated/first_goblin_encounter`.

Runtime structure:

- `intro`: full one-shot on encounter `Starting`
- `wave1`, `wave2`, `wave3`, `recap_loop`: loopable section/stem matrices
- `outro`: full one-shot after the fight clears, scheduled at the next 8-bar boundary

Stems:

- strings
- brass
- winds
- choir_pad
- mallets
- percussion

Gameplay mapping:

- Wave 1 starts sparse.
- Wave 2 adds motion/percussion.
- The delayed large brute in wave 2 fades brass/choir/percussion up.
- Wave 3 runs the full heavy layer set.
- On clear, aggressive stems fade down while strings/winds bridge into the outro.

The overlay also adds `--start-room` / `--room` so the sandbox can boot directly
into the mob lab encounter room.
