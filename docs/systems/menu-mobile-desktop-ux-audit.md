# Menu UX audit: mobile + desktop Bevy UI

Status (2026-05-22): planning note only. This document records issues found in
the current Bevy UI menu stack and sketches directions for a later polish pass.
It does not prescribe a crate migration. The current code already has useful
abstractions; the main problem is that menu feel is tuned in scattered local
constants instead of through a runtime adaptive UI profile.

## Context

The current menu implementation is built on Bevy UI and includes several strong
seams:

- `crates/ambition_sandbox/src/input/menu.rs` defines `MenuControlFrame`, a
  device-agnostic semantic menu input resource.
- `crates/ambition_sandbox/src/ui_nav/` contains shared list, pointer, and drag
  helpers.
- `crates/ambition_app/src/host/mobile_input/menu_bridge.rs` folds touch,
  joystick, mouse-drag testing, and on-screen buttons into the menu frame.
- Pause menu, adventure menu, map menu, and dialog are already mostly routed
  through semantic menu intent rather than raw device events.

This means the next pass should probably be an adaptive design-system pass, not
an immediate replacement of Bevy UI or the existing menu architecture.

## Main UX issues

### 1. Mobile/desktop layout is compile-time, not runtime

`crates/ambition_sandbox/src/pause_menu/ui.rs` currently derives its mobile
metrics from `cfg!(target_os = "android")`:

```rust
const IS_MOBILE: bool = cfg!(target_os = "android");
```

That makes Android builds get larger touch metrics, but it does not account for:

- iOS or future mobile targets;
- web builds running on phones;
- desktop windows resized to phone-like dimensions;
- Steam Deck / handheld / tablet form factors;
- orientation changes;
- user scale preferences;
- safe-area or system-bar insets.

The pause menu has useful mobile-specific values for row height, padding, font
size, panel width, and scrollbar width, but those values are currently selected
by target OS instead of the active window and input context.

### 2. Responsiveness is inconsistent across screens

The pause menu has the most explicit mobile treatment. Other menu-like screens
still use mostly fixed desktop-oriented values.

Examples:

- `crates/ambition_sandbox/src/inventory/ui.rs` uses a `620px` panel width with
  a `96%` maximum. This can work in landscape or desktop, but portrait phones
  need a more intentionally full-screen layout.
- Dialog choice rows in `crates/ambition_sandbox/src/dialog/ui.rs` use a
  `38px` minimum height. That is compact for touch and smaller than the pause
  menu's Android row height.
- Tab bars, settings rows, dialog choices, inventory rows, and map rows do not
  currently share a single set of metrics for minimum touch target, font size,
  panel padding, gaps, and scrollbar width.

This can make each screen feel tuned in isolation even though they are all part
of one menu system.

### 3. Touch activation is conservative, but not yet native-feeling

The current mobile menu policy intentionally avoids accidental activation by
using tap-to-select-then-confirm behavior for rows. That is safer than immediate
activation during a drag, but it can also feel like the first tap was ignored
unless the selected/armed state is obvious.

A more native-feeling mobile path usually needs explicit press semantics:

1. pointer/touch down visually depresses or arms the row;
2. moving outside the row cancels the press;
3. moving beyond a drag threshold turns the gesture into scroll instead of tap;
4. release inside the row activates it;
5. release outside the row cancels it.

The existing `ui_nav` and `MenuControlFrame` seams are good places to add this,
but the code should avoid treating `Interaction::Pressed` alone as the final
activation semantics for mobile.

### 4. Drag-to-scroll is useful, but too globally shaped

`fold_to_menu_control_frame` in `host/mobile_input/menu_bridge.rs` currently
adds drag-scroll input when a touch or pressed mouse cursor is outside the fixed
touch-control regions:

```rust
frame.scroll_y += gesture.drag_scroll.update(menu_pos, 30.0, 3.0, 5.0);
```

This is a good start for phone testing and desktop mouse-as-touch testing, but
it can be too coarse once more UI widgets exist. Drag behavior should eventually
know what the pointer began on:

- list row;
- scrollbar;
- slider;
- tab bar;
- panel background;
- fixed gameplay touch control;
- outside the active UI.

Only some of those origins should turn into scroll. Sliders and scrollbars need
ownership of their own drags, while row drags need a tap-vs-scroll threshold.

### 5. Touch HUD visibility is coupled to menu touch input

`fold_to_menu_control_frame` returns early when `TouchControlsVisible` is false.
That makes sense for the visible gameplay HUD, but it may be surprising if a
player hides the on-screen controls and then loses touch-driven menu scrolling,
selection, or back/start behavior.

Consider splitting the concepts:

```rust
pub struct TouchHudVisible(pub bool);
pub struct MenuTouchInputEnabled(pub bool);
```

`TouchHudVisible` would control the rendered virtual sticks/action buttons.
`MenuTouchInputEnabled` would decide whether touches and drags feed semantic
menu input.

### 6. Safe-area handling is missing

Mobile menu and HUD layout currently use fixed margins and percentages. A later
mobile polish pass should add an ECS-visible safe-area resource, even if it is
manual/configurable at first:

```rust
#[derive(Resource, Clone, Copy, Debug, Default)]
pub struct SafeAreaInsets {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}
```

The resource should feed root panel padding, bottom HUD placement, top/right
menu buttons, dialog panels, and any future phone portrait layout.

## Recommended direction: runtime UI profiles

Keep Bevy UI as the rendering/layout backend, but add a runtime profile and
metric layer that every menu screen consumes.

Sketch:

```rust
#[derive(Resource, Clone, Copy, Debug)]
pub struct UiMetrics {
    pub profile: UiProfile,
    pub scale: f32,
    pub safe: SafeAreaInsets,
    pub root_padding: f32,
    pub panel_width_pct: f32,
    pub panel_max_width_px: f32,
    pub row_height: f32,
    pub row_gap: f32,
    pub button_pad_h: f32,
    pub button_pad_v: f32,
    pub body_font: f32,
    pub button_font: f32,
    pub title_font: f32,
    pub scrollbar_width: f32,
    pub touch_slop_px: f32,
    pub drag_scroll_px_per_step: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UiProfile {
    PhonePortrait,
    PhoneLandscape,
    Tablet,
    Handheld,
    DesktopSmall,
    DesktopLarge,
}
```

A simple first classifier can use the primary window size and aspect ratio:

```rust
fn classify_ui_profile(width: f32, height: f32) -> UiProfile {
    let short = width.min(height);
    let long = width.max(height);
    let portrait = height > width;

    if short < 600.0 && portrait {
        UiProfile::PhonePortrait
    } else if short < 600.0 {
        UiProfile::PhoneLandscape
    } else if short < 900.0 {
        UiProfile::Tablet
    } else if long < 1400.0 {
        UiProfile::DesktopSmall
    } else {
        UiProfile::DesktopLarge
    }
}
```

This does not need to be perfect initially. The important architectural change
is that profile selection becomes data-driven and debuggable at runtime instead
of compile-time Android-only branching.

## Potential implementation plan

### Phase 1: document and centralize metrics

- Add `UiProfile`, `UiMetrics`, and `SafeAreaInsets` resources.
- Populate metrics from primary-window dimensions each frame or when the window
  changes.
- Add a debug log or overlay line showing the active profile and scale.
- Keep old menu layout code intact initially.

### Phase 2: migrate pause menu

- Replace `IS_MOBILE` constants in `pause_menu/ui.rs` with `UiMetrics`.
- Preserve the current Android-vs-desktop values as initial profile defaults.
- Make the pause menu respond to desktop window resizing and phone orientation.
- Avoid changing behavior and layout in the same patch if possible.

### Phase 3: migrate adventure/inventory and dialog

- Replace the inventory panel's fixed `620px` base width with profile-driven
  panel sizing.
- Give phone portrait a near-full-screen adventure menu layout.
- Raise dialog choice row height on touch profiles.
- Share button/font/gap metrics across pause, inventory, map, and dialog.

### Phase 4: improve mobile activation semantics

- Add an active pointer/touch press tracker for rows and buttons.
- Confirm on release-inside rather than press-down for touch profiles.
- Cancel activation when the finger leaves the row or crosses the drag
  threshold.
- Keep keyboard/gamepad/desktop mouse behavior simple and responsive.
- Revisit `MenuTapMode` after release semantics exist; it may become a safety
  option rather than the default mobile workaround.

### Phase 5: make drag ownership explicit

- Track where a drag starts: row, scrollbar, slider, tab, panel background, or
  outside UI.
- Let sliders and scrollbars own their drags.
- Let list backgrounds and row drags above threshold scroll.
- Keep desktop mouse-drag testing as a supported path.

### Phase 6: split touch HUD visibility from menu touch input

- Introduce separate resources for HUD visibility and menu touch enablement.
- Keep existing settings behavior mapped to HUD visibility unless a setting name
  change is desired.
- Ensure hiding the gameplay overlay does not disable menu touch gestures unless
  the user explicitly asks for that.

## Design notes

- Prefer different layouts over a single global scale factor. Desktop panels can
  stay centered and constrained; phone portrait should often be wider,
  taller, and more bottom-aware.
- Treat minimum row height, row gap, scrollbar width, and font size as shared
  tokens. Individual menus should not invent their own touch target sizes.
- Keep `MenuControlFrame` as the semantic input seam. It is one of the strongest
  parts of the current design.
- Use Bevy UI wrappers or role components rather than raw `Node`/`Button`
  literals in every screen once the metrics are introduced.
- Avoid switching UI crates only to get nicer defaults. A crate migration should
  be justified by solving a concrete hard problem: responsive layout, declarative
  authoring, pointer/touch semantics, or tooling.

## Suggested validation matrix

Test the same pause, adventure, map, and dialog flows under these profiles:

| Profile | Example shape | Main risk |
| --- | --- | --- |
| Phone portrait | narrow/tall window | crowded panels, small rows, bottom safe area |
| Phone landscape | short/wide window | vertical compression, accidental HUD overlap |
| Tablet | medium large touch | desktop layout may feel sparse but phone layout may feel oversized |
| Handheld | Steam Deck-like | needs gamepad and touch/mouse parity |
| Desktop small | laptop/windowed | avoid oversized mobile-feeling controls |
| Desktop large | ultrawide/fullscreen | avoid over-wide panels and long pointer travel |

For each profile, check:

- Back/start/confirm are available from keyboard, gamepad, mouse, and touch.
- Rows can be tapped without accidental scroll.
- Lists can be dragged without accidental activation.
- Dialog choices remain readable and reachable.
- Settings sliders do not fight list scrolling.
- Safe-area padding can be simulated and visibly respected.

## Non-goals for this note

- No source code changes are included here.
- No decision is made to replace Bevy UI.
- No final metric values are specified; the values above are scaffolding for a
  tuning pass.
