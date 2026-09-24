# Small Manifesto: Frame Awareness

> **Status: Jon's design position (2026-07-05), captured verbatim.** The third
> binding manifesto, beside [`../../architecture/spatial-model.md`](../../architecture/spatial-model.md) (space) and
> the relativity principle it generalizes. Adjudicated into working discipline
> as **AJ13** in the 2026-07-05 plan (in Git history);
> the live queue is [`../tracks.md`](../tracks.md). Like ADR 0020: do not
> deviate without raising an explicit challenge Jon accepts.

**Implementation state (checked 2026-09-17).** The frame vocabulary exists in
`crates/ambition_geometry/src/reference_frame.rs`. No general frame graph
exists, as this page asks.

| manifesto phrase | type that carries it |
|---|---|
| *"contact is relative to a surface frame"*, *"a jump is relative to a body and support frame"* | `LocalAxes`, `MotionFrame`, `ResolvedControlFrame` |
| *"relative to what?"* | `GameplayFramePolicy` (`ControlledBodyLocal`, `AccelerationFrame`, `WorldSpace`, `ScreenSpace`); rollback-registered |
| *"a moving platform is a support frame in motion"* | `AccelerationFrame`, from net down-defining acceleration, not snapped to a cardinal axis |
| *"a camera is not the world; it is an observer"* | `CameraReferenceFrame` (`WorldFixed`, `SubjectFrame`), a per-view player setting |

No production site sets `GameplayFramePolicy::WorldSpace` where a local frame
was available. The two production uses (`ActionRequest::world_space` and
`fire_held_ranged_system`) are deliberate. Open: a survey of code that assumes a
global up without naming a frame (a bare `Vec2::Y`, a hardcoded `-y` gravity, a
screen-space comparison).

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

For cameras this is now a concrete product decision, not only a manifesto. A
view may remain world-fixed/external-observer (the current ordinary mode), or it
may follow a designated subject's resolved frame so gravity changes visually
rotate the world around that body. The choice belongs to the **view/context**,
not to gravity simulation and not to a global player singleton. Existing modes
remain valid; future multiview may choose independently per view. See
[`../../systems/camera-reference-frames.md`](../../systems/camera-reference-frames.md).

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

## Scheduling facts for architectural moves

Every [migration packet](actor-monolith-work-frontier.md) names the producer,
consumer, clock, public phase ancestry and deferred-visibility boundary of its
per-frame facts. A public set name alone does not prove that a reader sees this
tick's commands or that an early-return population is unchanged.

A1 preserves checkpoint startup versus gameplay/reset gating; A2 preserves the
actual projectile movement leg and deterministic contact order. Avoid reconstructing
a path or event occurrence from endpoint state when an authoritative per-frame
record already exists. A move to a different installer must preserve these facts,
not just system membership.
