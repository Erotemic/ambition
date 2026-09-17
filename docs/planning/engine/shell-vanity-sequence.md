# Title launcher fade-in — remaining work

Status: **OPEN — VC5 only**

> **Re-checked 2026-09-17, and against `008b44120` (2026-09-02) before that:
> NOTHING HAS CHANGED, VC5 is still open.** No launcher content-alpha ramp
> exists — the only presentation fade in the shell is the vanity CARD's own
> (`crates/ambition_game_shell/src/basic_presentation.rs`), and the other `fade`
> hits in the workspace are audio tweens, the nameplate rank opacity, the
> kaleidoscope menu's own `KaleidoscopeFade`, and damage-label/dizzy-star eases.
> The "Verified landed" list below still describes the code.
>
> ⛔⛤ **AND THERE IS A SECOND CONSUMER-LESS FADE, WHICH WHOEVER TAKES VC5 SHOULD
> SEE BEFORE "reuse/generalize existing shell presentation fade machinery" SENDS
> THEM LOOKING FOR IT.** `CutsceneBeat::Fade` advances its timer and draws
> nothing — `CutscenePresentation::fade_alpha` has zero consumers outside its own
> crate, and neither does `camera_target` beside it. Both are labelled UNFINISHED
> at the definition, which is honest; what the labels did not say is that
> **shipped content authors one**: `test_intro` holds
> `Fade { to_alpha: 0.0, seconds: 0.8 }` and is bound to `central_hub_main`, so
> the first entry to the hub spends 0.8 s on a beat that draws nothing.
> `CameraPan` is authored only in tests. ⇒ VC5 and the cutscene fade are the same
> missing thing at two layers, and a screen-alpha consumer built for one is the
> obvious owner for the other — which is an argument for building it once, not an
> instruction to widen this card.

The original shell vanity-sequence campaign is complete except for the title
launcher fade-in. VC1–VC4 and VC6 are implemented. The full campaign history is
archived at
`docs/archive/planning-superseded/2026-08-13/engine/shell-vanity-sequence.md` — <!-- cite-ok: removed from the checkout 2026-09-05; naming the path is the point -->
removed from the checkout 2026-09-05, still in git history.

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
