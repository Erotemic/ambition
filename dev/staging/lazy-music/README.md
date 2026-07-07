# Lazy music tracks (Slice A) — staged diffs

Staged here while another agent works on the main tree. Nothing in this
directory is referenced by `cargo`; it does not affect builds or tests.

> 2026-07-06 E1b note: this staged patch targets the pre-extraction
> gameplay-core audio runtime and is now obsolete as written. The reusable
> playback library lives in
> [crates/ambition_audio/src/library.rs](../../../crates/ambition_audio/src/library.rs),
> while the SFX bank loader/drain lives in
> [crates/ambition_audio/src/bank_asset.rs](../../../crates/ambition_audio/src/bank_asset.rs).
> Re-measure the lazy-music slice against those files before applying any of
> the snippets below.

## What this slice does

Defers the eager `asset_server.load(...)` calls in `AudioLibrary::new`
([crates/ambition_audio/src/library.rs](../../../crates/ambition_audio/src/library.rs))
so file-backed music tracks load on first request instead of at startup.
File-backed tracks total ~58 MB across 25 OGGs today; only the default
track plus whatever the player switches to actually need to be live.

Procedural tracks (no `asset_path`) keep being synthesized at startup —
they're cheap, deterministic, and required by tests that don't have an
`AssetServer`.

## Decisions taken (defaults applied; revisit before applying)

1. **Slice A only.** Adaptive cue layers (`load_music_cues` in
   `music/director.rs`) are NOT touched here — that's Slice B in the
   scope. Open as a follow-up PR.
2. **Radio preload hook included.** When the player highlights a track in
   the radio menu, we kick a preload of that track's handle so play
   doesn't hitch on confirm. Costs one extra method call in
   `handle_radio_input`.
3. **Procedural fallback semantics unchanged.** A `MusicTrackSpec` with
   `asset_path: None` still synthesizes via `render_lofi_theme` at
   startup. Tests rely on this.

## Files in this directory

- `README.md` — this file.
- `changes.md` — file-by-file change manifest with before/after snippets
  for the small surgical edits.
- `runtime_new.rs` — obsolete full proposed replacement for the old
  gameplay-core audio runtime. Heavy edits — easier to review as a complete
  file than as a hand-written diff, but now requires a fresh port to
  `ambition_audio`.

## How to apply when the other agent is done

Mechanical, in order:

1. Re-port the staged idea to `crates/ambition_audio/src/library.rs`.
2. Apply the surgical edits in `changes.md` to:
   - `crates/ambition_actors/src/audio/tests.rs`
   - `crates/ambition_actors/src/music/director.rs`
   - `crates/ambition_actors/src/pause_menu.rs`
   (`audio.rs`'s re-exports list and `setup.rs` need NO changes.)
3. `cd crates/ambition_actors && cargo check --features audio,input`.
4. Run the audio tests: `cargo test -p ambition_actors --features audio
   --lib audio::`.
5. Smoke-test in the sandbox: confirm music starts on boot and that
   switching radio tracks still works.

## Pattern note for follow-up sprite slice

The shape established here transfers directly:

- `MusicTrackRuntime { ..., source: TrackSource }` where `TrackSource`
  carries either a synthesized `Handle` or `(asset_path, Option<Handle>)`.
- `AudioLibrary::resolve_track_handle(&mut self, id, asset_server)` —
  load-on-miss, cache.
- `AudioLibrary::preload_track(&mut self, id, asset_server)` — same call
  ignoring the result.

For sprites: `EntitySpriteSet` will grow the same shape (HashMap value
becomes an enum or a `(path, Option<Handle>)`), with a parallel
`resolve` / `preload` pair. We'll build it as the next slice and
re-evaluate whether to factor a generic `LazyAsset<A>` helper after
seeing the sprite duplication.

## Risks to confirm at apply-time

- **Borrow contention.** Three systems become `ResMut<AudioLibrary>`
  (`start_default_music`, `apply_encounter_music`, `drive_music_director`)
  plus the existing `pause_menu_navigate`. They serialize where they could
  parallelize — non-issue for music-rate systems.
- **First-play hitch on radio scroll.** Mitigated by the preload hook.
  Confirm by scrolling quickly through the radio menu.
- **Tests that build `AudioLibrary` without an `AssetServer`.** Both call
  sites in `audio/tests.rs` pass `None` for asset_server; they only use
  procedural specs (asset_path: None). Verify still true after data
  changes — a test track with `asset_path: Some` would now silently
  produce `None` handles instead of synthesizing.
