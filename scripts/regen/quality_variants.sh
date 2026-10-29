#!/usr/bin/env bash
# Generate reduced-resolution sprite and parallax assets after full-res publish.
# A sheet/tier is rebuilt when its source pages or generator are newer than the
# published output; --force bypasses this freshness check.
#
# Usage:
#   ./scripts/regen/quality_variants.sh [--sprites-only|--backgrounds-only]
#       [--target <fnmatch>]... [--tier <0_5x|0_25x|potato>]...
#       [--force] [--clean]
#
# Environment:
#   AMBITION_SPRITE_PYTHON=/path/to/python
#   AMBITION_QUALITY_VARIANTS=0
set -euo pipefail

# ⚠ TWO LEVELS UP: this script lives in `scripts/regen/`, not the repo root.
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

renderer_dir="$repo_root/tools/ambition_sprite2d_renderer"

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
        --target|--tier|--library|--asset-root)
            if [ "$#" -lt 2 ] || [ -z "${2:-}" ]; then
                echo "$1 requires a value" >&2
                exit 2
            fi
            args+=("$1" "$2"); shift 2 ;;
        --target=*|--tier=*|--library=*|--asset-root=*)
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
    ${args[@]+"${args[@]}"}
