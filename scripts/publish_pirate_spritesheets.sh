#!/usr/bin/env bash
# Render and publish the standalone pirate spritesheets.
set -euo pipefail

script_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
repo_root="$(cd "$script_dir/.." && pwd)"

select_python() {
    if [ -n "${PYTHON:-}" ]; then
        printf '%s\n' "$PYTHON"
    elif [ -n "${VIRTUAL_ENV:-}" ] && [ -x "$VIRTUAL_ENV/bin/python" ]; then
        printf '%s\n' "$VIRTUAL_ENV/bin/python"
    elif [ -x "$repo_root/.venv/bin/python" ]; then
        printf '%s\n' "$repo_root/.venv/bin/python"
    else
        printf '%s\n' python
    fi
}

python_bin="$(select_python)"
if ! command -v "$python_bin" >/dev/null 2>&1; then
    echo "python executable not found: $python_bin" >&2
    echo "run ./run_developer_setup.sh, activate a venv, or set PYTHON=/path/to/python" >&2
    exit 1
fi

"$python_bin" "$repo_root/tools/ambition_sprite2d_renderer/publish_pirate_spritesheets.py" "$@"
