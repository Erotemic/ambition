Music renderer refactor overlay v8

Apply:
  cd /home/joncrall/code/ambition
  unzip -o ~/Downloads/ambition_music_renderer_refactor_overlay_v8.zip
  python tools/audio/apply_music_renderer_refactor_overlay_v8.py

Regenerate/install:
  ./generate_audio_assets.sh

Preview:
  ffplay -autoexit -nodisp tools/audio/music_renderer/output/first_goblin_tune_v2/preview/first_goblin_tune_v2_*.full_soundtrack_preview.ogg

Audit:
  python tools/audio/audit_generated_cue_balance.py tools/audio/music_renderer/output/first_goblin_tune_v2

Compile:
  cargo check -p ambition_sandbox --bin ambition_sandbox --features dev_hot_reload

Run:
  RUST_LOG=ambition_music=debug,symphonia_format_ogg=warn,info \
  RUST_BACKTRACE=1 cargo run -p ambition_sandbox --bin ambition_sandbox \
    --features dev_hot_reload --release -- --start-room mob_lab
