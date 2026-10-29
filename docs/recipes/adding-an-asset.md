---
status: current
last_verified: 2026-10-28
related_docs:
  - docs/concepts/content-and-provider-boundaries.md
  - docs/tools/sprite-publication.md
---

# Add a new asset file

Put shared named content in a library selected by the game. Put game-specific
content in the game's own asset tree. Engine crates own formats and execution.

| Owner | Location in this repository | Runtime reference |
| --- | --- | --- |
| Shared library | `game/ambition_content_library/` | `content://ambition_shared/sprites/foo.png` |
| Ambition game | `game/ambition_content/assets/` | `game://worlds/foo.ldtk` |

A content pack selects libraries in its `libraries` list. The import path starts
at the pack root. A new game can select no shared library. The selected library's
manifest declares its authored and generated runtime roots, semantic asset
catalog, publication store, and static byte selection.

Register asset sources before Bevy creates its asset readers. The library reader
resolves canonical references and short paths with one owner. It refuses an
unqualified collision between libraries or between a game and a library.
The game's named source serves its own tree.

## Authored sources and generated products

Track small authored inputs, library manifests, and generation recipes. Generate
runtime payloads under the library's ignored `generated/` root. This includes
metadata, images, every quality tier, audio, publication bundles, and receipts.
Do not commit generated assets. Confirm the ignore rule with:

```bash
git check-ignore -v <generated-path>
```

An ignored payload can be present on disk. Check it before concluding it is
missing. In a worktree, copy local generated products and initialize map assets:

```bash
python3 scripts/mirror_assets_for_worktree.py
```

Build missing products from tracked inputs:

```bash
scripts/setup/generated_content.sh
./scripts/regen/sprites.sh --plan --json
./scripts/regen/sprites.sh --target <target>
```

The plan reads definitions without runtime products. The accepted publication
owns exact generated membership. See [sprite publication](../tools/sprite-publication.md)
for missing, stale, and unmanaged file queries.

## Load and validate

Use semantic catalog roles where the capability provides them. Use a qualified
reference for a direct library load:

```rust
let image: Handle<Image> = asset_server.load("content://ambition_shared/sprites/foo.png");
let world: Handle<LdtkProject> = asset_server.load("game://worlds/foo.ldtk");
```

Image quality is presentation policy. Simulation uses the admitted geometry and
must not require the highest image tier. A missing presentation file must report
a load failure without changing simulation timing.

The package tool reads pack declarations and the selected library inventories.
It stores each library under `__libraries/<id>/`. Check the composition with:

```bash
python3 scripts/package_asset_guard.py compose --help
cargo check -p ambition_app
cargo test -p ambition_app --test app_it -- declared_art_resolves
cargo test -p ambition_app --test app_it -- registered_character_art
```
