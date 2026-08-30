# ADR 0031: The public facade is the compatibility boundary, and it is enforced

## Status

**Accepted** (2026-07-30). Proposed and accepted the same day, which is not the
usual shape and is worth stating plainly: it was proposed before the first
slice was built, and accepted after the campaign it specified reached its
terminal condition — six slices, all three §4 conditions holding, every
consumer-matrix category naming a test.

What that means for this document is that it can now be read as a record rather
than a plan. Every mechanism it describes exists:
`scripts/check_absence_contracts.py` enforces the allowlist at zero, the
blind-agent gate has been run six times, and the Deferred section's largest
item — rollback as a public knob — was delivered by slice F with the six
properties it demanded, each carrying its own acceptance test.

⚠ One prediction it got wrong, recorded because the correction is more useful
than the score: it framed the `ambition_platformer2d_actor_monolith` decomposition as the carve most
likely to be authorised. The carve the campaign actually needed was
`SnapshotState` down to `ambition_platformer2d_core` — invisible from the API side,
and reachable only by asking why a consumer had to name `ambition_platformer2d_runtime` to
encode its OWN type. §4's carve condition covered it exactly; the ADR's guess
at WHICH boundary did not.

It records a decision reached across a maintainer conversation and three rounds
of external review
([`../archive/reviews/gpt56-jon-conv-2026-07-29.md`](../archive/reviews/gpt56-jon-conv-2026-07-29.md),
[`../archive/reviews/gpt56-reply-2026-07-29-v2.md`](../archive/reviews/gpt56-reply-2026-07-29-v2.md),
[`../archive/reviews/claude-reply-2026-07-30-api.md`](../archive/reviews/claude-reply-2026-07-30-api.md)),
so the constraint is agreed before the first slice is built rather than
discovered during it.

The historical executable campaign is archived at
[`../archive/planning-superseded/2026-08-13/engine/api-1.0-campaign.md`](../archive/planning-superseded/2026-08-13/engine/api-1.0-campaign.md).
It satisfied the original acceptance bar. Current facade evolution is owned by
[`../planning/engine/public-sdk-1.0.md`](../planning/engine/public-sdk-1.0.md) and
[`../planning/engine/capability-and-runtime-composition.md`](../planning/engine/capability-and-runtime-composition.md).

## Context

`crates/ambition_platformer2d/src/lib.rs` is 114 lines. Fifty of them are `pub use`, and
roughly forty are `pub use ambition_x as x`.

**The public API of this engine is currently the list of crates it happens to be
built from.** That is not a facade; it is a namespace mirror. It means:

* the compatibility surface changes whenever the crate graph changes, which is
  whenever anyone reorganises anything;
* a consumer's imports encode our implementation topology, so we cannot move an
  implementation without breaking them;
* and there is no answer to "what is public?" other than "everything".

The external-consumer fixture measures the consequence. Outlander depends only
on `ambition_platformer2d` and Bevy — which was the point — and reaches through it into
`ambition_platformer2d::actors::features` (7 uses), `ambition_platformer2d::runtime::rollback` (6),
`ambition_platformer2d::platformer::markers` (5), `ambition_platformer2d::characters::actor::character_catalog`,
`ambition_platformer2d::runtime::demo_fixture`, and `ambition_platformer2d::runtime::rollback::put_f32`.
A third party building a game is naming an internal serialisation helper.

The same fixture shows the cost at the composition level. `build_windowed_app`
is ~65 lines a consumer must write in a specific order: register your asset
source *before* `DefaultPlugins` (Bevy seals asset sources when `AssetPlugin`
builds), then `init_engine_states`, then `PlatformerEnginePlugins::fixed_tick()`,
then `PlatformerHostPlugins`, then the shell, then `PlatformerAssetsPlugin` —
after the content that registers the catalogs it reads and before the
presentation that draws from what it installs.

**This engine has closed two leaks of exactly this shape and won both times.**

* `ShellComposition` replaced *"seven hand-written steps whose ORDER is enforced
  by a resource-missing panic … two of whose omissions are silent"* with one
  call. LEAK CLOSED 2026-07-28.
* `drive_control_frame` replaced *"`PendingLocalInput` under GGRS, the
  `ControlFrame` resource under fixed tick … writing the wrong one is silently
  ignored: the walk runs, the body never moves, nothing says why"* with one
  seam. LEAK CLOSED 2026-07-27.

Both were found by Outlander. Both are the same shape: **a rule the engine knew
and made the consumer re-derive.** This ADR generalises that method to the API
itself.

It is also what
[`../architecture/package-and-capability-boundaries.md`](../architecture/package-and-capability-boundaries.md)
already anticipated. Its settled "no size-driven `ambition_platformer2d_actor_monolith` carve" ruling
ends: *"This ruling does not protect misplaced named content or prevent a later
split that **a real second consumer demonstrates**."* Building the public API
first is how that consumer gets a voice; it is the mechanism that ruling named,
not a reversal of it.

## Decision

**1. `ambition_platformer2d` is a semantic API, not a crate re-export list.** Public modules
are named for roles, not for implementation crates:

```text
ambition_platformer2d::app        ambition_platformer2d::experience   ambition_platformer2d::character
ambition_platformer2d::actor      ambition_platformer2d::world        ambition_platformer2d::combat
ambition_platformer2d::sim        ambition_platformer2d::lifecycle    ambition_platformer2d::effects
ambition_platformer2d::view       ambition_platformer2d::test         ambition_platformer2d::prelude
```

Provisional until the call-site prototype is accepted — the names are a
consequence of what the call sites need, not an input to them. **Each domain
carries its own prelude** (`ambition_platformer2d::character::prelude`); one enormous root
prelude is a discovery problem for an agent, not a convenience.

**2. The compatibility promise is made at that surface and nowhere else.** Inner
crates remain independently usable by engine developers and carry no stability
promise. A game depends on `ambition_platformer2d`.

**3. Game code may name only the reviewed public surface — an ALLOWLIST, and it
is executable.** `scripts/check_absence_contracts.py` already owns this class:
`DEPENDENCY_CONTRACTS` is a transitive, Cargo-metadata-backed table of
`{crate, forbidden, reason}` with four live rows, including
`engine-crates-do-not-consume-the-umbrella-facade`. The extension needed is
module-path granularity, not a new mechanism.

*Implemented 2026-07-30 as `MODULE_ALLOWLISTS`, one row scoped to
`fixtures/external_consumer/`. It parses use trees rather than matching a line
regex, because `use ambition_platformer2d::{time::Clock, audio::Bank};` is two leaks that a
`\bambition_platformer2d::([a-z_]+)` pattern does not see.*

⚠ **Allowlist, not denylist, and the numbers are decisive.** A denylist always
lags a namespace mirror. Outlander names **18 distinct top-level `ambition_platformer2d::`
modules**; the first draft of the campaign forbade six of them. It would have
gone green with twelve leaks still open — worse than no contract, because it
would have been believed.

> ⚠ **Corrected 2026-07-30.** This paragraph said nineteen, as did the campaign;
> both listed eighteen names. Measured: eighteen, with no brace-grouped
> `ambition_platformer2d::{…}` imports and no root-level type re-exports hiding from the
> count. The implemented contract takes its baseline from the instrument rather
> than from either document.

**It lands green against a recorded boundary that may not widen**, not as a
permanently failing migration gate. The contract protects the reviewed consumer
surface; there is no standing requirement to deliberately poison the check just
to prove that an ordinary assertion can fail. Historical ratchet construction is
recorded in the archived API campaign evidence.

**4. The engine owns composition ordering.** A consumer states policy —
windowed or headless, fixed-step or rollback session, which experience, where it
starts. It does not sequence asset sources, engine plugin groups, host groups,
shell composition, asset preparation and presentation. Every ordering constraint
the engine knows is a rule the engine states once.

**5. `PlatformerApp` is a Bevy plugin group, not a runtime.** A studio with an
existing Bevy `App` must be able to add it without surrendering the `App`. The
engine owns ordering *within its own installation*, not the consumer's process.

## Consequences

**A facade that owns no behavior.** It re-exports public contracts and provides
assembly contexts. Character behavior stays in the character domain, world
behavior in world, combat in combat. If the facade ever grows a leaf system, it
has become the next monolith and this ADR has failed.

**The durable acceptance surface is consumer behavior plus the dependency
boundary.** The campaign used blind-agent trials as a discovery instrument, but
those runs are historical evidence rather than a recurring gate. Current API
work should be driven by real external-consumer friction, focused facade tests,
and the dependency/module boundary that prevents consumers from reaching back
into implementation topology. See [`../concepts/api-growth.md`](../concepts/api-growth.md).

**A consumer matrix, not a consumer.** The compatibility surface may not be
declared complete until each category in
[the archived campaign's matrix](../archive/planning-superseded/2026-08-13/engine/api-1.0-campaign.md) has a proof:
external composition, a movement-only minimal game, a noncombat actor, a module
standalone *and* embedded, Smash, and Ambition itself. An API proven against one
consumer is an API shaped like that consumer.

**The `ambition_platformer2d_actor_monolith` decomposition is deferred, deliberately, and gains a
trigger.** The diagnosis that it has become a gravitational sink is accepted —
it owns audio, menu content, persistence compatibility, LDtk loading, session
lifecycle, cutscene playback and boss orchestration alongside actor simulation.
But the split designed today would fit today's *internal* topology. The API
campaign exists to find out which boundaries a *consumer* can feel, and those
are the ones worth paying for. See
[`../concepts/api-growth.md`](../concepts/api-growth.md)
for the condition that authorises the carve.

**A versioning obligation.** A compatibility promise needs a version and a test
that the inner crates have not leaked back through the facade. Both are campaign
work, not decided here.

## Alternatives considered

**Split `ambition_platformer2d_actor_monolith` first, then design the API over the result.** Rejected
on sequencing, not on merit. It designs boundaries from the inside, and the
stated objective is an engine another game can be built on — a property only a
consumer can measure. It also risks weeks of refactor during which new features
gravitate to whatever the new integration crate is.

**Write a capability-composition doctrine document first.** Rejected on
evidence. The previous large architecture document
([`../planning/architecture-campaign-2026-07-28.md`](../archive/architecture-campaign-2026-07-28.md))
is eight days old and now opens with a SUPERSEDED banner: its reasoning survived,
every status claim rotted within a week. A growth law written after three real
migrations is a description; written before, it is a prediction. The method for
deriving it *from* the migrations is
[`../concepts/api-growth.md`](../concepts/api-growth.md).

**Keep the namespace mirror and document which modules are "really" public.**
Rejected: that is a doc marker, and this repository has been burned three times
by absences asserted in prose. A boundary nothing enforces is a boundary that
erodes at the first deadline.

## Current implications for agents

- **A consumer names only the reviewed public surface.** `MODULE_ALLOWLISTS`
  in `scripts/check_absence_contracts.py` enforces this at module-path
  granularity over `fixtures/external_consumer/`, with the baseline at zero.
  When consumer code needs a new `ambition_platformer2d::` module, the module is reviewed
  into `allowed` — the baseline never grows.
- **Assembly goes through `PlatformerApp`** (`crates/ambition_platformer2d/src/app.rs`).
  Never hand-order asset source, engine plugin groups, host groups, shell,
  asset preparation and presentation in a consumer or fixture; every ordering
  rule the engine knows is stated once, in the builder.
  `outlander-does-not-hand-order-its-own-composition` guards the regression.
- **The facade owns no behavior.** A leaf system added to `crates/ambition_platformer2d`
  belongs in a domain crate; the facade holds assembly contexts and
  re-exported contracts only. If it grows behavior, this ADR has failed.
- **Blind-agent runs are archived discovery evidence, not standing process.**
  Do not create a new blind-agent series merely because the public surface
  grows. Prefer a real consumer, a focused facade/SDK regression, or an
  architectural dependency check that represents the actual failure mode.
- **The SDK reference is guarded both ways.**
  `scripts/tests/test_sdk_api_reference_is_current.py` cross-checks
  `docs/sdk/api-reference.md` against the facade's exports — extend the doc in
  the same commit that grows the surface.
