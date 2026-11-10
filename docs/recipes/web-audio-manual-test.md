# Manual browser audio check

Use this when changing web audio, gesture handling, asset delivery, music, SFX
or underwater mixing. Check audible behavior in a browser; Rust test results
cannot establish whether sound reaches the user.

## Build an audio-enabled host

```bash
./build_for_web.sh --served --serve
```

Ambition's default `./build_for_web.sh --serve` is a visual-smoke build without
its audio backend. The `--served` mode includes audio. For Mary-O, use
`./build_for_web.sh --game mary-o --serve`. Check the selected asset profile
in the startup logs before debugging sound.

The browser page loads `scripts/web/audio_unlock.js`, which resumes suspended
AudioContexts after a user gesture. `AudioUnlockState` and
`start_default_music_when_ready` defer music until both the gesture and asset
readiness; the SFX bank loads through the asset system.

## Playtest

1. Open the game and browser Console/Network panels. Disable cache if testing
   startup or packaging.
2. Click or touch the canvas (or press a key). Confirm the AudioContext reaches
   `running` using the `[ambition-audio]` diagnostics.
3. Listen for music, then trigger one representative SFX.
4. Change volume and mute; test track selection if affected by the change.
5. Investigate errors or failed asset requests through the console and the
   Rust `ambition_platformer2d::audio` logging target.

The outcome is audible behavior, rather than an exact sequence of old log
strings. When reporting a failure, include browser, URL, build mode, gesture,
observed sound, network errors and relevant diagnostics.

## Underwater mixing

If underwater audio changed, enter a water volume with swim enabled. Compare
music and SFX before submersion, while submerged, and after surfacing; check
that sliders and mute still work. The current
[underwater system](../systems/audio-underwater.md) ducks levels rather than
applying a low-pass filter.

## Troubleshooting

- No unlock log: confirm the page loads the shim; hard-reload.
- AudioContext remains suspended: inspect gesture targeting, browser autoplay
  rules, and any `resume()` errors.
- Music plays but SFX is silent: inspect SFX-bank loading and the actual logged
  URL instead of assuming a fixed path.
- Startup works only on a second reload: check browser caching and asset
  packaging before changing simulation code.

See [web build](web-build.md) for packaging and transport.
