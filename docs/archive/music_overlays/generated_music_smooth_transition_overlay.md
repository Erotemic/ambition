# Generated music smooth-transition overlay

This follow-up overlay assumes the initial generated-music integration overlay is
already applied/committed. It replaces `generated_music.rs` with a smoother
controller and registers a second bank of Kira channels.

## What changed

- Two stem banks are used for real section crossfades.
- Wave-to-wave music changes are queued to the next bar.
- Clear-to-outro is queued to a 4-bar phrase marker while combat stems fade into
  a quieter bridge.
- The default/room music resumes near the end of the generated outro.
- The normal run command must specify `--bin ambition_sandbox`.

## Apply

```bash
cd /home/joncrall/code/ambition && \
  unzip -o ~/Downloads/ambition_generated_music_smooth_transition_overlay_v2.zip && \
  python tools/audio/apply_generated_music_smooth_transition_overlay.py
```

## Run

```bash
cd /home/joncrall/code/ambition && \
RUST_BACKTRACE=1 cargo run -p ambition_sandbox --bin ambition_sandbox \
  --features dev_hot_reload --release -- --start-room mob_lab
```

