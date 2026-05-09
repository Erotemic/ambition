# Lessons learned

This journal records unexpected errors encountered while iterating on the Ambition sandbox, especially places where an overlay or generated build script looked reasonable but failed in a real local/device test. The goal is to make future LLM-generated patches less likely to repeat the same mistakes.

## 2026-05-08: Android APK bring-up

### Prefer generated Android projects, but keep the generated Java/Gradle side explicit

The first Android APK path successfully built the Rust shared library with `cargo-ndk`, but Gradle/device launch uncovered several Java-side assumptions. Each failure happened before Bevy gameplay started, so the fix belonged in the generated Android shell rather than in gameplay code.

Observed fixes:

- `android.useAndroidX=true` is required because GameActivity is distributed through AndroidX artifacts.
- The manifest should launch an app-local `.MainActivity`, not a Maven-coordinate-looking class such as `androidx.games.activity.GameActivity`.
- `MainActivity` should extend `com.google.androidgamesdk.GameActivity`.
- GameActivity extends `AppCompatActivity`, so the app needs both `androidx.appcompat:appcompat` and an AppCompat-derived theme.
- Transitive Kotlin dependencies may mix old `kotlin-stdlib-jdk7/jdk8` artifacts with newer `kotlin-stdlib`; the generated Gradle project now aligns Kotlin artifacts and excludes obsolete compatibility jars.
- A repo-local Gradle user home under `target/android/gradle-user-home` avoids unrelated host `~/.gradle` cache permissions breaking this project.

### Do not assume adb install flags are portable

I suggested `adb install --no-stream`, but the target device rejected it as an unknown package-manager option. The build script should prefer conservative install flags (`-r -d --install-location 0`) and provide a `--fresh-install` mode that force-stops/uninstalls first.

### Overlay patches must not clobber platform entrypoints

A later Android usability overlay replaced `crates/ambition_sandbox/src/lib.rs` from a source snapshot that did not contain the Android shared-library entry point. The APK still built and installed, but launch failed with:

```text
UnsatisfiedLinkError: dlopen failed: cannot locate symbol "android_main" referenced by libambition_sandbox.so
```

The lesson is that files touched by multiple overlay series need special care. Before overwriting `lib.rs`, `Cargo.toml`, or generated build scripts, preserve platform-critical entrypoints and feature definitions added by earlier overlays.

For Bevy Android GameActivity builds, the Rust library must export `android_main`. In this project the intended pattern is:

```rust
#[cfg(target_os = "android")]
#[bevy::prelude::bevy_main]
fn main() {
    app::run_visible();
}
```

Desktop still enters through `src/main.rs`; Android packages the library as `libambition_sandbox.so` and needs Bevy's `#[bevy_main]` macro to generate the Android boilerplate.

### Keep asset behavior platform-aware

Android packages runtime assets into the APK. Host-side `CARGO_MANIFEST_DIR/assets/...` existence checks are not valid on-device. On Android, let Bevy's APK asset reader attempt the load; on desktop, host-side existence checks are still useful for clearer diagnostics.

### Treat device logs as the source of truth

The Android sequence progressed through distinct phases:

1. APK installed but manifest activity class was missing.
2. Java activity compiled but AppCompat dependency/theme was missing.
3. Native library loaded but `android_main` was missing.

Each phase required a different layer of the stack to be fixed. Avoid guessing from the symptom alone; use `adb logcat` and identify whether the failure is Gradle, install/package-manager, Java activity startup, native library loading, or Rust/Bevy runtime.

## 2026-05-08: Keep Android HUD defaults and menu toggles separate

The Android build can boot with the same desktop sandbox systems, but phone usability needs
coarse user-facing switches for large overlays. Do not only change `DeveloperTools::default`
when a HUD is too large: add an explicit persisted setting and make the render system clear
its text when the setting is off. Quest/objective UI and debug HUD text should be controlled
separately because the quest panel is useful during play while the debug dump can consume most
of a phone screen.


## 2026-05-08: Android size is a separate profile and platform-composition problem

A large Android APK/native library should not immediately trigger semantic
feature-gate churn. First separate the size mechanics from the gameplay feature
set:

- build Android with `--no-default-features --features android` so desktop-only
  inspector/file-watcher tooling does not enter the phone artifact by default;
- keep the playable sandbox, touch controls, audio, LDtk runtime, UI, and RL/test
  seams in the Android composite feature;
- add a dedicated `android-size` Cargo profile before removing gameplay systems;
- strip the final `.so` explicitly with the NDK `llvm-strip` as a backstop;
- print before/after sizes so future patches compare measurements instead of
  guessing from APK size alone.

The principle is platform composition, not release minimalism: Android can remain
a dev/test build while excluding desktop inspector/editor conveniences that are
not useful on a phone screen.


## 2026-05-08: Android APK assets are not regular files

The Android build copied `assets/audio/sfx.bank` into the APK, but the game still
fell back to generated/fundsp SFX. The reason was that the SFX bank loader used
`std::fs` and normal paths such as `/assets/audio/sfx.bank`; packaged APK assets
are not visible at those paths on-device. Bevy's `AssetServer` can load many
runtime assets from the APK, but this specific SFX-bank path is a synchronous
custom loader built around `BankProvider::from_path` / `from_bytes`.

Temporary fix: let `build_for_android.sh` statically embed the SFX bank with a
separate `static_sfx_bank` feature when the bank exists locally. Long-term fix:
teach the SFX bank loader to read bytes from Android APK assets or route it
through Bevy asset loading, then remove the static embedding workaround.

The lesson is to distinguish "copied into APK assets" from "readable via
`std::fs`". Any custom synchronous loader needs an explicit Android asset path,
static fallback, or Bevy asset pipeline bridge.

## 2026-05-08: Size diagnostics should be automatic for phone builds

A 200 MiB native library became a much more reasonable ~49 MiB `.so` after using
a size-oriented Cargo profile, disabling desktop-only default features for
Android, and stripping with the NDK toolchain. Future Android patches should keep
printing `.so`, APK, and asset-tree sizes so we notice regressions immediately.

## 2026-05-08: menu controls need their own semantic frame

Touch menu polish should not be implemented by making every menu read raw
`Touches`, raw `ButtonInput<KeyCode>`, or Leafwing `ActionState` directly.
That repeats the same problem we solved for gameplay with `ControlFrame` and
makes Android ergonomics fight keyboard/gamepad/RL semantics.

The better pattern is a parallel `MenuControlFrame` resource:

- gameplay systems consume `ControlFrame` only;
- menus/dialogue/cutscenes consume `MenuControlFrame` only;
- keyboard/gamepad, mouse wheel, touch buttons, and touch drag gestures fold
  into the menu frame before menu systems run;
- mobile touch can add scroll/back/confirm semantics without adding
  Android-specific branches to every menu.

This came up when the Android pause/settings menu was hard to use: there was no
standard touch scroll/back seam, and some systems still consumed raw keyboard
or Leafwing actions. The fix is an intent layer, not a collection of menu-local
touch hacks.

## 2026-05-08: Menu controls should be semantic, tabbed, and touch-visible

The Android phone test showed that mapping touch controls directly onto a few
keyboard/gameplay actions is not enough for menus. A menu needs its own semantic
input layer (`MenuControlFrame`) and visible touch affordances. In particular:

- Back/cancel must be visible in player-facing overlays, not only implied by a
  keyboard Escape key or gamepad button.
- Left/right should be reserved for changing high-level menu pages where a
  Zelda-style tab model is desired.
- Scroll/drag should manipulate text-heavy menu content without making gameplay
  `ControlFrame` or RL action shapes more complex.
- Phone polish and battery life are related: avoid adding per-frame heavy menu
  work just to support touch; prefer small semantic resources and simple UI
  state transitions.

The inventory panel now acts as a small adventure menu with Items / Map / Quests
tabs. That keeps the phone UI understandable while preserving the existing
keyboard/gamepad menu contract.

## 2026-05-08: Touch controls need controller-shaped affordances, not keyboard labels

The Android touch overlay originally exposed the keyboard interaction action as
an `E` button and arranged six actions as a dense 3x2 grid. That worked as a
mechanical input bridge, but it was not a good phone interface: `E` is a
keyboard mnemonic, not a player-facing action, and small grid buttons are hard to
hit while the left thumb is also holding the movement stick.

Prefer touch-native labels and controller-like spatial grouping:

- use `Use` / `Talk` / `Open` style labels instead of keyboard letters;
- put primary actions in a right-thumb diamond, with secondary actions nearby;
- make buttons large enough for thumbs and keep raw-touch hit testing aligned
  with the visible layout constants;
- keep a visible Back/Menu affordance for escape/cancel rather than assuming a
  keyboard Escape key.

This keeps the semantic input seam intact (`MenuControlFrame` / `ControlFrame`),
but makes the phone UI legible and ergonomic.

## 2026-05-08: Use `ParamSet` for tabbed UI systems that mutate several `Text` queries

The tabbed adventure-menu overlay compiled but the desktop game panicked during Bevy
system initialization with error `B0001`. The system had several separate mutable
queries over `Text`: title text, tab labels, item rows, tab-content text,
description text, and status text. Even though the entities are intended to be
different, Bevy cannot prove that arbitrary mutable queries over the same component
are disjoint unless the filters make that explicit or the queries are wrapped in a
`ParamSet`.

For UI sync systems that update several text-bearing widgets in one pass, prefer one
of these patterns:

- use `ParamSet` and touch each query sequentially;
- make query filters explicitly disjoint with marker components plus `Without<T>`;
- split the system into smaller systems if the updates do not need shared local state.

Do not assume a successful `cargo check` catches every Bevy ECS query conflict. Some
access conflicts are validated when schedules initialize at runtime, so `./run_game.sh`
and Android launch smoke tests remain important after UI refactors.

## 2026-05-08: ParamSet does not replace explicit UI disjointness

A follow-up inventory patch tried to fix Bevy `B0001` by wrapping several
text-mutating inventory queries in a `ParamSet`, but the desktop game still
panicked during schedule initialization. The safer pattern for Bevy UI sync
systems is to make the entity families explicit in the query filters as well:
`With<InventoryTitleText>` should also carry `Without<InventoryTabButton>`,
`Without<InventoryItemRow>`, and the other mutually exclusive marker components.

When a system updates several widgets that all carry `Text`, use both tools:

- group conflicting queries in a `ParamSet` when they need to share local state;
- add marker-component `Without<T>` filters so Bevy can prove each widget family
  is disjoint;
- always run a real Bevy startup smoke test (`./run_game.sh`), because query
  access conflicts can pass compile and fail only when the schedule initializes.

## 2026-05-08: For Bevy UI text sync, one role-tagged query is safer than many mutable `Text` queries

The adventure-menu panel repeatedly hit Bevy `B0001` during desktop startup.
Several fixes tried to convince Bevy that independent UI widget families were
disjoint by using `ParamSet` and marker `Without<T>` filters. That can work, but
it is fragile for UI panels where many different widgets all share `Text`,
`TextColor`, `BackgroundColor`, `Node`, and `Visibility`.

The safer pattern for one-panel sync systems is to use one role-tagged query:
query every relevant widget once with `Option<&RoleMarker>` components and
`Option<&mut ...>` presentation components, then branch by marker in code. This
creates exactly one mutable access path to `Text`, so Bevy's schedule validator
has no aliasing ambiguity.

Also watch overlay archive mtimes. If an overlay zip normalizes entries to times
older than the existing `target/release` output, Cargo may run the old binary and
appear to ignore a source fix. When a source-only overlay is meant to fix a
runtime panic, either preserve a current timestamp in the zip or explicitly run
`touch` on the changed file before the smoke test.


## 2026-05-08 - Bevy UI visibility is also a mutable component access

When fixing query aliasing in Bevy UI systems, remember that `Visibility`,
`Node`, `Text`, `TextColor`, and `BackgroundColor` are all independent ECS
components with their own access rules. Moving text widgets into one mutable
query fixes `Text` conflicts, but a separate root query mutating `Visibility`
still conflicts with any child-widget query that also asks for `&mut
Visibility`. Prefer a single visibility owner for panel roots and use
`Node.display` for child-level show/hide inside the widget query.

## 2026-05-08: Touch buttons should name actions, not keyboard keys

The mobile HUD is used on Android and as a desktop mouse-test overlay, so labels like `E` are misleading even when the default keyboard binding still uses E. Touch buttons should use semantic action labels such as `Interact`, `Jump`, `Dash`, and `Fly`. When adding a new touch button, update all three seams together: visible UI layout, raw multitouch/mouse hit testing, and `TouchButtonEdges` folding into `ControlFrame`. Missing any one of those can make the button appear on screen but not reach gameplay.


## 2026-05-08: Bevy 0.18 moved BorderRadius into Node

The touch-controller overlay tried to make circular mobile buttons by adding
`BorderRadius::all(...)` as a standalone UI component. That worked in older
Bevy examples and still looks plausible, but Bevy 0.18 moved border radius into
`Node::border_radius`; `BorderRadius` is no longer a component. The symptom is a
confusing `is not a Bundle` error for the whole spawn tuple, because one element
of the tuple is not a component bundle item.

For Bevy 0.18 UI, put radius styling inside the `Node` literal:

```rust
Node {
    width: Val::Px(size),
    height: Val::Px(size),
    border_radius: BorderRadius::all(Val::Px(size * 0.5)),
    ..default()
}
```

When updating UI style code, check the current Bevy migration notes or local
examples before assuming a visual style type is still a component.
