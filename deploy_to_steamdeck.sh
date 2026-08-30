#!/usr/bin/env bash
set -euo pipefail

REPO="${REPO:-$HOME/code/ambition}"
DECK="${DECK:-deck@steamdeck}"

# Name of this deployed build on the Steam Deck.
#
# Default:
#     ~/Games/ambition-2026-08-30
#
# Disable the dated/snapshot name and deploy over the normal installation:
#     DEPLOY_NAME= ./deploy_steamdeck.sh
#
# Or choose any other side-by-side name:
#     DEPLOY_NAME=ambition-test ./deploy_steamdeck.sh
#
NAME_SUFFIX="${NAME_SUFFIX-2026-08-30}"
DEPLOY_NAME="${DEPLOY_NAME-ambition-$NAME_SUFFIX}"

if [[ -n "$DEPLOY_NAME" ]]; then
    APPDIR="${APPDIR:-/home/deck/Games/$DEPLOY_NAME}"
else
    APPDIR="${APPDIR:-/home/deck/Games/ambition}"
fi

PACKAGE_ROOT="$REPO/target/package-assets/steamdeck"
PACKAGE_ASSETS="$PACKAGE_ROOT/assets"
ASSET_CONTRACT="$PACKAGE_ROOT/asset-contract.steamdeck.json"
ASSET_HASH_MANIFEST="$PACKAGE_ROOT/asset-contract.steamdeck.sha256"


# Register this deployment as a Non-Steam game.
# Disable with:
#     REGISTER_STEAM=0 ./deploy_steamdeck.sh
REGISTER_STEAM="${REGISTER_STEAM:-1}"

STEAM_NAME="${STEAM_NAME:-Ambition $NAME_SUFFIX}"

# One name for the executable. Keep build, deploy, launch, and verification
# pointed at the same artifact.
BIN="${BIN:-ambition_game_bin}"

cd "$REPO"

# Fail before deployment if the map is invalid.
PYTHONPATH="$REPO/tools/ambition_ldtk_tools" \
    python -m ambition_ldtk_tools validate \
    game/ambition_content/assets/worlds/sandbox.ldtk

# Compose the exact installed asset tree before building.
python3 "$REPO/scripts/package_asset_guard.py" compose \
    --repo "$REPO" \
    --profile steamdeck \
    --output "$PACKAGE_ASSETS" \
    --contract "$ASSET_CONTRACT" \
    --hash-manifest "$ASSET_HASH_MANIFEST"

# Optimized incremental builds have previously linked stale codegen under mold.
# Respect an explicit setting from the caller, otherwise disable incremental.
if [[ -z "${CARGO_INCREMENTAL:-}" ]]; then
    export CARGO_INCREMENTAL=0
fi

cargo build \
    -p ambition_app \
    --bin "$BIN" \
    --release \
    --features static_map,static_content

# Verify that the build produced the executable we intend to deploy.
if [[ ! -x "target/release/$BIN" ]]; then
    echo "deploy: cargo build did not produce target/release/$BIN" >&2
    exit 1
fi

ssh "$DECK" "mkdir -p '$APPDIR'"

rsync -av --delete \
    "target/release/$BIN" \
    "$DECK:$APPDIR/"

# Deploy the already-composed asset tree.
rsync -av --delete \
    "$PACKAGE_ASSETS/" \
    "$DECK:$APPDIR/assets/"

rsync -av \
    "$ASSET_CONTRACT" \
    "$ASSET_HASH_MANIFEST" \
    "$DECK:$APPDIR/"

# Compatibility symlinks:
# - BEVY_ASSET_ROOT is the app directory.
# - sprites/audio/ambition/fonts are exposed at the app root.
# - assets/assets -> . tolerates launchers that point BEVY_ASSET_ROOT at assets.
ssh "$DECK" "bash -s" <<EOF_REMOTE
set -euo pipefail
APPDIR='$APPDIR'

cd "\$APPDIR"
rm -rf sprites audio ambition fonts
ln -sfn assets/sprites sprites
ln -sfn assets/audio audio
ln -sfn assets/ambition ambition
ln -sfn assets/fonts fonts

cd "\$APPDIR/assets"
ln -sfn . assets
EOF_REMOTE

# The launcher locates its own installation directory. This is important for
# side-by-side deployments: the same launcher works in ambition,
# ambition-2026-08-30, or any future snapshot directory.
ssh "$DECK" "cat > '$APPDIR/run_ambition.sh' && chmod +x '$APPDIR/run_ambition.sh'" <<EOF_INNER
#!/usr/bin/env bash
set -euo pipefail

APPDIR="\$(cd -- "\$(dirname -- "\${BASH_SOURCE[0]}")" && pwd)"
cd "\$APPDIR"

export BEVY_ASSET_ROOT="\$APPDIR"

export RUST_BACKTRACE=1
export RUST_LOG="\${RUST_LOG:-warn}"

exec "\$APPDIR/$BIN" "\$@"
EOF_INNER

# Verify the remote installation against the same byte contract used locally.
ssh "$DECK" "bash -s" <<EOF_CHECK
set -euo pipefail
APPDIR='$APPDIR'

test -x "\$APPDIR/$BIN"

cd "\$APPDIR/assets"
sha256sum -c "\$APPDIR/asset-contract.steamdeck.sha256"

test ! -e "\$APPDIR/assets/fonts/local"
test -f "\$APPDIR/sprites/robot_spritesheet.png"
test -f "\$APPDIR/assets/assets/audio/music/generated/long_lofi_drift/full.ogg"
EOF_CHECK

echo
echo "Deployed to $DECK:$APPDIR"
echo "Steam shortcut target: $APPDIR/run_ambition.sh"
echo "Launcher sets BEVY_ASSET_ROOT to its own installation directory"
echo "Compatibility symlinks created: sprites/audio/ambition/fonts -> assets/..."
echo "Excluded from deployment: assets/fonts/local/"


#-----


if [[ "$REGISTER_STEAM" == "1" ]]; then
    STEAM_DESKTOP="$APPDIR/ambition.desktop"

    ssh "$DECK" "cat > '$STEAM_DESKTOP'" <<EOF_STEAM
[Desktop Entry]
Type=Application
Name=$STEAM_NAME
Exec=$APPDIR/run_ambition.sh
Path=$APPDIR
Terminal=false
Categories=Game;
EOF_STEAM

    ssh "$DECK" "chmod +x '$STEAM_DESKTOP'"

    # Avoid creating another Steam shortcut every time this same snapshot
    # is redeployed.
    if ssh "$DECK" \
        "grep -aFq '$STEAM_NAME' \
            \$HOME/.local/share/Steam/userdata/*/config/shortcuts.vdf \
            2>/dev/null"
    then
        echo "Steam shortcut already exists: $STEAM_NAME"
    else
        ssh "$DECK" "/usr/bin/steamos-add-to-steam '$STEAM_DESKTOP'"
        echo "Registered Steam shortcut: $STEAM_NAME"
    fi
fi
