# External art library migration

**State:** DONE. Jon selected this work on 2026-10-28.

## Receipt

The engine selected named shared art through an implicit asset tree and
process-wide metadata. Generation, reload, and packaging used different roots.
Loose media directories also defined package membership.

Games now select zero or more content libraries. The shared library is
`game/ambition_content_library`. The engine owns formats and execution;
libraries own named art, geometry, animation, audio, and supported content.
Native content edits need no engine build. Each session keeps its admitted
mechanical sprite generation. Files and metadata advance together.

All declared media producers use the existing immutable publication store.
Developer setup runs fonts, backgrounds, sprites, VFX, atlases, quality variants,
music, and SFX, then checks presence, freshness, and coverage. Failed candidates
keep the accepted selection. Generated assets remain ignored.

Core receipts: `e1ad49163`, `24095151e`, `4fcab904d`, `f713a2d7c`, and
`8e1719cab`. The final SFX owner is `b0f6669`: one source per cue ID, supported
filter effects, and linear-time howl smoothing. All 701 cues render; the bank
has 701 entries. All 125 selected music cues have accepted renders.

## Verification

`scripts/setup/generated_content.sh --skip-navigation` passes. Every producer
reports zero missing or stale publications. Quality checks cover 471 products
with zero defects, and regeneration coverage has zero findings.
Native app checks (all targets) and static app checks pass. Final shared-art
resolution tests (7) and native reload tests (2) pass. A fresh 1280x720 capture
shows the robot, hub, backgrounds, fonts, and UI.

The independent consumer passes 29 tests (one ignored). Its standalone debug
artifact loads its own sprite with zero asset failures. All three asset reads
stay inside the artifact when run outside the checkout. It selects no shared
library. A release artifact build was not run.

Workspace policies (35), absence contracts (53), producer behavior tests (4),
music dependency/preflight tests (21), and SFX tests (34) pass. Fresh-art app
acceptance (22), native reload tests (2), audio core tests (63), and audio
adapter tests (50) also pass.

The actor renderer builds its default radial glow mask in its image store.
It does not load a glow file from a content library. A host can supply another
image before startup. Actor-glow tests cover image readiness without an asset
server, host selection, and body visibility and retirement.

## Standing rules

- Keep the shared library under `game/`. It is not a submodule.
- Do not commit generated assets or restore implicit engine art.
- Use the existing content compiler and publication store.
- Keep source, runtime, review, and quality roots distinct.
- Keep a session's mechanical metadata and sheet images in one generation.
- Define named content and defaults in data outside engine crates.

The source-only sprite plan lists 466 units without generated files. Exact
packed page membership needs layout resolution; use `--resolve-layout` for that
set. See [sprite publication](../../tools/sprite-publication.md) and the
[shared library guide](../../../game/ambition_content_library/README.md).

Separate baseline defects remain: the page-size ratchet reports 95 full-tier
pages above 2048 pixels against a limit of 83; `bevy_ecs_ldtk` rejects nullable
Booleans in the committed map. The migration does not change the ratchet or
coerce null values. The loader mismatch is recorded in
`dev/benchmark-candidates/nullable-ldtk-bools-rejected-by-the-presentation-loader-2026-10-29.md`.
