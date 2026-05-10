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

# The key detail: Bevy asset paths like "sprites/robot_spritesheet.png"
# should resolve inside $APPDIR/assets, not $APPDIR.
ssh "$DECK" "cat > '$APPDIR/run_ambition.sh' && chmod +x '$APPDIR/run_ambition.sh'" <<'EOF_INNER'
#!/usr/bin/env bash
set -euo pipefail

APPDIR="$HOME/Games/ambition"

# Bevy should resolve asset paths like:
#   sprites/robot_spritesheet.png
#   audio/music/...
# Therefore the asset root is the deployed assets directory itself.
export BEVY_ASSET_ROOT="$APPDIR/assets"

# Ambition's direct LDtk loader should use the Deck-side loose map,
# not the source path compiled on the build machine.
export AMBITION_LDTK="$APPDIR/assets/ambition/worlds/sandbox.ldtk"

export RUST_BACKTRACE=1
export RUST_LOG="${RUST_LOG:-warn}"

cd "$APPDIR"
exec "$APPDIR/ambition_sandbox" "$@"
EOF_INNER

# Cheap remote sanity check for the path that character_sprites/assets.rs probes.
ssh "$DECK" "test -f '$APPDIR/assets/sprites/robot_spritesheet.png' && test -f '$APPDIR/assets/sprites/entities/chest_closed.png'"

echo "Deployed to $DECK:$APPDIR"
echo "Steam shortcut target: $APPDIR/run_ambition.sh"
echo "Launcher sets BEVY_ASSET_ROOT=$APPDIR/assets"
