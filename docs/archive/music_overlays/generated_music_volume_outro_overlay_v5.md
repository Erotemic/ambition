# Generated music volume/outro overlay v5

This overlay builds on the generated goblin music runtime integration and the
logging/sync overlay.

Changes:

- Scales generated adaptive music by `GENERATED_MUSIC_RELATIVE_VOLUME = 0.36`
  after the user's music volume. This is intended to put the stacked generated
  stems closer to the legacy room music loudness.
- Allows the bridge/outro to continue if the encounter has already moved from
  `Cleared` back to `Inactive`.
- Resumes default room music near the end of the outro rather than immediately
  after the clear event.
- Slightly lengthens section crossfades to reduce abrupt changes.

Run with:

```bash
RUST_LOG=ambition_generated_music=debug,symphonia_format_ogg=warn,info \
RUST_BACKTRACE=1 cargo run -p ambition_sandbox --bin ambition_sandbox \
  --features dev_hot_reload --release -- --start-room mob_lab
```

If the generated cue is still too loud, lower `GENERATED_MUSIC_RELATIVE_VOLUME`
in `crates/ambition_sandbox/src/generated_music.rs`.
