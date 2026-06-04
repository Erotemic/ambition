# Grapple pull is under-powered — the 620 burst decays to ~45px (2026-06-04)

Found while authoring a grapple **traversal room** (purposing the Grapple flagship
alongside wall_run/ceiling_cross/portal_bridge/blink_run). The grapple could not
cross even a modest pit, and the trace shows why: the pull is a **one-shot
velocity burst that horizontal drag swallows in ~6 ticks**, so the player only
travels ~45px no matter how far the grappled surface is.

## What the grapple does (code)

`crates/ambition_sandbox/src/grapple.rs` — `Attack` while holding the grapple
casts `raycast_solids` along the aim up to `GRAPPLE_RANGE = 300`; on a hit it sets
the player's velocity toward the hit at `GRAPPLE_PULL_SPEED = 620` (a *burst
impulse* — "collision resolution then settles them at the surface"). The engine
unit test `grapple_yanks_the_player_toward_a_grappled_wall` confirms the burst:
one `app.update()` and `vel.length() ≈ 620`.

## What actually happens in a full sim (trace)

Driving the real `SandboxSim` (player holds grapple — verified via a world query,
`spec.id == "grapple"` — jumps, then grapples a post ~275px to the right):

```
airborne before grapple: pos=(246, 112)
  tick 0: x=254.5   (+8)
  tick 3: x=276.2   (cumulative +30)
  tick 6: x=288.4   (cumulative +42)
  tick 9: x=291.4   (stopped; now just falling)
```

The 620 burst decays to ~0 horizontal in ~6 ticks (~0.1s), total travel **~45px**.
Then the player simply falls. (Grounded it's even worse — input-driven ground
movement zeroes the burst almost immediately, so the grapple does nothing on the
floor; this is an *airborne-only* mechanic, and even airborne it's ~45px.)

## Why this is (probably) a bug

The targeting range is 300px and the pull speed is 620px/s, which read as "reel
yourself to a wall up to 300px away." But the one-shot burst is killed by drag
after ~45px, so the player **never reaches** any wall beyond ~45px. ~45px is also
well under a running jump (~150px), so the ability adds no traversal value as
shipped — you can target a far wall but you barely move toward it.

Two coherent designs; the code currently sits between them:
- **Reel-in (likely intent):** apply a sustained velocity toward the hit each tick
  until arrival (or a rope constraint), so a 300px grapple actually pulls you
  300px. This makes it the traversal tool the range/speed advertise.
- **Tiny burst assist (current behavior):** if ~45px is intended, the 300px range
  and 620 speed are misleading and should be retuned down so they don't imply
  reach the ability doesn't have.

## Recommendation

Non-autonomous feel/balance pass (this is a design decision, not a mechanical
fix). Until then, **do not author grapple-traversal rooms** — the mechanic can't
cross gaps. The other four flagship rooms this run (vector gravity ×2, portal,
blink) are unaffected; the grapple-room attempt was reverted cleanly.
