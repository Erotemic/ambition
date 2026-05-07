Audio generation level overlay v4

This backs out the blind post-render normalizer and makes the renderer/YAML
produce louder OGGs directly.

Changes:
- rewrites ./generate_audio_assets.sh cleanly
- renderer output / preview goes to tools/audio/music_renderer/output/first_goblin_tune_v2/
- removes normalize_generated_audio_assets.py and overlay patch scripts if present
- patches first_goblin_tune_v2.music.yaml mix levels:
  - preview postprocess peak target -3 dB instead of -7 dB
  - stem postprocess gain 0 dB instead of -4.5 dB
  - modest group gain trims, preserving relative balance
- resets ADAPTIVE_MUSIC_RELATIVE_VOLUME to 1.0

Apply:
  cd /home/joncrall/code/ambition
  unzip -o ~/Downloads/ambition_audio_generation_level_overlay_v4.zip
  python tools/audio/apply_audio_generation_level_overlay_v4.py

Regenerate/install:
  ./generate_audio_assets.sh

Preview:
  ffplay -autoexit -nodisp tools/audio/music_renderer/output/first_goblin_tune_v2/preview/first_goblin_tune_v2_*.full_soundtrack_preview.ogg
