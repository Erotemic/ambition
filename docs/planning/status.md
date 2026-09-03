# HEAD orientation

**Reviewed baseline:** `aa106cbe7` (2026-09-03 ~00:30Z, the second night's
gate: workspace 6933/6933, five of six jobs green, the sixth three Python tests
red on a missing Pillow in the tool venv — fixed in setup at `75fab498f`). The
asset and performance sections were re-verified at this baseline (Jon's
ruling on the room tier cap, the reveal barrier's host confirmation, the
attention budget); the rollback and item paragraphs at `881310ec7`
(2026-09-02) plus the world-item phase repair (`d220accee`); other sections
were last reviewed at `4e5f59cf` (2026-08-30). All five of those SHAs are
ancestors of HEAD, re-checked 2026-09-03 with `git merge-base --is-ancestor`
rather than `git cat-file` — an orphaned commit resolves under the second and
not the first.

⚠ **THE GATE LINE ABOVE IS NO LONGER REPRODUCIBLE, and the reason is not a
regression in what it measured.** As of 2026-09-03 the workspace job is RED on
two schedule tests added that morning
(`world_gating::tests::both_gate_solids_writers_are_scheduled_after_the_overlay_rebuild`
and `encounter_spawn_service::spawn_request_service_order::the_spawn_server_runs_after_the_wave_driver`,
`b67c1348f`). ⛔ **The systems they name are scheduled correctly** —
`WorldGatingSchedulePlugin` registers `contribute_encounter_lock_walls` — but the
tests look systems up by `system.name()`, which Bevy 0.19 strips to a
placeholder unless `bevy_ecs`'s `debug` feature is on, and it is enabled nowhere
in this workspace. So "workspace N/N" is not a number this page can carry
forward until those two are converted to the count-by-shape form the carve
checklist documents.

⭐ Re-verified at `6d2327903` (2026-09-03): the actor-monolith section below, the
only part of this page this pass re-read against the code. Everything else keeps
the baseline it was last reviewed at.

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

⚠ **THE PLUGIN-REGISTRATION HALF OF THAT SENTENCE IS SPENT (2026-09-02).** The
kernel makes five foreign plugin registrations and all five are accounted for, so
there is no misplaced one left to move; and the developer-tools edge — the last
production dependency the kernel held on a crate above it — is gone, enforced by
the manifest rather than a test. ⛔ What remains is INTERNAL decomposition (the
kernel's own `items/` module), which ends with no edge to point at, so do not
size it against the earlier slices. The owner doc carries the measurement and
why a reference count cannot find this.

⚠ **THAT PARAGRAPH WAS OVERTAKEN THE NEXT DAY. Five crates left the kernel on
2026-09-03**, and the one it named as "internal, no edge to point at" — the
`items/` module — was the first of them:

| crate | what left |
|---|---|
| `ambition_held_items` (`bbfa38a3d`) | the PRESSED collectible: `items/pickup` |
| `ambition_body_seed` (`962dba34d`) | `ActorClusterSeed`/`ActorMotionPath`/`ActorBody` |
| `ambition_match` (`7e625e5a5`) | the versus match, prepared |
| `ambition_encounter_features` (`b67c1348f`) | the room-feature side of encounters |
| `ambition_abilities` (`4c31111f9`) | the WIELDED ability kit |

**The kernel's own source went 112,733 → 101,042 lines in that day, −11,691.**

⛔ **AND THE METRIC THIS PAGE USED TO WATCH MOVES THE WRONG WAY.** The monolith's
`[dependencies]` table went 29 → 33 across those same carves, because a kernel
that stops CONTAINING a domain starts DEPENDING on it — it keeps the rollback
ledger, the checkpoint policy, or an inter-crate schedule edge, each of which
needs the crate named in the manifest. ⇒ Reading the dependency table for carve
progress reports every success as a regression. What shrinks is the SOURCE; what
grows is the manifest; both are the same event.

⇒ So the honest statement is the opposite of the one above: **the carve had
plenty of edge left, and finding it needed a per-module ownership question
rather than a reference count.** Two of those five also proved a directory can
hold two unrelated families —
[`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md)
carries the measurements, including why `possession`/`teleport`/`trapdoor`/
`flyline` stayed behind in a directory named `abilities/`.

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
(`2c8f27b32`). ⊙ **A SECOND CAUSE was measured 2026-09-03 and the barrier is
NOT it**: `converge_character_residency_to_active_quality` demotes every IN-USE
sheet Ready→Declared when the profile converges, un-claiming each body's render
family and re-demanding at one character per frame at Full. Host-confirmed the same evening on the two tier-independent
tells (0 placeholders, 0 frames over 33 ms after the transition, Ultra, 3090,
`desktop-timeline-run-20260902T215256Z`). ⛔ The room tier cap that shipped
beside it (`dc3cd0d91`, gallery → Quarter) blurred the hall at Ultra and Jon
rejected it on sight — "no lower quality tier for gallery previews"; removed
in `06a494f4e`, and the standing rule is that nothing draws fewer pixels than
the user's setting for any room, view or distance reason. Room exits now
retire realizations by OWNERSHIP (destination cast ∪ worn ∪ one-hop
neighbours) instead of by tier mismatch; a fighter's own vfx sheets follow its
realization (4 core sheets at boot, was 13); parallax themes retire the same
way. Still owed on the host: the hall's cover hold and post-reveal frame count
at FULL (the confirmed run was under the cap), and the UNCOVERED startup burst
Jon's run shows instead (98 images / 83 MB, 203 + 125 ms frames before the
first `room-loaded`). See
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
stated baseline).** `load_intro_npc_sprites_system` published every intro NPC <!-- cite-ok: deleted 301a07009; the row records it -->
sheet under its DISPLAY NAME while the world authors only `character_id`, so no
lookup could reach them; the system and the manifest rows it fed are both
deleted, and `extend_with_intro_sprite_entries` now adds intro PROPS only. ⛔ The
~26 MP is therefore a number for a road that no longer exists — it has NOT been
re-measured, so treat it as "was", not as current waste, until a host boot says
what the figure is now.
✔ **The reveal barrier's COUNT tells are CONFIRMED on the host (2026-09-02
evening, `desktop-timeline-run-20260902T215256Z`)**: placeholder warnings
111 → **0**, and `asset_wait_ms` 3 → **292** — the cover is up for the wait.
⛔ **BUT "both are tier-independent" IS FALSE, MEASURED 2026-09-03.** Two headless
`capture_scene hall_of_characters` runs differing only in
`AMBITION_QUALITY_PROFILE` gave **0 placeholders at Potato and 129 at Ultra**.
The count is tier-SCALED, because `materialization_units` charges Full 16 against
Quarter/Potato 1 against a per-frame budget of 16 — one character per frame at
Full, sixteen at Potato — and the warning trips after 5 consecutive unclaimed
frames, so only a long ramp trips it. ⇒ The host run read 0 while the gallery tier
cap was shortening that ramp by 16×, which is the same confound this section
already flags for the TIMING tell. `asset_wait_ms` is unaffected and remains a
genuine tell. See `engine/asset-preparation-and-residency.md` for the mechanism
and for ambition-df's code reading of what un-claims the bodies
(`converge_character_residency_to_active_quality`, not the barrier).

⚠ **THE TIMING TELL IS NOT CONFIRMED FOR THE SHIPPED PROGRAM, and this is not a
formality.** That run also had the gallery tier cap, which was cutting the very
decode load the barrier holds the cover for — the two fixes were measured
together and only one of them survives Jon's ruling. So "frames over 33.4 ms
after the last transition 9 → 0" is a fact about a program that no longer
exists, and a re-walk at the user's tier owes that number. `queue.md` carries
the checklist. Detail in `engine/asset-preparation-and-residency.md`.
⛔ **And the "gallery tier cap" half of this sentence is spent**: Jon removed the
room-level sprite tier cap on 2026-09-02 (*"I DO NOT WANT A LOWER QUALITY TIER
FOR GALLERY PREVIEWS"*), so there is no tier-cap change left to confirm. The
tier is the user's setting everywhere.
⇒ What the same run says is still open is STARTUP, not the hall.

Later the same day: sheet images load render-world-only by default
(`68d38076e`; captures byte-identical, peak RSS −141 MB in the hall at
Quarter on llvmpipe, `=0` restores the CPU copy for an A/B); a resident
character page must be owned by a live realization, guarded on the hall exit
(`124684f56`, 0 orphans); and ⛔ the headless composition had never decoded
an image — Bevy registers the `ImageLoader` in `Plugin::finish`, which
`App::update()` never calls — so every headless readiness/residency number
before `124684f56` was a number about the table, not the art. The no-window
builder finishes its plugins now; 746/746 app tests pass under it.
And the shell route's FIRST room decodes its cast and the player's own sheet
BEFORE the route activates (`aca57e636`, work item `prepare-first-room-art`
on the standard platformer plan; host captures showed the 7.6 MP player sheet
decoding 0.15 s after every first `room-loaded` as a 67-79 ms frame); the
GPU readiness term is positive proof (`8bd19f890`); two hub↔hall laps return
the identical working set (6 realizations / 16 pages / 13.4 MP) and a quality
round trip converges back with every page loaded (`a1c03c179`).

⭐⭐ **AND THE LEDGER HAS A FOURTH STAGE SINCE 2026-09-02: RESIDENT USE.**
`first_drawn` is stamped from the render world's `ExtractedSprites` (first write
wins, or the stage's own cost lands in what it measures), so the census can say
what was DRAWN and not only what arrived — `[image-drawn] … demand→draw NNNms via
<road>` and `never drawn N (M MP: by owner)`. Two rules a reader must carry: the
row prints `-` rather than `0` without a render world, because "nobody could draw
anything" is not "nothing was drawn"; and `PROCEDURAL` rows can never be
extracted at all, so their megapixels on that row are the instrument, not waste.

Its first findings, all Potato tier (llvmpipe classifies as `Cpu`, so these
megapixels are NOT comparable with the 434 MP Full / 38 MP Quarter figures above
— the SHAPE is what carries across):

- **the hall draws 14 of 138 resident cast pages** — the headroom neither tier
  mechanism may now spend. ⛔⛔ **RULED 2026-09-02, and the ruling is wider than
  the two mechanisms it settles:** *"I DO NOT WANT A LOWER QUALITY TIER FOR
  GALLERY PREVIEWS."* Jon saw blur in an Ultra host run where the hall drew from
  `sprites_0_25x` — the room-level cap doing exactly what it was built to do. The
  cap is being removed and the view-scoped tier that was *"blocked on a feel
  ruling"* is answered NO, not pending. ⇒ **The standing rule: nothing may draw
  fewer pixels than the user's quality setting asks for, for any room, view or
  distance reason, without Jon's explicit yes.** ⚠ The headroom is still real and
  still measured; what it licenses is deciding what to KEEP RESIDENT, never what
  to draw at lower resolution;
- **the FX set is 13 sheets / 9.4 MP loaded at boot in every room, and 76 of its
  196 rows are named by nothing** — including all 14 of the admiral's and 20 of
  George's, whose art was plainly drawn for their kits while their movesets ask
  for generic effects. A missing-wiring finding, not dead art;
- ⛔ **two road MIS-STAMPS have now been found this way** (vfx as
  `character-sheet`, and 28 entity icons as `fx-sheet` — which is why the FX
  number read 41 before it read 13). The remaining stamps were swept and are
  correct. Nothing else in the pipeline notices a bad stamp: the art loads, draws
  and frees correctly, and only the ATTRIBUTION is wrong — which is the entire
  product of the residency work.

See [`engine/performance-and-iteration.md`](engine/performance-and-iteration.md)
and [`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md).

### Build/test iteration

Build and test throughput is an independent engineering concern. Recent evidence
supports resource-aware test concurrency, supported feature-combination checks,
clean-checkout/generated-artifact guarantees, targeted touched-crate sweeps, and
revisiting expensive dev-profile choices when the rebuild cost is small relative
to runtime/debug value.

⛔ **AND THE BINDING CONSTRAINT ON THIS BOX IS NOW DISK, measured 2026-09-03.**
`target/debug/deps` alone is **141 GB**: every feature job builds its own variant
of the graph, cargo never prunes the last one, and five crates were carved out of
the actor monolith in a single day — each multiplying the variants a feature job
resolves. The decomposition campaign pays for itself in disk.

| what | cost |
|---|---|
| `check_disk_headroom.py` floor | **40 GB** (refused at 39.1) |
| `./run_tests.sh --rust` (5 jobs) | ~5 GB, 42 → 37 GB |
| one `cargo test --workspace` | **14 GB in under 3 minutes** |
| the exhaustive plan (49 jobs) | **68 minutes**, and it exhausted a 290 GB volume mid-run |

⚠ The headroom guard runs ONCE before the first job and once after the last,
never between — so a long suite that starts above the floor can still die of
ENOSPC halfway and report it as a link error. That is the shape of three of the
exhaustive run's seven failures.

⇒ Practical consequence for anyone orienting here: **prefer `cargo test -p` over
a lane.** Tonight's abilities carve was verified crate by crate for exactly this
reason, and it gave a sharper answer than the lane would have — the lane's one
red belongs to somebody else's tests. Reclaim order, all regenerable:
`target/debug/incremental`, `profiling`, `release`, `wasm32-unknown-unknown`,
`outlander`. Past those the only lever is `cargo clean`, which costs every
session's warm tree and is a coordination decision rather than a local one.

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
   composition gates. ⚠ Ranked fifth by VALUE and currently first by
   CONSTRAINT: as of 2026-09-03 this box cannot run its own full suite without a
   clean (see Build/test iteration above), so the gates that would validate 1–4
   are the thing that does not fit. Recorded rather than reordered — the
   strategic order is Jon's, and a constraint is not a priority;
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
  Their acceptance lists are owned by [`demos/sanic.md`](demos/sanic.md),
  [`demos/super-mary-o.md`](demos/super-mary-o.md) and
  [`demos/hollow-lite.md`](demos/hollow-lite.md) — this page defers to them and
  must not restate their status.
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
