#!/usr/bin/env bash
# Regenerate all generated runtime assets for the sandbox crate.
#
# Usage:
# ./scripts/regen/assets.sh                            # all declared producers
# ./scripts/regen/assets.sh --plan backgrounds sprites # preview selection
# ./scripts/regen/assets.sh --check --loose             # check direct changes
# ./scripts/regen/assets.sh --only-possibly-stale        # shared-only staleness
# ./scripts/regen/assets.sh --only-missing              # unpublished units
# ./scripts/regen/assets.sh --force sprites music       # rebuild both categories
# ./scripts/regen/assets.sh --jobs 1                    # one worker per renderer
#
# --loose keeps units that are possibly stale. --missing-only is an alias
# for --only-missing. Use one build selection per command.
#
# Category-specific options live on the category scripts:
# ./scripts/regen/backgrounds.sh --help
# ./scripts/regen/sprites.sh --help
# ./scripts/regen/quality_variants.sh --help
# ./scripts/regen/music.sh --help
# ./scripts/regen/sfx.sh --help
set -euo pipefail

# ⚠ TWO LEVELS UP: this script lives in `scripts/regen/`, not the repo root.
repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$repo_root"
source "$repo_root/scripts/lib/asset_roots.sh"

print_help() {
    awk '
        NR == 1 { next }
        /^set -euo pipefail$/ { exit }
        /^#$/ { print ""; next }
        /^# / { sub(/^# /, ""); print }
    ' "$0"
}

if [ "${1:-}" = "-h" ] || [ "${1:-}" = "--help" ]; then
    print_help
    exit 0
fi

query=""
selection=""
requested=()
jobs=()
while [ "$#" -gt 0 ]; do
    arg="$1"
    shift
    case "$arg" in
        --jobs)
            if [[ ! "${1:-}" =~ ^[1-9][0-9]*$ ]]; then
                echo "--jobs needs a positive integer" >&2; exit 2
            fi
            jobs=(--jobs "$1")
            shift ;;
        --plan|--check)
            if [ -n "$query" ] && [ "$query" != "$arg" ]; then
                echo "--plan and --check cannot be combined" >&2; exit 2
            fi
            query="$arg" ;;
        --loose|--only-possibly-stale|--only-missing|--missing-only|--force)
            [ "$arg" != --missing-only ] || arg=--only-missing
            if [ -n "$selection" ] && [ "$selection" != "$arg" ]; then
                echo "$selection and $arg cannot be combined" >&2; exit 2
            fi
            selection="$arg" ;;
        -h|--help) print_help; exit 0 ;;
        -*) echo "unknown option: $arg (see --help)" >&2; exit 2 ;;
        *) requested+=("$arg") ;;
    esac
done
if [ "$query" = --check ] && [ -n "$selection" ] && [ "$selection" != --loose ]; then
    echo "--check accepts only --loose as a selection option" >&2; exit 2
fi
set -- ${requested[@]+"${requested[@]}"}
producer_args=()
[ -z "$query" ] || producer_args+=("$query")
[ -z "$selection" ] || producer_args+=("$selection")

if [ "$#" -eq 0 ]; then
    producer_list="$(ambition_library_producer --list)"
    categories=()
    while IFS= read -r producer; do
        [ -z "$producer" ] || categories+=("$producer")
    done <<< "$producer_list"
else
    categories=("$@")
fi

run_category() {
    local category="$1"
    case "$category" in
        fonts)
            python3 "$repo_root/scripts/regen/grab_font_assets.py" ${producer_args[@]+"${producer_args[@]}"}
            ;;
        backgrounds|background)
            bash "$repo_root/scripts/regen/backgrounds.sh" "${producer_args[@]}" "${jobs[@]}"
            ;;
        sprites|sprite)
            bash "$repo_root/scripts/regen/sprites.sh" "${producer_args[@]}" "${jobs[@]}"
            ;;
        variants|quality-variants|visual-quality)
            bash "$repo_root/scripts/regen/quality_variants.sh" "${producer_args[@]}" "${jobs[@]}"
            ;;
        music)
            local -a music_args=("${producer_args[@]}" "${jobs[@]}")
            if [ -z "$selection" ] && [ "$query" != --check ]; then
                music_args+=(--include-shared)
            fi
            bash "$repo_root/scripts/regen/music.sh" "${music_args[@]}"
            ;;
        sfx|effects)
            bash "$repo_root/scripts/regen/sfx.sh" "${producer_args[@]}" "${jobs[@]}"
            ;;
        *)
            echo "unknown asset category: $category" >&2
            echo "valid categories: fonts backgrounds sprites variants music sfx" >&2
            exit 2
            ;;
    esac
}

if [ -n "$query" ]; then
    result=0
    for category in "${categories[@]}"; do
        echo "==> $category $query"
        run_category "$category" || result=1
    done
    exit "$result"
fi

# Profiling: log per-category timing to target/regen_assets/.
# target/ is gitignored. Each run writes its own jsonl file; a symlink
# `latest.jsonl` always points at the most recent run for quick inspection.
profile_dir="$repo_root/target/regen_assets"
mkdir -p "$profile_dir"
profile_log="$profile_dir/profile-$(date -u +%Y%m%dT%H%M%SZ).jsonl"
ln -sf "$(basename "$profile_log")" "$profile_dir/latest.jsonl"
echo "==> profile log: ${profile_log#$repo_root/}"

# ⛔⛔ A CATEGORY WHOSE TOOLCHAIN IS ABSENT MUST NOT RUN. It will not fail — it
# will DEGRADE, and a degraded asset is indistinguishable from a good one once
# it is on disk.
#
# ⚠ 2026-08-29 on this repo: the sprite renderer needs `resvg-py` for
# SVG-rigged targets, the tool venv in this shared checkout pointed at one
# user's Python, tool resolution fell back to a bare `python3` without it, and
# `sprites/player_robot_v3_spritesheet.png` was published byte-identical to
# v2's. The game drew the wrong robot for a day and no step in the pipeline
# said anything.
#
# ⭐ REFUSING IS THE FEATURE. The categories that can build still build; the one
# that cannot says which import is missing and what to run. That is strictly
# better than a green run that published the wrong character.
# ⚠ ONE CAUSE BEHIND ALL OF THEM: a tool venv that lives INSIDE this shared
# checkout while its interpreter lives in one user's home. Measured 2026-08-29 —
# all three `tools/*/.venv/pyvenv.cfg` name `/home/joncrall/.local/share/uv/…`,
# so the venv is real for one user and interpreter-less for anyone else on the
# same filesystem, and tool resolution then falls back to a bare `python3`
# missing the renderer's dependencies.
category_toolchain_missing() {
    case "$1" in
        sprites|sprite|variants|quality-variants|visual-quality)
            # ⛔ These two are the dangerous pair: they do not fail when the
            # renderer is unusable, they SUBSTITUTE. Ask the renderer directly,
            # because "can it import resvg" is more than a venv question.
            "$repo_root/scripts/regen/sprites.sh" --check-toolchain >/dev/null 2>&1 && return 1
            return 0
            ;;
        *) return 1 ;;
    esac
}

declare -a summary_rows=()
toolchain_skips=0
overall_start=$SECONDS

for category in "${categories[@]}"; do
    echo
    echo "==> regen $category"
    cat_start=$SECONDS
    status="ok"
    if category_toolchain_missing "$category"; then
        status="skipped-toolchain"
        echo "    ⛔ SKIPPED: the renderer this category needs is not usable here." >&2
        echo "       Running it anyway publishes degraded art that looks fine on disk." >&2
        echo "       Diagnose with: ./scripts/regen/sprites.sh --check-toolchain" >&2
        toolchain_skips=$((toolchain_skips + 1))
    else
        run_category "$category" || status="fail"
    fi
    elapsed=$((SECONDS - cat_start))
    printf '{"timestamp":"%s","category":"%s","seconds":%d,"status":"%s"}\n' \
        "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$category" "$elapsed" "$status" >> "$profile_log"
    printf '    [%s] %s — %ds\n' "$status" "$category" "$elapsed"
    summary_rows+=("$(printf '%6d\t%s\t%s' "$elapsed" "$category" "$status")")
    if [ "$status" = "fail" ]; then
        echo "==> aborting; see ${profile_log#$repo_root/}" >&2
        exit 1
    fi
done

total_elapsed=$((SECONDS - overall_start))

echo
if [ "$toolchain_skips" -gt 0 ]; then
    echo "generation incomplete: $toolchain_skips required producer(s) could not run" >&2
    exit 1
fi
echo "==> all requested assets regenerated in ${total_elapsed}s"
echo "==> profile summary (slowest first):"
printf '%s\n' "${summary_rows[@]}" | sort -rn | awk -F'\t' '{ printf "    %5ds  %-12s  %s\n", $1, $2, $3 }'
echo "    -----  ------------"
printf '    %5ds  total\n' "$total_elapsed"
