#!/usr/bin/env bash
# Re-render every SFX cue, then repack the binary .sfxbank consumed by
# the runtime.
#
# Pipeline:
#   1. ambition_sfx_renderer fetch-samples  →  tools/ambition_sfx_renderer/public_samples/
#      (the public recordings in sounds/public_samples.yaml; downloads only absent files)
#   2. ambition_sfx_renderer render-all  →  tools/ambition_sfx_renderer/output/<cue>/
#   3. ambition_sfx_pack                 →  game/ambition_content_library/generated/runtime/audio/sfx.bank
#
# Usage:
# ./scripts/regen/sfx.sh              # render (incremental) + repack (default)
# ./scripts/regen/sfx.sh --force      # force re-render every cue, then repack
# ./scripts/regen/sfx.sh --skip-render  # only repack from existing renders
#
# Environment:
#   AMBITION_SFX_PYTHON=/path/to/python  Override the tool-local .venv.
set -euo pipefail

# ⚠ TWO LEVELS UP: this script lives in `scripts/regen/`, not the repo root.
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"

renderer_dir="$repo_root/tools/ambition_sfx_renderer"
pack_script="$repo_root/tools/ambition_sfx_pack/pack.py"

# shellcheck disable=SC1091
if [ "${AMBITION_PRODUCER_WORKER:-}" != "sfx" ] && [ "${1:-}" != "--help" ] && [ "${1:-}" != "-h" ]; then
    exec python3 "$repo_root/scripts/generate_library_content.py" sfx "$@"
fi

source "$repo_root/scripts/lib/tool_python.sh"
source "$repo_root/scripts/lib/asset_roots.sh"
sfx_collection="$(ambition_library_producer sfx collection)"
[ -d "$renderer_dir/sounds/$sfx_collection" ] || { echo "missing SFX collection: $sfx_collection" >&2; exit 2; }

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

renderer_py="$(ambition_select_tool_python "$renderer_dir" AMBITION_SFX_PYTHON)"
ambition_require_python_module \
    "$renderer_py" ambition_sfx_renderer \
    "run ./run_developer_setup.sh or set AMBITION_SFX_PYTHON=/path/to/python"

if [ "$skip_render" -eq 0 ]; then
    echo "==> fetch public samples"
    (cd "$renderer_dir" && "$renderer_py" -m ambition_sfx_renderer fetch-samples)
    echo "==> render-all sfx cues (jobs=auto$([ "$force" -eq 1 ] && echo ', force'))"
    render_args=(render-all --group "$sfx_collection" --jobs auto)
    if [ "$force" -eq 1 ]; then
        render_args+=(--force)
    fi
    (cd "$renderer_dir" && "$renderer_py" -m ambition_sfx_renderer "${render_args[@]}")
fi

echo "==> pack -> $AMBITION_ASSETS_ROOT/audio/sfx.bank"
cue_list="$(mktemp)"
trap 'rm -f "$cue_list"' EXIT
(cd "$renderer_dir" && "$renderer_py" - "$sfx_collection" > "$cue_list" <<'PYTHON'
import json, sys
from ambition_sfx_renderer.schema import iter_cue_files, load_cue
print(json.dumps([load_cue(path).cue_id for path in iter_cue_files(group=sys.argv[1])]))
PYTHON
)
"$renderer_py" "$pack_script" --input "$renderer_dir/output" --output "$AMBITION_ASSETS_ROOT/audio/sfx.bank" --cue-list "$cue_list"

"$renderer_py" "$repo_root/scripts/check_regeneration_coverage.py" --family sfx

echo "==> done"
