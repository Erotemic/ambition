# Android: what an agent cannot see, and what the device actually said

**Written 2026-08-08 at Jon's instruction**, because agents make Android-affecting
decisions constantly and **will not have a device**. Everything below is measured
from a real run on real hardware unless marked as inference.

⚠ **This is a dated observation log, not a spec.** Numbers age. The *traps* are
the durable part; re-measure before quoting a figure.

---

## 0. The access boundary — read this first

- ⛔ **There is no `adb` on the dev box and no device attached to it.** An agent
  cannot install, launch, profile, or read logcat. Every device fact in this file
  arrived by Jon pasting logs.
- ⇒ **Never write "verified on Android."** The honest phrasing is "typechecked
  for `aarch64-linux-android`" or "predicted from the desktop profile."
- ⛔ **`cargo check --target aarch64-linux-android` does not work here either** —
  a C++ dependency's build script dies before reaching first-party code without
  an NDK. The workaround used on 2026-08-08 was to temporarily retarget a file's
  `cfg` gates to the host, typecheck, and **poison-check the probe** (inject a
  type error, confirm it surfaces) before trusting the green. Restore from a copy
  afterwards and re-count the gates.
- `scripts/profile_android.sh` is the collection tool Jon runs: simpleperf
  record, a perfetto heap trace, `gfxinfo`, and a packaged tarball.
  `--profile-build` is only needed for *symbol resolution* in simpleperf; markers
  and logcat work in an ordinary build.

## 1. The hardware, and two capabilities it lacks

```
AdapterInfo { name: "Adreno (TM) 620", device_type: IntegratedGpu,
              driver: "Qualcomm ... Adreno Vulkan Driver", backend: Vulkan }
INFO  bevy_render::batching::gpu_preprocessing: GPU preprocessing is not
      supported on this device. Falling back to CPU preprocessing.
WARN  bevy_egui::render: Feature TEXTURE_BINDING_ARRAY is not supported.
ERROR bevy_gilrs: Failed to start Gilrs. Gilrs does not support current platform.
```

⭐ **GPU preprocessing falls back to CPU** — so per-object CPU cost scales with
draw count in a way it does not on the desktop RTX 3090. A room with many
distinct sprites is more expensive here than desktop numbers predict.
The `gilrs` error is benign (no gamepad support); do not chase it.

## 2. Frame-time baseline, 2026-08-08

| where | p50 | p95 | note |
|---|---:|---:|---|
| launch window | 14–17 ms | 21–28 ms | ~60 fps, healthy |
| hub (`central_hub_complex`) | ~20 ms | — | ~50 fps |
| **Hall of Characters** | **47–64 ms** | 74–106 ms | **~20 fps** |
| Hall at Potato | markedly better (Jon, qualitative) | | |

Startup to first frame: **470.5 ms**. Launch spikes: 572 ms, 566 ms, 250 ms —
all sitting on `[image]` decode batches, same shape as desktop.

⛔ **The Hall is the stress case and `AGENTS.md` is explicit: when the Hall is
slow, fix the ENGINE, not the Hall.** Do not "optimise" by reducing what the Hall
stages.

## 3. ⛔⛔ The texture story — the dominant Android cost

**Android defaults to `VisualQualityProfile::Medium`**
(`default_visual_quality_profile()`), which sets sprites / backgrounds / parallax
to `TextureResolutionScale::Half`. That platform-awareness is real and correct —
do not "add" it, it exists.

What is broken around it, all measured on device:

1. **Nothing is ever evicted.** Resident textures only grow: 192.9 MB → 526.7 MB
   across one session, and 699.9 MB in another. No census in any run showed a
   decrease.
2. **A quality change does not re-decode what is already loaded.** Confirming
   `Low` re-ran `load_game_assets` (`140/140 catalog entries declared`) and added
   **+1 image**. Bodies already on screen keep their old handles; only
   newly-materialized characters pick up the new tier. ⇒ the setting appears to
   "work on next load", which is exactly what Jon observed.
3. **⇒ lowering quality currently makes memory WORSE**, never better: new tiny
   textures are added and nothing is released.
4. **The same sheet gets decoded at two tiers and both stay resident** —
   `sprites/alice_spritesheet.png` (9.3 MP) at t=2.5 s and
   `sprites_0_5x/alice_spritesheet.png` (2.3 MP) at t=68 s, in one session.
5. **Some sheets load full-res even at Half.** In one startup batch,
   `flying_spaghetti_monster_boss`, `giant_gnu` and `trex_enemy` came from
   `sprites_0_5x/` while `snakes_on_a_paper_plane`, `snakes_on_a_cartesian_plane`,
   `ai_slop`, `solid_snake`, `erdish`, `goblin`, `architect`, `alice` and `bob`
   came from `sprites/`. **Two load paths disagree about quality** — this is the
   likely reason Jon reports "snakes on a plane don't change quality at all."
6. Textures are **uncompressed RGBA8** — 4.19 bytes/pixel measured. ASTC is
   universal on modern Android and is the single largest available win.

## 4. The APK ships every quality tier

```
[android-build] size summary: native=239M  apk=1.1G  apk-assets=1.1G
final package matches asset contract: ... (4301 files under assets/)
```

**1.1 GB of assets**, because `PROFILE_EXCLUDES["android"] = ()` in
`scripts/package_asset_guard.py` excludes nothing, and all four sprite tiers plus
the ultrapacks are packaged: `sprites` 236 M, `_0_5x` 98 M, `_0_25x` 39 M,
`_potato` 5 M, plus `sprite_packs/{full,half,quarter,potato}` at 220/92/35/4.3 M.
⚠ trimming needs care — the settings menu can still request `Full` at runtime.

## 5. ⛔⛔ SYMLINKED ASSETS DO NOT SHIP, AND THE CONTRACT CANNOT SEE IT

`iter_regular_files` skipped every symlink, so when the six LDtk worlds moved
into the `game/ambition_map_assets` submodule on 2026-08-08 and became tracked
symlinks, **none of them entered the APK.** The device logged
`ERROR bevy_asset::server: Path not found: worlds/sandbox.ldtk`.

⭐ **The postcondition passed while this was true, and could not have failed.**
`final package matches asset contract: 4301 files` — the contract is built by the
SAME walk that dropped the files, so the two views agreed with each other and
both were wrong. **Two views derived from one broken enumeration cannot
disagree**, which is the vacuous-agreement sibling of
`enumerate-one-way-validate-another-2026-08-08.md`.

Fixed the same day: symlinked *files* are now followed and packaged by content
(the output-side "no symlinks in the package" rule is unchanged and still
enforced on the destination), and a dangling link raises with the
`git submodule update --init --recursive` hint instead of vanishing.

⚠ **Anything that adds a symlink under an asset root must be checked against the
package**, not just against desktop, where symlinks resolve transparently.

## 6. Lifecycle: a spurious suspend/resume is normal, and it can freeze the world

`InsetsChanged` fires on ordinary UI events and can produce a **0.6-second
suspend→resume pair** with no user backgrounding. Observed sequence around a
quality change: menu close at 12.3 s, `InsetsChanged` at 13.6 s, suspend at
13.7 s, resume at 14.4 s.

⛔ `apply_android_suspend_to_game_mode`
(`game/ambition_app/src/host/platform/android.rs`) does
`state.mode_before_suspend.take()` **before** checking
`matches!(mode.get(), GameMode::Paused)`. If anything moved the mode in between,
the saved mode is consumed and discarded with no second chance — the world stays
in a state the player never chose while menus keep working. **Not yet fixed**;
the `[game-mode]` / `[sim-clock]` probes were added first so the fix has
evidence.

⚠ **A frozen world is TWO globals, not one** — `GameMode` *and* a sim clock that
can sit at zero. See `[sim-clock]`.

## 7. Reading a device log — the markers that matter

All of these reach logcat through `RustStdoutStderr` (verified), so an ordinary
build is enough:

| marker | answers |
|---|---|
| `[frame-census]` / `[frame-spike]` | is it smooth, and when did it hitch |
| `[image]` / `[image-census]` | which sheet decoded, at which tier, total resident |
| `[game-mode]` / `[sim-clock]` | is the world actually running, **with frame numbers** |
| `[world-event]` | room transitions, session start/end |
| `[sprite-bind]` / `[sprite-size]` | which sheet a body bound, and its draw scale |

⭐ **Frame numbers, not timestamps, settle ordering questions.** Bevy applies
`NextState` in a deferred transition; two events 0.3 ms apart may be same-frame
or adjacent-frame, and only the frame index distinguishes them.

## 8. Noise to ignore (all benign, all present on a healthy run)

- `WARN winit::platform_impl::android: TODO: handle Android InsetsChanged` —
  upstream stub, fires constantly.
- `WARN bevy_egui::input: Cannot access an underlying winit window` at startup.
- `ERROR bevy_gilrs: ... does not support current platform`.
- `GgrsSchedule ... hierarchy contains redundant edge(s)` — a scheduling hint.
- `no render family claimed 'NpcSpawn-…'` — real but unrelated to Android; see
  the presentation-binding notes.

## 9. Standing open items (2026-08-08)

- texture eviction: none exists
- quality change does not re-decode live sprites
- two load paths disagree about which tier to use
- no GPU texture compression anywhere
- APK ships four tiers and uses one
- the suspend/resume mode-restore defect above

Related: [`enumerate-one-way-validate-another-2026-08-08.md`](../benchmark-candidates/enumerate-one-way-validate-another-2026-08-08.md)
