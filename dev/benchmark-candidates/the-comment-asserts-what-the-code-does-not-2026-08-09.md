# The comment asserts a behaviour the code does not have

**Tags:** `stale-documentation`, `diagnostic-method`, `silent-degradation`,
`agent-verification`, `fork-detection`

## The shape

A comment states what the code **does**. It is confident, specific, and often
explains *why*. The code stopped doing it — or never did.

⛔ **and the comment is load-bearing while it is wrong**, because it is what stops
the next reader from checking. A missing comment invites a look; a wrong one
forecloses it.

```text
"It falls through to the ordinary face resolution below,
 which is what produces the head contact the bonk reads."
                              ↑
        an `else if` chain. It does not fall through.
        The mechanic had been dead for as long as the comment existed.
```

## Five in one session (2026-08-09)

| the comment | the code | what it cost |
|---|---|---|
| *"falls through to the ordinary face resolution below"* | an `else if` arm — chains do not fall through | ⛔ **a whole game mechanic dead**: an invisible brick that never triggered, reported by the maintainer and diagnosed as an art bug first |
| *"the one place that can stamp what it actually loaded"* | all three callers hand it the request | the stale-quality bug the surrounding row existed to fix, one layer down |
| *"there is no `Pickup` cue in the shared vocabulary yet"* | there is; it was added later | a coin block played the **brick-smash** sound |
| *"The same world, started in `entry`"* | branches on one room id, else builds room 1-1 | no room but the first could be captured — **three open rows blocked on it** |
| *"reveals itself by being used"* | the reveal path deletes the picture | a hidden block that pays out invisibly |

⭐ **four of the five were found by reading the comment and then checking it**,
not by hitting the bug. The fifth (the `else if`) was found by a probe that
asserted the effect rather than the appearance.

## ⛔ Why this class is worse than ordinary staleness

- **The comment is usually RIGHT when written.** Every one of these was true at
  its commit. *"There is no Pickup cue yet"* was a fact; the cue arrived later and
  the sentence stayed. ⇒ **you cannot find these by looking for careless
  comments** — they were written by someone who understood the code.
- **It reads as an explanation, so it ends the investigation.** A reader hunting
  a missing head-contact finds a comment that explains where head contacts come
  from and moves on. The comment is doing the opposite of its job with the
  author's full authority behind it.
- **It survives the change that falsifies it**, because nothing compiles a
  sentence. ⇒ the moment of drift is invisible in review: the diff shows the code
  change, and the comment sits ten lines up, unmodified and now false.

## The tells, ranked by yield

⭐ **1. A comment asserting CONTROL FLOW.** *"falls through"*, *"runs before"*,
*"is called from"*, *"the caller filters these"*. Control flow is exactly what a
later edit reorders, and the language will not check the claim. **Highest yield
of the five.**

⭐ **2. A comment asserting ABSENCE.** *"there is no X yet"*, *"nothing else
does this"*, *"the only place that"*. Absence is the claim most likely to expire,
because the repository grows monotonically. ⇒ **grep for X before believing it.**

**3. A comment restating the function's CONTRACT in general terms** while the
body handles cases. *"The same world, started in `entry`"* over a two-arm match
is a summary of intent, not of behaviour.

**4. A citation** — *"mirrors X"*, *"same shape as Y"*, *"see Z"*. Citations go
stale silently when the cited thing moves, and they also **declare a fork**: two
places doing one job, which is worth checking on its own.

## How to apply

⭐ **When a comment tells you what you were about to go and check — check it
anyway.** That is the entire method, and it is cheap: each of these took one grep
or one read of ten lines.

⚠ **Especially when the comment is what makes you stop looking.** The feeling of
*"ah, that explains it"* is the signal, not the resolution.

⭐ **When you fix one, ask what ELSE the comment was holding up.** The `else if`
comment had persuaded at least one previous reader; the *"no Pickup cue"* comment
had authorised a wrong sound for months.

⛔ **and do not just delete the wrong sentence.** It records an intent somebody
had. Say what the code does now *and* that it used to do the other thing — a
deleted comment teaches nobody, and the next person re-derives the confusion.

## Related

* [`the-odd-one-out-among-siblings-2026-08-09.md`](the-odd-one-out-among-siblings-2026-08-09.md)
  — in five of its seven instances the correct sibling carried a comment stating
  the rule. **Comments are the highest-yield place to look in both directions**:
  right ones point at the defect, wrong ones hide it.
* [`a-capability-with-no-adopters-2026-08-09.md`](a-capability-with-no-adopters-2026-08-09.md)
  — the absence-tell above (*"there is no X yet"*) is how an unadopted capability
  stays unadopted.
* [`one-question-two-checkers-only-the-first-runs-2026-08-08.md`](one-question-two-checkers-only-the-first-runs-2026-08-08.md)
  — a comment claiming a fix that the unreachable copy never delivered.
