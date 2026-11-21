#!/usr/bin/env bash
# Prepare the recorded historical character stills.
set -euo pipefail
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$repo_root/scripts/lib/tool_python.sh"
renderer_dir="$repo_root/tools/ambition_sprite2d_renderer"
python_bin="$(ambition_select_tool_python "$renderer_dir" AMBITION_SPRITE_PYTHON)"
cd "$renderer_dir"
exec "$python_bin" -m design_history prepare "$@"
