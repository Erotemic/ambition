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
module. Its product policy is a classic fully opaque bottom panel with a speaker
nameplate, portrait frame, body, choices, and a footer whose width is bounded by
the panel. Dedicated portrait images are registered by stable character id;
unregistered speakers use deterministic monogram placeholders.

## Consequences

- Another game can replace the entire dialogue presentation by adding one
  plugin and touching no engine code.
- Engine dialogue/runtime tests do not encode Ambition's colors, portrait shape,
  title convention, or hint copy.
- Ambition portrait development becomes data registration rather than another
  UI refactor.
- Systems that need the dialogue tree order after `DialogPresentationSet`, not a
  concrete renderer function.
- `DialogOverlayRoot` remains the generic ownership marker used by shell/session
  cleanup tests and pointer-navigation systems.

## Current implications for agents

- Do not add game-specific colors, portraits, layout, or copy to
  `ambition_render::dialog_ui`.
- Extend `DialogView` only with presentation-neutral facts that multiple
  presenters can legitimately consume.
- Add named Ambition portrait mappings and layout changes under
  `game/ambition_content/src/presentation/dialog.rs`.
- A demo or downstream game may install `DefaultDialogUiPlugin` or claim the
  presenter seam with its own plugin; never install both.
- Keep choices on `ambition_ui_nav::DialogChoiceSlot` so pointer/touch and
  keyboard/gamepad navigation remain one input path.
