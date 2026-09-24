# Title launcher fade-in — remaining work

Status: **OPEN — VC5 only**

State on 2026-09-19: VC5 is open. No launcher content-alpha ramp exists; the
only shell presentation fade is the vanity card's own
(`crates/ambition_game_shell/src/basic_presentation.rs`).

A screen-alpha consumer now exists for cutscenes: `CutsceneBeat::Fade` has
`from_alpha`, `presentation()` interpolates over `elapsed`, and the render layer
draws a black sheet at `ZIndex(49)` (guards:
`a_fade_ramps_from_its_authored_start_to_its_authored_target`,
`a_fade_beat_draws_a_black_sheet_at_the_ramp_value`). VC5 can share that
consumer. VC5 does not need the `Q143` ruling, because the launcher fade knows
both ends of its ramp: transparent to authored opacity.

The original shell vanity-sequence campaign is complete except for the title
launcher fade-in. VC1–VC4 and VC6 are implemented. Git history has the full campaign
record.

## Verified landed

Current source contains the timed image-sequence model and playback machinery,
the committed vanity-card manifest/export path, host composition of the real
startup card, per-frame missing-asset degradation, and pointer/touch activation
for shared menus. Do not rebuild those pieces as part of this task.

## Remaining task — fade title content in

The title launcher still appears without the intended content alpha ramp.
Implement one reusable fade-in path for the launcher presentation rather than a
special-case animation owned by the Ambition game.

Requirements:

- fade the launcher content from transparent to its authored opacity;
- keep the launcher's opaque backdrop opaque rather than fading it with the
  foreground content;
- cover text, images, borders, and other ordinary menu presentation descendants
  through one coherent presentation mechanism;
- preserve keyboard, controller, pointer, and touch interaction while the fade is
  running and after it completes;
- reuse/generalize existing shell presentation fade machinery when that produces
  one cleaner ownership path; do not add a second competing menu-animation
  authority merely to close this card.

## Acceptance

The normal startup route hands off to the title launcher and the launcher content
fades in cleanly. Direct-start behavior remains unchanged, input continues to
work during/after the transition, and the implementation belongs to reusable
shell/menu presentation rather than named Ambition content.

After this lands, archive this residual plan; the vanity-sequence campaign has
no other open work.
