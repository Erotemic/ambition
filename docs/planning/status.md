# HEAD orientation

**Reviewed baseline:** `4e5f59cf753a62105cbc9fd53aa9697d337d0eed` —
`Update docs phase 2` (2026-08-30).

This file is a current orientation page. It intentionally does not preserve the
chronology of how the repository reached this state. Use git history, dated
reviews, and `dev/ambition_dev_measurements` for that evidence.

Immediate execution lives in [`queue.md`](queue.md). Standing deferred work lives
in [`tracks.md`](tracks.md). Focused plans own design details.

## Current architecture model

### Gameplay lifetime has three distinct scopes

The engine now distinguishes:

```text
process
  -> gameplay session          SessionScopeId
       -> rollback timeline    RollbackTimelineGeneration
```

This distinction is load-bearing. A rollback diagnosis may carry across a GGRS
rebase inside the same gameplay session, but it is not authority for a different
gameplay session. `ActiveRollbackAuthority` owns the gameplay-session scope,
timeline generation, content/schema contract, and timeline health together.
Gameplay reads confirmation through `SessionRollbackConfirmation`, which must
name the live scope. A foreign authority answers `Unavailable`, not `Unhealthy`.

Session-scoped process resources that remain during migration are re-established
on `SessionScopeActivated`; retirement cleanup remains hygiene rather than the
only protection against cross-session contamination. ADR 0027 is the durable
rollback/lifetime authority.

### Rollback correctness is broader than component serialization

The current model has separate questions:

1. **codec** — what authoritative state rewinds;
2. **participation** — which authoritative entities exist on the rollback
   timeline;
3. **semantic identity** — which logical simulation object an entity represents;
4. **deterministic composition** — how multiple valid peers select/compose when
   order affects an outcome;
5. **lifetime ownership** — which gameplay session/timeline may treat a piece of
   state as authority.

Rollback registration is federated by domain and the concrete GGRS backend lives
in `ambition_platformer2d_rollback_ggrs`; the generic runtime no longer owns a
census of concrete gameplay component types. The remaining work is correctness
at runtime-created populations, residual non-rewinding memory, deterministic
selection/composition, and the confirmed/external lifecycle boundary.

See [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md)
and [`engine/netcode.md`](engine/netcode.md).

### Construction is transactional; reconstruction still has more than one road

Prepared content and room construction use typed domain lanes under one
plan/preflight/commit/verify/publish transaction. Confirmed rollback room
transitions already wait for the same readiness/authorization transaction and
rebase onto a new frame-zero baseline; speculative rollback frames do not cross a
room boundary.

The remaining architectural problem is **reconstitution**, not another rollback
snapshot format. Fresh construction, room transition, same-room replay and
new-game reset now run one constructor; the same-room replay's hand-kept reset
ledger is deleted. Checkpoint/save restoration still corrects an already-built
world instead of informing its construction.

See [`engine/construction-and-reconstitution.md`](engine/construction-and-reconstitution.md).

### The actor program is now a residual-kernel program

The first controlled-character decision-authority decomposition is largely
complete. Generic simulation no longer needs the old primary-player combat-slot
arbitration, observation/decision phases are separated more clearly, and several
player-centric forks have disappeared.

The actor monolith remains a major ownership/dependency boundary. Its purpose is
no longer “reduce LOC” or “win frame time.” Carves should leave behind the
smallest coherent actor/body simulation kernel while moving unrelated domain
ownership, plugin registration, dependencies, and tests to their natural homes.

See [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md)
and [`engine/controlled-character-actor-kernel.md`](engine/controlled-character-actor-kernel.md).

### Capabilities are an architecture boundary, not a measured CPU optimization

Capability/runtime composition remains important for dependency closure, test
isolation, reusable engine packages, platform composition, and the public SDK.
A measured removal of several experiences produced no material frame-time or
plugin-registration startup win, so capability work should not be justified by
those claims without new evidence.

See [`architecture/package-and-capability-boundaries.md`](../architecture/package-and-capability-boundaries.md)
and [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md).

## Engine-product posture

Ambition is targeting a **Godot-class 2D engine capability surface**, but not a
Godot-style editor product. The comparison is engine expressiveness, runtime/build
efficiency, composition, diagnostics, portability and the ability for another
serious 2D game to use supported capabilities.

The preferred authoring surface is LLM-first and semantic: machine-readable
discovery, structured inspection, transactional mutation where formats are
fragile, validation/preparation, deterministic test scenarios, concise visual
review artifacts and noninteractive build/package commands. Human visual editors
are optional frontends for genuinely visual/manual tasks.

Current strengths include specialized platformer movement/collision, deterministic
headless/rollback-oriented simulation, prepared content and construction, strong
secondary-game pressure, generated sprite/music/SFX pipelines, LDtk semantic
tooling, and an increasingly semantic public facade.

The largest product-level completeness gaps are persistent/open-world
reconstitution, public SDK/capability closure, asset materialization/residency and
weak-GPU quality, external project build/package, structured provenance/why-not
diagnostics, authored gameplay orchestration, and remaining multiplayer/multiview
maturity. Ordinary rendering/UI/audio capability should be audited from real game
needs and composed from Bevy/ecosystem facilities where that is the cleaner path.

See
[`engine/godot-class-2d-capability.md`](engine/godot-class-2d-capability.md).

## Current performance model

### Simulation CPU

Representative headless Smash workloads are within the current 60 Hz budget and
show many small systems rather than one dominant gameplay hotspot. Current
evidence does not fund broad system-count reduction, generic change-driven
projection, physics rewrites, or parallelizing `GgrsSchedule`.

### Weak-GPU rendering

The corrected feature-matched Intel HD 630 comparison is:

```text
51.045 ms p50  ->  20.101 ms p50
about 2.54x
about 19.6 FPS -> 49.7 FPS
```

Framebuffer/display scale and MSAA moved together, so their independent shares
remain unmeasured. The faster 18.467 ms no-Tracy run is useful evidence but is
not the matched headline.

### Asset hitching and residency

The demonstrated large desktop hitch was dominated by render asset
extraction/device materialization after image work completed, not by synchronous
source decode. The follow-up work on early demand, bounded materialization,
retained handles, registry preparation, and avoiding unnecessary uploads reduced
large observed stalls substantially, while also showing that loaded image
population/residency needs explicit ownership and budgets.

See [`engine/performance-and-iteration.md`](engine/performance-and-iteration.md)
and [`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md).

### Build/test iteration

Build and test throughput is an independent engineering concern. Recent evidence
supports resource-aware test concurrency, supported feature-combination checks,
clean-checkout/generated-artifact guarantees, targeted touched-crate sweeps, and
revisiting expensive dev-profile choices when the rebuild cost is small relative
to runtime/debug value.

## Highest-value architecture fronts

The current strategic order is:

1. **authoritative-state correctness** — rollback participation, semantic
   identity, deterministic composition, non-rewinding memory, and session/timeline
   lifetime boundaries;
2. **canonical construction/reconstitution** — remove second constructors and
   make transition/replay/restore consume one construction model;
3. **persistent-world semantics** — occurrences, residency, custody, and
   reload/re-entry behavior built on those foundations;
4. **measured presentation/runtime quality** — weak-GPU raster budgets, asset
   preparation/materialization/residency, and useful hitch observability;
5. **developer iteration** — build/test/profile configuration and supported
   composition gates;
6. **residual actor-kernel, capability, and SDK boundaries** — continue from real
   ownership/dependency pressure rather than size or speculative performance;
7. **multiview/multiplayer, reactive world, and richer authoring** — advance from
   concrete Ambition/TwinTrack/Smash customers.

The detailed ordering is in [`roadmap.md`](roadmap.md) and the Engine 1.0
capability map is in
[`engine/engine-1.0-architecture-program.md`](engine/engine-1.0-architecture-program.md).

## Current execution and decisions

The queue has been reduced to current work; completed investigations live in git
history rather than in the live ledger. Start with [`queue.md`](queue.md).

Questions that genuinely need Jon rather than engineering inference are in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md). Answered
rulings live in [`maintainer-decisions.md`](maintainer-decisions.md).

Dated GPT review files are evidence, not status. Phase 3 removes the closed dated
review reports from live planning; git history retains them. Any review finding
that still needs work must be promoted directly to the queue, tracks, a focused
plan, a maintainer decision, or Jon's direct-observation file. The routing rule is
part of [`README.md`](README.md), not a second review-status ledger.

## Product and engine customers

- **Ambition** is the flagship and primary architecture driver.
- **Super Smash Siblings** is a serious platform-fighter customer; current
  product truth belongs in its parity inventory rather than historical campaign
  diaries.
- **TwinTrack** pressures independent view/reference-frame and multiview
  architecture.
- **Sanic, Super Mary-O, Hollow Lite, and focused demos** remain useful acceptance
  customers for movement, collision, authoring, encounters, and presentation.
- The external-consumer fixture is the proof that public capability/package
  boundaries work outside the flagship composition.

## Where to look next

1. [`queue.md`](queue.md) — executable current work.
2. The focused plan linked by the selected row.
3. [`tracks.md`](tracks.md) — standing reservoir when the queue needs another
   verified item.
4. [`roadmap.md`](roadmap.md) — strategic ordering.
5. [`../README.md`](../README.md) — durable documentation map.
6. [`../reviewer-guide.md`](../reviewer-guide.md) — review procedure; current
   finding status must still be re-verified against HEAD.
