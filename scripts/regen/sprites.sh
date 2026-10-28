#!/usr/bin/env bash
# Plan and publish immutable sprite bundles.
set -euo pipefail
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
source "$repo_root/scripts/lib/tool_python.sh"
renderer_dir="$repo_root/tools/ambition_sprite2d_renderer"
python_bin="$(ambition_select_tool_python "$renderer_dir" AMBITION_SPRITE_PYTHON)"
if [ "${1:-}" = "--check-toolchain" ]; then
    ambition_preflight_tool_pythons "$0" "$renderer_dir" AMBITION_SPRITE_PYTHON 1
    exit $?
fi
if [ "${LINE_PROFILE:-0}" = "1" ]; then
    export AMBITION_ART_PROFILE_DIR="${AMBITION_ART_PROFILE_DIR:-$renderer_dir/.profiles/regen-$(date +%Y%m%d-%H%M%S)}"
fi
cd "$renderer_dir"
exec "$python_bin" -m ambition_sprite2d_renderer.publication "$@"
