# Title launcher fade-in — remaining work

Status: **OPEN — VC5 only**

> **Re-checked against `008b44120` (2026-09-02): NOTHING HAD CHANGED, VC5 is still
> open.** No launcher content-alpha ramp exists — the only presentation fade in
> the shell is the vanity CARD's own (`basic_presentation.rs`), and the other
> `fade_*` hits in the workspace are audio tweens and the nameplate rank
> opacity. The "Verified landed" list below still describes the code.

The original shell vanity-sequence campaign is complete except for the title
launcher fade-in. VC1–VC4 and VC6 are implemented. The full campaign history is
archived at
[`docs/archive/planning-superseded/2026-08-13/engine/shell-vanity-sequence.md`](../../archive/planning-superseded/2026-08-13/engine/shell-vanity-sequence.md).

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
