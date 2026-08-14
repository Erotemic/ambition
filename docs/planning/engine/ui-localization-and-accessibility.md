# UI, localization and accessibility — Engine 1.0 program

**State:** OPEN — multiplayer/view ownership is urgent; localization/accessibility depth can grow with product need.

## Goal

Make UI an explicit participant/view-aware engine surface rather than a set of
single-screen assumptions spread across menus and HUD systems.

Existing crates such as `ambition_ui_nav`, `ambition_menu`,
`ambition_inventory_ui`, `ambition_settings_menu` and game-shell UI provide a
strong base.

## Program areas

- participant-scoped focus and input ownership;
- view-scoped HUD/presentation for split-screen;
- shared/global versus per-participant menus;
- controller/touch/mouse modality and prompts;
- safe areas and adaptive layouts;
- text/UI scaling and accessibility metadata;
- localization/pluralization and eventual RTL/layout support when needed;
- agent-native inspection of active UI/focus state.

## Candidate crate / Bevy ecosystem value

`ambition_ui_nav` is a plausible general Bevy plugin candidate if it can remain
independent of Ambition game modes. Other UI crates may be product-specific
compositions over a reusable navigation/focus core.

Follow Bevy UI rather than building a parallel widget framework unless a concrete
requirement proves Bevy UI insufficient.

## Open design questions — deliberately unresolved

- How does focus work when two local participants operate different views and
  one opens a menu?
- Which menus pause the whole simulation versus only one participant's control?
- What is the localization source format and who owns string identity?
- Which accessibility features are mandatory for the first shippable Ambition
  release?
- How should dialogue/UI text be scoped across shared and split views?
- Which UI functionality is generic enough for an ecosystem crate?
