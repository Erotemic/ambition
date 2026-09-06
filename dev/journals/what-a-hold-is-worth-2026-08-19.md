# What a hold is worth — D166's policy half, measured

Jon ruled on 2026-08-19: **do the policy half first.** He also named the class
before seeing any evidence — *"the problem is the AI isn't spacing their grab
right? Sounds like an AI problem, not a grab problem."* He was right, and the
authored geometry was never touched.

## What landed

`capture_value(foe)` in the fighter brain's own option scorer — not in the
shared frame data, so no carve was needed. A capture deals no damage, so
`max_damage` is honestly zero and `expected_payoff` is zero with it; the scorer
had no term for the only thing a capture buys, which is that the opponent is
HELD.

Every term is zero or small unless a specific fact is true right now:

- a **raised guard** — the third leg of the triangle `rollout.rs` already wrote
  down. L3's rollout has known "grab beats shield" since a shielding opponent
  made the whole kit worth zero; L2 did not, and L2 answers whenever L3 names
  nothing.
- the **percent** a throw converts against.
- an explicit **zero for hitstun** — the case a naive "they cannot answer, so
  grab" rule scores highest, and the case where spending the grab's startup
  trades a live combo for a hold.
- an explicit **zero for an airborne body** — `acquire_captures` skips any
  victim not on the ground, so that grab can never catch anybody.

It is a feature of its own rather than a value routed through
`expected_payoff`, and the reason was found before it was written:
`expected_payoff` is gated by `frame_advantage`, which is measured against
`is_punishable()`, which excludes `Shielding`. Routing a hold's worth through
that gate would have deleted it in exactly the situation a grab exists for.

## The measurement

`capture_probe -- 60`, George vs Alice, three runs on the same tree:

```text
                        before   +policy   +airborne
grab presses                 7        85          54
  inside the 42px reach      0        35          14
median press distance     ~110px     48px        67px
grabs started                1        22          14
attempts requested           3        66          42
holds established            0         1           1
pummels / throws           0/0       1/1         1/1
```

The whole chain — grab, hold, pummel, forward throw — fires end to end in a CPU
match for the first time.

## ⛔⛔ The correction that matters, because it changes which fix is next

The queue row and my own first report both said the presses are **spent while
committed** — 7 of 7, then 53 of 54. That reading is contaminated by the
instrument, and the queue's own caveat says why: *the sample is taken AFTER
`app.update()`, so the tick a grab STARTS reads as "mid-`grab`"*.

Counted properly on the third run:

```text
54 presses
  20 read "mid-`grab`"     of which 14 ARE the 14 grabs that started
   1 reads "body FREE"
  33 read mid-`smash_*`/`tilt_*`/`air_*`   <- the genuinely wasted ones
```

⇒ **the brain does press grab from a free body — fifteen times — and gets one
hold.** "Its presses are all spent while committed" was the wrong conclusion
from the right number, and it pointed at a start-gate that would have bought
almost nothing.

## What is actually left

**Thirteen of fourteen grabs whiff.** The median press is at 67px against a
42px reach, so most grabs are thrown from outside their own range and the body
cannot close the gap during a four-frame startup. That is not a policy defect —
`capture_value` is a function of the opponent, not of distance, and should stay
that way — it is the `REACH_TOLERANCE` question the queue already flagged as a
maintainer's call:

```text
REACH_TOLERANCE = 2.0   offers every move out to 3x its own reach.
                        For a poke that is fine: a swing thrown slightly long
                        still threatens and still trades.
                        For a GRAB there is no trade. It catches or it whiffs.
```

⇒ the open question is whether tolerance should be derived from what the body
can close during the move's own startup (principled, and it changes how every
CPU in every game this engine runs spaces itself), or whether a capture simply
gets a tighter one because it has no trade outcome.

⚠ **and a caution on comparing these runs**: the matches diverge completely
after the first behavioural change, so "median press distance went 48 -> 67"
is not evidence the airborne term hurt spacing. What IS comparable is the
outcome pair — same holds, a third fewer presses and attempts.

## Closing state, re-measured after the whole day's work

```text
                        before   +policy   +airborne   +legality   final
grab presses                 7        85          54           9       9
  wasted mid-smash           6        34          33           0       0
grabs started                1        22          14           9       9
attempts requested           3        66          42          27      27
holds established            0         1           1           2       2
pummels / throws           0/0       1/1         1/1         2/2     2/2
time spent held              —         —           —           —    1.3s
```

⭐ **nine presses, nine grabs, nothing spent.** Every press the brain issues now
becomes a move, and the chain completes twice in sixty seconds — grab, hold,
pummel, forward throw.

⚠ **the probe still prints "9 of 9 while COMMITTED", and that is the instrument,
not the brain.** The sample is taken after `app.update()`, so the tick a grab
STARTS reads as mid-`grab`. Nine presses started nine grabs, so all nine of those
lines are the artifact — which is exactly the reading that was wrong the first
time and is worth not re-deriving.

⛔ **`REACH_TOLERANCE` was never touched.** The spacing corrected itself once a
grab had a reason to be thrown at the right moment instead of being the last
option standing at long range. That was the row's own thesis, and it held.

⚠ **what is NOT solved**: two holds in a match is a start, not a target. 27
attempts still produce 2 holds, so most grabs still close on a body that has
moved by the time the active window opens. That is the `REACH_TOLERANCE`
question the queue flags as a maintainer's call — a grab has no trade outcome,
so it may deserve a tighter tolerance than a poke — and it is deliberately still
open rather than tuned away.
