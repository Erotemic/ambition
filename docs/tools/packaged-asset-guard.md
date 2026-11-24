---
status: current
last_verified: 2026-11-23
related_docs:
  - docs/recipes/adding-an-asset.md
  - docs/recipes/android-build.md
  - docs/systems/asset-manager.md
---

# Packaged asset guard

`scripts/package/package_asset_guard.py` is the package boundary for installed asset
trees. It reads the game's own files and each library declared by its content
pack. The content compiler exports each selected file's logical name, publication
address, and native source. The guard copies that file below `__libraries/<id>/`
at its publication address. It does not build a second library inventory.
The embedded runtime index keeps both names and uses the same installed path.
Qualified `content://<id>/...` references keep their library identity on every
platform. An unqualified reference must have one owner.

## Enforced invariants

A package build fails when:

- a sheet, parallax or LDtk manifest, or an asset constant in Rust, declares
  an asset absent from the selected sources;
- the content pack (`pack.ron`) does not compile;
- two roots provide different bytes at the same package-relative path;
- paths collide after case folding;
- an absolute path, root escape, or non-canonical package path is declared;
- a symlink or non-regular file reaches the composed package tree;
- a copied file differs from its source byte-for-byte;
- Gradle omits or changes an asset in the finished APK;
- Steam Deck deployment changes or omits a contracted file.

The content CLI uses the runtime sheet parser to export page-image dependencies.
A sheet drawn from parts needs no flat sheet image. Multi-page sheets and portrait
manifests require each declared page, so page zero cannot hide an absent page.

## Who says what a file declares

A file the content pack lists is read by a typed schema, and only the schema
knows how its asset paths resolve: a music cue joins each section `path` to the
cue's `asset_root`, and a music track derives its file from its id. The guard
does not pattern-match those files. A schema can also mark a metadata file with
`with_resolved_dependencies()` after it has emitted all dependencies. The guard
then uses those requirements and does not scan the metadata again. It runs the
content compiler and takes its answer:

```bash
cargo run -q -p ambition_content_check --bin ambition_content_check -- \
    game/ambition_content/assets --no-asset-check --package-inputs
```

`ambition_content_check` is the content CLI with Ambition's own schemas
beside the engine's: the generic binary would refuse `ambition_rules`.

The first run builds the binary; later runs cost milliseconds.
`AMBITION_CONTENT_BIN` names a built binary to use without cargo. A new schema
that calls `need_asset` reaches the package with no change here.

The pattern match over field names (`spritesheet`, `image`, `path`, ...) stays
for other manifests, such as parallax and LDtk files. A new manifest kind whose paths are not
package-relative belongs in a schema, not in a new pattern.

A pack asset that is absent from the selected sources is reported and the package ships without
it: a development checkout may lack rendered music that the registry lists, and
the desktop game runs without it too. `compose --strict-pack-assets` refuses
instead, which a release wants.

## Commands

The platform scripts invoke this automatically. For focused diagnosis:

```bash
python3 scripts/package/package_asset_guard.py compose \
    --repo . \
    --profile android \
    --output target/package-assets/manual/assets \
    --contract target/package-assets/manual/contract.json \
    --hash-manifest target/package-assets/manual/contract.sha256

python3 scripts/package/package_asset_guard.py audit-tree \
    --contract target/package-assets/manual/contract.json \
    --asset-root target/package-assets/manual/assets \
    --reject-extras
```

The JSON contract records path, source provenance, size, and SHA-256. The
standard `sha256sum` manifest is used for remote installed-tree verification.
Neither file is runtime game content.

## Profile differences

Android packages the complete composed tree. Steam Deck uses the same tree but
excludes `fonts/local/`, which is explicitly a local-machine fallback and not a
distributable font family. Any additional profile exclusion belongs in the
guard's named profile, not as a second ad hoc copy rule in a deploy script.
