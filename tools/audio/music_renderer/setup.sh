sudo apt update && \
sudo apt install -y \
  ffmpeg \
  fluidsynth \
  timgm6mb-soundfont \
  fluid-soundfont-gm \
  fluid-soundfont-gs \
  libsndfile1

uv pip install numpy scipy pretty_midi PyYAML soundfile
