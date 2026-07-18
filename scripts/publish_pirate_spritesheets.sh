#!/usr/bin/env bash
# Render and publish the standalone pirate spritesheets.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"
renderer_dir="$repo_root/tools/ambition_sprite2d_renderer"

# shellcheck disable=SC1091
source "$repo_root/scripts/lib/tool_python.sh"

python_bin="$(ambition_select_tool_python "$renderer_dir" AMBITION_SPRITE_PYTHON)"
ambition_require_python_module \
    "$python_bin" ambition_sprite2d_renderer \
    "run ./run_developer_setup.sh or set AMBITION_SPRITE_PYTHON=/path/to/python"

"$python_bin" "$renderer_dir/publish_pirate_spritesheets.py" "$@"
