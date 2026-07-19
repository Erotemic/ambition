#!/usr/bin/env bash
# Profile one or a few sprite targets without forcing the entire roster.
#
# Examples:
#   ./profile_sprites.sh --target oiler
#   ./profile_sprites.sh --target ninja_heavy --target stochastic_parrot_v2
#   ./profile_sprites.sh --suite quick
#   ./profile_sprites.sh --suite representative --product publish
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
renderer_dir="$repo_root/tools/ambition_sprite2d_renderer"

# shellcheck disable=SC1091
source "$repo_root/scripts/lib/tool_python.sh"
python_bin="$(ambition_select_tool_python "$renderer_dir" AMBITION_SPRITE_PYTHON)"
ambition_require_python_module \
    "$python_bin" ambition_sprite2d_renderer \
    "run ./run_developer_setup.sh or set AMBITION_SPRITE_PYTHON=/path/to/python"
ambition_require_python_module \
    "$python_bin" line_profiler \
    "install the sprite renderer development dependencies with ./run_developer_setup.sh"

targets=()
suite=""
product="sheet"
output_dir=""

usage() {
    cat <<'EOF'
Usage:
  ./profile_sprites.sh --target NAME [--target NAME ...]
  ./profile_sprites.sh --suite quick|representative

Options:
  --product sheet|portraits|publish  Product to profile. Default: sheet.
  --output-dir DIR                   Explicit report directory.
  --list-suites                      Show the built-in representative suites.

The default sheet product renders only the requested target into generated/.
It skips the full roster, runtime installation, review galleries, factions,
LDtk integration, and unrelated postconditions. Every run writes .lprof, .txt,
.log, and profile-index.txt files.
EOF
}

while [ "$#" -gt 0 ]; do
    case "$1" in
        --target|-t)
            [ "$#" -ge 2 ] || { echo "--target requires a name" >&2; exit 2; }
            targets+=("$2")
            shift 2
            ;;
        --suite)
            [ "$#" -ge 2 ] || { echo "--suite requires a name" >&2; exit 2; }
            suite="$2"
            shift 2
            ;;
        --product)
            [ "$#" -ge 2 ] || { echo "--product requires a value" >&2; exit 2; }
            product="$2"
            shift 2
            ;;
        --output-dir)
            [ "$#" -ge 2 ] || { echo "--output-dir requires a path" >&2; exit 2; }
            output_dir="$2"
            shift 2
            ;;
        --list-suites)
            echo "quick:          oiler, stochastic_parrot_v2"
            echo "representative: oiler, m_leblanc, ninja_heavy, stochastic_parrot_v2"
            exit 0
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "unknown argument: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
done

case "$product" in
    sheet|portraits|publish) ;;
    *) echo "unsupported product: $product" >&2; exit 2 ;;
esac

if [ -n "$suite" ]; then
    [ "${#targets[@]}" -eq 0 ] || { echo "--suite and --target are mutually exclusive" >&2; exit 2; }
    case "$suite" in
        quick) targets=(oiler stochastic_parrot_v2) ;;
        representative) targets=(oiler m_leblanc ninja_heavy stochastic_parrot_v2) ;;
        *) echo "unknown suite: $suite" >&2; exit 2 ;;
    esac
fi

[ "${#targets[@]}" -gt 0 ] || { usage >&2; exit 2; }

if [ -z "$output_dir" ]; then
    stamp="$(TZ=America/New_York date +%Y%m%dT%H%M%S%z)"
    output_dir="$renderer_dir/.profiles/targets-$stamp-$$"
fi
mkdir -p "$output_dir"
output_dir="$(cd "$output_dir" && pwd)"

export AMBITION_SPRITE_PROGRESS="${AMBITION_SPRITE_PROGRESS:-1}"
export AMBITION_SPRITE_PATH_OUTPUT="${AMBITION_SPRITE_PATH_OUTPUT:-summary}"

printf '==> targeted sprite profiling\n'
printf '    product: %s\n' "$product"
printf '    targets: %s\n' "${targets[*]}"
printf '    reports: %s\n' "$output_dir"

for target in "${targets[@]}"; do
    stem="${product}-${target}"
    lprof="$output_dir/$stem.lprof"
    log="$output_dir/$stem.log"
    command=("$python_bin" -m kernprof -l -o "$lprof" -m ambition_sprite2d_renderer)
    case "$product" in
        sheet) command+=(sheet "$target") ;;
        portraits) command+=(portraits "$target") ;;
        publish)
            publish_dir="$output_dir/published/$target"
            mkdir -p "$publish_dir"
            command+=(publish "$target" --dest-root "$publish_dir")
            ;;
    esac

    printf '\n==> profile %s %s\n' "$product" "$target"
    started="$(date +%s)"
    set +e
    (
        cd "$renderer_dir"
        env -u LINE_PROFILER_OWNER_PID "${command[@]}"
    ) 2>&1 | tee "$log"
    status="${PIPESTATUS[0]}"
    set -e
    elapsed="$(( $(date +%s) - started ))"

    if [ -f "$lprof" ]; then
        "$python_bin" "$repo_root/scripts/render_line_profiles.py" "$lprof" || true
    fi
    if [ "$status" -ne 0 ]; then
        echo "profile failed for $target after ${elapsed}s; preserved $log and any $lprof" >&2
        exit "$status"
    fi
    echo "    completed in ${elapsed}s"
done

"$python_bin" "$repo_root/scripts/render_line_profiles.py" "$output_dir" || true
printf '\n==> profile reports ready: %s\n' "$output_dir"
