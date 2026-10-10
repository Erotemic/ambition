#!/usr/bin/env bash
# Regenerate the scenery of the rooms: the placeholder background profiles,
# the parallax scenes and the terrain skins.
#
# Usage:
# ./scripts/regen/backgrounds.sh
#   AMBITION_BACKGROUND_PYTHON=/path/to/python ./scripts/regen/backgrounds.sh
#   AMBITION_SPRITE_PYTHON=/path/to/python ./scripts/regen/backgrounds.sh
#
# The default interpreters are the two tool-local virtualenvs created by
# run_developer_setup.sh. PYTHON remains a legacy override for both tools.
set -euo pipefail

# ⚠ TWO LEVELS UP: this script lives in `scripts/regen/`, not the repo root.
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

# shellcheck disable=SC1091
source "$repo_root/scripts/lib/tool_python.sh"

background_renderer_dir="$repo_root/tools/ambition_background_renderer"
# The scenes behind the play are art, and art is in the authoring submodule
# (`ambition_sprite2d_renderer.backgrounds`).
scene_renderer_dir="$repo_root/tools/ambition_sprite2d_renderer"
background_root="$repo_root/crates/ambition_platformer2d_actor_monolith/assets/backgrounds"
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
scene_python="$(ambition_select_tool_python "$scene_renderer_dir" AMBITION_SPRITE_PYTHON)"
ambition_require_python_module "$background_python" ambition_background_renderer "$setup_hint"
ambition_require_python_module "$scene_python" ambition_sprite2d_renderer "$setup_hint"

mkdir -p "$background_root" "$parallax_dir"

echo "==> placeholder background profiles -> $background_root"
(
    cd "$background_renderer_dir"
    "$background_python" -m ambition_background_renderer \
        --out "$background_root" --profile all
)

echo "==> parallax scenes -> $parallax_dir"
(
    cd "$scene_renderer_dir"
    "$scene_python" -m ambition_sprite2d_renderer.backgrounds draw \
        --out-dir "$parallax_dir"
)

# The terrain skin of each biome: what the game lays on the blocks of a room.
terrain_dir="$repo_root/crates/ambition_platformer2d_actor_monolith/assets/room_dressing"
mkdir -p "$terrain_dir"
echo "==> terrain skins -> $terrain_dir"
(
    cd "$scene_renderer_dir"
    "$scene_python" -m ambition_sprite2d_renderer.terrain draw \
        --out-dir "$terrain_dir"
)

required_outputs=(
    "$background_root/default/sky.png"
    "$background_root/default/manifest.txt"
    "$parallax_dir/hub_sky.png"
    "$parallax_dir/hub_clean_sky.png"
    "$terrain_dir/lab_fill.png"
)
for output in "${required_outputs[@]}"; do
    if [ ! -s "$output" ]; then
        echo "background generation did not produce: $output" >&2
        exit 1
    fi
done

echo "==> done"
