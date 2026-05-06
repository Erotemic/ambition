#!/usr/bin/env bash
# Render and install the first_goblin_tune_v2 adaptive cue.
#
# Usage:
#   ./generate_audio_assets.sh               # render + install (default)
#   ./generate_audio_assets.sh --skip-render # only re-install from existing render
#
# Useful environment overrides:
#   AMBITION_MUSIC_BACKEND=fast|fluidsynth-cli
#   AMBITION_MUSIC_SIMPLE_GROUPS=choir_pad
#
# Output staging / preview:
#   tools/audio/music_renderer/output/first_goblin_tune_v2/
#
# Installed assets:
#   crates/ambition_sandbox/assets/audio/music/generated/first_goblin_tune_v2/
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

renderer_dir="$repo_root/tools/audio/music_renderer"
spec="$renderer_dir/examples/first_goblin_tune_v2.music.yaml"
staging="$renderer_dir/output/first_goblin_tune_v2"
installer="$repo_root/tools/audio/install_first_goblin_tune_v2_assets.py"
backend="${AMBITION_MUSIC_BACKEND:-fast}"
simple_groups="${AMBITION_MUSIC_SIMPLE_GROUPS:-}"

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
    echo "    simple_groups=$simple_groups"
    rm -rf "$staging"
    mkdir -p "$staging"
    render_args=(
        "$spec"
        --outdir "$staging"
        --backend "$backend"
    )
    if [ -n "$simple_groups" ]; then
        render_args+=(--simple-groups "$simple_groups")
    fi
    (
        cd "$renderer_dir"
        python -m ambition_music_renderer.render_isolated "${render_args[@]}"
    )
fi

if [ ! -d "$staging/adaptive" ]; then
    echo "render output missing: $staging/adaptive" >&2
    exit 1
fi

echo "==> audit generated cue balance"
python "$repo_root/tools/audio/audit_generated_cue_balance.py" "$staging" || true

echo "==> installing into crates/ambition_sandbox/assets/audio/music/generated/first_goblin_tune_v2"
python "$installer" --src "$staging" --clean

echo "==> preview:"
find "$staging/preview" -maxdepth 1 -type f -name '*.full_soundtrack_preview.ogg' -print | sort | tail -n 1

echo "==> done"
