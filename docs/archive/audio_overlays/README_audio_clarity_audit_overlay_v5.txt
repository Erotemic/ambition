Audio clarity audit overlay v5

This is a targeted pass for:
- strings sounding underwater
- wave3 winds being faint / motif missing
- avoiding blind post-render normalization

Apply:
  cd /home/joncrall/code/ambition
  unzip -o ~/Downloads/ambition_audio_clarity_audit_overlay_v5.zip
  python tools/audio/apply_audio_clarity_audit_overlay_v5.py

Regenerate/install:
  ./generate_audio_assets.sh

Preview:
  ffplay -autoexit -nodisp tools/audio/music_renderer/output/first_goblin_tune_v2/preview/first_goblin_tune_v2_*.full_soundtrack_preview.ogg

Try full FluidSynth render if the fast string bank is still weird:
  AMBITION_MUSIC_BACKEND=fluidsynth-cli AMBITION_MUSIC_SIMPLE_GROUPS= ./generate_audio_assets.sh

Audit current generated balance:
  python tools/audio/audit_generated_cue_balance.py tools/audio/music_renderer/output/first_goblin_tune_v2
