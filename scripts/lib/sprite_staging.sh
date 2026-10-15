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
# Sourced by `scripts/regen/sprites.sh` and its tests. The caller sets
# `asset_root` (the directory that holds the runtime roots) and
# `staging_root` (on the same file system, so the swap is a rename).

# The runtime roots a publish writes. Each one is staged and swapped whole.
staged_root_names=(sprites sprites_0_5x sprites_0_25x sprites_potato sprite_packs)

# Make the staged copy, or continue the one a stopped run left. A root that
# the copy does not have is copied from the runtime root. This also repairs
# a swap that stopped part way: the roots it moved are copied back.
begin_staging() {
    local name
    mkdir -p "$staging_root"
    if [ -n "$(ls -A "$staging_root" 2>/dev/null)" ]; then
        echo "==> staging root: continuing the copy a stopped run left ($staging_root)"
    fi
    for name in "${staged_root_names[@]}"; do
        [ -d "$staging_root/$name" ] && continue
        rm -rf "$staging_root/$name.partial"
        if [ -d "$asset_root/$name" ]; then
            cp -a "$asset_root/$name" "$staging_root/$name.partial"
        else
            mkdir -p "$staging_root/$name.partial"
        fi
        # Moved into place whole: a copy cut short is never read as a root.
        mv "$staging_root/$name.partial" "$staging_root/$name"
    done
}

# Move each staged root into place, and remove the staging root.
finish_staging() {
    local name
    for name in "${staged_root_names[@]}"; do
        rm -rf "$staging_root/$name.old"
        if [ -e "$asset_root/$name" ]; then
            mv "$asset_root/$name" "$staging_root/$name.old"
        fi
        mv "$staging_root/$name" "$asset_root/$name"
    done
    rm -rf "$staging_root"
}
