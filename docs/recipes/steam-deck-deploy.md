# Deploy to Steam Deck

Run from the checkout to deploy:

```bash
python3 scripts/content_status.py
DECK=deck@steamdeck ./deploy_to_steamdeck.sh
```

The status query is advisory. Missing or stale generated content does not make
this query fail. Do not add `--require-complete` to the normal game start or
deployment path. Use the query to decide which content needs generation on
the machine that builds the package.

The deploy script uses its own checkout unless `REPO` selects another one.
It builds with `static_map,static_content`, copies the composed asset tree,
and checks each installed file against the package's SHA-256 manifest.
`DEPLOY_NAME` selects the installation name; its default is `ambition-latest`.

The launcher sets `BEVY_ASSET_ROOT` to its installation directory. Game files
are under `assets/`. Library files are under
`assets/__libraries/<library-id>/`. The embedded library index keeps the same
logical names; image bytes need not be embedded in the executable.

If sprites show as magenta squares, inspect the reader path and the installed
bytes before regenerating content. A valid package hash does not prove that
the runtime catalog admits the file. The installed-library integration test
runs outside the source checkout and checks both the catalog and reader:

```bash
cargo test -p ambition_asset_manager --features bevy --test installed_library
```

The test also checks that missing optional images remain optional and that a
missing quality tier can use the full-resolution image.

## Controller input in Desktop Mode

Launch Ambition from its Steam library shortcut. In the Deck test, Ambition
opened Steam's virtual Xbox 360 controller. The device sent no events to
`evtest` during play. Steam's `logs/controller.txt` showed Desktop Layout ID
`413080` while Ambition was open in a Wayland session. The stick and face buttons worked
when Steam selected the Ambition shortcut layout. See the
[SteamOS Desktop Mode input report](https://github.com/ValveSoftware/steam-for-linux/issues/13223).

The deploy launcher selects XWayland for a Steam launch in a KDE Wayland
session when an X11 display is available. To update the launcher on a Deck
that already has a build, run this from the checkout on the machine that can
reach the Deck.

```bash
./deploy_to_steamdeck.sh --launcher-only
```

Use the same `DECK`, `DEPLOY_NAME`, and `APPDIR` settings as the original
deployment. Close Ambition and start it again from Steam. To use native
Wayland, set `AMBITION_DECK_NATIVE_WAYLAND=1 %command%` in Steam's launch
options for the shortcut.

For a temporary test, use `steam steam://forceinputappid/<shortcut-app-id>`
while the game is open. Get the shortcut app ID from `SteamAppId` in the
game process environment. Run `steam steam://forceinputappid/0` when the test
is complete to clear the override. Set the graphical session environment
when you send the command over SSH. See the
[Steam Input layout override report](https://github.com/ValveSoftware/steam-for-linux/issues/13314).

## Display and quality

The launcher selects the handheld layout. The game uses a borderless window
in Gaming Mode and fits the initial window to the display in Desktop Mode.
The gate terminal and touch controls use smaller base sizes. The left and
right touch size settings still scale their respective controls.

The launcher caps the UI scale at 1× and uses one sample per pixel across
all quality presets. A preset still changes scene quality, including texture
tiers and the portal budget.

Steam launches write to `ambition.log` in the installation directory. If
the picture is blurry or a quality change closes the game, read the log
over SSH before you start the game again:

```bash
grep 'Window:' ~/Games/ambition-latest/ambition.log
tail -80 ~/Games/ambition-latest/ambition.log
```

The window lines give the physical framebuffer size, the logical size, and
the display scale. On the Deck's own screen in Gaming Mode, use the Steam
shortcut's native game resolution to give the game a 1280×800 surface.
