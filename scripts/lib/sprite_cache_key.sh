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
# `core_target_files` (from `sprite_cache_key_prepare`), `target_closures_file`
# and `target_reads_dir`; and, for `compute_fingerprint`, the function
# `renderer_capability_fingerprint`.

# The key of the whole publish: the authored inputs
# (`compute_authored_fingerprint`) and the files each recorded render read.
compute_fingerprint() {
    {
        printf 'authored %s\n' "$(compute_authored_fingerprint)"
        # The per-sheet keys read the same records, so a key of the whole
        # publish that omits one would hit where a sheet key misses.
        hash_reads "$target_reads_dir"/*.reads
    } | sha256sum | awk '{print $1}'
}

# The fingerprint to save after a publish that succeeded, or a non-zero status
# when an authored input changed while it ran (`started` is
# `compute_authored_fingerprint` from the start of the run).
#
# ⛔ The publish saved the fingerprint it computed BEFORE it rendered. Each
# render writes its read record, and the records are in the fingerprint, so a
# publish that changed a record saved a fingerprint that `--check` called
# stale at once, and the next run took the slow path again (review of
# 2026-10-14). The records a run leaves are the ones its renders read, so the
# fingerprint is computed again from them. An authored file edited during the
# run may not be what was rendered, so then nothing is saved and the next run
# looks again.
fingerprint_after_publish() {
    local started="$1"
    [ "$(compute_authored_fingerprint)" = "$started" ] || return 1
    compute_fingerprint
}

# Every authored input of the publish: the renderer's files, this script and
# the toolchain. Not the read records, which a render writes.
compute_authored_fingerprint() {
    # `cd` into renderer dir so the file paths in `sha256sum` output
    # are relative; absolute paths would make the hash depend on the
    # filesystem location.
    (
        cd "$renderer_dir" || exit 1
        {
            # ⛔ EVERY FILE OF THE RENDERER, NOT ONLY ITS CODE. This hashed the
            # `.py`, `.yaml` and `.json` files. A comment added to
            # `gnu_ton_scholar_villain.svg` gave a cache hit, and no target was
            # published (measured 2026-10-14). The phases with no per-sheet key
            # (entities, factions) record nothing, so they are keyed by
            # everything they can read here.
            local authored=(ambition_sprite2d_renderer)
            [ -d assets ] && authored+=(assets)
            find "${authored[@]}" -type f -not -path '*/__pycache__/*' -not -name '*.pyc' -print0 \
                | sort -z \
                | xargs -0 -r sha256sum
            find . -maxdepth 1 -type f \( -name '*.py' -o -name '*.sh' \) -print0 \
                | sort -z \
                | xargs -0 sha256sum
            # The orchestrator script itself: changes to the install
            # loops, expected-files list, or the cache logic must
            # invalidate the cache too. Hash relative to repo root
            # to keep stability across filesystem locations.
            sha256sum "$repo_root/scripts/regen/sprites.sh" \
                | awk -v root="$repo_root/" '{sub(root, "", $2); print}'
            # ⛔⛔ THE TOOLCHAIN IS AN INPUT. The same source art renders
            # DIFFERENTLY, or not at all, depending on whether the native SVG
            # rasteriser is importable — `resvg-py` is declared in the
            # renderer's pyproject, and v3 of the player robot is SVG-rigged
            # where v2 is not. A checkout whose tool venv is missing or points
            # at another user's Python silently resolves to a bare `python3`
            # without it.
            #
            # ⚠ MEASURED 2026-08-29: all three `tools/*/.venv/pyvenv.cfg` in
            # this shared checkout name `/home/joncrall/.local/share/uv/...`,
            # so the venv is real for one user and interpreter-less for anyone
            # else on the same filesystem. Without this line both produce
            # artifacts under one fingerprint and each treats the other's as
            # current.
            renderer_capability_fingerprint
        }
    ) | sha256sum | awk '{print $1}'
}

# The Ultrapack packer, relative to the renderer.
ultrapack_packer=ambition_sprite2d_renderer/authoring/ultrapack.py

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
        # Not the Ultrapack packer: no target renders through it (only the
        # `ultrapack` command imports it), so an edit to it must not make
        # every sheet stale. The Ultrapack tiers key it
        # (`ultrapack_inputs_key`).
        find ambition_sprite2d_renderer/core ambition_sprite2d_renderer/authoring \
            ambition_sprite2d_renderer/registry ambition_sprite2d_renderer/cli \
            -type f -name '*.py' ! -path "$ultrapack_packer" -print0 \
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

# Hash a target's own source and the authored data it renders from.
#
# The source is the static import closure of the files the registry says the
# target is made of (`target_closures_file`). The data is what the target's
# last render read, as the render recorded it (`<target_reads_dir>/<name>.reads`,
# written by `publish-many` under `AMBITION_SPRITE_READS_DIR`; see the
# renderer's `read_record.py`).
#
# ⛔ THE DATA WAS GUESSED BY NAME, AND A GUESS MISSES WHAT IS NOT NAMED FOR
# THE TARGET. The key hashed `rigged/<name>.rig.json` and
# `data/characters/<name>/`. `gnu_ton_scholar_fighter` draws
# `gnu_ton_scholar_villain.svg`, and `emu` is `rigged/emu/emu_side.rig.json`:
# an edit to either kept the key, and the sheet stayed stale (review of
# 2026-10-14). Before the name guess, nothing under `data/` was in the key, and
# the Officer fired over his shoulder for three days after his clip was fixed
# (2026-08-27).
#
# A record lists what was read last time. That is enough: a render can read a
# new file only when something it read or ran has changed, and each of those
# is in the key. A target with no record has never published under one, so
# its key is one that no stored key can equal.
leaf_hash() {
    local name="$1"
    (
        cd "$renderer_dir" || exit 1
        awk -F'\t' -v n="$name" '$1 == n { print $2 }' "$target_closures_file" \
            | tr '\n' '\0' | xargs -0 -r sha256sum
        local record="$target_reads_dir/$name.reads"
        if [ ! -f "$record" ]; then
            printf 'unrecorded %s %s %s\n' "$name" "$(date +%s%N)" "$RANDOM"
            exit 0
        fi
        hash_reads "$record"
    ) | sha256sum | awk '{print $1}'
}

# The text that keys the reads the records `$@` list: the content of each `f`
# file and the listing of each `d` directory, relative to `repo_root`. A
# record that does not exist lists nothing.
hash_reads() {
    (
        cd "$repo_root" || exit 1
        local rel files=() dirs=()
        while IFS= read -r rel; do
            if [ -f "$rel" ]; then
                files+=("$rel")
            else
                printf 'missing %s\n' "$rel"
            fi
        done < <(awk '$1 == "f" { print $2 }' "$@" 2>/dev/null | LC_ALL=C sort -u)
        [ "${#files[@]}" -eq 0 ] || sha256sum "${files[@]}"
        while IFS= read -r rel; do
            printf 'directory %s\n' "$rel"
            ls -A "$rel" 2>/dev/null | LC_ALL=C sort || true
        done < <(awk '$1 == "d" { print $2 }' "$@" 2>/dev/null | LC_ALL=C sort -u)
    )
}

# The key of a renderer script that is not a target (the director vanity
# card): the shared core, the script files `$3...` (relative to the renderer),
# each target in the space-separated list `$2` that the scripts import (its
# files and its reads), and what the script's last run read (the record `$1`,
# written by `read_record`). A script with no record has never run under one,
# so its key is one that no stored key can equal.
script_key() {
    local record="$1" targets="$2" target
    shift 2
    {
        printf '%s\n' "$core_shared_fingerprint"
        (cd "$renderer_dir" && sha256sum "$@")
        for target in $targets; do
            leaf_hash "$target"
        done
        if [ -f "$record" ]; then
            hash_reads "$record"
        else
            printf 'unrecorded %s %s %s\n' "$record" "$(date +%s%N)" "$RANDOM"
        fi
    } | sha256sum | awk '{print $1}'
}

unit_key() {
    printf '%s:%s' "$core_shared_fingerprint" "$(leaf_hash "$1")" \
        | sha256sum | awk '{print $1}'
}

# The inputs of the Ultrapack tiers: the shared core (it holds the
# `ultrapack` command), the packer (which the core leaves out), the pack plan
# `$2`, and the name, size and mtime of each sheet file the tiers pool from
# the sheet root `$1` (`ultrapack_rendered` reads the top-level
# `*_spritesheet.yaml` and its pages). A re-render writes a new mtime, so a
# changed sheet is a changed key; the staging copy keeps mtimes (`cp -a`), so
# an unchanged sheet keeps its key.
ultrapack_inputs_key() {
    local sheets_dir="$1" plan="$2"
    {
        printf 'core %s\n' "$core_shared_fingerprint"
        printf 'packer %s\n' "$( (cd "$renderer_dir" && sha256sum "$ultrapack_packer" 2>/dev/null) | awk '{print $1}')"
        if [ -f "$plan" ]; then
            printf 'plan %s\n' "$(sha256sum "$plan" | awk '{print $1}')"
        fi
        find "$sheets_dir" -maxdepth 1 -type f -name '*_spritesheet*' \
            \( -name '*.yaml' -o -name '*.png' \) -printf '%f %s %T@\n' | LC_ALL=C sort
    } | sha256sum | awk '{print $1}'
}
