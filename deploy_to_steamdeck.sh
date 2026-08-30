#!/usr/bin/env bash
set -euo pipefail

REPO="${REPO:-$HOME/code/ambition}"
DECK="${DECK:-deck@steamdeck}"
APPDIR="${APPDIR:-/home/deck/Games/ambition}"
PACKAGE_ROOT="$REPO/target/package-assets/steamdeck"
PACKAGE_ASSETS="$PACKAGE_ROOT/assets"
ASSET_CONTRACT="$PACKAGE_ROOT/asset-contract.steamdeck.json"
ASSET_HASH_MANIFEST="$PACKAGE_ROOT/asset-contract.steamdeck.sha256"

# ⛔⛔ ONE NAME FOR THE EXECUTABLE, BECAUSE THIS SCRIPT ONCE BUILT ONE BINARY AND
# SHIPPED ANOTHER. It built `ambition_game_bin` and then rsync'd, launched and
# verified `target/release/ambition_platformer2d_actor_monolith` — a name that
# stopped being an executable when that crate became a library. On a clean tree
# the rsync fails; on a machine with an old `target/`, it silently deploys a
# stale executable that no longer corresponds to anything in this tree. Every
# mention of the binary now reads this variable, and the build is checked below.
BIN="${BIN:-ambition_game_bin}"

cd "$REPO"

# Optional but useful: fail before deploying a bad map.
PYTHONPATH="$REPO/tools/ambition_ldtk_tools" \
    python -m ambition_ldtk_tools validate \
    game/ambition_content/assets/worlds/sandbox.ldtk

# Compose the exact installed asset tree before building. This is the same
# two-root collapse Android uses, with local-machine fallback fonts excluded.
# The tool fails on missing declarations, case collisions, symlinks, conflicting
# root overlays, or any byte mismatch in the composed tree.
python3 "$REPO/scripts/package_asset_guard.py" compose \
    --repo "$REPO" \
    --profile steamdeck \
    --output "$PACKAGE_ASSETS" \
    --contract "$ASSET_CONTRACT" \
    --hash-manifest "$ASSET_HASH_MANIFEST"

# Safest build: keep default desktop features, add the static_map + static_content
# fallbacks so the deployed binary carries its world JSON and generated content
# rather than needing the source tree beside it.
#
# ⛔ `CARGO_INCREMENTAL=0` for the reason `run_game.sh` sets it on every optimized
# profile: an incremental optimized build has linked stale codegen under mold,
# and rustc's incremental codegen-unit splitting means the artifact is not the
# one the profiling numbers describe. An explicit setting from the caller wins.
if [[ -z "${CARGO_INCREMENTAL:-}" ]]; then
    export CARGO_INCREMENTAL=0
fi

cargo build -p ambition_app --bin "$BIN" --release --features static_map,static_content

# The cheapest possible proof that what is about to ship is what was just built.
# Without it a wrong name is only caught by rsync's error — or not caught at all,
# because a stale file of that name is still sitting in `target/release`.
if [[ ! -x "target/release/$BIN" ]]; then
    echo "deploy: cargo build did not produce target/release/$BIN" >&2
    exit 1
fi

ssh "$DECK" "mkdir -p '$APPDIR'"

rsync -av --delete \
    "target/release/$BIN" \
    "$DECK:$APPDIR/"

# Deploy the already-composed tree, not two independently-maintained rsync
# recipes. --delete makes the remote tree exactly match the audited package.
rsync -av --delete \
    "$PACKAGE_ASSETS/" \
    "$DECK:$APPDIR/assets/"
rsync -av \
    "$ASSET_CONTRACT" \
    "$ASSET_HASH_MANIFEST" \
    "$DECK:$APPDIR/"

# Compatibility symlinks:
# - BEVY_ASSET_ROOT should be the app/root dir. Bevy's default asset folder is
#   then $BEVY_ASSET_ROOT/assets, which matches the rsync destination above.
# - Some preflight checks tolerate $BEVY_ASSET_ROOT/<rel_path>, so expose
#   sprites/audio/ambition/fonts at the app root as compatibility symlinks too.
# - The assets/assets -> . link also tolerates launchers that accidentally set
#   BEVY_ASSET_ROOT=$APPDIR/assets.
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

# ⛔ UNQUOTED HEREDOC: `$BIN` must expand HERE, while every other `$` belongs to
# the launcher that runs on the Deck and is escaped.
ssh "$DECK" "cat > '$APPDIR/run_ambition.sh' && chmod +x '$APPDIR/run_ambition.sh'" <<EOF_INNER
#!/usr/bin/env bash
set -euo pipefail

APPDIR="\$HOME/Games/ambition"
cd "\$APPDIR"

# Important: this is the app/root directory, not the assets directory.
# Bevy's default asset folder is "\$BEVY_ASSET_ROOT/assets".
export BEVY_ASSET_ROOT="\$APPDIR"

export RUST_BACKTRACE=1
export RUST_LOG="\${RUST_LOG:-warn}"

exec "\$APPDIR/$BIN" "\$@"
EOF_INNER

# Verify every remote file against the same byte contract used locally. A
# successful rsync is not enough evidence if a launcher/deploy path moved or a
# filesystem normalized names unexpectedly.
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

echo "Deployed to $DECK:$APPDIR"
echo "Steam shortcut target: $APPDIR/run_ambition.sh"
echo "Launcher sets BEVY_ASSET_ROOT=$APPDIR"
echo "Compatibility symlinks created: sprites/audio/ambition/fonts -> assets/..."
echo "Excluded from deployment: assets/fonts/local/"
