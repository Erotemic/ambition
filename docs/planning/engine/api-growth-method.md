# How the API campaign produces its next slice

**This document is a procedure, not a plan.** It does not say what slice 2 is.
It says how to *derive* slice 2 from what slice 1 measured, and how to know when
the campaign is over.

That separation exists because of a measured failure. The last large
architecture document here
([architecture-campaign-2026-07-28.md](../architecture-campaign-2026-07-28.md))
is eight days old and opens with a SUPERSEDED banner: its *reasoning* survived,
every *status claim* rotted within a week. A document that predicts five slices
is a document whose last four are wrong. A document that describes how to
compute the next one stays true as long as the method does.

**Read with:** [api-1.0-campaign.md](api-1.0-campaign.md) (slice 1),
[ADR 0031](../../adr/0031-public-facade-is-the-compatibility-boundary.md),
[ADR 0032](../../adr/0032-authoring-is-declarative.md).

---

## 1. The induction, in one line

> Each slice closes the leak that the previous slice measured as most expensive,
> and is not allowed to end until it has produced the measurements that select
> the next one.

The base case is slice 1. The step is §3. The terminal condition is §4.

The engine grows by **closing leaks**, where a leak is:

> a rule the engine knows and makes the consumer re-derive.

That definition is not invented for this document. It is the shape of the two
leaks this repository has already closed and recorded — `ShellComposition`
(seven ordered steps, two silently omissible) and `drive_control_frame` (two
resources, wrong one silently ignored). Both were found by the same instrument.
Both got the same fix: state the rule once.

---

## 2. The four evidence sources

A slice is not complete until all four have been collected. They are cheap; the
discipline is collecting them *before* deciding what is next, so the decision is
made from evidence rather than from whatever seems interesting.

### 2a. The contract diff

`scripts/check_absence_contracts.py` failure output, before and after.

* **Which forbidden paths does the consumer still name, and how many times?**
  Frequency is the crude cost proxy and it is usually right: seven uses of
  `ambition::actors::features` is a bigger leak than one use of
  `ambition::time`.
* A path that appears in *more than one* consumer is worth more than its count
  suggests — it is a rule multiple people re-derived independently.

### 2b. The fixture leak log

Outlander's source comments, in the established format (`LEAK CLOSED <date>`,
with the finding quoted verbatim before the fix).

This is the highest-quality source because each entry is a *sentence about what
the consumer had to know*, not a symbol count. Keep writing them. A leak closed
without its finding recorded is a lesson that has to be relearned.

New entries since the last slice are candidate work. Entries that were closed
tell you the method is working.

### 2c. The blind agent run

Fresh context, fixed script, `docs/sdk/` and `ambition::prelude` only.

The recorded fields are:

* did each task complete;
* **which engine file did it open first**;
* did validation catch the deliberately broken reference.

The first-file-opened field is the one that matters. It names the next leak the
way the fixture comments do, but from the population the API is *for*.

⚠ It must be a **fresh** agent. An agent resumed from a session that touched
engine internals measures its own memory. This is the single easiest way to get
a falsely green result, and the result is falsely green in the direction that
feels good.

### 2d. The deletion criteria

ADR 0032 lists them. Each names a compensating mechanism that should become
unnecessary:

* the `PreStartup` character-preparation backstop is deletable;
* provider plugin ordering no longer determines content completeness;
* repeated `App::finish()` cannot republish or alter prepared content;
* headless and visible hosts consume the same prepared-content fingerprint;
* Sanic standalone and embedded produce the same content and schema identities;
* no runtime character consumer reads a fallback authoring catalog.

**A criterion that did NOT become deletable is the most valuable single signal
this method produces.** It means a seam was added beside the old mechanism
rather than taking ownership from it — which is rule 1's violation, caught
mechanically. Investigate that before anything else on the list.

---

## 3. Selecting slice N+1

### 3a. Rank the candidates

Every leak from §2 gets three properties:

| Property | Question |
|---|---|
| **Cost** | How many consumers × how many times × does it fail *silently*? |
| **Closeable** | Can it be closed by adding a seam, without moving code between crates? |
| **Owned** | Is the rule the engine's to state, or is it a genuine product decision? |

**Silent failure triples the cost.** A leak that panics teaches; a leak that
falls back quietly does not. This repository's most expensive defects have all
been the quiet kind — a declared image indistinguishable from an unskinned bolt,
a peaceful wanderer swinging the protagonist's sword, an authored hurtbox
published into a component no damage path read.

### 3b. Route by the answers

```text
Closeable + Owned      -> slice N+1 candidate. Rank by cost.
Closeable + NOT owned  -> awaiting-maintainer-decision.md. Do not guess a product rule.
NOT closeable + Owned  -> decomposition evidence. §4.
NOT closeable + NOT owned -> both of the above; the decision comes first.
```

### 3c. Size the slice

One slice = **one leak closed end to end**, including migration and deletion. If
a candidate cannot be closed with its consumers migrated and the old path
deleted inside the slice, it is too big — split it by consumer, not by layer.

Splitting by layer produces "the new thing exists, migration next slice", which
is the parallel-paths failure with a schedule attached.

### 3d. Write the slice

Copy the shape of [api-1.0-campaign.md](api-1.0-campaign.md): numbered rows,
each with an acceptance test that is a test; a red-first contract where one
applies; explicit exit criteria; an explicit NOT-in-this-slice list.

**Every slice re-runs §2 at its end.** A slice that closes a leak and collects
no evidence has broken the induction — there is nothing to select from next.

---

## 4. The terminal condition, and what it authorises

The campaign is not open-ended. It ends in one of two ways, and the second is
the interesting one.

**Ordinary end:** the contract is green, the blind agent run completes without
opening an engine file, and the remaining leaks are all *not owned* — product
decisions, not engine rules. The API is done; write the doctrine document then,
derived (ADR 0031, Alternatives).

**The interesting end:** the highest-cost remaining leak is **NOT closeable
without moving code between crates.**

That is not a failure. It is the whole point, and it is the condition
[decomposition.md](decomposition.md) already specified for reopening its own
settled ruling:

> This ruling does not protect misplaced named content or prevent a later split
> that **a real second consumer demonstrates**.

When that condition fires, the decomposition is authorised — and it is
authorised *with an argument*, which is what it has never had before:

* the leak names the boundary (it is the seam a consumer could not be given);
* the contract diff names which paths it covers;
* the fixture log names what a consumer had to know;
* and the blind agent run names what an author had to learn.

**Design the carve from that, not from the module list.** A split derived from
today's internal topology fits today's internal topology; a split derived from a
leak fits a consumer.

⚠ Do not let a single leak authorise a full decomposition. Carve exactly the
boundary the leak names, migrate its consumers, delete the displaced path, guard
the absence — then return to §2. A decomposition is a slice like any other; it
just moves files.

---

## 5. What disqualifies a slice

Learned here, each more than once:

* **It ended with two paths alive.** Not a slice. Rule 1.
* **Its acceptance test is prose.** "Reads cleanly", "approximately N lines",
  "no longer requires". Name a test.
* **Its test was never seen red.** Then its subject is unverified. Three goal
  checks passed for the wrong reason before this rule existed.
* **It collected no evidence.** The induction is broken; slice N+2 will be
  chosen by taste.
* **It was selected because it was interesting.** The ranking in §3a exists to
  overrule that, and the person most convinced a row is important is usually the
  person who just finished the previous one.
* **It closed a leak nobody measured.** A leak with no consumer naming it is a
  guess about a future consumer. Guesses go in a queue row, not a slice.

---

## 6. Why this converges

Each slice strictly reduces one of:

* the count of forbidden paths a consumer names (§2a);
* the count of open fixture findings (§2b);
* the count of engine files a blind author must open (§2c);
* the count of undeleted compensating mechanisms (§2d).

All four are non-negative integers, all four are measured every slice, and no
slice may end without reducing at least one. If a slice reduces none, it did not
close a leak — and that is itself the most informative outcome available,
because it means the leak was misidentified and §3a's ranking needs the
correction more than the code does.
