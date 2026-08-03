#!/usr/bin/env bash
# Generate reduced-resolution sprite and parallax assets after full-res publish.
#
# Usage:
#   ./regen_visual_quality_variants.sh
#   ./regen_visual_quality_variants.sh --sprites-only
#   ./regen_visual_quality_variants.sh --backgrounds-only
#
# Environment:
#   AMBITION_SPRITE_PYTHON=/path/to/python  Override the sprite tool .venv.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

renderer_dir="$repo_root/tools/ambition_sprite2d_renderer"
asset_root="$repo_root/crates/ambition_platformer2d_actor_monolith/assets"

# shellcheck disable=SC1091
source "$repo_root/scripts/lib/tool_python.sh"

print_help() {
    awk '
        NR == 1 { next }
        /^set -euo pipefail$/ { exit }
        /^#$/ { print ""; next }
        /^# / { sub(/^# /, ""); print }
    ' "$0"
}

sprites_only=0
backgrounds_only=0
args=()
for arg in "$@"; do
    case "$arg" in
        --sprites-only)
            sprites_only=1
            args+=("$arg")
            ;;
        --backgrounds-only)
            backgrounds_only=1
            args+=("$arg")
            ;;
        -h|--help) print_help; exit 0 ;;
        *) echo "unknown arg: $arg" >&2; exit 2 ;;
    esac
done

if [ "$sprites_only" -eq 1 ] && [ "$backgrounds_only" -eq 1 ]; then
    echo "--sprites-only and --backgrounds-only are mutually exclusive" >&2
    exit 2
fi

python_bin="$(ambition_select_tool_python "$renderer_dir" AMBITION_SPRITE_PYTHON)"
ambition_require_python_module \
    "$python_bin" ambition_sprite2d_renderer \
    "run ./run_developer_setup.sh or set AMBITION_SPRITE_PYTHON=/path/to/python"

"$python_bin" "$repo_root/scripts/generate_visual_quality_variants.py" \
    --asset-root "$asset_root" "${args[@]}"

# ⛔ these named `player_robot_spritesheet.ron` until 2026-08-02, and the player
# sheet became `player_robot_v3` well before that. So the postcondition could
# only ever FAIL: every sprite run did its work, printed its three summary
# lines, and then exited 1 on a sheet that stopped existing. A guard that
# cannot pass is not weaker than no guard, it is worse — it trains the reader
# to ignore the exit code of the thing that publishes their assets.
if [ "$backgrounds_only" -eq 0 ]; then
    test -s "$asset_root/sprites_0_5x/player_robot_v3_spritesheet.ron" || {
        echo "sprite quality generation did not produce the half-resolution player sheet" >&2
        exit 1
    }
    test -s "$asset_root/sprites_potato/player_robot_v3_spritesheet.ron" || {
        echo "sprite quality generation did not produce the potato player sheet" >&2
        exit 1
    }
fi
if [ "$sprites_only" -eq 0 ]; then
    test -s "$asset_root/backgrounds/parallax_layers_0_5x/hub_sky.png" || {
        echo "background quality generation did not produce the half-resolution hub sky" >&2
        exit 1
    }
    test -s "$asset_root/backgrounds/parallax_layers_potato/hub_sky.png" || {
        echo "background quality generation did not produce the potato hub sky" >&2
        exit 1
    }
fi

echo "==> done"
