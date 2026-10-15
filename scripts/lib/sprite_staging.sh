# The staging root of a full sprite publish.
#
# A publish writes into a copy of the runtime roots and moves the copy into
# place only when every sprite phase passed. A publish that stops part way
# (a timeout, a kill, a failed phase) thus leaves the runtime roots as the
# last whole set. Before this, the publish wrote in place, and a stopped run
# left a mixed set: on 2026-10-13 `sandbag`'s part flipbook named a clip
# that its sheet did not have, and app_it reported it as a code defect.
#
# The staged copy stays after a stopped run, and the next run continues it.
# The per-sheet cache keys hash the files on disk, so the targets that the
# stopped run published into the copy stay fresh, and the rest are stale.
#
# The copy is not made again for each run. The swap keeps the set that it
# replaces in `mirror_root`, and the next run brings that set up to date with
# `rsync`, which copies only the files that differ: the files the last
# publish wrote. A full copy of the 1.6 GB is 26 s; an update with no change
# is 0.6 s (measured 2026-10-15). The mirror is a cache: it costs one more
# copy of the roots on disk, and a run without it makes the full copy.
#
# Sourced by `scripts/regen/sprites.sh` and its tests. The caller sets
# `asset_root` (the directory that holds the runtime roots), `staging_root`
# and `mirror_root` (on the same file system, so each move is a rename).

# The runtime roots a publish writes. Each one is staged and swapped whole.
staged_root_names=(sprites sprites_0_5x sprites_0_25x sprites_potato sprite_packs)

# Make the staged copy, or continue the one a stopped run left. A root that
# the copy does not have is copied from the runtime root, starting from the
# mirror when there is one. This also repairs a swap that stopped part way:
# the roots it moved are copied back.
begin_staging() {
    local name
    mkdir -p "$staging_root"
    if [ -n "$(ls -A "$staging_root" 2>/dev/null)" ]; then
        echo "==> staging root: continuing the copy a stopped run left ($staging_root)"
    fi
    for name in "${staged_root_names[@]}"; do
        [ -d "$staging_root/$name" ] && continue
        # A copy that stopped part way is a good start too: rsync finishes it.
        if [ ! -d "$staging_root/$name.partial" ] && [ -d "$mirror_root/$name" ]; then
            mv "$mirror_root/$name" "$staging_root/$name.partial"
        fi
        mkdir -p "$staging_root/$name.partial"
        if [ -d "$asset_root/$name" ]; then
            rsync -a --delete "$asset_root/$name/" "$staging_root/$name.partial/"
        else
            rm -rf "$staging_root/$name.partial"
            mkdir -p "$staging_root/$name.partial"
        fi
        # Moved into place whole: a copy cut short is never read as a root.
        mv "$staging_root/$name.partial" "$staging_root/$name"
    done
}

# Move each staged root into place, keep the root it replaces as the mirror
# for the next run, and remove the staging root.
finish_staging() {
    local name
    mkdir -p "$mirror_root"
    for name in "${staged_root_names[@]}"; do
        rm -rf "$staging_root/$name.old"
        if [ -e "$asset_root/$name" ]; then
            mv "$asset_root/$name" "$staging_root/$name.old"
        fi
        mv "$staging_root/$name" "$asset_root/$name"
        rm -rf "$mirror_root/$name"
        if [ -e "$staging_root/$name.old" ]; then
            mv "$staging_root/$name.old" "$mirror_root/$name"
        fi
    done
    rm -rf "$staging_root"
}
