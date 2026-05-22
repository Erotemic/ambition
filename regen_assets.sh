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

for category in "${categories[@]}"; do
    echo
    echo "==> regen $category"
    run_category "$category"
done

echo
echo "==> all requested assets regenerated"
