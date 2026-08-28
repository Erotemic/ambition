# Three poisons that passed, and what each one was actually testing

D249 recorded that the shark up-B shipped broken in play four times while the
suite stayed green, and named the discipline that worked: **probe the fix out and
demand red.** This is that discipline applied all day and failing three times in
the same way, with the commits so a reader can check rather than take it.

⚠ This is EVIDENCE, not a rule. The three cases below are real and named; the
sentence I would draw from them is at the bottom and is exactly the part I have
been unreliable at — see "What I got wrong the same day".

## The three

### 1. A nullable field, poisoned at the wrong layer — `085208345`

`JobResult.executed_seconds` became `float | None` so a nextest job's unmeasured
duration would stop being persisted as a real zero. Poisoned by forcing the
default back to `0.0`; the test failed; shipped.

**What it was actually testing:** `timings_payload`, the per-job JSON. Three
aggregates still summed `r.executed_seconds or 0.0` — the cost-ledger row, the
status payload, the human report — and `compile_report.py` derived
`build_seconds = seconds - executed_seconds`. So a 100s run was still persisted
as *0s executing, 100s building*. A review caught it, not the poison.

### 2. A menu arm, poisoned with one row — `9699d19c7`

The destructive-confirm arm was keyed by `format!("{action:?}")`; replacing that
with `Action` removed a real defect (debugging text as semantic state). Poisoned
by tapping one row twice; the test failed; shipped.

**What it was actually testing:** one row's own guard. Two DIFFERENT rows
carrying an equal action still armed each other — arm *Quit to Desktop* on one,
tap the other, and it fires on the first tap. The fix is `MenuFocusKey`, the
identity the menu already had.

### 3. A projectile fixture, 40px apart — `57551b453`

The victim loop `break`s on its first qualifying row and iterated a Bevy query,
so which of two overlapping bodies took the damage was archetype order — on
rollback-authoritative state. Ordered by geometry; poisoned by removing the sort.

**The poison PASSED.** The two bodies were 40px apart and a fireball at 50px/s
covers 0.8px in a tick, so only one was ever reachable and the loop had nothing
to arbitrate. At 8px apart the poison fails and reports the FAR body taking a
shot that passed through the near one — so the ordering defect was live, not
theoretical.

## A fourth, of a different shape — `789970ef7`

`EditableMovementTuning` rebuilt `parry_timing` from `default()` on every
projection, deleting a stage's declared ruleset on any unrelated slider. The test
declared `ParryTiming::OnRaise` — which **is** the default — so poisoning the
projection back to `default()` left it green. It declares `OnRelease` now.

⇒ a fixture that declares a DEFAULT value cannot straddle a rule about that
value. Same family as `dev/benchmark-candidates/the-comment-asserts-what-the-code-does-not-2026-08-09.md`.

## What I got wrong the same day

Recorded because it is the reason to treat the sentence below as a hypothesis.

- I told Jon that re-cropping sprite sheets would not resize anybody. True for
  the 107 characters on the `standing_height` road and **false** for the 27 on
  the legacy one, where `collision = body_px × ldtk_max × collision_scale /
  FRAME_H` and the frame is a divisor (`a33deb440`).
- I put this lesson in `docs/reviewer-guide.md`, which is not mine to edit. Jon
  reverted it (`2a64e8aee`).

## The sentence, offered rather than asserted

**Poison the CONSUMER of what you changed, not the place it is produced.** All
three misses were a poison aimed at the layer the fix touched: the payload rather
than the ledger that reads it, one row rather than two rows sharing a key, one
reachable body rather than two.

⛔ I have not tested this against cases where it would be wrong, which is the
same gap it describes.
