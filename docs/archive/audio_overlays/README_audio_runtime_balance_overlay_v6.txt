Audio runtime balance overlay v6

This pass is based on the audit output:
- full wave3 preview: RMS about -19 dB
- wave3 strings stem: RMS about -32 dB
- wave3 winds stem: RMS about -40 dB

It fixes two things:
1. Raises source-generated stem levels in the YAML instead of post-normalizing.
2. Patches Rust MusicDirector cue state gains, because the YAML state_map is not
   currently parsed by Rust.

Apply:
  cd /home/joncrall/code/ambition
  unzip -o ~/Downloads/ambition_audio_runtime_balance_overlay_v6.zip
  python tools/audio/apply_audio_runtime_balance_overlay_v6.py

Regenerate/install:
  ./generate_audio_assets.sh

Preview:
  ffplay -autoexit -nodisp tools/audio/music_renderer/output/first_goblin_tune_v2/preview/first_goblin_tune_v2_*.full_soundtrack_preview.ogg

Check compile:
  cargo check -p ambition_sandbox --bin ambition_sandbox --features dev_hot_reload

Run:
  RUST_LOG=ambition_music=debug,symphonia_format_ogg=warn,info \
  RUST_BACKTRACE=1 cargo run -p ambition_sandbox --bin ambition_sandbox \
    --features dev_hot_reload --release -- --start-room mob_lab
