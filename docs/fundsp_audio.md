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

## Realtime DSP/effects (status: ECS plumbed, real filter blocked)

The ECS [`AudioEnvironment`](../crates/ambition_sandbox/src/audio/environment.rs)
layer is in place: it reads the player's `WaterContact` (the same
field the swim mechanics already write), smooths a `wetness` value
on a wall-clock timer, and a single writer composes mixer-level
settings with the environment state.

**But the writer is a volume duck, not a filter.** Music drops ~8 dB
and SFX ~5 dB while underwater; the spectrum is unchanged. There is
**no high-frequency damping** today. See `docs/audio_underwater.md`
for the migration plan to a real Kira `FilterBuilder` (LowPass,
~20 kHz → ~800 Hz). Do not describe the current effect as
"underwater muffle" in any user-facing surface.

**Backend blocker:** `bevy_kira_audio` 0.25 hides the
`kira::AudioManager` (`pub(crate)` `AudioOutput.manager`), does not
expose `MainTrackBuilder::with_effect`, and offers no `add_sub_track`
or per-track effect-handle API. The recommended unlock is to replace
the wrapper with a thin direct-Kira layer (see
`docs/audio_underwater.md` Option A), not to forklift `fundsp` back
into the runtime as a parallel DSP engine.

Future effects (combat low-pass on damage, reverb-ish ambience, filter
sweeps tied to gameplay state) should follow the same pattern:

- An ECS state (`AudioEnvironment` already covers "what acoustic mode
  is the world in"; add new variants rather than parallel resources).
- A pure smoother (the `advance` method) that is deterministic and
  unit-testable on its own.
- A single channel writer that composes with the user mixer settings,
  caching the last-applied tuple to avoid per-frame writes.

`fundsp` (or any other DSP crate) is **not** the right vehicle for
realtime effects on the Kira pipeline — Kira already owns the audio
graph and ships its own effects. The right unlock for real low-pass
filtering is plumbing access to Kira's `MainTrackBuilder::with_effect`
or `SubTrackBuilder` through `bevy_kira_audio` (upstream PR or local
fork), not bringing back a parallel DSP engine.

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
