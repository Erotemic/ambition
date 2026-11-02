#!/usr/bin/env bash
# An unattended phone profile of one game: the hub boots into the game, the
# game plays itself by navigation, and `profile_android.sh` builds, installs,
# launches and records.
#
#   scripts/autoprofile_android.sh                 # Ambition (the hall of characters, then a flight through the primeval trail)
#   scripts/autoprofile_android.sh castle_godwin   # Castle Godwin combat route
#   scripts/autoprofile_android.sh mary_o          # Mary-O (the cosmic form, to each goal)
#   scripts/autoprofile_android.sh ambition --duration 60 --warmup 40
#
# PROFILE names an autoplay policy (`AMBITION_AUTOPLAY`). Every
# other argument goes to `profile_android.sh` and overrides the defaults
# below. The APK installs every game and bakes in nothing: the game and the
# boot route are this one launch's environment (`--launch-env`), which the
# app adopts and deletes at startup, and the app is stopped after the
# capture, so opening it from the icon afterwards is the plain game. The startup cards
# take about 6 s before the boot route: the warmup covers them and the walk
# to the first door.
#
# Read the result with `scripts/symbolize_android_profile.py DIR` (the
# script prints DIR at the end).
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)"

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
    sed -n '2,18p' "${BASH_SOURCE[0]}" | sed 's/^# \{0,1\}//'
    exit 0
fi

autoplay_profile="ambition"
if [[ $# -gt 0 && "$1" != -* ]]; then
    autoplay_profile="$1"
    shift
fi

case "$autoplay_profile" in
    ambition) game="ambition"; route="ambition_gameplay" ;;
    castle_godwin) game="ambition"; route="ambition_gameplay" ;;
    mary_o) game="mary_o"; route="mary_o_gameplay" ;;
    *)
        echo "autoprofile_android.sh: no autoplay policy for '$autoplay_profile' (ambition, castle_godwin, mary_o)" >&2
        exit 2
        ;;
esac

name="all-${autoplay_profile}-auto-$(date -u +%Y%m%dT%H%M%SZ)"
echo "[autoprofile] game=$game autoplay=$autoplay_profile boot_route=$route name=$name"
# Unset, so a build made here bakes in no autoplay (`option_env!`).
unset AMBITION_BOOT_ROUTE AMBITION_AUTOPLAY
exec "$repo_root/scripts/profile_android.sh" all \
    --profile-build --launch --launch-env "AMBITION_BOOT_ROUTE=$route" --launch-env "AMBITION_AUTOPLAY=$autoplay_profile" \
    --warmup 25 --duration 30 --name "$name" "$@"
