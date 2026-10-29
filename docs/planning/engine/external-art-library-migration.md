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

## Current defects

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

The implementation checkpoint contains:

- An optional library manifest, explicit pack imports, role roots, a semantic
  catalog, source ownership, and game-owned static embedding.
- Shared authored data under `game/ambition_content_library/authored`. Generated
  products moved on disk under its ignored `generated/` root. No generated
  product is staged. Engine shaders remain with their rendering owner.
- Qualified file references and refusal of implicit overrides. Native readers,
  static builds, inspection, generation, and packaging use library selection.
- Immutable `SpriteContent` values owned by each App and retained by sessions.
  Geometry, rigs, landmarks, portraits, parts, effects, and quality lookups use
  that context. The old process-wide selection and duplicate registries are
  removed.
- One sprite mechanical identity section for session and peer identity. Image
  packing and quality products do not change that section.
- Joint file and metadata reload admission. Invalid candidates retain the
  accepted selection. Mechanical changes wait for the session boundary.
- Owned generation payloads. A repeated preparation probe retained about
  140 MB per call before the fix. After the fix, RSS stabilized near 171 MB
  across three calls. A weak-reference test verifies generation release.

Verification at this checkpoint:

- Workspace library test targets compile. The first broad run completed.
  Focused reruns cleared the migrated content, actor, combat, construction,
  view, and metadata fixture failures.
- Sprite owner tests, seven scope tests, three native reload tests, four font
  tests, and provider identity tests passed before the latest fixture cleanup.
- Sixty-two focused renderer, publication, provenance, inspector, quality,
  symlink, and package tests passed.
- Remaining failures: the app's two-room HUD activation test, two parallax
  fixture catalogs, one qualified rig assertion, and the existing New Game
  checkpoint assertion. The parallax and rig fixtures have fixes pending their
  rerun. `origin/main` has a checkpoint restoration fix to integrate.
- The broad tooling sweep also found unrelated music registry drift and stale
  repository policy baselines. The sweep was stopped after collecting failures.

## Remaining work

1. Merge `origin/main` with the implementation checkpoint. Preserve user edits
   in the map submodule and `scripts/audit_test_corpus.py`.
2. Complete assembled native and static checks, source-export and edit-to-play
   integration tests, current-game art acceptance, and workspace policy checks.
   Diagnose the HUD activation failure. Run focused reruns for fixed failures.
3. Verify generation under the new descriptor, including quality freshness.
   Do not check in generated assets. Publish the verified migration.
4. Measure repeated preparation cost as a separate optimization pass. The
   lifetime leak was a correctness blocker; its remaining CPU cost is not.

## Resume

Inspect Git status, this page, and `docs/tools/sprite-publication.md`. The first
broad library test log is `/tmp/ambition-library-workspace-lib-tests.log`.
Focused rerun logs use `/tmp/ambition-library-rerun-<crate>.log`.
The source checkpoint is not a completed acceptance receipt. Replace this page's
progress details with a short receipt when the remaining work is complete.
