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
