---
status: current
last_verified: 2026-07-18
---

# Input, control authority, prompts, and UI

This page describes the current cross-crate control flow. For durable principles,
read [`../concepts/input-and-game-modes.md`](../concepts/input-and-game-modes.md)
and ADR 0025.

## Runtime flow

1. `ambition_input` maps keyboard/gamepad state into semantic gameplay and menu
   frames. `ambition_touch_input` contributes to the same frames.
2. possession/control relationships choose the controlled subject. Input does
   not assume that the home avatar is always controlled.
3. actor integration builds actor-local control intent.
4. `ambition_characters::action_scheme::derive_action_scheme` derives the action
   scheme from the body's current abilities, moveset, and registered techniques.
   Each reader derives it; no body stores it.
5. the shared slot resolver strips unavailable actions, leaves moveset-owned
   verbs, and reroutes technique slots into keyed edges.
6. movement, interaction, and `MovePlayback` consume the resolved intent.
7. `ambition_sim_view::ControlPrompt` is rebuilt from the same controlled
   subject and action scheme.
8. touch UI, gamepad/dynamic labels, menu adapters, and product UI consume the
   prompt/read model instead of reimplementing capability checks.

## Action slots

The device-shaped slot vocabulary is defined in
`ambition_entity_catalog::action_scheme`. A slot is stable across characters;
its action ID, label, gate, and availability are character/context dependent.
Content techniques may replace a base action on a slot, but must consume the
shared resolver's keyed technique edge rather than intercepting a raw verb.

The standard gamepad uses LT for Shield, RB for Interact, and RT for Dash
(the shared Burst slot). Right stick press toggles the run or stance modifier.
Keyboard and touch modifier controls use hold. View/Select opens inventory.
Menus do not change the toggle. Disconnects and binding changes clear it.
Reset uses keyboard or touch controls. Saved control changes and game layouts
can replace these defaults. Bodies with Dash show Dash for the Burst slot;
bodies that only have Dodge keep that action name.

## Local seats and controllers

Owner: `ambition_input` (`seating.rs`, `local_seats.rs`, `sources.rs`,
`channels.rs`).

- **The pad table is the one answer to "which controller is pad `n`".**
  `LocalDeviceOrder` gives each connected controller a numbered slot, and
  `LocalInputSource::Pad(n)` is slot `n`. A slot keeps its number: a disconnect
  empties the slot and moves no other controller. A controller that comes back
  takes the slot it had (by its entity first, because Bevy keeps the entity of a
  disconnected gamepad, and then by its name and USB identifiers). A new
  controller takes the lowest empty slot.
- **A seat listens to one source.** The frozen `LocalChannelPlan` of a session
  names the source of each channel. Without a plan, `sources::source_for_seat`
  names it from the seat number and the keyboard owner: the keyboard seat is
  the keyboard, and the other seats count their pads from zero. A lobby hand,
  its label and the roster the lobby publishes use this one mapping.
- **`assign_local_seat_devices` projects the source onto the seat's input map**
  and decides nothing else. A seat with no pad gets `NO_PAD`. ⛔ Do not leave a
  seat with no association while pads are connected: leafwing gives such an
  input map the first connected pad, so the seat follows another seat's
  controller.
- **One player with no plan** hears the pad that player uses. The seat follows
  the connected pad that shows input.
- **A frozen session** (`LocalSeatTopology`) keeps the pad table as it was at
  the freeze. A seat hears only the controller that was frozen for it. A
  controller that disconnects leaves its seat with no input, the same controller
  gets the seat back, and a different controller does not get it.
- **Without a frozen session** (a lobby, a menu) the live pad table decides, so
  controllers connect and disconnect freely.

The Smash select screen gives a hand to each source: the keyboard and up to
four pads (`MAX_SELECT_SOURCES`). A hand takes one of the four slots, so four
pads can play with the keyboard in no slot.

Witnesses: `seating::pad_table_tests` and `local_seats::tests` in
`ambition_input` (run with `--features input`),
`smash_in_the_host::a_pad_that_disconnects_on_the_select_screen_moves_no_other_hand`
in `app_it`, and `each_pad_drives_its_own_seat` in `ambition_demo_smash_app`
(run with `--features input`).

## Menus and shell modes

Gameplay control and menu control are separate semantic frames. Shell routing,
pause/home, settings, dialogue, and focused menu verbs may reuse a physical
button, but their authority is selected by explicit mode/focus/scope state.

Current responsibilities:

- `ambition_menu`: renderer-neutral menu/page models.
- `ambition_settings_menu`: settings and system-menu IR.
- `ambition_ui_nav`: list, pointer, focus, and drag navigation helpers.
- `ambition_game_shell`: top-level experience/route/input policy.
- app/render crates: concrete presentation and product-specific menu content.
- dialogue: `ambition_dialog` owns progression, `ambition_sim_view::DialogView`
  publishes raw visible facts, and one provider-selected presenter owns the UI
  tree through `DialogPresentationSet` (ADR 0028).

In an Ambition session, Start opens or closes the unified inventory and
System menu, from gameplay and over any interface: a player can always pause
during a conversation, a cutscene or a terminal (Q184). The keyboard's Start
is Enter, which is also Select: outside the open menu it pauses, and the
opening press confirms nothing underneath; inside the menu it confirms. A
dedicated Start, such as gamepad Start, also closes the menu.

Menu controls are keyed by pad button and do not move with a gameplay layout
(`presets::MENU_BUTTONS`): A confirms, B backs out, the bumpers turn pages,
the D-pad moves. Each keyboard preset binds the key standing in for each
button (`KeyboardPreset::key_for_button`): the Jump key confirms and the Blink
key backs out. The keyboard's own keys sit beside them
(`presets::KEYBOARD_MENU_KEYS`): Enter, numpad Enter and Space confirm, Escape
and Backspace back out, Q/E and Page Up/Down turn pages, the arrows move. A
stand-in wins a key both claim, so no key means two things inside a menu.
Changing a menu control is one row in one of those tables.

A prompt names a control as one `ambition_input::NamedControl`: its word,
its published picture and the control itself, chosen once
(`ActionBindings::named_for`, `confirm_named`, `back_named`). Dialogue boxes,
control prompts, signs and touch buttons read that value, so the word and the
picture cannot name different controls. Every drawn picture carries a
`GlyphPress` (`ambition_render::control_glyphs`): while its control is held
the picture sinks, shrinks and darkens, and it springs back on release. A
keyboard key or pad button presses it from the device; a touch button
presses its own picture.

Back opens the menu only when gameplay owns input or no interface claims it.
A gate, dialogue, or cutscene keeps its own Back policy. A dialogue that
forbids cancel ignores Back. Back inside the menu returns from a System page
before it closes the menu. A Back control shared with gameplay, such as the
default pad's Blink button, retains its gameplay action when no interface is
open.

The open menu captures input above in-session interfaces and below developer
and launcher interfaces. A gate terminal suspends its panel and pointer
input while its participant's menu is open. Its model and simulation state
remain intact. Menu pause restores Playing, Dialogue, or Cutscene after the
last local menu closes. A local menu does not stop a remote shared timeline.

## Touch

`ambition_touch_input` owns the touch HUD lifecycle and folds virtual controls
into the same semantic frames. Button visibility/labels come from
`ControlPrompt`; unavailable actions are masked before they reach simulation.
Touch exclusion zones keep menu drag gestures from becoming gameplay input.

The buttons keep their current action words beside the same published control
pictures used by dialogue and signs. The primary seat's bindings and saved
control changes select each picture. Keyboard and mouse use keycaps. Touch uses
the last active gamepad style for that seat, or Xbox-style pictures before a pad
is used. A keyboard-only seat uses its gamepad binding recipe for these touch
pictures. This reference does not change input ownership. A missing or loading
picture shows the control's text label instead. Menu confirmation and Back use
their menu bindings, rather than the gameplay action in the same touch position.

## Clock contract

There is one body simulation path. Human precision during bullet-time is
represented by `InputState::control_dt` supplied by the input side. Brains leave
that affordance at simulation time. Do not rebuild the retired split between a
player-control phase and a separate player-simulation phase.

## Validation

```bash
./run_tests.sh -p ambition_input
./run_tests.sh -p ambition_touch_input
./run_tests.sh -p ambition_ui_nav
./run_tests.sh -k action_scheme
./run_tests.sh -k control_prompt
./run_tests.sh -k temporary_control
```
