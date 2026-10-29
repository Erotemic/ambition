# Sprite publication

The renderer registers authored targets. A collection selects the targets to
publish. All targets use one bundle publisher, including static props, parts,
portraits, quality variants, shared atlases, and the director card.

The default library is `game/ambition_content_library/library.json`.
Use `--library <manifest>` or `AMBITION_CONTENT_LIBRARY` to select another library.

## Query without rendering

```bash
./scripts/regen/sprites.sh --list --scope all
./scripts/regen/sprites.sh --plan --json
./scripts/regen/sprites.sh --plan --target hand_saint --json
./scripts/regen/sprites.sh --check --json
./scripts/regen/sprites.sh --audit --json
python3 scripts/art_inspector.py status --needs-generation --json
```

The default scope is the selected library's collection in
`game/ambition_content_library/authored/sprites.yaml`. `--scope all` includes other
registered targets. Registration, publication selection, and known game use are
separate facts. A target outside the collection is not a missing runtime asset.

A plan lists known logical paths and unresolved membership. The number of
packed pages can depend on rendered pixels. `--list-outputs` lists declared
full-resolution paths; it is not an exact page inventory. To resolve exact
membership without publishing:

```bash
./scripts/regen/sprites.sh --resolve-layout --target hand_saint
```

This can render art. It leaves the selected publication unchanged.

## Build and publish

Art never needs cargo:

```bash
./scripts/regen/sprites.sh --target hand_saint
./scripts/regen/sprites.sh
python3 scripts/check_quality_variants_are_fresh.py --require-evidence
```

Each build reads a private source snapshot and writes into a clean directory.
The publisher checks required members, image data, sheet rows, portraits, part
tables, body rigs, and referenced pages. When a built game validator is already
there (`target/debug/ambition_content`, or `$AMBITION_ART_CONTENT_BIN`), it also
parses each sheet with the game's own parser. Without one, that check is not run
and the game's loader reports metadata it cannot read.

Publication does not refuse concurrent work:

- A source or toolchain change during a request does not discard the request.
  Each bundle records the inputs it was built from, so it reads as stale and the
  next build replaces it.
- The targets that build are published. A target that fails keeps its previous
  bundle, and the request reports the failure.
- When another agent published after this request planned, the request's
  changes are merged onto the newer selection, unit by unit (the last writer of
  a unit wins).
- A derived bundle (an atlas) whose parent changed stays selected and is listed
  under `stale` in `published.json`.

Accepted bundles can be reused by a later build if their recipes and files are current.

A focused request includes the target's named static props and quality products.
Shared atlas dependencies are rebuilt when their accepted parent bundles change.
Native source quality rendering remains available for targets that support it.
Other sheets use frame conversion and packing. Every request builds all tiers
that its product definition supports. Quality conversion uses the same
publication contract.

`AMBITION_ART_CONTENT_BIN` selects a prebuilt validator. `--jobs N` limits render
workers, to a maximum of six. `--force` rebuilds selected targets.
`AMBITION_ART_PROFILE_DIR` records worker line profiles. `LINE_PROFILE=1` selects
a profile directory when the shell launcher is used.

## Accepted files

The default store is
`game/ambition_content_library/generated/art_publication/`.
`AMBITION_ART_STORE` selects another store. The store is generated and ignored
by Git. Its structure is:

```text
bundles/<digest>/manifest.json
bundles/<digest>/runtime/full/...
bundles/<digest>/runtime/half/...
bundles/<digest>/runtime/quarter/...
bundles/<digest>/runtime/potato/...
bundles/<digest>/review/full/...
published.json
```

The bundle manifest lists every member, its hash, the recipe, and parent bundle
identities. `published.json` maps logical asset paths to accepted immutable files.
One atomic replacement selects a complete request. A changed publication base
refuses the request. Previously selected bundles remain readable.

Runtime readers, static embedding, packaging, and the inspector use this
selection. Logical asset names remain independent of bundle storage paths.
Consumers can still provide authored overrides through their own asset source.

`--audit` reports old loose files as unmanaged. It compares their logical names
and bytes with the selected publication when possible. It also lists historical
bundles. It deletes nothing. No known game use is not a reason to delete art.

For an existing checkout, `--adopt-existing PATH` can import old runtime files.
Imported bundles have unknown source evidence. Only a real build establishes a
current recipe. The import does not claim that current source produced old pixels.

Generation and Cargo builds can run at the same time.
