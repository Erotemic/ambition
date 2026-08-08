#!/usr/bin/env bash
# Generate reduced-resolution sprite and parallax assets after full-res publish.
#
# Work is incremental by default: a tier is rebuilt for a sheet only when the
# authored sheet, one of its page PNGs, or the generator itself is newer than
# what was published. A full run with nothing changed costs seconds.
#
# Usage:
#   ./regen_visual_quality_variants.sh
#   ./regen_visual_quality_variants.sh --sprites-only
#   ./regen_visual_quality_variants.sh --backgrounds-only
#   ./regen_visual_quality_variants.sh --target patent_clerk        # one character, every tier
#   ./regen_visual_quality_variants.sh --target 'pirate_*' --tier 0_5x
#   ./regen_visual_quality_variants.sh --force                      # ignore the freshness check
#   ./regen_visual_quality_variants.sh --clean                      # empty each tier root first
#
#   --target takes an fnmatch pattern and repeats; --tier is one of
#   0_5x / 0_25x / potato and repeats.
#
# Environment:
#   AMBITION_SPRITE_PYTHON=/path/to/python  Override the sprite tool .venv.
#   AMBITION_QUALITY_VARIANTS=0             Skip variant generation entirely.
#
# ⛔ there is no hand-written postcondition here any more. This script asserted
# `sprites_0_5x/player_robot_spritesheet.ron` for weeks after the player sheet
# became `player_robot_v3`, so every run did its work, printed its summary, and
# then exited 1 on a file that had stopped existing. The generator now verifies
# the units it actually planned, which is a list that cannot drift because
# nothing maintains it.
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

args=()
while [ "$#" -gt 0 ]; do
    case "$1" in
        --sprites-only|--backgrounds-only|--force|--clean)
            args+=("$1"); shift ;;
        --target|--tier)
            if [ "$#" -lt 2 ] || [ -z "${2:-}" ]; then
                echo "$1 requires a value" >&2
                exit 2
            fi
            args+=("$1" "$2"); shift 2 ;;
        --target=*|--tier=*)
            args+=("$1"); shift ;;
        -h|--help) print_help; exit 0 ;;
        *) echo "unknown arg: $1" >&2; exit 2 ;;
    esac
done

if [ "${AMBITION_QUALITY_VARIANTS:-1}" = "0" ]; then
    echo "==> quality variants skipped (AMBITION_QUALITY_VARIANTS=0)"
    exit 0
fi

python_bin="$(ambition_select_tool_python "$renderer_dir" AMBITION_SPRITE_PYTHON)"
ambition_require_python_module \
    "$python_bin" ambition_sprite2d_renderer \
    "run ./run_developer_setup.sh or set AMBITION_SPRITE_PYTHON=/path/to/python"

exec "$python_bin" "$repo_root/scripts/generate_visual_quality_variants.py" \
    --asset-root "$asset_root" ${args[@]+"${args[@]}"}
