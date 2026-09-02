#!/usr/bin/env bash
# How many image paths the Hall entry decodes a SECOND time.
#
# `asset-preparation-and-residency.md` Open work 5 — accidental re-preparation.
# The ledger has counted `re_decodes` all along; nothing had run that counter
# over the one transition big enough for a repeat to cost anything.
#
# ⛔⛔ WHY A SCRIPT AND NOT JUST A TEST. `ambition_app` has ONE `[[test]]`
# target, so every file under `tests/` is a module of `app_it` sharing a process,
# and cargo runs those tests as PARALLEL THREADS. The image ledger is a
# process-global static: a sibling test booting its own app mid-window lands its
# decodes in the count. The test's own before/after delta already excludes
# everything that ran EARLIER in the process; only concurrency is left, and the
# only fix for that is to run the test alone. Hence `#[ignore]` plus this exact
# filter — running it any other way produces a number, and the number is wrong.
#
# USAGE: scripts/measure_hall_redecodes.sh
set -euo pipefail
cd "$(git rev-parse --show-toplevel)"

TEST=hall_redecode_census::the_halls_entry_is_counted_for_art_it_decodes_twice

# `--exact` and `--test-threads=1` together: the filter keeps siblings out of the
# run, the thread cap keeps anything the filter still matches from overlapping.
# ⛔ Not piped into grep — a pipeline's exit status is the LAST command's, and
# this script's whole value is that a red test is a red script.
cargo test -p ambition_app --test app_it -- \
    --ignored --exact --nocapture --test-threads=1 "$TEST"
