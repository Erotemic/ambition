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
portal facing). Move that group to the game (`Q68`). Nothing blocks the move:
the admission road the group feeds is built
([SETTINGS-ROLLBACK](../queue.md#settings-rollback--a-settings-change-reaches-simulation-only-at-an-admitted-rebase---done-2026-10-08)).

### Launcher theme, quality and provider composition

**What landed (2026-10-12), the quality half:** with the parallax budget
off, the launcher's backdrop goes at once (it returned and left the old
backdrop); a scene records the image folder of the tier it was made at, and
a scene of another tier is not the wanted scene, so the new tier's scene
fades in over it. Witness: `ambition_app title_screen
the_backdrop_follows_the_quality_while_the_launcher_is_up`; poisons (no
despawn, folder not compared) fail at their own assertions.

**What landed (2026-10-12), the provider half:** a provider names the look
of its launcher row. `ExperienceRegistration::launcher_look` is a semantic
key (a theme name), which `PlatformerExperienceAuthoring::with_launcher_look`
sets and `ShellLaunchEntry::look` carries; the shell does not read it. The
host's title screen resolves it with `ParallaxTheme::from_key`, and a row
with no look, or a look that is no theme, gets the hub. The route-name
prefixes in `scene_of` are deleted. Sanic names `open_sky`, Mary-O
`starlight`, Smash and Versus `eclipse`.

- Witness: `ambition_app title_screen a_provider_names_the_look_of_its_row`.
  A route name the host has never seen gets the look it names; the controls
  are no look, an unknown look, and the Exit row.
- `app_it shell_host_lifecycle::each_game_of_the_launcher_names_its_own_look`:
  the shipped launcher's games each have their own look.
- Poisons: a resolver that ignores the look fails at "the look it names";
  an authoring that does not pass the look on fails the shipped-launcher
  test with every row at the hub.

The text below is the case as found.

**October 11 real customer:** `game/ambition_app/src/app/title_screen.rs`.
The new host title screen chooses `ParallaxTheme` from route string
prefixes (`scene_of`, lines 79-86), even though `ShellExperienceRegistry`
contains independently registered providers. A provider with a new route
name therefore falls back to `Hub` unless this one host source is edited.
That is acceptable as a default, **not** as the only way to customize a
provider's launch look.

**Composable presentation contract:** a provider can declare optional
launcher-art intent as part of its visual/presentation registration, using
stable semantic keys or a small descriptor that the launcher presentation
adapter resolves. No game-specific route prefix appears in the reusable
shell. Keep theme asset choice, fallback and quality budget in the host's
presentation composition; never force the headless shell registry to import
the `ParallaxTheme` renderer enum or to require game assets. An absent
visual provider gives a readable, keyboard-accessible default launch row.
If this needs a new API, start with one existing provider and one
independent fixture; do not design an extensible title-screen scene graph.

**Immediate lifecycle correction:** `show_the_scene_of_the_cursor` returns
early if `budget.parallax.enabled` is false but does not despawn a prior
`TitleBackdrop`; `take_the_backdrop_away` currently only runs when the
launcher exits. Once a theme is present, disabling the quality budget can
leave it visible and resident indefinitely. Changing the parallax resolution
scale while the route/theme stays the same also leaves prior layer handles;
`TitleScene` is keyed only by theme. Define one visual-generation key that
includes effective quality and theme, and reconcile the active scenes and
scrim/window styling on both disabling and re-enabling, not only on the
first render. Do not let a visual budget affect shell routing, focus or
input admission.

**Witnesses:** start title with background, disable parallax without leaving
launcher, observe zero `TitleBackdrop` roots and appropriate original UI
contrast; re-enable, see precisely one rebuilt scene; switch quality
resolution, see the new-tier assets; register an unrelated-route provider,
see its declared look without editing the host mapping; run the shell in
headless/asset-absent mode. Poison: keep the `if !enabled { return; }`
without cleanup, or key active scenes only by `ParallaxTheme`.

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

**Shown text is pack data (2026-10-18).** The trigger here was not a
translation: a reworded banner cost a Rust build. Each game's lines are now a
pack source with a schema from `ambition_content_pack::shown_text`
(`shown_text_schema::<K>`). The game owns the string identity: it declares
each key and the holes the key fills (`TextKeys`), and admission refuses a
missing line, an unknown line and an unknown hole, each with the file and the
key named. Keys name lines, not words, so a later catalog of languages states
the same keys; there is one language and no locale switch. Each game is
done: Smash, Mary-O, Sanic, Versus (`data/<game>_text.ron`) and Ambition's
launcher row (`data/ambition_text.ron`). A host's words are the host's: the
start cover and the pause menu's own rows are `host_text` in `ambition_app`'s
own pack (`assets/host/pack.ron`), admitted as every game's pack is. Each
setting's label and description are still the settings crate's Rust.
Each owner states its own words, so no crate reads another's text to show
its own surface.

**The engine's own words stay Rust for now (decided 2026-10-18).** The
settings crate's labels and descriptions, the engine's "Talk" and "HP" and
the cutscene hint are engine chrome, not a game's content, and their crates
sit below `ambition_platformer2d`, so they cannot read a pack. Moving them
needs either a new data road at that level or every host stating words for
every setting; each makes composing a new host harder (Jon, 2026-10-18: no
new coupling between crates unless no other elegant design exists). Q68
gives them to the evergreen shell; build that shell text file when the
localization trigger above fires, with the road chosen then.

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
- What is the localization source format? (Who owns string identity is
  answered above: the game that shows the line declares its key.)
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
