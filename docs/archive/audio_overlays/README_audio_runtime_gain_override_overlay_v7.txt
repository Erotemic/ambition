Audio runtime gain override overlay v7

v6 patched YAML but failed to patch hardcoded Rust cue-state gains. This overlay
patches the generic gains_for_state() function instead, adding a targeted runtime
override for first_goblin_tune_v2 states.

Apply:
  cd /home/joncrall/code/ambition
  unzip -o ~/Downloads/ambition_audio_runtime_gain_override_overlay_v7.zip
  python tools/audio/apply_audio_runtime_gain_override_overlay_v7.py

Then:
  ./generate_audio_assets.sh
  cargo check -p ambition_sandbox --bin ambition_sandbox --features dev_hot_reload

Run:
  RUST_LOG=ambition_music=debug,symphonia_format_ogg=warn,info \
  RUST_BACKTRACE=1 cargo run -p ambition_sandbox --bin ambition_sandbox \
    --features dev_hot_reload --release -- --start-room mob_lab

Inspect patch:
  grep -n "apply_first_goblin_runtime_balance_overrides" crates/ambition_sandbox/src/music.rs
