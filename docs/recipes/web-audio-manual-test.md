# Manual browser audio checklist

A person with a browser runs this checklist and reports what they saw. The web
audio path is verified only after that report.

## How the web audio path works

1. **JS AudioContext-unlock shim** in `game/ambition_app/web/index.html`. It
   patches `window.AudioContext` to track every context cpal creates, then calls
   `ctx.resume()` from a real DOM `pointerdown` / `keydown` / `touchstart` /
   `click` handler. Without it, Kira's `play()` calls from Bevy's RAF loop do not
   resume the context.
2. **Deferred music startup.** `start_default_music_when_ready`
   (`crates/ambition_audio/src/library.rs`) polls each Update for
   `AudioUnlockState.unlocked` (first user gesture observed) and for the default
   music asset to finish loading. The first `play()` fires only after both.
3. **Diagnostics.** Every audio-pipeline event logs under target
   `ambition_platformer2d::audio` (`AUDIO_LOG_TARGET`).

## Setup

```sh
./build_for_web.sh --served --serve
# open http://localhost:8000/
# devtools -> Network -> Disable cache -> hard reload (Cmd-Shift-R / Ctrl-Shift-R)
```

⚠️ **The `--served` flag is required for audible audio.** Without
it, `./build_for_web.sh --serve` produces a `web` (visual smoke)
build that does NOT include `bevy_kira_audio` — the browser will
boot silent regardless of the JS shim, because there's no audio
backend in the wasm at all. The build script warns about this:

```text
[web-build] warning: audio: DISABLED in build. This is a visual-smoke build only.
[web-build] warning: audio: bevy_kira_audio is NOT in the wasm; the browser will boot silent.
[web-build] warning: audio: for audible web audio rebuild with: ./build_for_web.sh --served --serve
```

With `--served`, you should see:

```text
[web-build] audio: ENABLED in build (bevy_kira_audio in wasm). The browser must show 'AssetProfile = web_served_assets' in the boot banner.
```

`--served` builds with `--features web_served_assets`, which includes
`web_audio` (so `bevy_kira_audio` is in the wasm) and auto-symlinks
`crates/ambition_platformer2d_actor_monolith/assets/` to `game/ambition_app/web/assets/`
so the served URLs resolve.

### First sanity check after page load

The very first log line under target `ambition_platformer2d::platformer_assets` is:

```text
web start: AssetProfile = <PROFILE> | static_map = ... | static_core_assets = ... | static_sfx_bank = ...
```

**`<PROFILE>` must be `web_served_assets`.** If it shows `web_static`
instead, you ran a `web` (visual-smoke) build. Stop, rerun
`./build_for_web.sh --served --serve`, hard-reload.

Diagnostic table:

| Boot banner says | Means | Audio status | Fix |
| --- | --- | --- | --- |
| `web_served_assets` | served-assets build, audio compiled in | should be audible after gesture | follow checklist below |
| `web_static` | visual-smoke build, no Kira | silent, by design | rebuild with `--served` |
| `no_assets` | smoke-test build, no assets | silent, no visuals either | rebuild with `--served` |

## Console logs to watch for

The JS shim and Rust side both log to the browser console.

### Filter `[ambition-audio]` (JS shim)

Expected, in order:

1. `[ambition-audio] AudioContext unlock hook installed (listeners: pointerdown, keydown, touchstart, click)`
2. `[ambition-audio] AudioContext created (state=suspended, sampleRate=44100). Waiting for first user gesture to resume.`
3. After first click/keypress:
   `[ambition-audio] resume() succeeded via pointerdown (now state=running)`
   then
   `[ambition-audio] attempted resume on 1 suspended context(s); current states=[running]`

### Filter `ambition_platformer2d::audio` (Rust side)

Expected, in order:

1. `ambition audio: kira plugin installed; AudioContext is suspended until first user gesture …`
2. `ambition audio: loading sfx bank from \`audio/sfx.bank\` (async via AssetServer)`
3. `default music: waiting for asset \`audio/music/generated/<id>/full.ogg\` (track \`<id>\`) to load before first play` *(may flash for a frame on fast loaders; on slow networks it may sit here for several seconds)*
4. After first click/keypress:
   `ambition audio: first user gesture observed; flagging AudioUnlockState. Music + SFX startup will now fire.`
5. `default music: track \`<id>\` asset \`<path>\` loaded; starting playback`
6. `ambition audio: sfx bank loaded async (N entries) — promoting to SfxBankResource`
7. `ambition audio: SfxBankResource installed (audio_library_refreshed=true)`
8. On first jump/dash/hit:
   `ambition audio: first SFX play attempt (cue=Some(Jump), bank_loaded=true)`

## What to report back

For each item: paste the literal log line, or note "missing" /
"saw error: <text>".

### Boot

- [ ] **Boot banner** — `web start: AssetProfile = web_served_assets …`
- [ ] **`[ambition-audio]` unlock hook installed** — paste the line
- [ ] **`AudioContext created`** — paste the line; what is `state` and
      `sampleRate`?

### Unlock

- [ ] **Action you used** — click on canvas, keypress, touch?
- [ ] **`resume() succeeded via …`** — paste the line; what does
      `state` say afterwards?
- [ ] **`first user gesture observed`** (Rust side) — paste the line
- [ ] If you see `resume() failed via …` — **paste the error message
      verbatim**; that is the smoking gun.

### Music

- [ ] **`default music: waiting for asset …`** — appears at all?
      Disappears after a moment?
- [ ] **`default music: track … loaded; starting playback`** — paste
      the line
- [ ] **Audible?** — does anything come out of your speakers /
      headphones? Pause-menu radio selector cycles tracks?
- [ ] **First music `.ogg` Network status** — devtools → Network →
      filter `.ogg` → status code on the first OGG?

### SFX

- [ ] **`loading sfx bank from …`** — paste the line
- [ ] **`sfx bank loaded async (N entries)`** — what is `N`?
- [ ] **`SfxBankResource installed (audio_library_refreshed=true)`** —
      paste the line; was `audio_library_refreshed` `true`?
- [ ] **`first SFX play attempt`** — try jumping / dashing / hitting
      an enemy; paste the line
- [ ] **Audible?** — any click / pop / whoosh when you trigger an
      action?
- [ ] **`/assets/audio/sfx.bank` Network status** — devtools →
      Network → filter `sfx.bank` → status code?

### Errors

- [ ] **Console errors** — any red-text entries? Paste them.
- [ ] **Any panic stack** — full text.
- [ ] **`/assets/audio/...` 404s** — anything return 404 / 500 / CORS
      error?

## Underwater audio

> ⚠️ **The underwater effect is a volume duck, not a low-pass filter.**
> It is ~8 dB on music and ~5 dB on SFX, ramped over ~350 ms. The mix
> gets quieter; the spectrum is unchanged. See
> [`../systems/audio-underwater.md`](../systems/audio-underwater.md).

### How to enter underwater state

1. Build + serve as above.
2. After audio unlocks (first click / key / touch), find a room with
   a water volume — the LDtk `water_test` room and the hub basement
   pool both work.
3. Walk in and let the player sink so the head is below the surface
   (`WaterContact.submersion >= 0.5`). Without the `swim` ability
   this triggers a reset, so toggle swim on in dev tools first.

### What you should hear

- Within ~350 ms after submersion crosses the threshold, **music
  audibly drops in level** by roughly 8 dB and SFX by roughly 5 dB.
  The mix sounds the same, just quieter.
- Surface again → both return to full level over the same window.
- Adjust the music slider (pause menu) while submerged — the
  level reduction **stays applied** on top of whatever new mixer
  level you pick. Muting still produces silence.
- Pause the game; the wetness transition keeps running (audio buses
  are on the wall clock), so unpausing while still submerged should
  not "snap" the mix.

### Things to report back

- [ ] **Submerge → mix dips?** Within ~½ second? (volume-only is OK
      for this checkpoint — that's all the current backend can do)
- [ ] **Surface → mix returns?**
- [ ] **Slider sweep while submerged** — does music volume still
      respond?
- [ ] **Mute while submerged** — silence?
- [ ] **Clipping / artifacts** — pops at the transition edges?
- [ ] **Do you actually hear high-frequency damping (muffled)?** Most
      likely **no** — the docs say the current backend can't do that.
      If somehow you do, that's surprising and worth a note.

## Troubleshooting

- If `[ambition-audio] AudioContext unlock hook installed` is missing
  from console, the JS shim was bypassed (cached `index.html`?). Hard
  reload, disable cache.
- If `resume() succeeded` never fires despite gestures: the browser
  blocked autoplay on a different layer; try clicking the canvas
  specifically, or check `chrome://settings/content/sound` /
  Firefox autoplay policy.
- If music plays but SFX is silent: the bank likely failed to load.
  Check the Network tab for `/assets/audio/sfx.bank` — should be a
  successful 200 with the right MIME type. The Bevy wasm HTTP reader
  serves any bytes through the registered `AssetReader`; the custom
  `SfxBankAsset` loader then parses them into a `BankProvider`.
- If audio works on the second hard-reload but not the first: a
  caching issue. Force-reload with devtools "Disable cache" should
  fix it; if not, the `web/assets/` symlink may be stale (re-run
  `./build_for_web.sh --served`).
