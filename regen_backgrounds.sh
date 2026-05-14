#!/usr/bin/env bash
# Regenerate procedural background / parallax layer assets for the sandbox.
#
# Usage:
#   ./regen_backgrounds.sh
#   PYTHON=/path/to/python ./regen_backgrounds.sh
#
# Generated backgrounds intentionally live under assets/backgrounds/, not
# assets/sprites/. Sprite regeneration does not create or publish them.
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

python_bin="${PYTHON:-python}"
renderer_dir="$repo_root/tools/ambition_parallax_renderer"
background_dir="$repo_root/crates/ambition_sandbox/assets/backgrounds/parallax_layers"

for arg in "$@"; do
    case "$arg" in
        -h|--help) grep '^# ' "$0" | sed 's/^# //'; exit 0 ;;
        *) echo "unknown arg: $arg" >&2; exit 2 ;;
    esac
done

if ! command -v "$python_bin" >/dev/null 2>&1; then
    echo "python executable not found: $python_bin" >&2
    echo "activate your venv or set PYTHON=/path/to/python" >&2
    exit 1
fi

mkdir -p "$background_dir"

echo "==> background sky/parallax layers -> $background_dir"
(cd "$renderer_dir" && "$python_bin" -m ambition_parallax_renderer draw-backgrounds --out-dir "$background_dir")

echo "==> done"
