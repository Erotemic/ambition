# Sprite bundle publication

**State:** in progress. Parent: [authoring and tools](authoring-and-tools.md).

## Contract

- The target registry defines available products. A collection selects targets.
  Discovery does not require generated images. Invalid sources remain visible.
- A build reads a source snapshot and writes into a clean private directory.
  Each bundle records its recipe, exact files, hashes, and parent bundles.
- Sheet, portrait, and part metadata name required pages. The publisher rejects
  missing pages and invalid sheet metadata. Custom installation hooks do not
  write runtime files.
- Accepted bundles stay in `art_publication/bundles/<digest>`. Runtime files,
  quality tiers, and review files have separate directories.
- The `published.json` file in the art publication store selects bundles and maps logical
  paths to immutable files. One file replacement publishes a request. A changed
  base or failed build keeps the previous selection.
- Runtime loading, static embedding, packaging, and the inspector read this
  selection. Old loose images are unmanaged files. They are not runtime input.
- Native quality rendering and frame conversion retain their different source
  capabilities. Both publish through the bundle contract.

## Queries

`scripts/regen/sprites.sh --plan --json` lists definitions, selected products,
known paths, and unresolved page membership. `--scope all` includes targets
outside the runtime collection. `--resolve-layout` builds exact membership
without changing the published selection. Pixel trimming can require rendering
before the number of pages is known.

`--check` compares recipes and checks all accepted members. `--deep` bypasses
digest caches. `--audit` lists unmanaged loose files and historical bundles.
An imported bundle has unknown source evidence until a real build replaces it.

## Acceptance

- Exercise source changes, missing pages, changed outputs, failed builds,
  changed publication bases, and derived bundle dependencies.
- Build representative real targets, including static props, source quality,
  multipart sheets, and the director card.
- Generate the runtime collection from source and verify its publication.
- Verify the sprite owners, the assembled app, reload, packaging, and inspector.
- Remove the old staging, shell roster, custom installers, and cache checks.
