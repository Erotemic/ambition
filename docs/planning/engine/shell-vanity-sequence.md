# Title launcher fade-in

**State:** OPEN. VC5 only. The rest of the shell vanity-sequence campaign
(VC1-VC4, VC6) is implemented.

## Current shape

Implemented: the timed image-sequence model and playback, the committed
vanity-card manifest and export path, host composition of the real startup
card, per-frame missing-asset degradation, and pointer/touch activation for
shared menus. Do not rebuild these.

No launcher content-alpha ramp exists. The only shell presentation fade is the
vanity card's own (`crates/ambition_game_shell/src/basic_presentation.rs`).

A screen-alpha consumer exists for cutscenes. `CutsceneBeat::Fade` has
`from_alpha`, `presentation()` interpolates over `elapsed`, and the render layer
draws a black sheet at `ZIndex(49)` (guards:
`a_fade_ramps_from_its_authored_start_to_its_authored_target`,
`a_fade_beat_draws_a_black_sheet_at_the_ramp_value`). VC5 can share it. The
launcher fade knows both ends of its ramp (transparent to authored opacity).

## Work: fade title content in

Implement one reusable fade-in path for the launcher presentation. Do not make
it a special-case animation owned by the Ambition game.

- Fade the launcher content from transparent to its authored opacity.
- Keep the launcher's opaque backdrop opaque.
- Cover text, images, borders and other menu presentation descendants through
  one presentation mechanism.
- Keep keyboard, controller, pointer and touch input working during and after
  the fade.
- Reuse or generalize the existing shell presentation fade. Do not add a second
  menu-animation authority.

## Acceptance

The normal startup route hands off to the title launcher, and the launcher
content fades in. Direct start is unchanged. Input works during and after the
fade. The implementation belongs to reusable shell/menu presentation, not to
named Ambition content. When this lands, delete this page.
