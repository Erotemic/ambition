#!/usr/bin/env bash
# **WHERE THIS REPO KEEPS ITS GENERATED ASSETS — declared once.**
#
# ⛔ Every generator that installs into the game used to hard-code the consuming
# crate's name, and the music renderer hard-coded it FOUR times inside a git
# submodule. That name is not stable and never was: it has been
# `ambition_actors`, it is `ambition_platformer2d_actor_monolith`, and it moves
# again when the monolith is decomposed. On 2026-08-07 the submodule and this
# repo disagreed about it, so `radio render-publish` CREATED the directory it
# expected, wrote 69 cues into it, and exited 0 while the registry generator read
# the crate that exists and found nothing new. Both halves correct, disagreeing.
#
# ⭐ So the consumer declares, and the tools are TOLD. A tool that guesses is
# wrong for every value it does not hold; a tool that is told is wrong once, here,
# where one edit fixes it.
#
# Usage: source this file, then use the variables. Paths are absolute.

ambition_repo_root() {
    # This file is <repo>/scripts/lib/asset_roots.sh.
    (cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)
}

# The crate whose `assets/` directory ships to the game. ⚠ THE ONE LINE TO EDIT
# when the monolith is renamed or decomposed.
AMBITION_ASSET_CRATE="${AMBITION_ASSET_CRATE:-ambition_platformer2d_actor_monolith}"

AMBITION_ASSETS_ROOT="$(ambition_repo_root)/crates/${AMBITION_ASSET_CRATE}/assets"
# What the music renderer publishes into and the registry generator projects from
# — the same directory, named once, so the two cannot drift apart again.
AMBITION_MUSIC_PUBLISH_ROOT="${AMBITION_ASSETS_ROOT}/audio/music/generated"

export AMBITION_ASSET_CRATE AMBITION_ASSETS_ROOT AMBITION_MUSIC_PUBLISH_ROOT
