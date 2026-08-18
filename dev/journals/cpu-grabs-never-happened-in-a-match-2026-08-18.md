# The grab worked everywhere except in a match — 2026-08-18

**Time lost: about two hours, all of it in the right direction.** Nothing here
was a wrong turn; each step ate one candidate. Written down because the SHAPE
repeats: a mechanic can be correct at every layer and still never occur, and
every layer's own test will stay green while that is true.

## The claim under test

Capture landed end to end: acquisition, the hold, the pummel, the throw, the
escape clock, and a CPU policy for both ends of a hold. Unit guards green,
acceptance chain green. The one question none of them asks: **does it happen in
a fight nobody arranged?**

`capture_probe` (`game/ambition_demo_smash_app/src/bin/capture_probe.rs`) drives
the real demo — select, seat two CPUs, route to the stage, step 60s — and
watches the relationship table.

## What it said, and what each answer cost

```text
run 1   holds 0                        the mechanic does not occur
run 2   holds 0   Grab pressed 0×      the brain never even asks
run 3   grab=None verbs=11             the BODY has no grab to press
```

⛔ **Only George authored a grab.** The default seats are the shared STAND-IN
table (`fighter_moveset()`), which authors eleven attack verbs and no capture
kit. A rock-paper-scissors triangle with one leg on one character is not the
game — so the stand-ins got a grab, a pummel and a forward throw of their own.

```text
run 4   Grab pressed 11×, grab played 3×, attempts 9, holds 0
```

⚠ **and now the interesting failure.** The chain worked: press → move → effect →
typed request → acquisition. Acquisition declined all nine. The probe dumped the
eligibility predicate's own terms at each attempt — both bodies complete
participants, both grounded, hostile to each other — and the distance:

```text
attempt #1  seat 0 at x=315   seat 1 at x=408     93px apart
attempt #2  ...                                   89px
attempt #3  ...                                   84px
```

**The grab reaches 42px.** It was not declined; it whiffed. And the fighters
spend **35% of the match inside 42px** — so the range was there and the grab was
never thrown in it.

## The cause was mine, one commit earlier

`capture_candidate` priced a grab at its FORWARD THROW's damage — *what is
catching somebody worth, if not the throw?* The rollout therefore saw a
9-damage option against a 3-damage jab, predicted the gap closing, and picked
the grab from 110px every time.

⇒ **a grab deals no damage.** `max_damage` is what a move does on CONTACT, and
the honest number is zero. Reverted; the long-range grabs went with it.

## What this exposed, which is worth more than the fix

The value of a grab is that the opponent is **held** — and that depends on the
throw it sets up, the escape risk, the captive's percent and the stage. The
generic option scorer has no term for it, and **should not grow one**: it is
shared by every actor in every game the engine runs, and "how valuable is a
hold" is platform-fighter policy. Recorded on queue **D166** as the customer
that row was waiting for.

## The lessons, in the order they generalise

1. ⛔⛔ **A mechanic's tests can all be green while the mechanic cannot happen.**
   Every layer tested its own hop. Nobody asked whether the bodies in a real
   match had the move at all.
2. ⛔ **"Only one character authored it" is a content bug that reads as an
   engine bug.** The first three runs looked like AI, eligibility and
   acquisition problems in turn.
3. ⭐ **Instrument the SPLIT, not the outcome.** "Zero holds" has five causes
   that are indistinguishable from the relationship table. Counting presses,
   moves started and typed requests localised it in one run each.
4. ⛔⛔ **A payoff number that reads as obviously right can invert a behaviour.**
   Pricing the grab at its throw made the CPU grab exclusively at ranges it
   could not reach — the opposite of the mechanic — and every unit test stayed
   green.
5. ⭐ **When the honest number is zero, take the zero.** The missing value is a
   capability's to express, and inventing it in the generic layer would have
   bought a plausible number and a wrong game.
