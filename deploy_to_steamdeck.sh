#!/usr/bin/env bash
set -euo pipefail

REPO="${REPO:-$HOME/code/ambition}"
DECK="${DECK:-deck@steamdeck}"
APPDIR="${APPDIR:-/home/deck/Games/ambition}"

cd "$REPO"

# Optional but useful: fail before deploying a bad map.
PYTHONPATH="$REPO/tools/ambition_ldtk_tools" \
    python -m ambition_ldtk_tools validate \
    crates/ambition_sandbox/assets/ambition/worlds/sandbox.ldtk

# Safest build: keep default desktop features, add static_map fallback.
cargo build -p ambition_sandbox --bin ambition_sandbox --release --features static_map

ssh "$DECK" "mkdir -p '$APPDIR'"

rsync -av --delete \
    target/release/ambition_sandbox \
    "$DECK:$APPDIR/"

rsync -av --delete \
    crates/ambition_sandbox/assets/ \
    "$DECK:$APPDIR/assets/"

# Compatibility symlinks:
# - BEVY_ASSET_ROOT should be the app/root dir. Bevy's default asset folder is
#   then $BEVY_ASSET_ROOT/assets, which matches the rsync destination above.
# - Current sprite preflight code also checks $BEVY_ASSET_ROOT/<rel_path>, so
#   expose sprites/audio/ambition at the app root as symlinks too.
# - The assets/assets -> . link also tolerates launchers that accidentally set
#   BEVY_ASSET_ROOT=$APPDIR/assets.
ssh "$DECK" "bash -s" <<EOF_REMOTE
set -euo pipefail
APPDIR='$APPDIR'
cd "\$APPDIR"
rm -rf sprites audio ambition
ln -sfn assets/sprites sprites
ln -sfn assets/audio audio
ln -sfn assets/ambition ambition
cd "\$APPDIR/assets"
ln -sfn . assets
EOF_REMOTE

ssh "$DECK" "cat > '$APPDIR/run_ambition.sh' && chmod +x '$APPDIR/run_ambition.sh'" <<'EOF_INNER'
#!/usr/bin/env bash
set -euo pipefail

APPDIR="$HOME/Games/ambition"
cd "$APPDIR"

# Important: this is the app/root directory, not the assets directory.
# Bevy's default asset folder is "$BEVY_ASSET_ROOT/assets".
export BEVY_ASSET_ROOT="$APPDIR"

# Ambition's direct LDtk loader should use the Deck-side loose map,
# not the source path compiled on the build machine.
export AMBITION_LDTK="$APPDIR/assets/ambition/worlds/sandbox.ldtk"

export RUST_BACKTRACE=1
export RUST_LOG="${RUST_LOG:-warn}"

exec "$APPDIR/ambition_sandbox" "$@"
EOF_INNER

# Remote sanity checks for both real files and compatibility paths.
ssh "$DECK" "bash -s" <<EOF_CHECK
set -euo pipefail
APPDIR='$APPDIR'
test -x "\$APPDIR/ambition_sandbox"
test -f "\$APPDIR/assets/sprites/robot_spritesheet.png"
test -f "\$APPDIR/sprites/robot_spritesheet.png"
test -f "\$APPDIR/assets/sprites/entities/chest_closed.png"
test -f "\$APPDIR/sprites/entities/chest_closed.png"
test -f "\$APPDIR/assets/audio/music/generated/long_lofi_drift/full.ogg"
test -f "\$APPDIR/audio/music/generated/long_lofi_drift/full.ogg"
test -f "\$APPDIR/assets/assets/audio/music/generated/long_lofi_drift/full.ogg"
EOF_CHECK

echo "Deployed to $DECK:$APPDIR"
echo "Steam shortcut target: $APPDIR/run_ambition.sh"
echo "Launcher sets BEVY_ASSET_ROOT=$APPDIR"
echo "Compatibility symlinks created: sprites -> assets/sprites, audio -> assets/audio, ambition -> assets/ambition"
