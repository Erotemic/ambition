#!/usr/bin/env bash
# Regenerate all generated runtime assets for the sandbox crate.
#
# Usage:
# ./scripts/regen/assets.sh                    # backgrounds, sprites, quality variants, music, sfx
# ./scripts/regen/assets.sh sprites music      # selected categories, in the given order
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

if [ "$#" -eq 0 ]; then
    categories=(backgrounds sprites variants music sfx)
else
    categories=("$@")
fi

run_category() {
    local category="$1"
    case "$category" in
        backgrounds|background)
            bash "$repo_root/scripts/regen/backgrounds.sh"
            ;;
        sprites|sprite)
            bash "$repo_root/scripts/regen/sprites.sh"
            ;;
        variants|quality-variants|visual-quality)
            bash "$repo_root/scripts/regen/quality_variants.sh"
            ;;
        music)
            bash "$repo_root/scripts/regen/music.sh"
            ;;
        sfx|effects)
            bash "$repo_root/scripts/regen/sfx.sh"
            ;;
        *)
            echo "unknown asset category: $category" >&2
            echo "valid categories: backgrounds sprites variants music sfx" >&2
            exit 2
            ;;
    esac
}

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
tool_venv_unusable() {
    local cfg="$repo_root/$1/.venv/pyvenv.cfg" home
    [ -f "$cfg" ] || return 1   # no venv at all is someone else's problem to report
    home="$(sed -n 's/^home = //p' "$cfg" | head -1)"
    [ -n "$home" ] && [ ! -d "$home" ]
}

category_toolchain_missing() {
    case "$1" in
        sprites|sprite|variants|quality-variants|visual-quality)
            # ⛔ These two are the dangerous pair: they do not fail when the
            # renderer is unusable, they SUBSTITUTE. Ask the renderer directly,
            # because "can it import resvg" is more than a venv question.
            "$repo_root/scripts/regen/sprites.sh" --check-toolchain >/dev/null 2>&1 && return 1
            return 0
            ;;
        music)
            # Fails loudly rather than substituting, but a loud failure still
            # aborts the umbrella and costs every category after it.
            tool_venv_unusable tools/ambition_music_renderer
            ;;
        sfx|effects)
            tool_venv_unusable tools/ambition_sfx_renderer
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
        case "$category" in
            sprites|sprite|variants|quality-variants|visual-quality)
                echo "       Diagnose with: ./scripts/regen/sprites.sh --check-toolchain" >&2 ;;
            *)
                echo "       Its tool venv points at an interpreter that is not on this machine." >&2
                echo "       Re-provision it: ./run_developer_setup.sh" >&2 ;;
        esac
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
echo "==> all requested assets regenerated in ${total_elapsed}s"
echo "==> profile summary (slowest first):"
printf '%s\n' "${summary_rows[@]}" | sort -rn | awk -F'\t' '{ printf "    %5ds  %-12s  %s\n", $1, $2, $3 }'
echo "    -----  ------------"
printf '    %5ds  total\n' "$total_elapsed"
