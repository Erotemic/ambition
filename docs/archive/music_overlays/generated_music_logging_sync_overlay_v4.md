# Generated music logging/sync overlay v4

This overlay is intended to be applied after the generated goblin music runtime
integration and the smooth-transition fix.

Changes:

- Adds structured logging under `ambition_generated_music`.
- Logs cue asset loading, intro start, queued/fired loop transitions, loop-bank
  crossfades, stem starts, outro requests, outro starts, default-music resume,
  shutdown, and periodic state summaries.
- Starts all loop-section stems with channel volume at zero and with no per-stem
  fade-in. The crossfade is now driven by shared bank gain smoothing, which
  reduces one source of perceived stem desynchronization.

Run with logs:

```bash
cd /home/joncrall/code/ambition && \
RUST_LOG=ambition_generated_music=debug,info \
RUST_BACKTRACE=1 cargo run -p ambition_sandbox --bin ambition_sandbox \
  --features dev_hot_reload --release -- --start-room mob_lab
```

Useful log lines to inspect:

- `start_intro`
- `queue_loop_transition`
- `fire_loop_transition`
- `begin_loop_crossfade`
- `started_section_stems`
- periodic `state` lines with `bar_beat`, `active_bank`, and gains

If stems still sound out of phase while logs show that all section stems are
started in the same system tick, the likely causes are in the rendered assets
rather than the Rust state machine: mismatched leading silence, OGG encoder / decoder
padding differences, unequal stem durations, or loop tails that were not wrapped
identically per stem.
