#!/usr/bin/env bash
# Does a theme the player walked away from actually leave `Assets<Image>`?
#
# `asset-preparation-and-residency.md` open work 4 — residency ownership.
# `ParallaxLayerSet::retain_themes` drops the set's handles; Bevy frees the
# pixels only when the LAST handle drops. The crate-level guards prove the
# first half. This proves the second, in a real composition.
#
# ⛔⛔ WHY A SCRIPT AND NOT JUST A TEST, same reason as
# `measure_hall_redecodes.sh`: `ambition_app` has ONE `[[test]]` target, so every
# file under `tests/` is a module of `app_it` sharing a process, and cargo runs
# them as PARALLEL THREADS. A sibling test booting its own app populates
# `Assets<Image>` underneath this one's assertions. Hence `#[ignore]` plus an
# exact filter — run it any other way and the number is wrong.
#
# ⛔ THE ROUTE IS NOT THE HALL DOOR. `central_hub_main` and `hall_of_characters`
# both resolve to `ParallaxTheme::Hub`, so that walk crosses no theme boundary
# and a retire assertion on it would pass while doing nothing. This walks
# `tech_bros_door` into the Basement theme instead.
#
# USAGE: scripts/measure_parallax_retire.sh
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

TEST=parallax_theme_retires_on_walk::a_theme_the_player_walked_away_from_leaves_assets_image

# ⛔ Not piped into grep — a pipeline's exit status is the LAST command's, and
# this script's whole value is that a red test is a red script. (Learned twice
# in one day: a `cargo check --workspace` read through `| tail` reported success
# while the build was failing.)
cargo test -p ambition_app --test app_it -- \
    --ignored --exact --nocapture --test-threads=1 "$TEST"
