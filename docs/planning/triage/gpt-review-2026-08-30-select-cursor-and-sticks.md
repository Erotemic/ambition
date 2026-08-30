# GPT review 2026-08-30 — the select cursor, and a Switch Pro that cannot Smash

Two reports. The first is a feel bug with a fully code-level explanation; the
second is a hardware hypothesis with a very specific, testable prediction.

Jon's instruction: **the stick fix, plus the Pro Controller diagnostic so the
laptop can be tested.** Rows 8–9 are therefore a measurement to run, not a change
to make.

| # | Row | State |
|---|-----|-------|
| 1 | Split analog pointer input from digital menu navigation | ▣ landed |
| 2 | Analog select-cursor movement is continuous ONLY | ▣ landed |
| 3 | Nonlinear magnitude response | ▣ landed |
| 4 | Speed scaled from portrait geometry, not viewport width | ▣ landed |
| 5 | Clamp the cursor's integration `dt` | ▣ landed |
| 6 | Regression tests for precision and screen aspect | ▣ landed |
| 7 | Gamepad-axis diagnostic | ▣ landed — `Shift+F6` |
| 8 | Compare the Switch Pro endpoint on both machines | ☐ **Jon runs this** |
| 9 | Outer-stick saturation, IF the endpoint is the cause | ☐ blocked on 8 |

---

## ▣ 1–6 — the cursor was three bugs, not one constant

`CURSOR_SPEED_PER_SECOND = 1.15` of the **viewport's width**:

1. **Far too fast.** ~2200 px/s at 1920 wide. A full-stick sweep crossed the
   whole screen in under a second — that is not a hand, it is a thrown object.
2. **WIDTH FOR BOTH AXES.** A vertical sweep crossed 1080px at the horizontal
   rate: **0.49s down against 0.87s across**, so vertical was intrinsically 1.78×
   more sensitive — and the ratio changed with the window.
3. **The wrong unit.** What the player aims at is a PORTRAIT. A speed that does
   not know how big a portrait is gets worse every time the roster or the window
   changes size.

⛔⛔ **And under all three, two semantics fighting over one device.**
`MenuControlFrame::nav` summed the analog stick with the HELD d-pad and arrow
keys into one vector. A direction EDGE implies the same direction is held on that
very frame, so a stick flick fired the snap AND then roamed away from the
portrait it had just landed on, on the next frame, at 2200 px/s. *"You flick
toward something, it lands there, and then immediately shoots away unless you
release precisely."*

⭐ **Reordering the branches could not have fixed that**, and this file's history
shows somebody trying: the previous repair put the edge first, which made the
snap reachable and left the roam-away intact. Both halves are true on the same
frame for one device, so the fix has to be at the PRODUCER. `nav` is now
`analog` — the stick, post-deadzone, screen space, and nothing else. The d-pad
and arrow keys already speak through `up`/`down`/`left`/`right` with repeat,
which is the whole vocabulary a list navigator needs.

The screen then states the rule plainly: **a hand on the stick is a pointer and
never snaps; everything else walks the grid target to target.** The stick wins
when both are live, because snapping under a moving hand is the fight this
screen was losing.

### The velocity model

`cursor_travel(analog, cell, dt)`:

- **`CURSOR_CELLS_PER_SECOND = 4.0`** — one scalar, both axes, derived from the
  portrait cell. Aspect-independent by construction, scales with the grid rather
  than the screen, and it is a number a designer can reason about. Measured at
  1920×1080 with 8 fighters: **1004 px/s at full deflection, against 2208 before.**
- **Magnitude SQUARED.** A half push is a quarter speed, a fifth push is a
  twenty-fifth. A linear stick has to choose between "too slow to cross" and "too
  twitchy to place"; a curve does not — which is why the repair is a curve and
  not a smaller constant.
- **The curve bends speed, not direction.** Curving the components independently
  would turn a 45° push into something else: a steering bug wearing a feel change.
- **`MAX_CURSOR_DT = 1/30`.** One 100 ms hitch would otherwise travel 0.4 of a
  cell in a frame the player never saw.
- **No sticky-target magnetism**, per the review. A good curve gives enough
  precision on its own, and magnetism makes the cursor fight the player near
  anything selectable — the exact complaint that started this.

⚠ Cells are 0.86 as wide as they are tall, so a vertical cell takes ~16% longer
to cross than a horizontal one. That is the honest consequence of one uniform
pixel rate, and a great deal smaller than the 78% it replaces.

### Tests, and one that had to be rewritten

Seven arms on the pure model — same distance up as across, same seconds-per-cell
on 16:9 and 4:3, quarter speed at half deflection, direction preserved, the dt
clamp, rest, and a guard that the new rate is far under the old one.

⛔ **`a_flick_snaps_even_though_the_same_direction_is_held` was DELETED, not
fixed.** Its premise — "a real device never sends a direction EDGE with an idle
deflection" — was true only because the producer merged the two, and it is the
thing that got fixed. It is replaced by the pair that states the new contract:
`a_d_pad_edge_snaps_to_the_next_portrait` (edge, `analog` at rest) and
`a_stick_flick_does_not_snap_even_though_it_also_fires_a_direction_edge` — the
reported bug, asserted directly.

smash 127/127, app 192 lib + 509 it, 36/36 contracts.

---

## ▣ 7 — the gamepad-axis probe (`Shift+F6`)

The review's arithmetic is **confirmed by test**, not by reading. A Smash flick
needs `AttackGestureTuning::flick_threshold = 0.8` applied AFTER the inner
deadzone:

```text
post = (raw - deadzone) / (1 - deadzone)
```

A Switch Pro classifies as `GamepadStyle::Switch` → `ControllerProfileId::Generic`
→ the baseline **0.18**. So:

| gesture | post-deadzone | RAW needed |
|---------|---------------|-----------|
| Smash flick | 0.80 | **0.836** |
| directional tilt | 0.50 | **0.590** |

A pad topping out near 0.80 raw therefore runs, tilts, and drives menus
perfectly while Smash attacks **cannot exist**. That is a very specific
prediction and it is what the probe tests.

`a_switch_pro_needs_a_far_bigger_raw_push_for_a_smash_than_for_a_tilt` pins both
numbers; `the_raw_requirement_round_trips_through_the_real_deadzone_transform`
pins the inverse against `apply_deadzone` itself, so a hand-derived formula
cannot drift and make every verdict confidently wrong; and
`a_probe_states_the_same_thresholds_the_gesture_uses` keeps the overlay's mirrored
constants honest against the gesture's own.

⭐ **It reads Bevy's `Gamepad` directly**, not the action layer — the question is
about the DEVICE, and bindings/seats/action state would put four more suspects
between the stick and the number.

⚠ **The peak is the measurement.** Nobody can hold a stick at its maximum and
read a screen at the same time, so every row peak-holds; toggling resets, and
toggling off logs the readout.

The overlay prints a verdict rather than two numbers and an expectation that the
reader does arithmetic mid-push:

```text
OK — this pad reaches a Smash flick
SMASH UNREACHABLE — tilts work, flicks cannot fire on this pad/host
the stick is not reaching a tilt either — check the pad is the one being read
```

---

## ☐ 8–9 — what Jon runs, and what it decides

1. Launch on **both** machines, `Shift+F6`, push the Switch Pro fully left,
   right, up and down, read `PEAK`.
2. If the laptop's peak RAW is **under ~0.836** and the desktop's is not, the
   mystery is solved. Corroborate outside Ambition with `jstest` /
   `evdev-joystick`: a calibrated stick should approach ±32767.

**If confirmed, the fix is an OUTER saturation stage at the shared input seam** —
inner deadzone → usable range → outer threshold at which the stick reads 1.0 —
so `0.8` means the same gesture on every pad and every host.

⛔ **Not a weaker Smash threshold.** Lowering `flick_threshold` to suit one host
changes the mechanic for everyone to compensate for a normalisation bug, and the
outer value should come from measurement rather than from picking a
Nintendo-specific number now.

⚠ There is already a `StrongAttack` semantic input the Smash gamepad layout does
not bind. A C-stick/strong-attack binding is a genre-appropriate second way to
Smash and worth having eventually — but it is not a substitute for making
left-stick flick detection hardware-independent, and it is not in scope here.
