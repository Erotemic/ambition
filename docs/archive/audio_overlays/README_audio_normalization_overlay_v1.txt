Audio normalization overlay v1

Adds tools/audio/normalize_generated_audio_assets.py and patches
./generate_audio_assets.sh to normalize generated OGG files after rendering and
before installing. It also resets ADAPTIVE_MUSIC_RELATIVE_VOLUME in music.rs to
1.0 so Rust-side volume stays a mix trim rather than a fake mastering knob.

Apply:
  cd /home/joncrall/code/ambition
  unzip -o ~/Downloads/ambition_audio_normalization_overlay_v1.zip
  python tools/audio/apply_audio_normalization_overlay_v1.py

Then regenerate/install:
  ./generate_audio_assets.sh

Preview:
  ffplay -autoexit -nodisp target/generated-audio/first_goblin_tune_v2/preview/first_goblin_tune_v2_*.full_soundtrack_preview.ogg

Run:
  RUST_LOG=ambition_music=debug,symphonia_format_ogg=warn,info \
  RUST_BACKTRACE=1 cargo run -p ambition_sandbox --bin ambition_sandbox \
    --features dev_hot_reload --release -- --start-room mob_lab
