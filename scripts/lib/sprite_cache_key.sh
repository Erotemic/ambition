#!/usr/bin/env bash
# The per-sheet cache key of `scripts/regen/sprites.sh`, sourced by it. A file
# of its own so that `scripts/tests/test_sprite_cache_key.py` can call the
# functions the publish calls.

# Resolve the two inputs the key reads beyond the renderer files: the files
# under targets/ that the core imports, and each target's files. Fails (returns
# non-zero, with the reason on stderr) when the renderer cannot say: a key
# without a target's files is the key of a different target.
sprite_cache_key_prepare() {
    core_target_files="$(
        cd "$renderer_dir" && "$python_bin" "$repo_root/scripts/lib/sprite_target_closure.py" core .
    )" || return 1
    (
        cd "$renderer_dir" && "$python_bin" "$repo_root/scripts/lib/sprite_target_closure.py" targets .
    ) > "$target_closures_file" || return 1
}

# --- Per-sheet cache ------------------------------------------------------
# The global cache commits only after a fully successful run. Per-sheet keys
# allow an interrupted run to resume without rebuilding current targets. Each
# key combines the shared renderer core with the files of that target: a core
# change invalidates every target, a change to a target's files only the
# targets that read them.
#
# ⭐ A TARGET'S FILES ARE WHAT IT IMPORTS, NOT WHAT ITS NAME MATCHES
# (`scripts/lib/sprite_target_closure.py`). Every `targets/**/_*.py` helper
# was in the shared core, so an edit to one family's rig (the crew's
# `_person_rig.py`) made all 271 sheets stale: a republish of about one hour
# (measured 2026-10-13). And four kinds of file a target reads were in no key:
# the module that defines a target whose name is not a file (`craig` is
# `crypto_crew.py`), a config target's config file, the modules a helper
# package loads by name (`_toon_presets/`), and a shared module with no
# underscore (`targets/super_mary_o_common.py`).
#
# Requires, set by the caller: `renderer_dir`, `repo_root`, `python_bin`,
# `core_target_files` (from `sprite_cache_key_prepare`) and
# `target_closures_file`.

compute_core_shared() {
    # NOTE: each `find` gets its OWN `| sort -z | xargs -0 sha256sum`
    # pipeline. Piping several `find … -print0` from one `{ … }` block
    # into a single `xargs` silently drops all but the first find's
    # output, so keep them separate (same structure as
    # `compute_fingerprint`).
    (
        cd "$renderer_dir" || exit 1
        # Top-level package modules (__init__, __main__, ldtk_manifest).
        find ambition_sprite2d_renderer -maxdepth 1 -type f -name '*.py' -print0 \
            | sort -z | xargs -0 sha256sum
        # Shared render infra subpackages — the draw primitives, sheet
        # spines, RON emitter, packer, discovery, and CLI every target
        # renders through. These live in subpackages (core/, authoring/,
        # registry/, cli/), NOT at the package top level, so the maxdepth-1
        # find above does not see them.
        find ambition_sprite2d_renderer/core ambition_sprite2d_renderer/authoring \
            ambition_sprite2d_renderer/registry ambition_sprite2d_renderer/cli \
            -type f -name '*.py' -print0 \
            | sort -z | xargs -0 sha256sum
        # Package markers under targets/, and the files under targets/ that
        # the core itself imports: the core runs for every target.
        find ambition_sprite2d_renderer/targets -type f -name '__init__.py' -print0 \
            | sort -z | xargs -0 sha256sum
        printf '%s\n' "$core_target_files" | sed '/^$/d' | tr '\n' '\0' \
            | xargs -0 -r sha256sum
        # Renderer-dir top-level scripts (e.g. the mockingbird generator).
        find . -maxdepth 1 -type f \( -name '*.py' -o -name '*.sh' \) -print0 \
            | sort -z | xargs -0 sha256sum
        # NB: this orchestrator (`scripts/regen/sprites.sh`) is deliberately NOT
        # hashed into CORE_SHARED. It only chooses *which* targets to
        # publish and *how* to loop — it never affects a sheet's pixels.
        # Folding it in here meant that wiring a new sprite (adding its
        # name to `tackon_targets`) changed CORE_SHARED and invalidated
        # EVERY per-sheet key, forcing a full regen just to render the
        # one new sheet. The global fingerprint above (`compute_fingerprint`)
        # still includes this script, so the all-or-nothing fast-path
        # correctly re-checks when the script changes.
    ) | sha256sum | awk '{print $1}'
}

# Hash a target's own source (single-file module or package dir) AND the
# authored DATA it renders from.
#
# ⛔⛔ THE DATA HALF IS NOT OPTIONAL, AND LEAVING IT OUT SHIPPED A KNOWN-FIXED
# BUG FOR THREE DAYS. This hashed only `.py`/`.rig.json` leaves, and
# CORE_SHARED hashes only `.py` — so NOTHING under `data/` was in the key. On
# 2026-08-27 `f21eac9` ("The Officer stops shooting over his own shoulder")
# rewrote `data/motion/humanoid/officer_brawler_v1/clips/shoot.clip.json`. The
# key did not move, the Aug-28 publish hit the cache and skipped the render,
# and the Officer kept firing over his shoulder in the game until 2026-08-30.
#
# ⛔ AND THE OUTPUT-DIGEST GUARD COULD NOT SEE IT. `sheet_cache_fresh` asks
# whether the files on disk are the ones this cache PRODUCED; they were. Output
# hashing catches a SUBSTITUTED sheet. It cannot catch a faithfully-cached sheet
# built from a source that has since changed — that is what the key is for.
#
# ⭐ RESOLVED PER-CHARACTER, NOT GLOBALLY. Folding `data/` into CORE_SHARED
# would make one clip edit invalidate every sheet in the roster, which is the
# same mistake the CORE_SHARED note above already refuses for this script. A
# character names its own motion library, so follow that edge and hash exactly
# what this target reads.
leaf_hash() {
    local name="$1"
    (
        cd "$renderer_dir" || exit 1
        local f d
        # Rigged characters (GUI .rig.json docs) are data, not a .py leaf —
        # hash the document so editing a rig invalidates only its sheet.
        local rig="ambition_sprite2d_renderer/targets/characters/rigged/$name.rig.json"
        if [ -f "$rig" ]; then sha256sum "$rig"; fi
        # The files the registry says this target is made of, and every file
        # under targets/ they import.
        awk -F'\t' -v n="$name" '$1 == n { print $2 }' "$target_closures_file" \
            | tr '\n' '\0' | xargs -0 -r sha256sum
        # The authored data this character renders from: its SVG, sockets and
        # `<name>.motion.json`.
        local cdir="ambition_sprite2d_renderer/data/characters/$name"
        if [ -d "$cdir" ]; then
            find "$cdir" -type f -print0 | sort -z | xargs -0 sha256sum
            # …and the motion library that file BINDS to, which is where the
            # poses and clips actually live. Path is relative to the character
            # dir, exactly as the document states it.
            local motion="$cdir/$name.motion.json"
            if [ -f "$motion" ]; then
                local rel lib
                rel="$(sed -n 's/.*"motion_library"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$motion" | head -1)"
                if [ -n "$rel" ]; then
                    lib="$(cd "$cdir" 2>/dev/null && cd "$(dirname "$rel")" 2>/dev/null && pwd)"
                    if [ -n "$lib" ] && [ -d "$lib" ]; then
                        find "$lib" -type f -print0 | sort -z | xargs -0 sha256sum \
                            | awk '{print $1}'
                    fi
                fi
            fi
        fi
    ) | sha256sum | awk '{print $1}'
}

unit_key() {
    printf '%s:%s' "$core_shared_fingerprint" "$(leaf_hash "$1")" \
        | sha256sum | awk '{print $1}'
}
