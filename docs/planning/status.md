# HEAD orientation

**Reviewed baseline:** `881310ec7` — `I1: the hand is the record of what is
equipped` (2026-09-02, later the same day). The rollback and item paragraphs
below were re-verified at this baseline; the performance model was rewritten at
`91d5d4a9c` (2026-09-02); other sections were last reviewed at `4e5f59cf`
(2026-08-30).

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
census of concrete gameplay component types. Closed 2026-09-02: non-rewinding
memory (S2), the three named selection sites (S3 — projectile victims now
tie-break on `SimId`), and the populated-timeline half of S1
(`rollback_populated_timeline.rs`: the event-created families resimulate
identically under BOTH oracles — the session checksum, which is blind to the 47
probed-only registrations, and `RollbackRestoreAudit`, which is not). The
remaining work is semantic identity across a rewind (only indirect today), S4–S6,
and the confirmed/external lifecycle boundary.

See [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md)
and [`engine/netcode.md`](engine/netcode.md).

### Construction is transactional; reconstruction still has more than one road

Prepared content and room construction use typed domain lanes under one
plan/preflight/commit/verify/publish transaction. Confirmed rollback room
transitions already wait for the same readiness/authorization transaction and
rebase onto a new frame-zero baseline; speculative rollback frames do not cross a
room boundary.

**Reconstitution** runs one constructor on every road: fresh construction, room
transition, same-room replay, new-game reset, and — since 2026-08-31 — a save
load, which informs its first construction with the file's occurrence ledger at
the activation edge instead of building a room and correcting it
(`engine/construction-and-reconstitution.md` C3). The same-room replay's
hand-kept reset ledger is deleted.

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

### The frame, measured on the shipped program (2026-09-02)

The shipped build (release optimisation, no Tracy, V-Sync Off) runs **250-310
fps in every room** on the reference desktop (i9-11900K + RTX 3090), including
the Hall of Characters once its art is resident. "Under 100 fps" was three
things stacked and none of them the shipped program: the dev build
(opt-level 1), `Fifo` v-sync at 144 Hz (any frame over 6.9 ms shows as 72; an
unfocused window shows 60), and — while profiling — the Tracy build's ~2.5x
per-frame span cost. There is no frame-rate campaign. See the top of
[`engine/performance-and-iteration.md`](engine/performance-and-iteration.md).

### Simulation CPU

The production rollback host ticks the full hall (129 actors) in 1.9 ms, linear
at ~7 us per actor per tick (`scripts/sim_scaling_curve.py`); the decision
pipeline is closed. The shipped host's whole main-world frame is ~3.4-4.5 ms
of which ~3.4 ms is a floor independent of the cast: `bevy_ecs` bookkeeping
over ~2400 system runs per frame, no hotspot, not the executor. Current
evidence does not fund physics rewrites or parallelizing `GgrsSchedule`; the
only lever on the floor is running fewer systems, and it is not needed at
144 Hz.

### Asset loading — the one user-visible performance problem

Entering the hall cost nine frames of 89-355 ms while 434 MP of Full-tier art
arrived AFTER the cover lifted. Root cause found and fixed 2026-09-02: the
reveal barrier never waited for the cast beyond the per-frame load ration
(`2c8f27b32`); the hall now realizes its cast at Quarter (`dc3cd0d91`). Both
await one host capture to confirm; see
[`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md).

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

Since 2026-09-02 an image's three stages are one ledger
(`ambition_asset_manager::image_stages`): demand → insertion → GPU
preparation, with `[image]` / `[image-gpu]` / `[image-dropped]` lines and
re-decode / dropped-before-upload counts on the census line. Its first hall
reading: every sheet the reveal demanded was prepared in ONE render frame
(the upload half of the hitch, unlimited `RenderAssetBytesPerFrame`), and the
intro cast's startup preload decoded ~26 MP on every boot that nothing draws.
⚠ **THAT PRELOAD ROAD IS GONE as of `301a07009` (2026-09-02, after this file's
stated baseline).** `load_intro_npc_sprites_system` published every intro NPC
sheet under its DISPLAY NAME while the world authors only `character_id`, so no
lookup could reach them; the system and the manifest rows it fed are both
deleted, and `extend_with_intro_sprite_entries` now adds intro PROPS only. ⛔ The
~26 MP is therefore a number for a road that no longer exists — it has NOT been
re-measured, so treat it as "was", not as current waste, until a host boot says
what the figure is now.
The hall-entry fix itself (reveal barrier + gallery tier cap) still awaits its
host confirmation; the tells are in `queue.md`.

Later the same day: sheet images load render-world-only by default
(`68d38076e`; captures byte-identical, peak RSS −141 MB in the hall at
Quarter on llvmpipe, `=0` restores the CPU copy for an A/B); a resident
character page must be owned by a live realization, guarded on the hall exit
(`124684f56`, 0 orphans); and ⛔ the headless composition had never decoded
an image — Bevy registers the `ImageLoader` in `Plugin::finish`, which
`App::update()` never calls — so every headless readiness/residency number
before `124684f56` was a number about the table, not the art. The no-window
builder finishes its plugins now; 746/746 app tests pass under it.

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
