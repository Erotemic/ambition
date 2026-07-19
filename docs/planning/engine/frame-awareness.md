# Small Manifesto: Frame Awareness

> **Status: Jon's design position (2026-07-05), captured verbatim.** The third
> binding manifesto, beside [`spatial-model.md`](spatial-model.md) (space) and
> the relativity principle it generalizes. Adjudicated into working discipline
> as **AJ13** in the archived 07-05 plan
> ([`../../archive/reviews/fable-demo-plan-2026-07-05.md`](../../archive/reviews/fable-demo-plan-2026-07-05.md));
> the live queue is [`../tracks.md`](../tracks.md). Like ADR 0020: do not
> deviate without raising an explicit challenge Jon accepts.

Frame awareness is an architectural bias before it is a runtime subsystem.

Ambition does not need to simulate full relativistic spacetime. It does need
to stop pretending that every meaningful relationship happens in one global
x/y frame. Bodies move relative to surfaces. Portals transform space. Moving
platforms carry local motion. Cameras observe from a presentation frame.
Controlled bodies interpret intent through their own capabilities. These are
not special cases; they are signs that the engine needs a coherent language
of frames.

The world frame may remain the default. AABB collision may remain the fast
path. Most rooms may remain simple, rectangular, and cheap. But the engine
should treat that simplicity as a specialization, not as the ontology of
space.

The core question should become:

```text
relative to what?
```

A contact is relative to a surface frame. A jump is relative to a body and
support frame. A portal crossing is a transform between frames. A moving
platform is not just a block with velocity; it is a support frame in motion.
A camera is not the world; it is an observer.

We should not build a grand frame graph before we need it. We should not
infect every system with abstract machinery too early. But we should write
APIs, docs, and mental models that leave room for local frames to emerge
naturally.

The design rule is simple:

```text
Use the world frame by default.
Do not make the world frame sacred.
```

Frame awareness lets slopes, loops, moving platforms, angled portals,
possession, surface locomotion, and future relativity-inspired mechanics
belong to one elegant model instead of becoming a pile of hacks.

Ambition should grow toward an engine where bodies, surfaces, portals, rooms,
and cameras know how they relate to each other.

Not because the game must be physically realistic.

Because the game should be architecturally honest.
