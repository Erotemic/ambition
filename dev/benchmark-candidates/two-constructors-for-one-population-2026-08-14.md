# Two constructors for one population, and the reset is the one that lies

**Tags:** `fork-detection`, `lifecycle`, `silent-degradation`, `agent-verification`

## The shape

A population has a CONSTRUCTOR and a RESET. The constructor sets a fact
directly; the reset RE-DERIVES it from a proxy. They agree for as long as the
proxy happens to be honest, and then one of them is wrong on exactly the frame a
player is watching.

```text
construction   flight.fly_enabled = ActorTuning::is_aerial        -> true
reset          flight.fly_enabled = fly && !fly_toggle            -> false
```

⇒ **the diagnostic question is not "which field did we forget".** It is *"is
there a second thing that builds this population, and does it compute anything
the first one was told?"*

## ⭐ The tell is free, and a maintainer usually hands it to you

> *"Leaving the room and re-entering restores its movement."*

That sentence means **the other constructor produces a different body.** Any
symptom repaired by a room transition, a reload, a respawn, or a re-entry is
this shape until proven otherwise. Stop looking for the missing field and go diff
the two construction paths.

## The 2026-08-14 instance

A boss revived by the same-room replay stood perfectly still while its brain sat
in `Approach` commanding `velocity_target = (-141.75, 0)` every tick. Nothing was
dead: the brain decided correctly, the body was in every query, its ground state
and motion model matched a fresh boss exactly. Only the flight limb reads a
commanded velocity, and the reset had grounded it.

The boss's kit declared `fly_toggle: true` — *"the toggled kind a boss has always
had"* — and nothing has ever toggled a boss. The fix was to stop the body lying
about itself, not to add `fly_enabled` to the reset's ledger.

⚠ **the same function already documented this class, three lines above the one
that bit us**, for `base_size`: *"A reset restores the body to its BASE size; it
does not redefine what the base IS."* A doc block naming your bug is a
dependency, and this one had already paid for the lesson once.

## Why the ledger is never the fix

`reset_ecs_room_features` is a hand-kept reconstruction ledger — pickups, chests,
breakables, actor spawn state, dispositions, aggression, pinned poses, boss
health/phase/brain-cursor/control/anim, hazards, switches, encounter entities.
Every row is a fact somebody noticed was missing. Nothing enumerates what a
constructed room actually contains, so nothing can notice the next missing row:
the list only ever grows, one bug report at a time, and each addition postpones
the next report by exactly one bug.

## How to pin it

⛔ **not with a component checklist** — that is the ledger again, written as a
test. Compare BEHAVIOUR between a freshly constructed instance and a reset one:

```text
fresh:     does it wake?   does it leave its spawn?
replayed:  does it wake?   does it leave its spawn?
```

Whatever the two constructors disagree about, that fails — including the
disagreement nobody has found yet.

⚠ and measure DISPLACEMENT from spawn, not per-frame travel: a contact chaser
closes its gap and then holds, so a per-frame sum only sees motion when the
window happens to straddle the approach. The first version of this test measured
the fixture's timing rather than the boss.

## Related

- [`a-capability-with-no-adopters-2026-08-09.md`](a-capability-with-no-adopters-2026-08-09.md)
  — the sibling shape where the second path exists and nobody uses it.
- [`the-comment-asserts-what-the-code-does-not-2026-08-09.md`](the-comment-asserts-what-the-code-does-not-2026-08-09.md)
  — the cut-rope comment claimed the replay "despawns + respawns the boss"; it
  never has, and that sentence is why nobody looked here.
