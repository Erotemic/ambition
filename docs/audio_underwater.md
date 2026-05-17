# Underwater audio — status + migration plan

## Status (2026-05-17, post `b9f8003` + this commit)

The sandbox has an ECS [`AudioEnvironment`](../crates/ambition_sandbox/src/audio/environment.rs)
layer that:

- reads the player's `WaterContact.submersion` from the existing
  movement simulator (single source of truth);
- smoothly ramps a `wetness` value (0.0 = dry, 1.0 = full underwater)
  on a wall-clock timer over ~350 ms;
- writes the resulting mix to the Kira music + SFX channels.

**What the writer actually does today: a volume duck.** Music drops
~8 dB, SFX ~5 dB, and the spectrum is unchanged. This is not a real
underwater muffle and the docs / UI must not describe it that way.

The architecture is correct (ECS state, smoothed wetness, one writer,
composes with user mixer). The *backend* is wrong: there is no
low-pass filter. This document explains why and what to do about it.

## Why we don't have a real low-pass yet

The brief asks for a Kira `LowPass` filter tweened from ~20 kHz
(transparent) to ~800 Hz (muffled). Kira itself ships exactly that:

- `kira::effect::filter::FilterBuilder` — LowPass / BandPass /
  HighPass / Notch with tweenable `cutoff: Value<f64>`.
- `kira::track::MainTrackBuilder::with_effect(...)` — attach
  effects to the main mixer track.
- `kira::AudioManager::add_sub_track(builder)` — create per-bus
  sub-tracks with their own effect chains.

The wrapper this sandbox uses, **`bevy_kira_audio` 0.25**, hides all
of this:

| Thing we need | Where it is in bevy_kira_audio | Reachable? |
| --- | --- | --- |
| `kira::AudioManager`              | `AudioOutput.manager` (`pub(crate)`) | ❌ |
| `AudioOutput`                     | `audio_output` module (private)      | ❌ |
| `MainTrackBuilder::with_effect`   | Not surfaced by `AudioSettings`      | ❌ |
| `add_sub_track`                   | Not surfaced anywhere                | ❌ |
| Per-channel effect handles        | Not modeled                          | ❌ |
| Per-channel volume / pan / rate   | `AudioChannel::set_{volume,panning,playback_rate}` | ✅ (what we use today) |

Verified by reading `bevy_kira_audio-0.25.0/src/audio_output.rs` and
`backend_settings.rs`. `AudioSettings` only forwards `sound_capacity`
to `MainTrackBuilder::new()`; there is no user-supplied effect chain
plumbing, and `AudioOutput` is never re-exported.

## Migration plan

Three viable paths. Recommended order: A → C (B is documented for
completeness but not preferred).

### Option A — Replace `bevy_kira_audio` with a thin direct-Kira layer (RECOMMENDED)

Build a small `ambition_kira` (or in-tree `crate::audio::backend`) module that owns:

- one `kira::AudioManager<DefaultBackend>` as a `NonSendResource`;
- two pre-built `add_sub_track(...)` tracks: `music_bus` and
  `sfx_bus`, each constructed with a
  `FilterBuilder::new().mode(FilterMode::LowPass).cutoff(20_000.0)`;
- the `FilterHandle`s for each, stored on the resource so
  `apply_audio_environment` can call
  `filter_handle.set_cutoff(target_hz, Tween::default())` directly;
- a `play_sound(track, asset_bytes)` API that wraps
  `StaticSoundData::from_cursor`;
- an explicit `resume()` for web (cpal exposes this through `Stream`,
  but we can also call `audio_manager.main_track().resume_at(...)`
  semantics by calling `play()` from a gesture-originated call
  stack).

What it costs:
- ~400-600 lines of Rust for the backend module;
- mechanical refactor of every `AudioChannel<X>::play(...)` call site
  in `audio/runtime.rs`, `music/`, `pause_menu/`, and the layered
  music director (~50 call sites by `grep`);
- new asset loader: bypass Bevy's `AudioSource` and load raw bytes
  for `StaticSoundData::from_cursor`.

What it buys:
- real low-pass underwater (the primary deliverable);
- access to reverb, distortion, compressor for future effects;
- direct control of cpal's `Stream::play()` for web unlock;
- removes the largest external opaque box in the audio path.

### Option B — Fork `bevy_kira_audio` and add the missing accessors

Patch the crate to:
- `pub use audio_output::AudioOutput;`
- `impl AudioOutput { pub fn manager(&mut self) -> Option<&mut AudioManager> { … } }`
- (optionally) `pub fn add_track_with_effects(...) -> AudioChannel<...>`.

What it costs:
- vendor or path-dep the fork;
- upstream PR + maintenance burden;
- a public seam (`fn manager`) that we have to keep stable.

What it buys:
- same as Option A but without re-doing the channel API.

Skip unless the upstream PR is already accepted.

### Option C — Replace the playback path entirely with a direct browser-WebAudio shim (wasm-only)

Bypass Kira on web and synthesise music+SFX through the browser's
`BiquadFilterNode` for true filtering in the host. Loses
cross-platform parity (desktop still goes through Kira), so the same
ECS writer would need two backends to drive.

What it costs:
- two backends behind the same ECS interface;
- per-platform sound-format handling;
- a parallel asset path for raw bytes → `AudioBufferSourceNode`.

What it buys:
- real filter on web *now* without a Kira refactor.

Only worth it if Option A drags on and web is the urgent target.

## Acceptance criteria for "underwater is implemented"

When one of the above lands, the following must be true (and at least
one test must pin each):

- `AudioEnvironment::apply_audio_environment` calls a real Kira
  `FilterHandle::set_cutoff(...)`, not just a per-channel
  `set_volume(...)`.
- The cutoff target moves smoothly between ~20 kHz (Normal) and
  ~800 Hz (Underwater) over 200–600 ms.
- User mixer settings still compose (master/music/sfx/mute work both
  underwater and on the surface).
- The filter chain stays installed across track switches (radio
  cycle, encounter music swap, etc.).
- Browser test: Jon can hear the audible high-frequency reduction on
  submerge and the bounce-back on surface.

Until then, the code keeps the volume-duck placeholder and docs
clearly say so.

## Until then — what to expect today

| Submerge | Surface | Volume nudge while submerged | Mute while submerged |
| --- | --- | --- | --- |
| ~8 dB music + ~5 dB SFX duck over ~350 ms | duck releases over ~350 ms | still respected (composes with environment) | silence (mute wins) |

This is enough for gameplay validation of `AudioEnvironment` plumbing
but should not be marketed as "underwater muffle". Users will hear
"the same mix, quieter".
