#!/usr/bin/env bash
# Compatibility shim. Prefer:
#   python -m ambition_sprite2d_renderer render-publish sandbag
set -euo pipefail
echo "[deprecated] tools/generators/gen2d/generate_sandbag_assets.sh -- use" >&2
echo "  python -m ambition_sprite2d_renderer render-publish sandbag" >&2

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
pkg_root="$repo_root/tools/ambition_sprite2d_renderer"

venv_python="$repo_root/tools/generators/gen2d/.venv/bin/python"
if [ -x "$venv_python" ]; then
    PY="$venv_python"
else
    PY="${PYTHON:-python3}"
fi

PYTHONPATH="$pkg_root${PYTHONPATH:+:$PYTHONPATH}" "$PY" -m ambition_sprite2d_renderer render-publish sandbag "$@"
