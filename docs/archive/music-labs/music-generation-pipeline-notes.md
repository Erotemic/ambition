# Music generation pipeline notes

`scripts/regen_first_goblin_tune_v2.sh` is the focused helper for one adaptive
cue: `first_goblin_tune_v2`.

The default path now matches what the game actually loads for that cue:

```bash
./scripts/regen_first_goblin_tune_v2.sh --force
```

That renders:

- the mastered full soundtrack preview;
- per-section `*.full.ogg` adaptive files for intro/wave/outro playback;
- the adaptive manifest.

It skips per-stem OGG encodes by default because the Rust cue spec plays the
per-section full mixes directly. This keeps iteration faster and avoids filling
the installed asset tree with stems that are not currently used by the runtime.
Use this only when you want to inspect or restore stem-driven runtime playback:

```bash
./scripts/regen_first_goblin_tune_v2.sh --force --with-stems
```

## Scratch `.npy` files

The isolated renderer still writes temporary `.npy` buffers under
`scratch_stems/` while a render is running. Those files are the worker-process
handoff format: each stem worker renders audio in its own process, writes one
buffer, and the parent process loads those buffers to assemble the full mix and
per-section full mixes.

By default, the buffers are deleted after a successful render. Keep them only
for debugging:

```bash
./scripts/regen_first_goblin_tune_v2.sh --force --keep-debug-stems
```

So the current speedup is not "avoid every `.npy` write"; it is "avoid the
unused per-stem OGG encodes and install only the full mixes that the game uses."
A future renderer could replace the `.npy` handoff with shared memory or pipes,
but the current file handoff is simple and robust.

## Goblin cue balance direction

The current first-goblin cue should sound dry and continuous across the
intro-to-wave1 handoff. Avoid using long baked fades to hide section boundaries;
the runtime owns the adaptive handoff. Instead, render sections as bar-aligned,
cut/loop-friendly full mixes with small de-click tails, matched peak/loudness,
and a low noise floor.
