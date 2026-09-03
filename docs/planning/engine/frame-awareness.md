# Small Manifesto: Frame Awareness

> **Status: Jon's design position (2026-07-05), captured verbatim.** The third
> binding manifesto, beside [`../../architecture/spatial-model.md`](../../architecture/spatial-model.md) (space) and
> the relativity principle it generalizes. Adjudicated into working discipline
> as **AJ13** in the archived 07-05 plan
> ([`../../archive/reviews/fable-demo-plan-2026-07-05.md`](../../archive/reviews/fable-demo-plan-2026-07-05.md));
> the live queue is [`../tracks.md`](../tracks.md). Like ADR 0020: do not
> deviate without raising an explicit challenge Jon accepts.

> **RE-MEASURED against `14e5beeb0` (2026-09-03), two months on. ⭐ THIS
> MANIFESTO WAS BUILT, AND A READER SHOULD KNOW THAT BEFORE READING IT AS
> ASPIRATION.** The page asks for *"a coherent language of frames"* while warning
> against building *"a grand frame graph before we need it"*. What exists is the
> first and not the second: `crates/ambition_geometry/src/reference_frame.rs`,
> **816 lines, 11 public types**, referenced from **17 crates** — the last
> by `grep -rl 'reference_frame::\|AccelerationFrame\|GameplayFramePolicy'` over
> `crates/` and `game/`, counted by owning crate. ⚠ It read **16** when this block
> was written hours earlier the same day; `ambition_abilities` arrived in between.
> ⇒ The instrument is here because the number moved while the page was being
> written, which is the shortest possible demonstration of why a transcribed
> count needs one.
>
> | the manifesto's phrase | what carries it at HEAD |
> |---|---|
> | *"contact is relative to a surface frame"* / *"a jump is relative to a body and support frame"* | `LocalAxes`, `MotionFrame`, `ResolvedControlFrame` |
> | *"relative to what?"* as the core question | `GameplayFramePolicy` — `ControlledBodyLocal`, `AccelerationFrame`, `WorldSpace`, `ScreenSpace` |
> | *"a moving platform is a support frame in motion"* | `AccelerationFrame`, built from net down-defining acceleration, **not cardinal-snapped** — arbitrary-angle `down` is supported |
> | *"a camera is not the world; it is an observer"* | `CameraReferenceFrame` |
>
> ⭐ **AND THE CAMERA PARAGRAPH BELOW IS NOW LITERALLY TRUE, INCLUDING THE PART
> THAT WAS EASIEST TO GET WRONG.** `CameraReferenceFrame` has exactly the two
> modes it names — `WorldFixed` (*"screen orientation stays tied to the world
> frame even when the subject enters sideways or inverted gravity"*) and
> `SubjectFrame` (*"a gravity change presents as the world rotating around an
> upright body"*) — and it is a player-facing setting, cycled by
> `cycle_camera_reference_frame`
> (`ambition_persistence/src/settings/gameplay.rs:275`). The manifesto insisted
> the choice belongs to the view *"and not to a global player singleton"*; the
> type's own doc keeps that promise in situ: *"the subject is a view's subject,
> not a protagonist. The resolver takes a direction, never an entity, so a
> spectator, a replay or a second local view can orient on whatever body it is
> watching."*
>
> ⇒ **So the standing question for this page is no longer "should we" but "what
> is still world-frame by default that should not be".** `GameplayFramePolicy`
> is rollback-registered (`snapshot_unit_enum!`,
> `platformer2d_core/src/snapshot_impls.rs:342`), so the vocabulary is
> simulation truth rather than a presentation convenience. ⚠ Not measured here:
> how many systems still reach for `WorldSpace` where a local frame is
> available. That is the next honest question and it needs a survey, not a grep.

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
