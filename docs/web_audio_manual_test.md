# Manual browser audio checklist

> The agent that landed the audio port (this checklist's source) cannot
> open a browser. Jon needs to walk through these steps and report back
> what he saw — only then is the web audio path considered verified.

## Setup

```sh
./build_for_web.sh --served --serve
# open http://localhost:8000/
# devtools -> Network -> Disable cache -> hard reload (Cmd-Shift-R / Ctrl-Shift-R)
```

`--served` builds with `--features web_served_assets`, which now
includes `web_audio` (so `bevy_kira_audio` is in the wasm) and
auto-symlinks `crates/ambition_sandbox/assets/` to
`crates/ambition_sandbox/web/assets/` so the served URLs resolve.

## What to look for in the devtools console

Filter: `ambition` (catches both `ambition::sandbox_assets` boot
banner and `ambition::audio` unlock lines).

Expected lines, in order:

1. **Boot banner:**
   `web start: AssetProfile = web_served_assets | static_map = true | static_core_assets = false | static_sfx_bank = false`
2. **Audio lock notice (wasm only):**
   `audio locked until first user gesture (click / key / touch); kira will start playback once the AudioContext resumes`
3. **Audio unlock notice (after first click / key / touch):**
   `audio unlocked (user gesture detected)`
4. **SFX bank load:** one of —
   - `loading sfx bank from `audio/sfx.bank` (async via AssetServer)`
     followed by
     `sfx bank loaded async (NN entries) — promoting to SfxBankResource`,
     **or**
   - if the bank fails to fetch: a Bevy load error referencing
     `/assets/audio/sfx.bank` and the audio runtime falls back to silent
     stubs (the game still runs).

## What Jon should report back

For each of the items below, please paste the literal log line or note
"missing" / "saw error: <text>":

- [ ] **Boot banner** — exact line + which `AssetProfile` it reported.
- [ ] **FPS overlay** — still visible bottom-right? Roughly what FPS?
- [ ] **Visuals** — does the main character sprite render? Do parallax
      layers still scroll? Any sprites that paint as colored rectangles
      instead of art?
- [ ] **Audio lock message** — saw it? Paste the line.
- [ ] **Audio unlock message** — what action triggered it (click on
      canvas, keypress, touch)? Paste the line.
- [ ] **SFX plays** — try jumping, dashing, hitting an enemy — does
      anything audible come out? Even a soft click is fine.
- [ ] **Music plays** — does the room music start after the unlock
      gesture? Pause menu music selector cycles through tracks?
- [ ] **Console errors** — any red-text errors? Paste them.
- [ ] **`/assets/audio/...` 404s** — Network tab → filter `audio` →
      anything return 404 / 500 / CORS error?
- [ ] **Any panic stack** — full text from the console.

## Notes for the next iteration

- If music plays but SFX is silent: the bank likely failed to load.
  Check the Network tab for `/assets/audio/sfx.bank` — should be a
  successful 200 with the right MIME type. The Bevy wasm HTTP reader
  serves any bytes through the registered `AssetReader`; the custom
  `SfxBankAsset` loader then parses them into a `BankProvider`.
- If audio works on the second hard-reload but not the first: a
  caching issue. Force-reload with devtools "Disable cache" should
  fix it; if not, the `web/assets/` symlink may be stale (re-run
  `./build_for_web.sh --served`).
- If audio NEVER unlocks despite clicks: `kira`'s `cpal` backend may
  not be auto-resuming the `AudioContext`. The current plugin only
  logs; if needed, `web/index.html` can be extended with a JS
  `audioCtx.resume()` shim triggered by the first canvas pointerdown.

## Underwater audio environment (this checklist's new section)

The sandbox now ships an ECS [`AudioEnvironment`](../crates/ambition_sandbox/src/audio/environment.rs)
layer. When the player's `WaterContact.submersion >= 0.5` the target
flips to `Underwater`, the wetness ramps over ~350 ms, and a single
writer (`apply_audio_environment`) re-pushes music + SFX channel
volumes with an attenuation multiplier composed on top of the user's
mixer settings.

> **Backend reality:** `bevy_kira_audio` 0.25 does not expose
> Kira's track-level `FilterBuilder` API, so the "underwater muffle"
> currently lands as a music-and-SFX duck rather than a true
> 500–1200 Hz low-pass. The swap points are tagged
> `TODO: kira_underwater_filter_backend` in
> [`audio/environment.rs`](../crates/ambition_sandbox/src/audio/environment.rs).

### How to enter underwater state

1. Build + serve as above (`./build_for_web.sh --served --serve`).
2. After audio unlocks (first click / key / touch), find a room with
   a water volume — the LDtk `water_test` room and the hub basement
   pool both work.
3. Walk in and let the player sink so the head is below the surface
   (`WaterContact.submersion >= 0.5`). Without the `swim` ability
   this triggers a reset, so toggle swim on in dev tools first.

### Expected audible change

- Within ~350 ms after submersion crosses the threshold, music
  audibly ducks by roughly 8 dB and SFX by roughly 5 dB.
- Surface again → both return to full level over the same window.
- Adjust the music slider (pause menu) while submerged — the
  underwater duck **stays applied** on top of whatever new mixer
  level you pick. Muting still produces silence.
- Pause the game; the wetness transition keeps running (audio buses
  are on the wall clock), so unpausing while still submerged should
  not "snap" the mix.

### Things to report back

- [ ] **Submerge** — does the mix dip within ~½ second? Paste
      anything visible in the console (no per-frame logs are wired,
      so this is mostly an audible check).
- [ ] **Surface** — does the mix come back over the same window?
- [ ] **Slider sweep while underwater** — does music volume still
      respond to the slider?
- [ ] **Mute while underwater** — silence?
- [ ] **Any clipping / artifacts** — pops at the transition edges?
      Step-wise jumps instead of smooth ramps?

If the duck never lands, the most likely cause is `apply_audio_environment`
never running (check the system order in `app/plugins.rs`) or the
`PrimaryPlayer` query coming up empty.

## Future work (not part of this verification)

- "Click to enable audio" banner in `web/index.html`.
- Detect and surface "audio context never resumed" as a visible UI
  warning, not just a console log.
- Per-room ambience and combat-stem layering on the web build (the
  music director already supports it on desktop; the limiter is just
  whether the layered assets are reachable through the catalog under
  `WebServedAssets`).
- **`TODO: kira_underwater_filter_backend`** — replace the
  `AudioEnvironment::{music,sfx}_attenuation` multipliers with a
  real Kira `FilterBuilder` (LowPass) once `bevy_kira_audio` exposes
  track-level effect insertion, or once we ship a thin direct-Kira
  shim that bypasses the wrapper for filter access.
