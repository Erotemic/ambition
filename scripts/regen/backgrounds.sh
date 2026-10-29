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
if [ "${AMBITION_PRODUCER_WORKER:-}" != "backgrounds" ] && [ "${1:-}" != "--help" ] && [ "${1:-}" != "-h" ]; then
    exec python3 "$repo_root/scripts/generate_library_content.py" backgrounds "$@"
fi

source "$repo_root/scripts/lib/tool_python.sh"
source "$repo_root/scripts/lib/asset_roots.sh"
background_profiles="$(ambition_library_producer backgrounds profiles)"

background_renderer_dir="$repo_root/tools/ambition_background_renderer"
# The scenes behind the play are art, and art is in the authoring submodule
# (`ambition_sprite2d_renderer.backgrounds`).
scene_renderer_dir="$repo_root/tools/ambition_sprite2d_renderer"
background_root="$AMBITION_ASSETS_ROOT/backgrounds"
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
        --out "$background_root" --profile "$background_profiles"
)

echo "==> parallax scenes -> $parallax_dir"
(
    cd "$scene_renderer_dir"
    "$scene_python" -m ambition_sprite2d_renderer.backgrounds draw \
        --out-dir "$parallax_dir"
)

# The parts of the Mockingbird's space, which the game poses as its fight
# climbs (`backgrounds/sky_parts.py`).
sky_parts_dir="$background_root/sky_parts"
mkdir -p "$sky_parts_dir"
echo "==> sky parts -> $sky_parts_dir"
(
    cd "$scene_renderer_dir"
    "$scene_python" -m ambition_sprite2d_renderer.backgrounds parts \
        --out-dir "$sky_parts_dir"
)

# The terrain skin of each biome: what the game lays on the blocks of a room.
terrain_dir="$AMBITION_ASSETS_ROOT/room_dressing"
mkdir -p "$terrain_dir"
echo "==> terrain skins -> $terrain_dir"
(
    cd "$scene_renderer_dir"
    "$scene_python" -m ambition_sprite2d_renderer.terrain draw \
        --out-dir "$terrain_dir"
)

"$scene_python" - "$repo_root" <<'PYTHON'
from pathlib import Path
import sys
repo = Path(sys.argv[1])
sys.path.insert(0, str(repo / "scripts"))
from lib.content_libraries import generated_root, producer, relative
for logical in producer(repo, "backgrounds")["required_outputs"]:
    output = generated_root(repo) / relative(logical)
    if not output.is_file() or not output.stat().st_size:
        raise SystemExit(f"Background generation did not produce: {output}")
PYTHON

"$scene_python" "$repo_root/scripts/check_regeneration_coverage.py" --family backgrounds

echo "==> done"
