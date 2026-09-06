# Does headless behave like the real simulation?

Jon, 2026-08-19: *"seems important to make sure headless actually behaves like
the real simulation. we can't trust tests unless we fix it. high priority."*

He reached that from a smaller observation, and he was right to distrust the
instrument before distrusting the room: *"there must be sounds in the pirate
sky. they fire their gun swords… I'm just giving you an example of something
obvious the instrument should be able to hear. it points at defect in the
instrument."*

## The answer, measured

`Platformer2dSimHarness` on `pirate_sky_lookout`, 1800 ticks (30 seconds):

```text
HEADLESS schedules            13 non-empty, 797 systems total
the real shell host                            780 systems total
actors in the room            10   (4 flying sharks, 4 pirate riders, 2 parrots)
DORMANT actors                 0   every one awake
actors holding a TARGET     10/10  every one has somebody to shoot
ticks where somebody pressed an attack   4/1800   (4 presses, 8 combatants)
ticks where somebody was mid-move      282/1800
  move-ticks   176 x `ranged`
  move-ticks   128 x `attack_air`
upstream FxRequests            0
ticks with any audio          12/1800
```

⭐⭐ **THE COMPOSITION IS NOT THE PROBLEM.** Headless carries *more* systems
than the shell host a player runs (797 vs 780 — the extra are the harness's own
observation/action seam), it composes the same
`compose_ambition_gameplay_host`, the enemies are awake, un-dormant and
targeted, and they really do fire: `ranged` plays for 176 move-ticks.

⇒ **so "the harness is deaf" was wrong, and so was "the room is quiet".** Two
separate things are true and neither is a composition gap.

## 1. Firing a gun sword makes no sound, in ANY composition

`MoveEventKind::Ranged`'s dispatch arm spawns the shot and writes nothing to
`FxRequest`; the projectile crate emits no cue either. 176 move-ticks of
`ranged` produced **zero** effect requests, and that is not a headless
artifact — the same code runs in the game.

⇒ what Jon hears in the pirate sky cannot be the shots. The candidate that
survives is the IMPACTS: `on_hit` does play a cue, so several pirates hitting
one player is several impact cues, and nothing between `SfxMessage::Play` and
Kira caps or dedupes voices.

⛔ **the shot being silent is itself worth a decision** — a gun that fires
silently is either a content gap or a deliberate choice nobody wrote down.

## 2. The trustworthiness problem is REAL, and it is the AI, not the wiring

**Eight awake combatants pressed an attack four times in thirty seconds.**
That is 0.13 presses/second for a whole room. A behaviour test built on this
harness that concludes "nothing bad happened" in a populated room is largely
observing an idle world — which is exactly the class Jon named.

⭐ and it is the SAME class as the grab defect fixed the same day: the CPU spent
36.5% of a match inside its own grab range without a usable press, because the
brain had no term for what a capture buys and no notion of whether the body
could act at all. This is that shape again, one room over.

⇒ **the honest next step is not an audio fix.** It is to ask why eight awake,
targeted combatants choose to do nothing, using the same route that worked for
the grab: trace the actual decision, name the missing authority, fix the
boundary.

## What NOT to conclude

⛔ do not read the census's green as "the sky is quiet". It is quiet because
almost nothing happens in it.
⛔ do not read 797 > 780 as "headless is a superset". The counts say the
schedules are comparably populated, not that every system is the same one.
