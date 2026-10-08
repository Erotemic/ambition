# UI, localization and accessibility — Engine 1.0 program

**State:** OPEN. Multiplayer/view ownership is urgent. Localization and
accessibility depth grow with product need.

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

### Settings ownership (Q68, 2026-10-03)

The evergreen shell owns how the player interfaces with the program, and every
game inherits it: audio (master, music, SFX), display/window, input bindings,
reusable accessibility, localization. Each game owns its game settings:
difficulty, gameplay modifiers, combat behaviour, camera policy and
mechanics-specific accessibility.

Today the shell-level `UserSettings` (`ambition_persistence::settings`) also
holds the game-owned group `gameplay` (difficulty, assist, player damage,
portal facing). Move that group to the game when the settings admission work
([SETTINGS-ROLLBACK](../queue.md#settings-rollback--finish-the-settingsmechanics-admission-boundary))
is picked up.

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

## Testing menus: who else writes the component

Before you drive a component in a test, ask who else writes it every frame.
`Interaction`, `Visibility`, `Transform` and their kin are engine outputs.

| Harness | Who writes `Interaction` | Writing it yourself |
| --- | --- | --- |
| Minimal crate app (`StatesPlugin` + systems under test) | nobody | the only way to produce the press edge |
| Assembled host (`build_visible_app`) | Bevy's UI focus system, every frame, from live pointer state | overwritten with `None` before any consumer runs |

- **Crate level:** exercise the handler (for example `grid_backend/tests.rs`).
- **App level:** assert the wiring. Is the road installed in this composition,
  do the entities carry their markers, and does the consumer move state when the
  message arrives? `the_shipped_title_screen_is_wired_for_a_pointer` does this
  for the title tab strip (two `Button`s with `BevyUiMenuTab`, the
  `install_bevy_ui_menu_tabs` road, the shell consuming `MenuTabActivated`).
- The press edge itself cannot be exercised headless in the assembled host. A
  human check discriminates: if a tab highlights on hover but does not switch,
  the fault is downstream of the press edge (testable here); if it does not
  highlight, the fault is in picking.

### Hover is a third state (Q70)

Ruling Q70 (2026-10-04, [`../maintainer-decisions.md`](../maintainer-decisions.md)):
pointer-driven menus and settings give normal hover feedback, and hover,
selection and keyboard/controller focus stay three distinguishable states.
This is ordinary UI, not a product question.

Selected means the active value or tab. Focused means the
keyboard/controller cursor. Hovered means the pointer is over the control.

Current shape (flat `bevy_ui` renderer, `ambition_menu/src/render/bevy_ui/`):

- One rule, `hover_lift`, lightens the fill that selection and focus chose.
  A hovered selected control stays teal, a hovered focused control stays
  gold, and a hovered tab keeps its active fill and its focus ring. Hover never
  uses the focus color.
- `sync_bevy_ui_menu_hover` copies `Interaction` into
  `MenuVisualState::hovered`. It is the only writer of that field; hosts write
  `focused` and `selected`. `restyle_bevy_ui_menu_controls` and
  `restyle_bevy_ui_menu_tabs` recolor in place, with no rebuild.
- The launcher does not read `MenuActionPreviewed`. A hover does not move
  `ShellLauncherState::selected`; a click still activates the pointed row.

- The in-game Grid menu has no hover writer of its cursor (2026-10-08): the
  observer that moved `KaleidoscopeCursor` on `Pointer<Over>` is deleted, so
  a hover is drawn by the renderer and a press still activates the row.
  Witnesses: `the_grid_install_observes_no_pointer_hover` (the shipped
  install; control: its press observers are counted) and
  `a_hover_over_a_grid_row_leaves_the_cursor_and_a_press_still_activates_it`.

Tests set `Interaction` directly only in a crate-level harness. In a fully
assembled Bevy host, the UI focus system rewrites `Interaction` from the real
pointer.

## Profile and participant scope in the architecture review

A9 in the [frontier](actor-monolith-work-frontier.md) requires a render/UI-absent
simulation profile. UI can consume participant/view facts without owning control
or requiring HUD state in simulation construction. Two participants, two views
and two live rooms are distinct configurations. Split views by live room exist,
and the music is chosen by authored priority (Q150). The banner follows the
primary seat. The built-in vitals HUD is per participant, also on a shared
view (`ViewHudFacts`, `SharedViewHudFacts`); the declared HUD readouts are
still one per session, and two stacked HUDs do not yet say whose each is (see
[open-world-runtime-and-residency.md](open-world-runtime-and-residency.md)).

Structure semantic labels, diagnostics and action descriptions so machine-facing
authoring and human-facing localized presentation can consume the same supported
contracts. Do not turn a localized/display string into a stable entity, technique
or provider key. Product accessibility choices retain their existing owner.
