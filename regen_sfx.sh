#!/usr/bin/env bash
# Re-render every SFX cue, then repack the binary .sfxbank consumed by
# the runtime.
#
# Pipeline:
#   1. ambition_sfx_renderer render-all  →  tools/ambition_sfx_renderer/output/<cue>/
#   2. ambition_sfx_pack                 →  crates/ambition_sandbox/assets/audio/sfx.bank
#
# Usage:
#   ./regen_sfx.sh              # render (incremental) + repack (default)
#   ./regen_sfx.sh --force      # force re-render every cue, then repack
#   ./regen_sfx.sh --skip-render  # only repack from existing renders
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$repo_root"

renderer_dir="$repo_root/tools/ambition_sfx_renderer"
renderer_py="$renderer_dir/.venv/bin/python"
pack_script="$repo_root/tools/ambition_sfx_pack/pack.py"

force=0
skip_render=0
for arg in "$@"; do
    case "$arg" in
        --force) force=1 ;;
        --skip-render) skip_render=1 ;;
        -h|--help) grep '^#' "$0" | sed 's/^# \{0,1\}//'; exit 0 ;;
        *) echo "unknown arg: $arg" >&2; exit 2 ;;
    esac
done

if [ ! -x "$renderer_py" ]; then
    echo "sfx renderer venv missing: $renderer_py" >&2
    echo "run: (cd tools/ambition_sfx_renderer && ./setup.sh)" >&2
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

echo "==> pack → crates/ambition_sandbox/assets/audio/sfx.bank"
python3 "$pack_script" --dump

echo "==> done"
