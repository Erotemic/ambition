#!/usr/bin/env bash
# Render and install the first_goblin_tune_v2 adaptive cue.
#
# Usage:
#   ./generate_audio_assets.sh               # render + install (default)
#   ./generate_audio_assets.sh --skip-render # only re-install from existing render
#
# Useful environment overrides:
#   AMBITION_MUSIC_BACKEND=pretty-midi|fluidsynth-cli|fallback|auto
#
# Output staging / preview:
#   tools/ambition_music_renderer/generated/first_goblin_tune_v2/
#
# Installed assets:
#   crates/ambition_sandbox/assets/audio/music/generated/first_goblin_tune_v2/
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

renderer_dir="$repo_root/tools/ambition_music_renderer"
spec="$renderer_dir/scores/active/first_goblin_tune_v2.music.yaml"
staging="$renderer_dir/generated/first_goblin_tune_v2"
installer="$renderer_dir/install_first_goblin_tune_v2.py"
auditor="$renderer_dir/audit_cue_balance.py"
backend="${AMBITION_MUSIC_BACKEND:-pretty-midi}"

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
    echo "==> rendering first_goblin_tune_v2"
    echo "    backend=$backend"
    rm -rf "$staging"
    mkdir -p "$staging"
    (
        cd "$renderer_dir"
        python -m ambition_music_renderer.render_isolated \
            "$spec" \
            --outdir "$staging" \
            --backend "$backend"
    )
fi

if [ ! -d "$staging/adaptive" ]; then
    echo "render output missing: $staging/adaptive" >&2
    exit 1
fi

echo "==> audit generated cue balance"
python "$auditor" "$staging" || true

echo "==> installing into crates/ambition_sandbox/assets/audio/music/generated/first_goblin_tune_v2"
python "$installer" --src "$staging" --clean

echo "==> previews:"
find "$staging/preview" -maxdepth 1 -type f -name '*.ogg' -print | sort

echo "==> done"
