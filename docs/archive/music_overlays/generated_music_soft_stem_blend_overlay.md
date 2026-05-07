# Generated music soft stem blend overlay v6

This follow-up patch keeps section stems synchronized by starting them at volume 0 in the same Bevy update tick, but changes the channel gain ramp from an absolute slew to a proportional/perceptual blend. The previous absolute slew made low-gain stems reach their final target in only a few frames, so new stems could sound like they appeared abruptly even when their final gain was small.

Key behavior:

- all stems in a new section still start together with `fade_ms=0`;
- channel volume now ramps toward targets with a roughly one-second time constant;
- logs now include `volume_blend=1.00s` on `started_section_stems`.

Apply:

```bash
cd /home/joncrall/code/ambition && \
unzip -o ~/Downloads/ambition_generated_music_soft_stem_blend_overlay_v6.zip && \
python tools/audio/apply_generated_music_soft_stem_blend_overlay_v6.py
```

Run:

```bash
cd /home/joncrall/code/ambition && \
RUST_LOG=ambition_generated_music=debug,symphonia_format_ogg=warn,info \
RUST_BACKTRACE=1 cargo run -p ambition_sandbox --bin ambition_sandbox \
  --features dev_hot_reload --release -- --start-room mob_lab
```
