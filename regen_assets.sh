#!/usr/bin/env bash
# Regenerate all generated runtime assets for the sandbox crate.
#
# Usage:
#   ./regen_assets.sh                    # backgrounds, sprites, music, sfx
#   ./regen_assets.sh sprites music      # selected categories, in the given order
#
# Category-specific options live on the category scripts:
#   ./regen_backgrounds.sh --help
#   ./regen_sprites.sh --help
#   ./regen_music.sh --help
#   ./regen_sfx.sh --help
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
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
    categories=(backgrounds sprites music sfx)
else
    categories=("$@")
fi

run_category() {
    local category="$1"
    case "$category" in
        backgrounds|background)
            bash "$repo_root/regen_backgrounds.sh"
            ;;
        sprites|sprite)
            bash "$repo_root/regen_sprites.sh"
            ;;
        music)
            bash "$repo_root/regen_music.sh"
            ;;
        sfx|effects)
            bash "$repo_root/regen_sfx.sh"
            ;;
        *)
            echo "unknown asset category: $category" >&2
            echo "valid categories: backgrounds sprites music sfx" >&2
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

declare -a summary_rows=()
overall_start=$SECONDS

for category in "${categories[@]}"; do
    echo
    echo "==> regen $category"
    cat_start=$SECONDS
    status="ok"
    run_category "$category" || status="fail"
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
