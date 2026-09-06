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

## ◐ The title screen's pointer road: what is VERIFIED, what is RULED OUT, and the one link a headless repo cannot reach (2026-09-06)

Jon reported the Settings tab unreachable by click or tap, then reported it still
bugged after the first fix. This records the investigation so nobody repeats it.

✔ **FIXED AND VERIFIED IN THE ASSEMBLED HOST** (`the_shipped_title_screen_is_wired_for_a_pointer`):
* the strip is drawn as **two real `Button`s** carrying `BevyUiMenuTab`;
* the **tab road is installed** — this was the actual defect:
  `install_bevy_ui_menu_tabs` had exactly ONE caller in the workspace and it was
  the kaleidoscope menu, so `publish_bevy_ui_menu_tabs` was never registered on
  this screen and the buttons reached no system;
* the shell **consumes** the renderer's `MenuTabActivated` and moves the strip.

⛔ **RULED OUT BY MEASUREMENT, each a plausible story that is false:**
| hypothesis | measurement |
|---|---|
| the pointer system is not registered | `basic_shell_pointer` is added `.after(BevyUiMenuInteractionSet)` |
| that set has a run condition that excludes the title screen | no `configure_sets`/`run_if` names it anywhere |
| the shell composition never builds | `ambition_platformer2d/basic_shell_presentation` IS in `ambition_app`'s default feature closure |
| a node occludes the tabs | menu root is `GlobalZIndex(1000)`, the shell's other node is 900 |
| picking is blocked on the tab tree | the only `Pickable::IGNORE` in the renderer is a scrollbar thumb |
| the UI picking BACKEND is absent | `bevy_ui_menu = ["bevy/ui_picking"]` and `ambition_platformer2d/bevy_ui_menu` is in the default closure |
| **the fix is on the WRONG SURFACE** — the shipped title screen is the kaleidoscope menu, not the shell launcher | ⭐ the most dangerous of the six, and false: `rendered_app()` builds `ambition_app::app::build_visible_app(VisibleRenderMode::NoWindow, true)` — the SHIPPED visible-app builder, windowless. The fixture is the real composition, so the two tab buttons it finds are the ones a player sees |

⚠ **THE ONE LINK THIS REPO CANNOT EXERCISE: the press EDGE.** Bevy's UI focus
system **recomputes `Interaction` every frame from live pointer state**, so a
headless test that writes `Interaction::Pressed` has it overwritten with `None`
before any consumer runs — measured directly (`DIAG tab has Button=true
Interaction=Some(None)`). ⇒ An earlier version of that test reported "the tab strip
is drawn as buttons that nothing listens to" **while the shipped chain was fine**,
and it took one `eprintln!` of the component being written to see it.
⭐ **A test that writes a component the engine OWNS is asserting against its own
write, not against the system under test.** Same family as a test that constructs
its subject.

⇒ **The next reader's cheapest discriminator is a HUMAN one**: does the Settings
tab change appearance on hover? The active tab draws filled gold, inactive dark
blue. Highlight-but-no-switch puts the fault downstream of the press edge, where
this repo can test; no highlight at all puts it upstream, in picking, where it
cannot.

### ⭐⭐ WHY THE SAME TECHNIQUE IS SOUND AT CRATE LEVEL AND UNSOUND AT APP LEVEL — read this before writing a menu test

`grid_backend/tests.rs` drives a tap by writing `Interaction::Pressed` then
`Interaction::Hovered`, and it is **correct**. I wrote the identical thing against
the assembled host and it was **wrong**. The difference is not the code — it is
which harness is running.

| harness | who writes `Interaction` | writing it yourself |
|---|---|---|
| minimal crate app (`StatesPlugin` + the systems under test) | nobody | ✔ the only way to produce the edge |
| assembled host (`build_visible_app`) | **Bevy's UI focus system, every frame, from live pointer state** | ⛔ overwritten with `None` before any consumer runs |

⇒ **So the two levels can only assert different things, and pretending otherwise
produces a confident false negative.** Mine reported *"the tab strip is drawn as
buttons that nothing listens to"* while the shipped chain was fine.

* **Crate level** — exercise the HANDLER. A minimal app has no competing writer, so
  a written `Interaction` is a real input and the system under test sees it.
* **App level** — assert the WIRING. Is the road installed in this composition, do
  the entities carry their markers, does the consumer move state when the message
  arrives? These are the links that actually go missing: the shipped defect here
  was an INSTALL with one caller in the whole workspace.

⚠ **The general rule, worth more than the table: before driving a component in a
test, ask who else writes it every frame.** `Interaction`, `Visibility`,
`Transform` and their kin are engine outputs, not test inputs. Writing one tests
your own assignment.
