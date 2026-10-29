# External art library migration

**State:** ACTIVE. Jon selected this work on 2026-10-28.

## Contract

A content library can supply images, geometry, animation, SFX, VFX, music,
and supported script artifacts. A game selects zero or more libraries.
All current games select the shared library under `game/ambition_content_library`.
This library is part of this repository. It is not a submodule.
A new game can use the engine
with that library absent. A content edit must not require an engine rebuild.

Use the existing content compiler, domain schemas, prepared generations,
extension host, and publication bundles. Do not add a second content engine.
Generated assets are not checked in. Source files, runtime products, review
products, and quality tiers have named
roots. The engine owns formats and execution. Libraries own named content.

## Problems addressed

The asset source installer adds an implicit monolith fallback. Engine build
scripts select named art. Sprite metadata, rigs, landmarks, portraits, and
packs share one process-wide publication. Some gameplay lookups bypass
App-local sheet declarations. Manifest lookup discards the source identity.
Source-qualified images lose quality variants. Desktop overrides and package
collision rules disagree. Generation and inspection assume the monolith path.
The publisher also depends on superproject quality code and game-local
validator arguments. The sheet mechanics digest is not in session identity
(the existing Q157 ruling requires it).

## Implementation packets

1. Define library identity, explicit game selection, source roots, and product
   locations. Extract shared assets from the actor crate. Preserve current
   generated bundles and authored data. Track only authored sources and recipes.
2. Route source installation, generation, inspection, embedding, and packaging
   through that selection. Remove monolith-relative roots and named core art
   from engine services. Use one collision and override rule.
3. Make selected sprite metadata an App-local immutable generation. Resolve
   geometry, rigs, landmarks, portraits, packs, effects, and quality products
   from that generation. Remove process-global content selection. Pin semantic
   content through existing session preparation and activation.
4. Include the admitted mechanical projection in content identity. Keep image
   packing and quality policy out of simulation identity. Use the same selected
   library facts for native, static, web, Android, and package products.
5. Verify current games with the shared library, an independent composition
   with it absent, and two Apps with different selections. Verify source edits
   without a Cargo build, failed candidate retention, quality changes, and
   package byte identity. Run the assembled app and workspace policy gates.

## Progress

Implementation checkpoint: `e1ad49163`. Main integration: `a03796df6`.
The shared library is optional, selected by game packs, and stored under
`game/ambition_content_library`. Generated products are ignored.

The engine no longer selects named art or embeds the shared library. Source
readers, generation, inspection, static embedding, and packaging use declared
libraries. Qualified references retain their library identity. Ambiguous short
references are refused.

Each App owns an immutable `SpriteContent` generation. Sessions retain their
admitted geometry. Files and metadata advance together. Failed candidates keep
the accepted selection. Q157 is complete: one mechanical projection includes
sheet geometry, rigs, landmarks, and portrait vocabulary. Image packing and
quality products do not change simulation identity.

The follow-up fixes immutable image lookup and qualified FX loading. FX images
and frame rectangles now come from matching tier metadata. Candidate construction
and encounter spawns use the session's frozen generation. The old FX folder
cache is removed.

Verification:

- All workspace test targets typecheck. All 35 workspace policy tests pass.
- The assembled native app passes 21 art-resolution and edit-to-play tests.
  The visible app's two-room HUD activation test passes.
- Provider tests (46), sprite scope tests (7), native reload tests (3), font
  tests (4), and five focused rendering acceptance tests pass.
- Sixty-two focused renderer, publication, provenance, inspector, quality,
  symlink, and package tests pass.
- A source-only checkout simulation lists 466 publication units in 2.69 s.
  It creates no generated directory. Exact packed page membership still needs
  layout resolution.
- The repeated preparation leak is fixed. RSS stabilized near 171 MB across
  three preparations instead of retaining about 140 MB per preparation.
  A weak-reference test verifies release after the last session drops.

## Remaining work

1. Push the acceptance fixes. The broad Smash host run passed 51 tests; its
   one old unqualified-name assertion now passes on focused rerun. The isolated
   session check and the assembled static check pass. All 53 absence contracts
   pass.
2. Commit and push the acceptance fixes. Refresh local sprite and quality
   products under the new recipes. All 470 preserved bundles report stale
   recipe paths; their payloads remain intact. Do not commit generated assets.
3. Complete a visual capture, package checks, and quality freshness checks.
   Record any unrelated baseline failures separately. Merge new `origin/main`
   changes, if present, and push the final receipt.
4. Keep performance work separate from the source migration. The quality entry
   point now uses the publisher's existing worker limit and accepts `--jobs`.
   The primary sprite publisher already used that limit.

## Resume

Inspect Git status, this page, and `docs/tools/sprite-publication.md`.
Preserve user edits in the map submodule and `scripts/audit_test_corpus.py`.
The latest gate logs are `/tmp/ambition-library-last-*.log` and
`/tmp/ambition-library-commit-*.log`. The latter batch stopped on an isolated
session fixture's undeclared test feature. That fixture now uses an empty
library selection, which is sufficient for its music-path claim.

The target volume filled during verification. Only this migration's compiled
test binaries and recent incremental sessions were removed. The cleanup record
is `.agent/library-build-cleanup.json`. Check target storage before another
large feature-cell build. Never delete another worktree's data.
