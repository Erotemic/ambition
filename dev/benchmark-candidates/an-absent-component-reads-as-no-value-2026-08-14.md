# An absent component reads as "no value", so widening a population makes latent rules live

**Tags:** `fork-detection`, `presentation`, `silent-degradation`, `agent-verification`

## The shape

A component is published for a NARROW population. Every consumer joins on it
with `Option<&T>` and a sensible fallback. The fallback is written for *"this
entity has no value yet"* and is silently also serving *"this entity was never in
the population"* — two different facts that arrive identically.

```text
Option<&PresentedPose>  ->  None
                            ├─ a body's first frame        (fallback correct)
                            └─ every boss and actor alive  (fallback is a LIE)
```

⇒ **before widening a population, enumerate its consumers and ask what each one's
absence arm currently MEANS.** Expect a red test on the widening commit, and read
it as the population working rather than the widening being wrong.

## The 2026-08-14 instance

`PresentedPose` followed `BodyPoseView`, whose rebuild is filtered
`With<PlayerVisual>`. Four consumers, three of them silently degraded and one
silently *safe*:

| consumer | what a non-player body got |
|---|---|
| combat overlay | strike drawn on the TICK clock beside a player's on the frame clock |
| unauthored attack stand-in | skipped entirely, and its warn named a population that could never be there |
| slash visual | miss arm — the drawn blade never followed a boss |
| camera resolve | `pos = presented.presented()` **looked correct** because it never fired |

Widening the query to `BodyKinematics` — where the pose view copied `pos`/`vel`
from verbatim, so the existing population was unchanged to the bit — fixed the
first three and made the fourth live and WRONG: for a framed cast, `pos` is the
pair's centre and the sample comes from one anchor seat, so assigning pointed the
camera at seat 0. A two-CPU smash test went red on the same commit.

⭐ **two of the three degraded consumers carried a comment asserting the
population was identical**, because the rebuild *"requires only
`BodyKinematics`"* — true of its OPTIONAL facts and false of its filter. One
wrong claim, copied twice, hid three defects.

## The repair that generalises

Carry a rigidly-attached value by a **DELTA**, never by replacing it with an
absolute position:

```text
delta = presented - authoritative

followed body's own pose:   pos + delta == presented      (identical)
a framing centre:           pos + delta != presented      (only delta is right)
```

The two rules agree exactly when the value IS the body's own pose, which is why
the absolute form survives review — and diverge silently the moment it is not.
The same delta then covers every row of one body in one frame: collision
envelope, hurtboxes, body-anchored strikes, the readout hanging off the box.

⛔ **translating a subset does not fix a shudder, it relocates it.** The first
attempt moved only body-anchored strikes; the maintainer watched the red box
settle and a different box start jumping.

## Related

- [`a-capability-with-no-adopters-2026-08-09.md`](a-capability-with-no-adopters-2026-08-09.md)
- [`the-comment-asserts-what-the-code-does-not-2026-08-09.md`](the-comment-asserts-what-the-code-does-not-2026-08-09.md)
