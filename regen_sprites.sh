#!/usr/bin/env bash
# Re-render every sprite asset and install into the sandbox crate.
#
# Covers:
#   - Adapter targets (robot / goblin / boss): re-renders every job in
#     tools/ambition_sprite2d_renderer/configs/ straight into
#     crates/ambition_sandbox/assets/sprites/.
#   - Entity sprites (chest, breakable, door zone, etc.): re-rendered into
#     crates/ambition_sandbox/assets/sprites/entities/.
#   - Tack-on targets (sandbag): rendered into the renderer's generated/
#     dir then installed into crates/ambition_sandbox/assets/sprites/.
#
# Usage:
#   ./regen_sprites.sh   # render + install everything
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

renderer_dir="$repo_root/tools/ambition_sprite2d_renderer"
renderer_py="$renderer_dir/.venv/bin/python"
sprites_dir="$repo_root/crates/ambition_sandbox/assets/sprites"
entities_dir="$sprites_dir/entities"

for arg in "$@"; do
    case "$arg" in
        -h|--help) grep '^#' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "unknown arg: $arg" >&2; exit 2 ;;
    esac
done

if [ ! -x "$renderer_py" ]; then
    echo "sprite renderer venv missing: $renderer_py" >&2
    echo "create one with: (cd tools/ambition_sprite2d_renderer && uv venv && uv pip install -e .)" >&2
    exit 1
fi

echo "==> adapter targets (robot / goblin / boss) → $sprites_dir"
(cd "$renderer_dir" && "$renderer_py" -m ambition_sprite2d_renderer draw-all --out-dir "$sprites_dir")

echo "==> entity sprites → $entities_dir"
(cd "$renderer_dir" && "$renderer_py" -m ambition_sprite2d_renderer draw-entities --out-dir "$entities_dir")

echo "==> tack-on: sandbag (render-publish into $sprites_dir)"
(cd "$renderer_dir" && "$renderer_py" -m ambition_sprite2d_renderer render-publish sandbag --dest-root "$sprites_dir")

echo "==> done"
