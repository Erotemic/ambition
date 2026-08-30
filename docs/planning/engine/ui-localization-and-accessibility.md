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

## Triggered localization/accessibility backlog

The broad presentation/shell audit is closed. Its surviving product gaps belong
here rather than in a second audit plan.

### Localization trigger

There is no translation catalog/runtime locale system yet. Build one when the
first non-English shipping target or another concrete translated-UI/dialogue
consumer appears. Keep authored IDs language-independent, resolve display text at
the presentation boundary, and report missing keys with provider/source
provenance. Do not create an i18n framework only because the old audit named the
absence.

### Accessibility gaps

Current remaining capability gaps are:

- make the colorblind setting drive a real presentation/palette transform;
- add user-controlled text/UI scaling when a shipping target requires it;
- integrate the Bevy accessibility tree/screen-reader path when non-visual menu
  navigation has a target;
- add captions/subtitles for non-dialogue audio cues when required.

Treat each as a presentation capability with a real acceptance case rather than
building a parallel UI stack.

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
