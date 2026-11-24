#!/usr/bin/env bash
set -euo pipefail

REPO="${REPO:-$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)}"
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
#NAME_SUFFIX="${NAME_SUFFIX-2026-08-30}"
NAME_SUFFIX="${NAME_SUFFIX-latest}"
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

install_launcher() {
    ssh "$DECK" "cat > '$APPDIR/run_ambition.sh' && chmod +x '$APPDIR/run_ambition.sh'" <<EOF_INNER
#!/usr/bin/env bash
set -euo pipefail

APPDIR="\$(cd -- "\$(dirname -- "\${BASH_SOURCE[0]}")" && pwd)"
cd "\$APPDIR"

export BEVY_ASSET_ROOT="\$APPDIR"
export AMBITION_PRESENTATION_ENV="\${AMBITION_PRESENTATION_ENV:-handheld}"
export AMBITION_MAX_SCALE_FACTOR="\${AMBITION_MAX_SCALE_FACTOR:-1}"
# Keep the window cameras at one sample count during quality changes.
export AMBITION_MSAA="\${AMBITION_MSAA:-1}"

export RUST_BACKTRACE=1
export RUST_LOG="\${RUST_LOG:-warn,ambition_app::host::platform::desktop=info}"

# Steam Input can select the Desktop Layout when Steam cannot see the
# focus of a native Wayland window. Use XWayland for the Steam shortcut in KDE.
if [[ \${XDG_SESSION_TYPE:-} == wayland &&
      :\${XDG_CURRENT_DESKTOP:-}: == *:KDE:* &&
      -n \${SteamGameId:-} &&
      -n \${DISPLAY:-} &&
      \${AMBITION_DECK_NATIVE_WAYLAND:-0} != 1 ]]; then
    unset WAYLAND_DISPLAY WAYLAND_SOCKET
fi

if [[ ! -t 2 ]]; then
    exec > "\$APPDIR/ambition.log" 2>&1
fi
exec "\$APPDIR/$BIN" "\$@"
EOF_INNER
}

if [[ ${1:-} == --launcher-only ]]; then
    ssh "$DECK" "test -x '$APPDIR/$BIN'"
    install_launcher
    echo "Updated $DECK:$APPDIR/run_ambition.sh"
    exit 0
fi

cd "$REPO"

# Fail before deployment if the map is invalid.
PYTHONPATH="$REPO/tools/ambition_ldtk_tools" \
    python -m ambition_ldtk_tools validate \
    game/ambition_content/assets/worlds/sandbox.ldtk

# Compose the exact installed asset tree before building.
python3 "$REPO/scripts/package/package_asset_guard.py" compose \
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

AMBITION_STAMP_BUILD=1 cargo build \
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

install_launcher

# Verify the remote installation against the same byte contract used locally.
ssh "$DECK" "bash -s" <<EOF_CHECK
set -euo pipefail
APPDIR='$APPDIR'

test -x "\$APPDIR/$BIN"

cd "\$APPDIR/assets"
sha256sum -c "\$APPDIR/asset-contract.steamdeck.sha256"
EOF_CHECK

echo
echo "Deployed to $DECK:$APPDIR"
echo "Steam shortcut target: $APPDIR/run_ambition.sh"
echo "Launcher sets BEVY_ASSET_ROOT to its own installation directory"
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

    # Start the shortcut through Steam so it can apply the gamepad layout.
    # Steam can take a moment to save the shortcut.
    STEAM_LAUNCHER="$APPDIR/ambition-steam.desktop"
    GAME_ID=""
    for _ in 1 2 3 4 5; do
        GAME_ID="$(ssh "$DECK" "python3 - '$STEAM_NAME'" < "$REPO/scripts/package/steam_shortcut_gameid.py" || true)"
        [[ -n "$GAME_ID" ]] && break
        sleep 2
    done
    if [[ -n "$GAME_ID" ]]; then
        ssh "$DECK" "cat > '$STEAM_LAUNCHER' && chmod +x '$STEAM_LAUNCHER' && mkdir -p ~/.local/share/applications && cp '$STEAM_LAUNCHER' ~/.local/share/applications/" <<EOF_LAUNCHER
[Desktop Entry]
Type=Application
Name=$STEAM_NAME (controller)
Exec=steam steam://rungameid/$GAME_ID
Terminal=false
Categories=Game;
EOF_LAUNCHER
        echo "Desktop-mode launcher with the controller: $STEAM_LAUNCHER (steam://rungameid/$GAME_ID)"
    else
        echo "Steam has not saved the shortcut yet: in desktop mode, start '$STEAM_NAME' from Steam's Library to get the controller."
    fi
fi
