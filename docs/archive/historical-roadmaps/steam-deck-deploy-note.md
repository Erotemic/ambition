# Archived: steam-deck-deploy.md

Historical deploy note. Current platform policy lives in `docs/concepts/platform-targets.md` and build recipes.

Original path: `docs/recipes/steam-deck-deploy.md`

---

# Steam Deck deploy note

This overlay only replaces `deploy_to_steamdeck.sh`.

The launcher sets `BEVY_ASSET_ROOT` to the app/root directory:

```bash
export BEVY_ASSET_ROOT="$APPDIR"
```

Assets are still deployed to `$APPDIR/assets/`. The script also creates compatibility symlinks at the app root:

```text
sprites -> assets/sprites
audio -> assets/audio
ambition -> assets/ambition
assets/assets -> .
```

This makes the runtime work with both Bevy's default `$BEVY_ASSET_ROOT/assets/...` lookup and the current sprite preflight code that checks `$BEVY_ASSET_ROOT/sprites/...`.


# Steam Deck asset-root robustness patch

This overlay keeps `BEVY_ASSET_ROOT` set to the app root (`$APPDIR`) so Bevy
resolves assets from `$APPDIR/assets/<relative-path>`, which matches Bevy's
normal project layout.

The Rust preflight probes for sprites, boss sprites, entity sprites, and UI
fonts now tolerate both common launch layouts:

- `BEVY_ASSET_ROOT=/path/to/app` -> checks `/path/to/app/assets/<rel>`
- `BEVY_ASSET_ROOT=/path/to/app/assets` -> checks `/path/to/app/assets/<rel>`

They also keep direct-binary and `cargo run` fallbacks. Android behavior remains
unchanged: Android branches skip host filesystem probing and let Bevy's Android
asset reader resolve packaged assets.

The deploy script copies only distributable bundled fonts under
`assets/fonts/bundled/` and excludes `assets/fonts/local/`.

