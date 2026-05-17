# Android power plan

Status: first-pass hygiene plan. The Android build runs well enough for phone
playtesting, but battery life should be treated as a first-class constraint
before the game grows heavier.

## Goals

- Keep desktop / Steam Deck as the most expressive target.
- Keep Android feature-parity where reasonable, but avoid desktop-only runtime
  baggage in phone builds.
- Make phone builds quiet when healthy: warnings should point at actionable
  battery, compatibility, or asset issues.
- Prefer build-time composition and runtime settings over one-off Android forks.

## Current baseline decisions

### Platform features

Android builds use the crate's `android_platform` feature instead of Bevy's
broad `default_platform` feature. This keeps GameActivity, winit, std, Bevy's
fallback font, and multi-threaded runtime support while excluding unsupported or
unnecessary phone-side platform features such as `bevy_gilrs`, desktop display
backends, web platform support, and the sysinfo plugin.

Desktop builds keep `desktop_platform = ["bevy/default_platform"]`, so normal
PC development retains desktop gamepad support and the default Bevy platform
experience.

### LDtk startup map

Android `static_map` builds load the statically embedded `sandbox.ldtk` first
unless an explicit `--ldtk`, `--map`, or `AMBITION_LDTK` override is provided.
This avoids probing a source-tree filesystem path that cannot exist inside the
APK. The APK still packages the loose LDtk asset tree so a future async
AssetServer-driven or app-storage map workflow can recover more desktop-like
map iteration on Android.

### UI fonts

The game expects bundled fonts under:

```text
crates/ambition_sandbox/assets/fonts/bundled/
```

Run:

```bash
./scripts/grab_font_assets.py
```

The generated files are intentionally ignored by git. Review the manifest and
licenses, then force-add or IPFS-track them when accepted.

## Near-term power work

1. **Frame pacing / FPS cap**
   - Add a setting for 30 / 45 / 60 / uncapped.
   - Default Android to 60 for now, with a Battery Saver option that selects 30
     or 45.
   - Keep desktop default uncapped or vsync-driven.

2. **Background / inactive throttling**
   - Pause or reduce simulation/render/audio when Android loses focus.
   - Make sure the game does not continue full-rate update while covered by the
     launcher, app switcher, or system dialogs.

3. **Audio budget**
   - Do not keep procedural preview/render helpers active in the phone runtime.
   - Avoid doing music regeneration on-device.
   - Reduce or pause music/SFX when backgrounded.

4. **UI and touch overdraw**
   - Keep large translucent overlays from covering the full screen when a small
     panel would do.
   - Fade idle touch controls instead of rendering fully opaque controls every
     frame.
   - Prefer compact mobile row layouts when menus would otherwise scroll.

5. **Diagnostics modes**
   - Keep Android release/dev-phone builds free of desktop debug overlays by
     default.
   - Gate expensive debug trace or gizmo systems behind explicit settings or
     cargo features.

6. **Startup parsing and asset loading**
   - Avoid repeated filesystem probes that always fail on Android.
   - Prefer static startup data until the Android asset-loading path can be made
     asynchronous and user-configurable.

## Measurement checklist

When profiling battery or thermals, record:

- device model and Android version;
- APK profile (`debug`, `release`, or `android-size`);
- target ABI;
- FPS cap / vsync state;
- whether touch controls, music, SFX, and debug HUD are enabled;
- approximate scene / room / enemy count;
- 5-minute and 15-minute battery deltas;
- thermal throttling messages from logcat.

Useful commands:

```bash
./build_for_android.sh --run --size-profile
adb shell dumpsys battery
adb shell dumpsys thermalservice
adb logcat RustStdoutStderr:I event:W AndroidRuntime:E '*:S'
```

## Longer-term Android parity ideas

- Support importing an LDtk world into app-private storage for phone-side map
  testing without recompiling.
- Add a developer asset-sync command that pushes changed maps/audio/fonts into
  app storage and restarts the app.
- Add an in-game diagnostics page for FPS, frame time, audio activity, and power
  mode.
- Consider a lower-power Android rendering profile if the Steam Deck / desktop
  visual target grows beyond phone needs.
