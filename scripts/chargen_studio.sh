#!/usr/bin/env bash
# Chargen Studio: a local page to make characters with the character
# generator (every field, eight views, sweeps, crowds, poses).
#
#   ./scripts/chargen_studio.sh [--port 8765] [--jobs 6] [--host 0.0.0.0] [--open]
#
# Runs in the sprite renderer's interpreter; see
# tools/ambition_sprite2d_renderer/ambition_sprite2d_renderer/chargen/studio.py.
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
# shellcheck source=lib/tool_python.sh
source "$ROOT/scripts/lib/tool_python.sh"
PYTHON="$(ambition_select_tool_python "$ROOT/tools/ambition_sprite2d_renderer" AMBITION_SPRITE_PYTHON)"
cd "$ROOT/tools/ambition_sprite2d_renderer"
exec "$PYTHON" -m ambition_sprite2d_renderer.chargen studio "$@"
