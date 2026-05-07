Music renderer refactor fix v11

Fixes the remaining v8/v10 TypeError:
  _inst_volume() takes 1 positional argument but 2 were given

Apply:
  cd /home/joncrall/code/ambition
  unzip -o ~/Downloads/ambition_music_renderer_refactor_fix_v11.zip
  python tools/audio/apply_music_renderer_refactor_fix_v11.py

Then rerun:
  ./generate_audio_assets.sh
