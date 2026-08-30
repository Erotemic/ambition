# Awaiting a maintainer decision

Only questions whose next step is **Jon's product/authoring judgement** belong
here. Engineering work goes to [`queue.md`](queue.md) or [`tracks.md`](tracks.md).
Answered rulings belong in [`maintainer-decisions.md`](maintainer-decisions.md);
the investigation that led to an answered question remains available in git
history.

This file intentionally does not retain answered decision transcripts.

## Open decisions

### 37. Should the F9 rollback proof pulse survive a gameplay-session change?

`LocalSessionPolicy::check_distance` is raised by the F9 proof pulse and returns
to normal only when that pulse finishes. If the player quits to title during the
pulse and launches another game, the elevated verification distance currently
survives.

The recent session-ownership fix deliberately did **not** decide this because the
value is developer tuning rather than gameplay authority.

Choose one policy:

- **session-scoped:** a new gameplay session always starts with the ordinary
  check distance; or
- **process-scoped developer intent:** the proof pulse deliberately spans a
  relaunch until it completes/cancels.

This is primarily a developer-iteration/expectation decision. The gameplay
rollback authority itself is already session-owned by ADR 0027.

### 36. What are the authored standing heights of the puppy slug, stochastic parrot and burning flying shark?

These are the remaining characters whose old size derivation cannot be replaced
by preserving one existing placement size because their authored spawn boxes
disagree substantially across rooms.

Representative placement variation:

```text
npc_puppy_slug            (48,22), (32,48), (64,32), (48,32),
                          (64,16), (52,66), (42,42), (28,44)
stochastic_parrot         three different boxes
npc_burning_flying_shark  mostly (108,96), also (32,48)
```

The needed value is one character-authored `standing_height` in world units for
each. Do not choose by majority box size: the box was editor/layout data, not a
stature authority.

Decision 32 still applies: there is no standard adult/humanoid height and no
bulk normalization. Character stature is authored individually.

Related visual followups that should be judged by playtest rather than another
population average: the cove pirates relative to Robot v3, slop size, and the
Mary-O snake's post-rescale size.

### 35. What should own fighter reach during move startup?

The current fighter brain uses one global `REACH_TOLERANCE = 2.0`, effectively
allowing a move to remain viable out to roughly three times its authored reach.
The bug that exposed this proxy is fixed; the constant is not currently known to
cause a product defect.

The design choices are:

1. keep the proxy until platform-fighter option ranking has its own capability
   boundary;
2. add a per-move tolerance field;
3. derive reachable distance from move startup plus the body's movement
   capability, which requires threading capability/top-speed information into
   perception;
4. for moves with authored startup impulse/travel, derive that part directly
   from the move and keep a fallback for ordinary movement.

**Default if no change is desired:** leave it. Do not widen generic actor data for
a proxy that is not currently hurting play merely to close a planning row.

### 34. Should external/launch-owned motion become an explicit cross-game fact?

Three shared movement decisions have historically inferred "this velocity belongs
to a launch rather than locomotion" from speed magnitude: initial-dash settling,
shield braking and body-contact resistance. The approximation fails when a launch
has decayed below ordinary run speed.

Smash already has a live tumble mechanic and therefore a genre-specific fact that
can represent external/launch-owned motion. Ambition does not necessarily want
Smash tumble semantics for ordinary bodies.

The decision is whether to:

- keep the current thresholds until a visible defect requires more;
- introduce a generic carried/external-motion ownership fact with Smash tumble as
  one producer; or
- let the platform-fighter capability own the richer rule while the shared kernel
  keeps the simpler behavior for other games.

Do not solve this by simply reading Smash's `tumble_speed` from the generic
kernel; the question is exactly whether that game-specific semantic is shared.

### 33. How should a recharging ranged weapon communicate that it is unavailable?

The firing cadence is implemented: `BodyMelee::ranged_cooldown` follows the
weapon's authored refire interval, and an early press is refused before spending
the proposer so ordinary combat buffering can retry when the weapon becomes
ready. The unresolved part is presentation.

Choose the product channel when this becomes important in play:

- character/muzzle VFX driven by recharge fraction;
- a presentation treatment on the firing limb/body; or
- a HUD indicator.

The mechanic does not need to block architecture work while the unavailable
state is merely invisible rather than incorrect. Prefer character-local
presentation if it reads clearly; do not add another gameplay authority to show
the cooldown.

### 38. Does an actor released in a foreign room stay there?

Today an actor moved away from its authored home and then left in another room is
retired when that room unloads and is authored again at home when encountered
later. The current construction road honestly refuses to claim the actor was
persistently relocated.

Two valid product policies:

- **go home:** authored home placement is restored when the actor is no longer
  live/resident;
- **stay where left:** persist a `Placed`/relocation occurrence for actors as is
  already done for relevant item occurrences, and teach reconstruction to honor
  it.

If choosing “stay,” the producer and reconstruction consumer must land together;
recording a moved placement that construction refuses would only add warnings and
still teleport the actor home.

This decision feeds
[`engine/construction-and-reconstitution.md`](engine/construction-and-reconstitution.md)
and [`engine/open-world-runtime-and-residency.md`](engine/open-world-runtime-and-residency.md).

### 39. Which authored move, if any, should adopt the dormant windbox/armor vocabulary?

The windbox mechanic is implemented and can express outward gust or inward
suction. `WindowTag::Armor` also exists but has no shipped authored customer.
There is no engine defect merely because the vocabulary is currently unused.

If one should become product-visible, name a fighter/move. Otherwise leave the
mechanism dormant until a character design asks for it. Do not invent a customer
to make an adoption count nonzero.

## Waiting on maintainer measurement, not a decision

### Switch Pro outer stick range

The remaining cross-machine controller question needs the actual hardware
measurement: run the existing `Shift+F6` axis probe on both machines, push the
Switch Pro to each extreme/corner, and compare reported peak magnitude.

The proposed shared outer-saturation fix should be judged only after that number
exists. This is tracked in the execution queue as an external measurement, not a
maintainer design decision.
