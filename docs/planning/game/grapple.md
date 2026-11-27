# Grapple

**Status:** asked by Jon 2026-11-27; G1 landed the same day; G2–G4 below, in order. Queue: GRAPPLE.

Jon (2026-11-27): *"make grapple a supported mechanic. The current
implementation if any is broken. Grapple needs to be able to swing, it needs
to be able to grab from a distance, or potentially whip, it needs to be
amenable to making a recovery move in smash — although it might need to be
composed with other abstractions to publish for real in smash, but I think a
lot of the smash things we would want from a grapple are something ambition
can do."*

## What there was

The grapple (`ambition_content_modules::grapple`, slot 13) cast a line along
the aim, and on a wall within 300 px set ONE velocity toward it (620 px/s) and
drew sparks. Nothing held: no rope, no swing, no reel, nothing grabbed, no
release. A pull into the air ended in a fall.

What the engine already had, and what the grapple is built from:

- **The wire** (`movement::integrate_wire_clusters`, `WireState`): a pendulum
  whose position is `(anchor, length, angle)`, swung by gravity and by the
  stick (angular acceleration, fading toward its cap), steered by velocity so
  the sweep stays the authority (a lip stops it), with one release that writes
  the exit velocity (the swing's tangential speed) and a cut that writes none
  (`cut_the_wire`, which a hit already calls). The flyline recovery uses it
  as a timed lift with the anchor straight above. The rope is drawn from the
  body's motion facts (`rendering::flyline`).
- **The tether reel** (`ambition_demo_smash_modules::tether`): a Smash up-B
  that bites the first ledge ahead and reels to it, then lets the ledge
  authority catch.
- **Capture** (`CaptureAttemptRequested` with an authored reach volume): a
  standing tether grab; S0.4 rules out airborne captures, and a tether's air
  move is a hit, not a capture.

## The mechanic

A line is cast along the aim. What it bites decides what it does:

| Bites | Does |
| --- | --- |
| A solid above the body (within the swing's cap) | **Swing**: the body hangs on a rope from the bite. The rope's length is the distance at the bite, and the body's velocity at the bite becomes the swing's angular speed, so a running or falling grab keeps its momentum. The stick swings; up reels in, down pays out (bounded). Jump lets go with the swing's speed plus a hop; a second press of the grapple lets go with the swing's speed alone. A hit cuts it (no velocity of its own). |
| A solid level with or below the body | **Zip**: a pull toward the bite (the old behaviour, kept as the downward/sideways case). |
| An item | **Grab from a distance**: the item is pulled to the hand. |
| A foe | **Whip**: a hit at the line's end, and a small foe is tugged toward the body. |
| Nothing in reach | A miss sound; nothing spent. |

## Stages

- **G1 — the kernel's held wire.** `WireState` gains a held release (no lift
  clock; Jump releases with a hop), a reel (stick up/down within authored
  bounds), and `catch_the_grapple`, which hangs a body from any anchor with its
  length, angle and angular speed taken from where and how fast it is. Core
  tests: momentum carries into the swing and out of it; the reel is bounded;
  the stick pumps a swing higher; a lip stops the body and the rope does not
  fling it; a jump release keeps the swing; a cut writes nothing; the flyline's
  timed wire is unchanged. Rollback schema 552.
  *Landed:* `movement/tests/grapple.rs`. A held rope is damped at 0.12/s,
  not the lift's 2.2/s (which ate 60% of a swing's height in one
  half-period); a jump off it sets `carried_run` to the release's run, as a
  fling does, or the hands-off air stop assist took 60 px/s a tick from it.
  The lip case is the wire's own (`WIRE_MAX_TRACK_SPEED`, the sweep stays the
  authority) and is not re-tested.
- **G2 — the grapple item swings.** A port for a module to hang its body on
  the wire (and to let go); the grapple module swings on a bite above and zips
  otherwise; the rope is drawn while it holds. App witness in a real room and
  a capture of the swing.
- **G3 — grab and whip.** The cast reports the first body or item on the line;
  an item comes to the hand, a foe takes the line's hit.
- **G4 — Smash recovery.** The same wire as a Smash move: a tether that swings
  to a ledge, with the ledge authority catching the body; authored on a
  fighter; the recovery planner knows the route.
