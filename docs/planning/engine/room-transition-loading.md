# Adaptive room-transition loading

**Status:** OPEN planning document

**Assumption:** the action/control P3 migration is owned by another active
campaign. This plan does not reopen that work.

**Coordinates with:**
[`immutable-content-and-transactional-construction.md`](immutable-content-and-transactional-construction.md),
[`architecture.md`](architecture.md), and the historical
[`startup-loading-and-frontend-flow-2026-07-13.md`](../../archive/reviews/startup-loading-and-frontend-flow-2026-07-13.md).

## Competitive objective

Ambition is intended to compete with Unity, Unreal, and Godot as a serious game
engine, not merely provide enough room switching for the first game. A
professional engine must let games move between worlds, scenes, rooms, or levels
without exposing partially prepared content, stale authority, or unexplained
main-thread stalls.

The player-facing promise is:

> **A target room is never visible, interactive, or authoritative until all
> activation-critical work is ready. Fast transitions remain unobtrusive. Slow
> transitions progressively disclose an intentional transition cover and then
> an honest loading experience instead of dropping the player into a half-loaded
> room.**

The engine-facing promise is:

> **Fast and slow room transitions use one readiness and commit transaction.
> Presentation adapts to elapsed time and commit cost; correctness does not.**

This plan describes the strongest route visible from the current architecture.
Its outcomes and invariants are binding. Proposed type names, thresholds, crate
placement, and intermediate mechanisms are provisional. A future agent should
replace them when a simpler or more capable Bevy-native design preserves the
same player and engine guarantees.

## Decision summary

The desired experience is not a binary choice between an immediate transition
and a loading screen. It is progressive disclosure:

1. **Prepare invisibly.** The current room remains authoritative while target
   work starts or promoted prefetch completes.
2. **Use the authored transition as cover.** A door close, fade, wipe, tunnel,
   elevator, or other normal transition animation provides intentional visual
   occlusion.
3. **Escalate only when necessary.** If readiness is still unresolved after a
   configurable grace period, hold the cover and reveal loading evidence.
4. **Commit once.** Activate the target only after readiness is proven and the
   presentation state makes the commit safe to perform.
5. **Reveal the complete room.** Remove the cover only after the new room is
   authoritative and its required presentation state exists.

For a millisecond-scale transition, the user should see at most the ordinary
room-transition effect and usually no loading foreground at all. For a slower
transition, the same transaction holds the effect at an opaque point and reveals
an honest loading view. There must not be a separate unsafe fast path that skips
readiness checks.

## Current source-backed gap

The repository already contains substantial pieces:

- `ambition_load` is a contributor-neutral coordinator for work, barriers,
  cancellation, supersession, progress evidence, speculative promotion, and
  one-shot commit authorization.
- `ambition_load_presentation` implements hidden grace, delayed reveal, semantic
  progress, activities, ready hold, and cleanup.
- provider startup preparation uses those systems for shell-route activation.
- `RoomStaging` provides transactional preparation for snapshot-driven room
  restoration.
- the current construction plan establishes the intended future seam for pure
  room planning and atomic world replacement.

The ordinary room-transition path does not compose those pieces. A
`RoomTransitionRequested` currently proceeds to the synchronous room-loading
path. Target-room readiness is not represented by an `ambition_load` barrier,
and normal transitions do not wait for target construction or required asset
readiness before mutation.

The existing presentation crate is also shell-specific despite its generic
name:

- its foreground is keyed by `ShellRouteId`;
- it observes `ShellEvent::WaitingForLoad` directly;
- it manipulates `ShellRouteHolds` directly;
- retry and cancellation actions are routed through `ShellRouter`.

Room transitions should not fabricate shell routes to reuse this machinery.
The presentation lifecycle needs a contributor-neutral command surface, with
shell and room-transition adapters as separate owners.

The standard provider preparation plan also declares speculative work named
`prewarm-neighbor-room`, but no implementation currently discovers neighboring
room dependencies, performs concrete prewarming, or promotes that work when a
transition is requested.

## Binding invariants

### R1. No partial target authority

The old room remains the active simulation and presentation authority until the
target activation transaction commits. A partially prepared target must not be
observable by ordinary actor, collision, camera, interaction, render, audio, or
room queries.

### R2. One readiness path

Every room transition, including apparently instant transitions, uses the same
required-work barrier and one-shot commit authorization. A fast transition may
avoid visible loading presentation; it may not bypass readiness.

### R3. Presentation never manufactures readiness

The loading or transition UI consumes load facts. It cannot mark assets,
construction, or room data ready. Conversely, content readiness alone does not
prove that it is visually safe to run an expensive synchronous commit.

### R4. A blocking commit is covered before it starts

If commit may exceed the accepted frame budget, the engine must present an
opaque transition cover for at least one rendered frame before beginning the
blocking operation. Rendering a loading screen after the stall has already
occurred does not satisfy this invariant.

### R5. No loading-screen flash

A load foreground uses delayed reveal. If it becomes visible, presentation may
apply a small configurable minimum-visible duration to avoid a one-frame flash.
This policy must never hold a hidden fast transition unnecessarily.

### R6. Honest evidence

Progress labels and percentages come from real contributor evidence. Unknown
work is described as unknown. The UI does not fake a smooth percentage merely
to appear polished.

### R7. Supersession is exact

A new transition, cancellation, retry, session replacement, hot reload, or
content-epoch change invalidates stale work and stale commit authorization.
Late completion from an old transaction cannot activate its target.

### R8. Failure preserves a valid world

Preparation failure leaves the current room intact. While the transition cover
is visible, failure presentation can offer retry, return, or another
provider-authored action without exposing a broken target room.

### R9. Required and degradable work are explicit

Only activation-critical work blocks room authority. Decorative or degradable
assets may continue after activation when the target can render and simulate
correctly without them.

### R10. Masking and optimization are distinct

Showing a cover before a synchronous commit fixes presentation correctness; it
does not make the commit fast. Commit-duration telemetry and explicit frame
budgets remain required, and measured over-budget commits must motivate real
construction or render-path optimization.

## Terminology

### Room transition transaction

The complete lifecycle from an accepted transition request through target-room
activation or cancellation.

### Required barrier

The `ambition_load` barrier proving all activation-critical target work complete.

### Hidden grace

A short interval in which target preparation runs without showing a loading
foreground. The current room or authored transition animation remains visible.

### Transition cover

An opaque presentation state such as a closed door, full fade, wipe, tunnel, or
elevator interior. A cover is not necessarily a loading screen.

### Loading foreground

The explicit loading presentation shown when hidden preparation and the ordinary
transition cover are insufficient. It may show stage labels, progress evidence,
failure actions, or an optional loading activity.

### Commit

The one-shot operation that makes the prepared target room authoritative. It
includes the room/session ownership update required to make ordinary systems see
exactly the new room.

### Reveal

The presentation step that removes the transition cover after successful commit.

### Prefetch

Speculative, non-authoritative work for likely future destinations. Prefetch may
be promoted into required work without restarting equivalent work.

## Desired user experience

### Fast path

```text
transition accepted
    -> target barrier prepares during hidden grace / authored door animation
    -> target ready
    -> commit is within the proven frame budget
    -> commit once
    -> complete target is revealed
```

No loading foreground appears. A game may still show its authored door, fade, or
wipe because that is transition presentation rather than evidence of a slow
load.

### Covered fast commit

```text
transition accepted
    -> target becomes ready quickly
    -> commit is expected to exceed the no-cover frame budget
    -> transition cover reaches opaque and is presented
    -> commit once behind the cover
    -> reveal target
```

This is the likely first safe policy for the Hall of Characters: the existing
one-frame construction cost is hidden behind an intentional cover, even before
construction itself is optimized.

### Slow path

```text
transition accepted
    -> target still preparing when ordinary cover reaches its hold point
    -> cover remains opaque
    -> delayed threshold elapses
    -> loading foreground appears with real evidence
    -> target barrier becomes ready
    -> commit authorization is requested
    -> commit succeeds
    -> loading foreground and cover leave cleanly
    -> complete target is revealed
```

### Failure path

```text
transition accepted
    -> target preparation fails
    -> current room remains authoritative
    -> cover/loading foreground reports a player-safe failure
    -> retry mints a fresh transaction, or cancel reveals the old room
```

## Architecture

## 1. Move generic barrier references out of the shell domain

A room transition needs to refer to an `ambition_load` plan and barrier without
depending on `ambition_game_shell`.

Move or replace the current shell-owned pair with a load-domain type such as:

```rust
pub struct LoadBarrierRef {
    pub load_id: LoadId,
    pub barrier_id: LoadBarrierId,
}
```

The final name is provisional. The ownership is not: the reference belongs with
the load coordinator.

The shell should consume the generic type rather than own it.

## 2. Make loading presentation contributor-neutral

Introduce a generic foreground protocol conceptually similar to:

```rust
pub enum LoadPresentationCommand {
    Begin {
        owner: LoadPresentationOwnerId,
        barrier: LoadBarrierRef,
        experience: LoadExperienceId,
    },
    Finish {
        owner: LoadPresentationOwnerId,
    },
    Cancel {
        owner: LoadPresentationOwnerId,
    },
}
```

And tag user actions with the same owner:

```rust
pub enum LoadPresentationAction {
    Continue { owner: LoadPresentationOwnerId },
    Retry { owner: LoadPresentationOwnerId },
    Cancel { owner: LoadPresentationOwnerId },
}
```

Exact APIs may differ. The requirements are:

- foreground state is keyed by a generic activation/owner identity;
- presentation observes `LoadCoordinator` facts directly;
- shell routes are one adapter;
- room transitions are another adapter;
- retry/cancel semantics remain with the owning adapter;
- no synthetic `ShellRouteId` is created for a room.

The current shell behavior should remain through a thin shell integration plugin.
The generic presentation plugin should not schedule itself relative to shell sets
when no shell integration is installed.

## 3. Add a headless room-transition coordinator

The coordinator owns the transaction state, not the loading UI.

A provisional state model is:

```rust
pub enum RoomTransitionPhase {
    PreparingHidden,
    CoverRequested,
    CoveredWaiting,
    CommitRequested,
    Committing,
    Revealing,
    Complete,
    Failed,
    Cancelled,
    Superseded,
}
```

The state should carry at least:

- transaction identity;
- source room and target room;
- arrival/portal information;
- content epoch;
- required barrier reference;
- selected transition presentation policy;
- whether an opaque cover has been presented;
- commit authorization and result;
- retry/cancellation policy;
- timestamps and telemetry.

The transaction coordinator should be usable in a no-window headless app. A
headless test may acknowledge the presentation-safe gate directly; visible hosts
must receive the acknowledgment from presentation after the cover has actually
been rendered.

## 4. Separate readiness from presentation-safe commit

`ambition_load` should continue to authorize commit only when required work is
ready. The room-transition coordinator decides when to request that authorization.

The coordinator requests commit only when:

```text
required barrier is ready
AND
(no cover is required OR the cover has been presented)
AND
this transaction is still current
```

This avoids treating a fade animation as content work while still guaranteeing
that an expensive commit does not start before the player can see the cover.

## 5. Progressive-disclosure presentation policy

Room transitions need a policy independent of load correctness. A provisional
shape is:

```rust
pub struct RoomTransitionExperience {
    pub cover: TransitionCoverStyle,
    pub loading_reveal_after: Duration,
    pub minimum_visible: Duration,
    pub fast_commit_budget: Duration,
    pub ready_policy: ReadyTransitionPolicy,
}
```

The values are game- and platform-configurable. The architecture must not encode
one universal millisecond threshold.

Room transitions should normally use automatic continuation. A `Continue` hold
is appropriate only when a game deliberately offers an engaging loading activity
or a design-specific pause. A room that is ready should not routinely require an
extra button press.

Presentation tiers are:

1. **Hidden:** no loading foreground.
2. **Covered:** normal transition cover is opaque; no progress panel yet.
3. **Visible loading:** cover remains and semantic loading evidence appears.
4. **Failure:** failure controls replace progress while the old room remains
   recoverable.

## 6. Define target-room contributors

A room-transition adapter should declare concrete work rather than one generic
"load room" step.

Initial required contributors should include, where applicable:

- resolve target room and arrival;
- validate target-room/world data;
- prepare the target construction or staging plan;
- discover required target asset dependencies;
- request and confirm activation-critical visual assets;
- prepare collision and room geometry;
- prepare required character, prop, and boss presentation assets;
- prepare target parallax or required background layers;
- prepare required room/session routing facts.

Potential degradable work includes:

- high-resolution presentation upgrades;
- decorative particles;
- ambient variants;
- noncritical packed audio;
- distant-room or off-camera presentation.

Potential speculative work includes:

- neighboring room construction plans;
- neighboring room asset dependencies;
- shader/pipeline prewarming where supported;
- likely music or ambient transitions.

Each contributor owns its own work state. The room coordinator aggregates facts
through `ambition_load`; it does not poll subsystem internals ad hoc.

## 7. Give rooms an asset-dependency contract

A transition cannot wait honestly for assets until the engine can identify which
assets are required for a room.

The immediate contract may be a deterministic derived manifest keyed by:

```text
content epoch + room id + platform profile + quality profile
```

It should identify logical assets, not raw filesystem paths. The manifest can be
assembled from:

- room geometry and tile/background references;
- placed actor and prop recipes;
- character and boss visual definitions;
- parallax definitions;
- room music/ambient intent;
- required UI or dialogue assets where activation depends on them.

The long-term owner is the asset dependency graph/cooker. The first integration
may derive the manifest at runtime from current catalogs, provided the result is
deterministic and testable.

## 8. Implement real neighboring-room prefetch

The current speculative `prewarm-neighbor-room` label is not sufficient.
Prefetch needs concrete target identities and reusable artifacts.

While room A is active, the engine may inspect the room graph and prepare likely
neighbors B and C. Prefetch must not create active simulation entities.

A target prefetch may produce:

- validated room lookup and arrival metadata;
- a pure construction/staging plan;
- requested asset handles;
- completed asset dependencies;
- warmed render pipelines when practical;
- cached deterministic manifests.

When A -> B is requested, equivalent work should be promoted or reused rather
than restarted. Promotion must preserve content-epoch identity and reject stale
results after hot reload or provider replacement.

Prefetch remains an optimization. A transition without a prefetch hit still uses
the same required barrier.

## 9. Keep the old room authoritative during preparation

After a transition is accepted, games may choose among authored control policies:

- capture the player in a door animation;
- freeze the controlled body;
- pause fixed simulation when the cover becomes opaque;
- allow a cancellable boundary approach before final acceptance.

Regardless of the presentation choice:

- the old room remains the only active room until commit;
- repeated portal triggers do not create duplicate transactions;
- old-room teardown does not start during target preparation;
- failure can return to a valid old-room state.

## 10. Integrate transactional construction without blocking early progress

The room-loading integration and the immutable-content/construction campaign are
complementary but should not deadlock each other.

### Before the construction plan lands

The transition coordinator can use existing room lookup, asset preparation, and
`RoomStaging::prepare`-like validation as contributors. The final room mutation
may remain synchronous, but it must occur only after readiness and behind a
presented cover when over budget.

### After the construction plan lands

The target `ConstructionPlan` becomes the authoritative preparation artifact.
Activation, reset, transition, hot reload, and restore share its executor.
The room-transition transaction carries the prepared content epoch and plan
identity into one atomic commit.

The transition plan must not introduce a second placement-lowering or spawn
authority. It is a consumer of the one App-installed lowering/construction path.

## 11. Treat expensive commit as a measured engineering problem

The first safe implementation may perform a synchronous ECS commit behind an
opaque cover. That is acceptable as an integration milestone because it prevents
the player from observing a partial room.

It is not the final performance target.

Record at least:

- request-to-ready duration;
- prefetch hit/miss;
- time spent discovering work;
- target asset wait time;
- cover request-to-presented latency;
- commit duration;
- first complete rendered frame latency;
- loading foreground visible duration;
- fast-path budget overruns.

If measured commit exceeds the platform budget, choose an isolation strategy
based on evidence:

- move more work into pure preparation;
- reduce redundant entity construction;
- prewarm first-use render resources;
- build in a disposable staging world;
- incrementally construct under an inactive staging scope whose entities are
  structurally invisible to normal simulation and presentation;
- atomically publish a completed room root.

Do not incrementally spawn a target into the live active world unless types and
queries make partial visibility impossible by construction.

## Scope

This push includes:

- generic load-presentation ownership;
- room transition transactions backed by `ambition_load`;
- delayed loading-screen reveal;
- transition-cover acknowledgment;
- required room readiness barriers;
- failure, cancellation, retry, and supersession;
- neighboring-room dependency discovery and prefetch;
- room asset dependency manifests;
- one-shot room commit;
- transition telemetry;
- integration with transactional construction.

## Non-goals

This push does not initially require:

- open-world streaming;
- seamless background loading of arbitrary world sectors;
- a custom editor;
- network replication of transition presentation;
- a universal scene graph;
- cross-release asset compatibility;
- loading minigames for ordinary room changes;
- eliminating every room-entry hitch before correctness integration lands;
- keeping both the old and new room fully simulated simultaneously.

## Execution plan

## Phase 0 — establish evidence and budgets

### Tasks

1. Add schedule-level tracing around current room transitions, including deferred
   command application and the first complete target render frame.
2. Record transition cost for a small room and the Hall of Characters.
3. Define provisional per-platform budgets for:
   - no-cover commit;
   - covered synchronous commit;
   - loading foreground reveal.
4. Add a source-backed current-state test proving ordinary transitions do not yet
   use an `ambition_load` barrier.

### Exit

- the Hall transition has a reproducible request-to-complete trace;
- the synchronous commit cost is measured rather than inferred;
- the plan has an explicit initial fast-commit budget.

## Phase 1 — decouple loading presentation from shell routes

### Tasks

1. Move the generic barrier reference into `ambition_load`.
2. Add contributor-neutral presentation begin/finish/cancel commands.
3. Key foreground state and actions by a generic owner/activation identity.
4. Move shell-specific observation, holds, retry, and cancellation into a shell
   adapter.
5. Preserve current provider-startup behavior and tests.
6. Add a no-shell test that drives hidden grace, visible loading, ready, failure,
   and cleanup through generic presentation commands.

### Exit

- `ambition_load_presentation` can present an arbitrary load barrier without a
  `ShellRouteId`;
- shell activation still has identical behavior through its adapter;
- room integration no longer needs a fake shell route.

## Phase 2 — gate ordinary room transitions on real readiness

### Tasks

1. Introduce one exact room-transition transaction identity.
2. Convert `RoomTransitionRequested` into a load plan and required target barrier.
3. Keep the source room authoritative while contributors prepare target data.
4. Add work contributors for target room lookup, arrival validation, current room
   staging/construction preparation, and required parallax readiness.
5. Request commit only after barrier readiness.
6. Authorize and execute commit exactly once.
7. Publish `RoomLoaded` only after successful complete-room commit.
8. Make failure and cancellation leave the source room intact.

### Exit

- no normal transition mutates the active room before its target barrier is ready;
- stale completion cannot commit a superseded destination;
- a failing target does not despawn or corrupt the source room.

## Phase 3 — add cover-first adaptive presentation

### Tasks

1. Add a room-transition presentation adapter.
2. Support an authored cover style and hidden loading-foreground grace period.
3. Add an explicit "cover presented" acknowledgment.
4. Require that acknowledgment before known-expensive synchronous commits.
5. Show no loading foreground for a fast transition.
6. Hold the opaque cover and reveal loading evidence for a slow transition.
7. Prevent loading-screen flash with a configurable minimum-visible policy.
8. Reveal only after commit and required target presentation are complete.

### Exit

- the Hall's existing synchronous construction stall occurs only after an opaque
  cover has rendered;
- small-room transitions do not flash a loading panel;
- slow injected loads show honest progress and never show a partial target.

## Phase 4 — room asset readiness and concrete prefetch

### Tasks

1. Define a deterministic room asset dependency manifest.
2. Classify dependencies as required, degradable, or speculative.
3. Add concrete per-room work IDs and remove the placeholder-only neighbor prewarm.
4. Start speculative preparation for likely neighboring rooms.
5. Promote or reuse equivalent prefetched work on transition request.
6. Invalidate prefetched artifacts on content-epoch, platform-profile, or
   quality-profile change.
7. Report prefetch hit rate and saved transition time.

### Exit

- a room cannot commit before activation-critical assets are ready;
- a prefetched neighbor uses the same artifacts after promotion;
- prefetch misses and stale prefetches remain safe.

## Phase 5 — converge on transactional construction

### Tasks

1. Replace provisional staging artifacts with the canonical `ConstructionPlan`.
2. Make target-room plan identity and content epoch part of the transaction.
3. Make activation, reset, transition, hot reload, and restore share the same
   execution authority.
4. Prove planned and committed target rosters match.
5. Ensure partial staging is unobservable.
6. Remove legacy direct room-load mutation after all target families migrate.

### Exit

- normal transition and snapshot room reconstruction use one room-construction
  authority;
- target-world commit is atomic at the active-room ownership boundary;
- no legacy room transition can bypass the load transaction.

## Phase 6 — close measured performance gaps

### Tasks

1. Set platform-specific commit and reveal budgets from recorded evidence.
2. Move expensive pure work before commit.
3. Prewarm measured first-use render resources.
4. Optimize pathological room-specific scaling without room-specific engine
   exceptions.
5. Introduce isolated incremental construction only if synchronous commit remains
   over budget after simpler fixes.
6. Add regression thresholds for the Hall and representative small rooms.

### Exit

- covered commit is not merely correct but meets the agreed target on supported
  hardware, or an isolated incremental path prevents long main-thread stalls;
- fast transitions remain below the loading-foreground threshold;
- performance regressions are visible in automated or repeatable tooling.

## Testing strategy

### Headless transaction tests

Prove:

- required work must be ready before commit authorization;
- commit authorization is one-shot;
- cancellation and supersession make late completion inert;
- retry creates a fresh transaction;
- source room remains authoritative until commit;
- failure preserves the source room;
- streamable work does not block activation;
- prefetch promotion reuses equivalent work.

### Presentation tests

Prove:

- fast completion remains in hidden grace;
- cover-only completion never shows a loading foreground;
- slow completion reveals semantic evidence;
- a blocking commit cannot begin before cover acknowledgment;
- loading foreground does not flash for one frame;
- visible loading cleans up after successful reveal;
- failures route actions back to the owning room adapter rather than the shell.

### Integration tests

Include:

1. hub -> small room, fast and no loading foreground;
2. hub -> Hall of Characters, cover shown before commit;
3. injected slow asset load, visible loading evidence;
4. missing target asset, source room preserved with failure UI;
5. repeated portal requests, one current transaction;
6. request A then supersede with B, only B can commit;
7. prefetched neighbor promotion;
8. content hot reload invalidating stale prefetch;
9. no-window room transition using the same readiness transaction;
10. complete-room roster visible on the first revealed target frame.

### Performance tests and traces

Track distributions rather than one fragile exact number:

- p50/p95 request-to-ready;
- p50/p95 commit duration;
- p50/p95 first-complete-frame latency;
- loading foreground display rate;
- loading foreground visible duration;
- prefetch hit rate;
- fast-path budget overruns;
- Hall transition regression.

## Risks and countermeasures

### Risk: the loading screen merely hides a frozen frame

**Countermeasure:** require cover-before-commit for correctness, but separately
measure commit duration and retain a performance milestone. Masking is an
intermediate UX fix, not a claim that loading is asynchronous.

### Risk: partial target entities leak into live queries

**Countermeasure:** do not stage inside the active world until an isolation model
is proven. Prefer pure plans, a disposable world, or a structurally inactive
scope with exhaustive authority tests.

### Risk: shell decoupling regresses startup loading

**Countermeasure:** preserve shell semantics through an adapter and keep the
existing hidden-grace, ready-hold, retry, and cancellation test suite as a parity
oracle.

### Risk: every transition shows a loading panel

**Countermeasure:** delayed reveal and progressive disclosure are binding.
Ordinary cover presentation is distinct from a loading foreground.

### Risk: hidden grace leaves the player free to retrigger or escape

**Countermeasure:** transition acceptance owns a clear control policy and exact
transaction identity. Repeated trigger events are ignored, cancelled, or
superseded deliberately.

### Risk: prefetch duplicates work and memory

**Countermeasure:** key artifacts by target/content/profile identity, expose
promotion and cancellation, and enforce provider-owned memory budgets.

### Risk: estimated percentage becomes misleading

**Countermeasure:** percentages remain optional and confidence-tagged. Prefer
named stages when discovery is open or cost estimates are weak.

### Risk: room loading and construction plans create two authorities

**Countermeasure:** loading coordinates readiness and commit authorization;
construction owns target planning and mutation. Neither duplicates the other's
responsibility.

## Milestones

### LRT-A — generic presentation

- load presentation is no longer shell-bound;
- shell startup remains a supported adapter;
- arbitrary load owners can use hidden grace and semantic evidence.

### LRT-B — safe room barrier

- ordinary room transitions use `ambition_load`;
- source authority remains intact until target readiness;
- target commits exactly once or not at all.

### LRT-C — adaptive transition UX

- fast transitions avoid the loading foreground;
- expensive commits occur behind a rendered cover;
- slow transitions reveal honest loading evidence;
- failures retain a valid source room.

### LRT-D — room-aware prefetch

- room dependencies are explicit;
- neighbors can be prepared speculatively and promoted;
- stale prefetch cannot commit.

### LRT-E — construction convergence

- the canonical construction plan drives normal transitions and reconstruction;
- partial target state is structurally unobservable;
- legacy direct mutation is retired.

### LRT-F — performance closure

- the Hall and representative rooms have repeatable budgets and traces;
- remaining long commits are optimized or isolated rather than merely hidden.

## Recommended implementation order

```text
1. Instrument current small-room and Hall transitions
2. Move LoadBarrierRef into ambition_load
3. Generalize ambition_load_presentation ownership
4. Preserve shell behavior through an adapter
5. Add a headless room-transition transaction
6. Gate normal room mutation on a required barrier
7. Add transition cover and cover-presented acknowledgment
8. Add delayed room-loading foreground
9. Add room asset dependency manifests
10. Implement concrete neighboring-room prefetch and promotion
11. Replace provisional staging with ConstructionPlan
12. Optimize or isolate measured over-budget commits
```

## Management decision

Approve **Adaptive Room-Transition Loading** as an immediate integration plan and
as a primary customer of Immutable Content Assembly and Transactional
Construction.

The central decision is:

> **Never choose between correctness and an instant-feeling transition. Always
> prove readiness. Keep that proof invisible when it is fast, use the authored
> transition as cover when commit needs protection, and reveal a loading
> foreground only when the wait becomes perceptible.**

This gives Ambition the professional scene-transition behavior expected from a
Unity-, Unreal-, or Godot-class engine without creating separate fast and slow
room-loading architectures.
