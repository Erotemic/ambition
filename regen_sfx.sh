#!/usr/bin/env bash
# Re-render every SFX cue, then repack the binary .sfxbank consumed by
# the runtime.
#
# Pipeline:
#   1. ambition_sfx_renderer render-all  →  tools/ambition_sfx_renderer/output/<cue>/
#   2. ambition_sfx_pack                 →  crates/ambition_actors/assets/audio/sfx.bank
#
# Usage:
#   ./regen_sfx.sh              # render (incremental) + repack (default)
#   ./regen_sfx.sh --force      # force re-render every cue, then repack
#   ./regen_sfx.sh --skip-render  # only repack from existing renders
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

renderer_dir="$repo_root/tools/ambition_sfx_renderer"
pack_script="$repo_root/tools/ambition_sfx_pack/pack.py"

has_renderer() {
    [ -x "$1" ] && "$1" -c 'import ambition_sfx_renderer' >/dev/null 2>&1
}

select_python() {
    # Explicit override always wins.
    if [ -n "${PYTHON:-}" ]; then
        printf '%s\n' "$PYTHON"
        return
    fi
    # Otherwise prefer the first candidate that actually has the renderer
    # installed. The dedicated venv at $renderer_dir/.venv is created by
    # run_developer_setup.sh when the active interpreter's Python version is
    # outside the renderer's requires-python window (>=3.11,<3.13), so we must
    # be willing to pick it over an incompatible $VIRTUAL_ENV.
    local candidate
    for candidate in \
        "${VIRTUAL_ENV:+$VIRTUAL_ENV/bin/python}" \
        "$repo_root/.venv/bin/python" \
        "$renderer_dir/.venv/bin/python"; do
        [ -n "$candidate" ] || continue
        if has_renderer "$candidate"; then
            printf '%s\n' "$candidate"
            return
        fi
    done
    # Fall back to the original preference order for the error message below.
    if [ -n "${VIRTUAL_ENV:-}" ] && [ -x "$VIRTUAL_ENV/bin/python" ]; then
        printf '%s\n' "$VIRTUAL_ENV/bin/python"
    elif [ -x "$repo_root/.venv/bin/python" ]; then
        printf '%s\n' "$repo_root/.venv/bin/python"
    elif [ -x "$renderer_dir/.venv/bin/python" ]; then
        printf '%s\n' "$renderer_dir/.venv/bin/python"
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

force=0
skip_render=0
for arg in "$@"; do
    case "$arg" in
        --force) force=1 ;;
        --skip-render) skip_render=1 ;;
        -h|--help) print_help; exit 0 ;;
        *) echo "unknown arg: $arg" >&2; exit 2 ;;
    esac
done

renderer_py="$(select_python)"
if ! command -v "$renderer_py" >/dev/null 2>&1; then
    echo "python executable not found: $renderer_py" >&2
    echo "run ./run_developer_setup.sh, activate a venv, or set PYTHON=/path/to/python" >&2
    exit 1
fi

if ! "$renderer_py" -c 'import ambition_sfx_renderer' >/dev/null 2>&1; then
    echo "ambition_sfx_renderer is not installed in: $renderer_py" >&2
    echo "run ./run_developer_setup.sh to initialize the submodule and install it" >&2
    exit 1
fi

if [ "$skip_render" -eq 0 ]; then
    echo "==> render-all sfx cues (jobs=auto$([ "$force" -eq 1 ] && echo ', force'))"
    render_args=(render-all --jobs auto)
    if [ "$force" -eq 1 ]; then
        render_args+=(--force)
    fi
    (cd "$renderer_dir" && "$renderer_py" -m ambition_sfx_renderer "${render_args[@]}")
fi

echo "==> pack → crates/ambition_actors/assets/audio/sfx.bank"
python3 "$pack_script" --dump

echo "==> done"
