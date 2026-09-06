# ADR 0028: Dialogue presentation is provider-selected

## Status

**Accepted; implemented for Ambition and the reusable default presenter**
(2026-07-18).

## Context

The dialogue runtime and observation boundary were already reusable:
`ambition_dialog::DialogState` owned Yarn progress and input requests, while
`ambition_sim_view::DialogView` projected the current visible line and options.
The final presentation step was not reusable in the same sense. The Ambition app
scheduled one concrete `ambition_render::dialog_ui::sync_dialog_ui` function,
and that renderer hard-coded panel geometry, transparency, title formatting,
choice rows, and the input hint.

A style resource would only move that coupling into a large cross-game schema.
Different games may want portraits, speech balloons, visual-novel framing,
minimal subtitles, diegetic panels, or no dialogue overlay at all. They should
not edit or parameterize one universal UI tree.

## Decision

The engine owns dialogue facts and presenter lifecycle, not a mandatory visual
composition:

- `DialogState` owns dialogue progression and exposes raw speaker /
  conversation labels.
- `DialogView` contains presentation-neutral facts: active state, dialogue id,
  current speaker label, conversation label, revealed body, choices, and
  selection. It does not preformat a title.
- `ambition_render::dialog_ui` exposes `DialogPresentationSet`, the shared
  `DialogOverlayRoot`, and `claim_dialog_presentation`.
- A visible App installs exactly one dialogue presenter plugin. Installing two
  presenters is a composition error.
- `DefaultDialogUiPlugin` is an opt-in, deliberately plain engine fallback.
  Reusable platformer presentation does not silently install it.
- Concrete games own their own UI tree and portrait policy while continuing to
  use the shared `DialogChoiceSlot` input marker.

Ambition installs `AmbitionDialogUiPlugin` from its content-owned presentation
module. Its product policy is a classic fully opaque panel horizontally
centered in an upper-screen safe band, never vertically centered. A responsive
choice window bounds the panel's normal height so the player focus region and
lower touch controls usually remain visible. The speaker header, body, and
footer keep their measured height and are never sacrificed to fit more options;
only the choice list is windowed. The layout selects a mobile/short-window
profile from the actual viewport and never shrinks body or choice text below the
product's readability floor.

Long option lists do not create a second presentation-only cursor. The renderer
windows rows around `DialogView.selected_option`; every visible row keeps its
absolute `DialogChoiceSlot` index. Keyboard, physical gamepad, touch joystick,
touch buttons, mouse wheel, touch drag, mouse click, and direct touch press all
therefore manipulate the same dialogue selection. Directional controls wrap;
scroll gestures preserve their step magnitude and clamp at list edges. Pointer
presses use the shared user `MenuTapMode` rather than an operating-system branch.
Direct touch promotes the desktop guard default to select-then-confirm so a
finger-down may safely become a drag; an explicit `SingleTap` preference remains
unconditional.

A character's ordinary generated portrait reference lives on its stable
character-catalog row. A game-owned override catalog may replace that image or
deliberately force a placeholder; speakers without portrait art use
deterministic monograms.

## Consequences

- Another game can replace the entire dialogue presentation by adding one
  plugin and touching no engine code.
- Engine dialogue/runtime tests do not encode Ambition's colors, portrait shape,
  title convention, or hint copy.
- Ambition portrait development becomes a generated asset plus character-catalog
  reference rather than another UI refactor. Game-specific portrait overrides
  remain presentation data.
- Systems that need the dialogue tree order after `DialogPresentationSet`, not a
  concrete renderer function.
- `DialogOverlayRoot` remains the generic ownership marker used by shell/session
  cleanup tests and pointer-navigation systems.

## Current implications for agents

- Do not add game-specific colors, portraits, layout, or copy to
  `ambition_render::dialog_ui`.
- Extend `DialogView` only with presentation-neutral facts that multiple
  presenters can legitimately consume.
- Add ordinary generated portrait references to `character_catalog.ron`;
  reserve `AmbitionDialogPortraitCatalog` in
  `game/ambition_content/src/presentation/dialog.rs` for presentation overrides,
  alternate art, or deliberate placeholders.
- A demo or downstream game may install `DefaultDialogUiPlugin` or claim the
  presenter seam with its own plugin; never install both.
- Keep choices on `ambition_ui_nav::DialogChoiceSlot` with absolute option
  indices, even when a presenter windows a long list.
- Do not add a renderer-local selection or scroll cursor. Semantic menu input
  mutates `DialogState.selected_option`; presentation derives the visible
  option window from `DialogView.selected_option`.
- Pointer activation must use the shared `UserSettings.controls.menu_tap_mode`;
  do not branch on Android, desktop, or another operating system. Derive the
  drag-safe direct-touch effective policy from the active device, not the OS.
