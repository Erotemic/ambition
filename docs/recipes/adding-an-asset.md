---
status: current
last_verified: 2026-11-10
related_docs:
  - docs/concepts/content-and-provider-boundaries.md
  - docs/tools/sprite-publication.md
---

# Add a new asset file

Shared named assets live in a game-selected content library; game-specific
assets belong in that game's asset tree. Engine crates provide formats,
resolution and execution, without owning game content.

| Owner | Example source root | Runtime reference |
| --- | --- | --- |
| Shared library | `game/ambition_content_library/` | `content://ambition_shared/sprites/foo.png` |
| Ambition game | `game/ambition_content/assets/` | `game://worlds/foo.ldtk` |

The content pack selects its libraries. Library manifests define source and
runtime roots, asset catalogs, publication stores and packaging selections.
Qualified references identify the owner. Unqualified paths are allowed only
when they resolve without a collision.

## Authored source and published product

Commit authored definitions, small source assets and generation recipes.
Generated runtime products, receipts and quality tiers belong under ignored
`generated/` roots. Do not commit them. Check the exact ignore rule:

```bash
git check-ignore -v <generated-path>
```

Before regenerating a supposedly missing product, check whether it exists
locally but is ignored by Git. When explicitly working in an authorized
worktree, the existing assets can be copied using:

```bash
python3 scripts/mirror_assets_for_worktree.py
```

For a missing generated product:

```bash
./scripts/regen/sprites.sh --plan --json
./scripts/regen/sprites.sh --target <target>
```

The target publisher selects immutable bundles. Consult
[sprite publication](../tools/sprite-publication.md) for the distinction
between an unregistered asset, a stale publication and an absent local file.

## Runtime loading and packaging

Prefer semantic catalog references when available. For direct loads, use the
qualified scheme:

```rust
let image: Handle<Image> = asset_server.load("content://ambition_shared/sprites/foo.png");
let world: Handle<LdtkProject> = asset_server.load("game://worlds/foo.ldtk");
```

Register sources before the engine creates asset readers. Image quality is
presentation policy; physics and simulation must not depend on which texture
tier is available.

The package guard composes only the sources selected by the pack and library
manifests. It stores library assets under `__libraries/<id>/` so runtime
references remain stable. To inspect its interface:

```bash
python3 scripts/package/package_asset_guard.py compose --help
```

Check the newly added asset in its intended game or with a focused package or
content validation command. Broad integration suites are not the default for
an asset change; use them when the changed reference affects a shared loader
or packaging contract.
