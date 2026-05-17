# FunDSP audio rendering — RETIRED

> **End-of-life note.** Ambition no longer uses FunDSP for runtime audio.
> The procedural music generator and procedural SFX synthesizer have been
> deleted from the runtime; the `fundsp` crate is no longer a dependency
> of `ambition_sandbox`. This doc remains as historical context so future
> readers can understand what the now-deleted `audio/render.rs` code did
> and why it was retired.

## What changed

The runtime audio pipeline is now purely **authored**:

```text
authored OGGs (pre-rendered by tools/ambition_music_renderer)
  -> Bevy AssetServer  ->  bevy_kira_audio  ->  Kira channels

packed .sfxbank (tools/ambition_sfx_pack)
  -> Bevy AssetServer (audio/sfx.bank) -> SfxBankAsset loader
  -> ambition_sfx::BankProvider  ->  Kira static sounds
```

Music tracks must declare an `asset_path` (a pre-rendered OGG); tracks
left at `asset_path: None` are skipped at startup with a loud warning.
SFX cues come from the packed bank; cues missing from the bank fall back
to a short silent stub so playback never panics.

The deleted code lived at:

- `crates/ambition_sandbox/src/audio/render.rs::render_lofi_theme`
- `crates/ambition_sandbox/src/audio/render.rs::render_sfx*`
- `crates/ambition_sandbox/src/audio/runtime.rs::TrackSource::Procedural`
- `crates/ambition_sandbox/src/bin/tune_preview.rs`
- `crates/ambition_sandbox/src/bin/music_track_report.rs`
- the `fundsp` Cargo dependency on `ambition_sandbox`.

## Why retired

- The startup procedural render added ~2.4s to visible boot.
- Generated tracks were rarely listened to once authored OGGs existed.
- `fundsp` is a non-trivial dep tree that contributed to long sandbox
  rebuilds.
- The wasm port can't synthesize on a worker thread without rework;
  authored OGGs ship to every platform unchanged.

## Future: realtime DSP/effects

Effects like underwater muffle, low-pass filter on damage, reverb-ish
ambience, or filter sweeps tied to gameplay state are still on the
roadmap. When that work lands, prefer adding a small `audio_fx` feature
behind which `fundsp` (or a lighter DSP crate) can be re-introduced as
a *processing layer over Kira playback*, not as a content generator.
Keep this distinction crisp: future audio_fx is meant for live
gameplay-driven effects, not for synthesizing music at startup.

## Where to look for runtime audio today

- `crates/ambition_sandbox/src/audio.rs` — module root + re-exports
- `crates/ambition_sandbox/src/audio/runtime.rs` — `AudioLibrary`,
  music track table, `audio_play_sfx_messages` system, SFX cue handles
- `crates/ambition_sandbox/src/audio/render.rs` — bank-clip → Kira
  `AudioSource` adapter + `SfxBankHandleCache` for ad-hoc
  `SfxMessage::Play { id }` lookups
- `crates/ambition_sandbox/src/audio/bank_asset.rs` — Bevy `Asset` +
  `AssetLoader` for the packed `.sfxbank`; promotes to
  `SfxBankResource` once decoded
- `crates/ambition_sandbox/src/audio/web_unlock.rs` — telemetry around
  the browser AudioContext gesture unlock
- `crates/ambition_sandbox/src/music.rs` — the music director, layer
  channels, adaptive cue catalog
- `crates/ambition_sandbox/src/setup.rs::try_load_sfx_bank_via_catalog`
  — sync bank load fast path (static embed + dev env override)
