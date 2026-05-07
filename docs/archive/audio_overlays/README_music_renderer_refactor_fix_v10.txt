Music renderer refactor fix v10

Fixes the v9 TypeError:
  _inst_pan() takes 1 positional argument but 2 were given

Apply:
  cd /home/joncrall/code/ambition
  unzip -o ~/Downloads/ambition_music_renderer_refactor_fix_v10.zip
  python tools/audio/apply_music_renderer_refactor_fix_v10.py

Then rerun:
  ./generate_audio_assets.sh
