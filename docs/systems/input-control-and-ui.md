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
4. `ambition_characters::action_scheme` derives `ActorActionScheme` from the
   body's current abilities, moveset, and registered techniques.
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

## Touch

`ambition_touch_input` owns the touch HUD lifecycle and folds virtual controls
into the same semantic frames. Button visibility/labels come from
`ControlPrompt`; unavailable actions are masked before they reach simulation.
Touch exclusion zones keep menu drag gestures from becoming gameplay input.

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
