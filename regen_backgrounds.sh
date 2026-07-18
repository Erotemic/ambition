#!/usr/bin/env bash
# Regenerate every procedural background family used by the desktop game.
#
# Usage:
#   ./regen_backgrounds.sh
#   AMBITION_BACKGROUND_PYTHON=/path/to/python ./regen_backgrounds.sh
#   AMBITION_PARALLAX_PYTHON=/path/to/python ./regen_backgrounds.sh
#
# The default interpreters are the two tool-local virtualenvs created by
# run_developer_setup.sh. PYTHON remains a legacy override for both tools.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

# shellcheck disable=SC1091
source "$repo_root/scripts/lib/tool_python.sh"

background_renderer_dir="$repo_root/tools/ambition_background_renderer"
parallax_renderer_dir="$repo_root/tools/ambition_parallax_renderer"
background_root="$repo_root/crates/ambition_actors/assets/backgrounds"
parallax_dir="$background_root/parallax_layers"

print_help() {
    awk '
        NR == 1 { next }
        /^set -euo pipefail$/ { exit }
        /^#$/ { print ""; next }
        /^# / { sub(/^# /, ""); print }
    ' "$0"
}

for arg in "$@"; do
    case "$arg" in
        -h|--help) print_help; exit 0 ;;
        *) echo "unknown arg: $arg" >&2; exit 2 ;;
    esac
done

setup_hint="run ./run_developer_setup.sh or set the corresponding AMBITION_*_PYTHON override"
background_python="$(ambition_select_tool_python "$background_renderer_dir" AMBITION_BACKGROUND_PYTHON)"
parallax_python="$(ambition_select_tool_python "$parallax_renderer_dir" AMBITION_PARALLAX_PYTHON)"
ambition_require_python_module "$background_python" ambition_background_renderer "$setup_hint"
ambition_require_python_module "$parallax_python" ambition_parallax_renderer "$setup_hint"

mkdir -p "$background_root" "$parallax_dir"

echo "==> placeholder background profiles -> $background_root"
(
    cd "$background_renderer_dir"
    "$background_python" -m ambition_background_renderer \
        --out "$background_root" --profile all
)

echo "==> background sky/parallax layers -> $parallax_dir"
(
    cd "$parallax_renderer_dir"
    "$parallax_python" -m ambition_parallax_renderer draw-backgrounds \
        --out-dir "$parallax_dir"
)

required_outputs=(
    "$background_root/default/sky.png"
    "$background_root/default/manifest.txt"
    "$parallax_dir/hub_sky.png"
    "$parallax_dir/parallax_manifest.json"
)
for output in "${required_outputs[@]}"; do
    if [ ! -s "$output" ]; then
        echo "background generation did not produce: $output" >&2
        exit 1
    fi
done

echo "==> done"
