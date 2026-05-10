#!/usr/bin/env bash
# Re-render and republish all in-game music cues.
#
# Covers:
#   - first_goblin_tune_v2 (delegates to generate_audio_assets.sh, which
#     renders, audits, and installs the adaptive boss cue).
#   - All radio cues: scores/active/* (auto-discovered) plus the curated
#     EXTRA_RADIO_CUES list (example-tree cues we expose on the radio).
#     Driven via `ambition_music_renderer radio render-publish`.
#
# Usage:
#   ./regen_music.sh                # render + install everything (default)
#   ./regen_music.sh --skip-render  # only republish from existing renders
#
# Useful environment overrides:
#   AMBITION_MUSIC_BACKEND=pretty-midi|fluidsynth-cli|fallback|auto
#                                   # forwarded to generate_audio_assets.sh
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

renderer_dir="$repo_root/tools/ambition_music_renderer"
renderer_py="$renderer_dir/.venv/bin/python"

skip_render=0
for arg in "$@"; do
    case "$arg" in
        --skip-render) skip_render=1 ;;
        -h|--help) grep '^#' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "unknown arg: $arg" >&2; exit 2 ;;
    esac
done

if [ ! -x "$renderer_py" ]; then
    echo "music renderer venv missing: $renderer_py" >&2
    echo "run: (cd tools/ambition_music_renderer && ./setup.sh)" >&2
    exit 1
fi

echo "==> first_goblin_tune_v2 (delegating to generate_audio_assets.sh)"
if [ "$skip_render" -eq 1 ]; then
    bash "$repo_root/generate_audio_assets.sh" --skip-render
else
    bash "$repo_root/generate_audio_assets.sh"
fi

echo "==> radio cues (scores/active/* + EXTRA_RADIO_CUES, simple-mix)"
if [ "$skip_render" -eq 1 ]; then
    (cd "$renderer_dir" && "$renderer_py" -m ambition_music_renderer radio publish)
else
    (cd "$renderer_dir" && "$renderer_py" -m ambition_music_renderer radio render-publish)
fi

echo "==> done"
