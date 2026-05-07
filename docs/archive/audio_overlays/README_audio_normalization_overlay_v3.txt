Audio normalization overlay v3

Replaces the normalizer with an ffmpeg-based implementation. This avoids the
libsndfile/Vorbis segfault seen in v1/v2 when Python soundfile rewrites OGGs.

Apply:
  cd /home/joncrall/code/ambition
  unzip -o ~/Downloads/ambition_audio_normalization_overlay_v3.zip
  python tools/audio/apply_audio_normalization_overlay_v3.py

Regenerate/install:
  ./generate_audio_assets.sh

Temporary workaround without normalization:
  ./generate_audio_assets.sh --skip-normalize
