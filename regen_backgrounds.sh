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

renderer_dir="$repo_root/tools/ambition_parallax_renderer"
background_dir="$repo_root/crates/ambition_actors/assets/backgrounds/parallax_layers"

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

python_bin="$(select_python)"
if ! command -v "$python_bin" >/dev/null 2>&1; then
    echo "python executable not found: $python_bin" >&2
    echo "run ./run_developer_setup.sh, activate a venv, or set PYTHON=/path/to/python" >&2
    exit 1
fi

if ! "$python_bin" -c 'import ambition_parallax_renderer' >/dev/null 2>&1; then
    echo "ambition_parallax_renderer is not installed in: $python_bin" >&2
    echo "run ./run_developer_setup.sh, activate the configured venv, or set PYTHON=/path/to/python" >&2
    exit 1
fi

mkdir -p "$background_dir"

echo "==> background sky/parallax layers -> $background_dir"
(cd "$renderer_dir" && "$python_bin" -m ambition_parallax_renderer draw-backgrounds --out-dir "$background_dir")

echo "==> done"
