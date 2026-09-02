#!/usr/bin/env bash
# Regenerate every generated runtime asset, plus the `.agent/` navigation index.
#
# Usage:
#   scripts/setup/generated_content.sh
#   scripts/setup/generated_content.sh --skip-navigation
#   scripts/setup/generated_content.sh --help
#
# ⚠ Needs `scripts/setup/python_tools.sh` and `scripts/setup/audio_libraries.sh`
# first: a category whose toolchain is absent REFUSES rather than publishing
# degraded art, and music refuses rather than rendering General-MIDI stand-ins.
set -euo pipefail

repo_root="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/../.." && pwd)"
AMBITION_SETUP_LABEL=generated-content
# shellcheck source=../lib/setup_common.sh
. "$repo_root/scripts/lib/setup_common.sh"
# shellcheck source=../lib/tool_python.sh
. "$repo_root/scripts/lib/tool_python.sh"
setup_load_cargo_env

skip_assets=0
skip_python=0
skip_navigation=0
while [ "$#" -gt 0 ]; do
    case "$1" in
        --skip-navigation) skip_navigation=1 ;;
        -h|--help) setup_usage "$0"; exit 0 ;;
        *) fatal "unknown option: $1" ;;
    esac
    shift
done

# ⛔ "THE SETUP RAN" IS NOT "THE CONTENT IS CURRENT", and the gap is invisible
# from inside the game. Measured on `calculex` 2026-08-29: 998 sprite sheets
# ~11 days stale and the parallax quality tiers absent entirely, so the game drew
# OLD art at Low/Medium and NO parallax art at all. The tiers meant for weak
# hardware were the ones most likely to be broken, on the machines least able to
# notice — nothing in the game says "this sheet is eleven days behind its source".
#
# ⭐ THE CHECKER ALREADY EXISTED AND NOTHING CALLED IT. That is the whole defect;
# the regeneration itself was already composed correctly by `regen/assets.sh`.
#
# Reports rather than fails: a stale variant is a degraded picture, not a broken
# checkout, and a setup script that exits non-zero over it would block work that
# does not care.
verify_generated_content_is_current() {
    have python3 || return 0
    [ -f "$repo_root/scripts/check_quality_variants_are_fresh.py" ] || return 0
    if python3 "$repo_root/scripts/check_quality_variants_are_fresh.py" >/dev/null 2>&1; then
        log "generated quality variants are current"
        return 0
    fi
    warn "generated quality variants are STALE or MISSING after regeneration"
    log "   the game draws old art at Low/Medium and may draw none at all:"
    log "     python3 scripts/check_quality_variants_are_fresh.py   # what is stale"
    log "     ./scripts/regen/quality_variants.sh                   # rebuild it"
}

# ⛔⛔ FRESHNESS AND PRESENCE ARE TWO DIFFERENT FAILURES, and only one of them
# had an instrument. `verify_generated_content_is_current` above asks *"is the
# published tier art OLDER than its source?"* — which says nothing about art
# that was never published at all, because A FILE THAT DOES NOT EXIST CANNOT BE
# STALE.
#
# ⚠ MEASURED 2026-08-30, the first time `cargo test --workspace --lib` was run
# to completion (D-QTT-1): it failed inside `ambition_render` on
# `a_left_drawn_character_faces_the_way_they_are_going_like_a_right_drawn_one`,
# which uses `goblin_cave_dagger` as its canonical RIGHT-drawn sheet.
# `record_for_sheet_key` returned `None` because no roster line ever published
# it. The panic named handedness; the defect was a missing ASSET, and the whole
# goblin weapon family was absent with it.
#
# ⭐ GENERATED ART IS GITIGNORED, so "it works on my checkout" is the EXPECTED
# symptom of this class rather than a surprising one: whoever rendered the
# target by hand has it and nobody else does. That is exactly why it belongs in
# the bootstrap and not in a person's memory.
#
# GENERATES rather than only reporting, because the whole lesson of the
# quality-variant row above is that a checker nothing acts on is a checker
# nobody runs.
regenerate_missing_published_sheets() {
    have python3 || return 0
    local checker="$repo_root/scripts/check_published_sheets_are_present.py"
    [ -f "$checker" ] || return 0

    # Through the renderer's own interpreter: the check asks each target what it
    # DECLARES it installs, which needs the package importable. Without it the
    # script says "cannot check" and succeeds rather than inventing a verdict.
    #
    # ⛔ WHICH MADE THIS A FALSE GREEN FOR AS LONG AS IT EXISTED. The call here
    # was `tool_python ambition_sprite2d_renderer` — a function that is not
    # defined anywhere, given a bare tool name where the resolver wants a
    # DIRECTORY. It therefore always fell through to the bare `python3` that
    # cannot import the renderer, the checker duly reported "cannot check" and
    # exited 0, and setup logged "every rostered sheet is published" without
    # having checked one. `[ -x "$py" ]` could not catch it either: that test is
    # false for the bare name `python3`, so the guard "corrected" a good path to
    # the same fallback.
    local py
    py="$(ambition_select_tool_python \
        "$repo_root/tools/ambition_sprite2d_renderer" AMBITION_SPRITE_PYTHON)"
    ambition_python_exists "$py" || py="python3"

    local missing
    if missing="$("$py" "$checker" 2>/dev/null)"; then
        log "every rostered sheet is published"
        return 0
    fi

    warn "rostered sheets are MISSING — regenerating them"
    printf '%s\n' "$missing" | sed 's/^/     /'
    local names
    names="$(printf '%s\n' "$missing" | awk '/^ *missing /{print $2}')"
    [ -n "$names" ] || return 0
    local name
    for name in $names; do
        log "  publishing $name"
        "$repo_root/scripts/regen/sprites.sh" --target "$name" --force >/dev/null 2>&1 \
            || warn "  could not publish $name — run sprites.sh --target $name to see why"
    done
    if "$py" "$checker" >/dev/null 2>&1; then
        log "every rostered sheet is published"
    else
        warn "some rostered sheets are still missing; see the list above"
    fi
}

regenerate_assets() {
    if [ "$skip_assets" -eq 1 ]; then
        log "skipping generated assets"
        return 0
    fi
    if [ "$skip_python" -eq 1 ]; then
        log "checking existing tool-local environments"
        verify_tool_environments
    fi

    log "regenerating all runtime assets"
    "$repo_root/scripts/regen/assets.sh"
    regenerate_missing_published_sheets
    verify_generated_content_is_current
}

# ⛔ `scripts/agent_query.py` IS STEP 2 OF THE COLD START AGENTS.md MANDATES, AND
# ON A FRESH CLONE IT ANSWERED NOTHING. The `.agent/` packets it queries are
# generated, not committed — `git ls-files .agent` is two files — so a new
# checkout got `⚠ index has no generation stamp` and an empty packet from the
# first command the guide tells an agent to run. Every other kind of generated
# content in this repo is built by this script; this one was left to a person
# knowing to run it.
#
# Reports rather than fails: `source_navigation.sh` ends with the agent-KB
# audit, which is repository DOC hygiene (frontmatter keys, stray files, a
# dangling doc reference) and has nothing to do with whether this checkout is
# runnable. Setup must not refuse a working environment over it.
regenerate_source_navigation() {
    if [ "$skip_assets" -eq 1 ]; then
        log "skipping generated navigation"
        return 0
    fi
    [ -x "$repo_root/scripts/regen/source_navigation.sh" ] || return 0
    # The ECS inventory resolves the dependency graph through cargo, and prints
    # `resolved dependency graph unavailable` and carries on when it cannot —
    # a quietly poorer index rather than an error.
    if [ -f "$HOME/.cargo/env" ]; then
        # shellcheck disable=SC1091
        . "$HOME/.cargo/env"
    fi
    log "regenerating .agent/ navigation"
    if "$repo_root/scripts/regen/source_navigation.sh" >/dev/null 2>&1; then
        log "generated navigation is current"
        return 0
    fi
    # The indexes are written before the audit runs, so a non-zero exit here
    # does not mean they are missing. Say which it was.
    if [ -d "$repo_root/.agent/index" ]; then
        warn "navigation regenerated, but the agent-KB audit reported doc-hygiene problems"
        log "   they do not affect the build:"
        log "     ./scripts/regen/source_navigation.sh   # what it objects to"
    else
        warn "navigation regeneration FAILED; scripts/agent_query.py will answer nothing"
        log "     ./scripts/regen/source_navigation.sh   # see why"
    fi
}

regenerate_assets
if [ "$skip_navigation" -eq 0 ]; then
    regenerate_source_navigation
fi
