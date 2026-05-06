#!/usr/bin/env bash
# Render and install the first_goblin_tune_v2 adaptive cue.
#
# Usage:
#   ./generate_audio_assets.sh             # render + install (default)
#   ./generate_audio_assets.sh --skip-render   # only re-install from existing render
#
# Output staging:    target/generated-audio/first_goblin_tune_v2/
# Installed assets:  crates/ambition_sandbox/assets/audio/music/generated/first_goblin_tune_v2/
#
# Neither directory is committed (target/ is gitignored, the assets dir is too).
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

renderer_dir="$repo_root/tools/audio/music_renderer"
spec="$renderer_dir/examples/first_goblin_tune_v2.music.yaml"
staging="$repo_root/target/generated-audio/first_goblin_tune_v2"
installer="$repo_root/tools/audio/install_first_goblin_tune_v2_assets.py"

skip_render=0
for arg in "$@"; do
    case "$arg" in
        --skip-render) skip_render=1 ;;
        -h|--help)
            grep '^#' "$0" | sed 's/^# \{0,1\}//'
            exit 0 ;;
        *) echo "unknown arg: $arg" >&2; exit 2 ;;
    esac
done

if [ ! -f "$spec" ]; then
    echo "spec not found: $spec" >&2
    exit 1
fi

if [ "$skip_render" -eq 0 ]; then
    echo "==> rendering first_goblin_tune_v2 (this can take a few minutes)"
    rm -rf "$staging"
    mkdir -p "$staging"
    (
        cd "$renderer_dir"
        python -m ambition_music_renderer.render_isolated \
            "$spec" \
            --outdir "$staging" \
            --backend fast \
            --simple-groups strings,choir_pad
    )
fi

if [ ! -d "$staging/adaptive" ]; then
    echo "render output missing: $staging/adaptive" >&2
    exit 1
fi

echo "==> installing into crates/ambition_sandbox/assets/audio/music/generated/first_goblin_tune_v2"
python "$installer" --src "$staging" --clean

echo "==> done"
