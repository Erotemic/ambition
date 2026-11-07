#!/usr/bin/env bash
# An unattended phone profile of one game: the hub boots into the game, the
# game plays itself by navigation, and `profile_android.sh` builds, installs,
# launches and records.
#
#   scripts/measure/autoprofile_android.sh                 # Ambition (the hall of characters, then a flight through the primeval trail)
#   scripts/measure/autoprofile_android.sh castle_godwin   # Castle Godwin combat route
#   scripts/measure/autoprofile_android.sh mary_o          # Mary-O (the cosmic form, to each goal)
#   scripts/measure/autoprofile_android.sh smash           # 2 CPUs, real Smash stage
#   scripts/measure/autoprofile_android.sh smash_4p        # 4 CPUs, real Smash stage
#   scripts/measure/autoprofile_android.sh ambition --duration 60 --warmup 40
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
# Read the result with `scripts/measure/symbolize_android_profile.py DIR` (the
# script prints DIR at the end).
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"

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
    smash|smash_4p) game="smash"; route="smash_select" ;;
    *)
        echo "autoprofile_android.sh: no autoplay policy for '$autoplay_profile' (ambition, castle_godwin, mary_o, smash, smash_4p)" >&2
        exit 2
        ;;
esac

name="all-${autoplay_profile}-auto-$(date -u +%Y%m%dT%H%M%SZ)"
echo "[autoprofile] game=$game autoplay=$autoplay_profile boot_route=$route name=$name"
# Unset, so a build made here bakes in no autoplay (`option_env!`).
unset AMBITION_BOOT_ROUTE AMBITION_AUTOPLAY AMBITION_HUB_GAMES
# A CPU match needs time for the app's startup, the select route, stage
# preparation, and Smash's opening ceremony. A long default capture makes
# thermal throttling visible; explicit CLI flags still override both.
if [[ "$game" == "smash" ]]; then
    default_warmup=35
    default_duration=60
else
    default_warmup=25
    default_duration=30
fi
exec "$repo_root/scripts/measure/profile_android.sh" all \
    --profile-build --launch --launch-env "AMBITION_BOOT_ROUTE=$route" --launch-env "AMBITION_AUTOPLAY=$autoplay_profile" \
    --warmup "$default_warmup" --duration "$default_duration" --name "$name" "$@"
