---
status: current
last_verified: 2026-07-21
related_docs:
  - docs/recipes/adding-an-asset.md
  - docs/recipes/android-build.md
  - docs/systems/asset-manager.md
---

# Packaged asset guard

`scripts/package_asset_guard.py` is the package boundary for installed asset
trees. Desktop development exposes two roots:

- `crates/ambition_actors/assets`
- `game/ambition_content/assets`

Android and installed desktop builds expose one `assets/` directory. The guard
is the single implementation that collapses the two roots and proves that the
package reproduces the desktop-resolvable view.

## Enforced invariants

A package build fails when:

- a runtime catalog or manifest declares an asset absent from both roots;
- two roots provide different bytes at the same package-relative path;
- paths collide after case folding;
- an absolute path, root escape, or non-canonical package path is declared;
- a symlink or non-regular file reaches the composed package tree;
- a copied file differs from its source byte-for-byte;
- Gradle omits or changes an asset in the finished APK;
- Steam Deck deployment changes or omits a contracted file.

Spritesheet and portrait manifests are expanded through their page-image lists,
so a present page zero cannot hide a missing secondary page.

## Commands

The platform scripts invoke this automatically. For focused diagnosis:

```bash
python3 scripts/package_asset_guard.py compose \
    --repo . \
    --profile android \
    --output target/package-assets/manual/assets \
    --contract target/package-assets/manual/contract.json \
    --hash-manifest target/package-assets/manual/contract.sha256

python3 scripts/package_asset_guard.py audit-tree \
    --contract target/package-assets/manual/contract.json \
    --asset-root target/package-assets/manual/assets \
    --reject-extras

python3 -m unittest scripts.tests.test_package_asset_guard
```

The JSON contract records path, source provenance, size, and SHA-256. The
standard `sha256sum` manifest is used for remote installed-tree verification.
Neither file is runtime game content.

## Profile differences

Android packages the complete composed tree. Steam Deck uses the same tree but
excludes `fonts/local/`, which is explicitly a local-machine fallback and not a
distributable font family. Any additional profile exclusion belongs in the
guard's named profile, not as a second ad hoc copy rule in a deploy script.
