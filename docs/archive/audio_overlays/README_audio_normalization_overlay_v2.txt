Audio normalization overlay v2

Fixes v1's apply-script SyntaxError by removing the misplaced future import.

Apply:
  cd /home/joncrall/code/ambition
  unzip -o ~/Downloads/ambition_audio_normalization_overlay_v2.zip
  python tools/audio/apply_audio_normalization_overlay_v1.py

Then regenerate/install:
  ./generate_audio_assets.sh
