# Portal capability and motion-admission boundary review — 2026-11-09

Status: **P01–P06 and P08–P10 implemented; P07 acceptance remains open**.
Code is committed in `4afbf6100` and `9e626f08b`. See the
[capability and publication receipt](#capability-and-publication-packet) for tests,
measurements and failures. This review does not modify the shared task queue.

## Purpose and evidence

Review the portal work from 2026-11-02 through 2026-11-09 for wrong ownership, unwanted dependencies, partial updates, unverified behavior, and avoidable data duplication. The first reported defect was that Mary-O imports an API named `portal_passages::PoseAdmission` although Mary-O has no portal mechanic. Correct the **architecture**, not only the import string. The review also includes older defects discovered while tracing that reference. Do not attribute those older defects to this week's portal agent.

The reviewed source archive is `ambition-agent-source-20261110T004608Z-9f2a8fa1a675.tar.gz`, with Git HEAD `9f2a8fa1a675444e4a779b304206dff7b964f07d`. All path and line references below refer to that revision. Use symbol search and `git blame` after checking out a newer tree; line numbers may move. The source archive's manifest says the source checkout had dirty files when it was made, but the reviewed extracted Git tree is clean. Reconcile current work by other agents before applying changes. No new Cargo build or runtime validation was performed for this review; test results cited below are those recorded in the relevant commits and planning receipts.

Read before modifying code:

- `AGENTS.md`; `docs/reviewer-guide.md`; `docs/architecture/engine-architecture.md` (domain ownership and assembly); `docs/concepts/one-body-one-path.md`.
- `docs/adr/0024-frame-aware-unified-movement-kernel.md` (pose and motion authority); `docs/adr/0020-mounts-and-vehicles.md` (saddle ownership).
- `docs/planning/engine/portal-parts-similarity-and-chirality.md` (current, partly open portal geometry and acceptance work); `docs/planning/engine/portals-a-body-in-two-places.md` (earlier portal and same-tick transit accounting).
- The specific source and test files identified in each issue below.

**Key commits:** `ded3301` (2026-11-08, native similarity partitions and admission), `63ab8ed` (2026-11-08, canonical roots and budgets), `7b2a306` (2026-11-08, material-owner rays), `be364eb` (2026-11-09, host travel and material snapshots), `821ce15` (2026-11-09, loose-item body state), `9098b5d` and `46058c2` (2026-11-09, time-aware motion and refusal), `0aeb65a` and `b7e11ad` (2026-11-09, canonical transit and source rotation), and **`9f2a8fa`** (2026-11-09, external pose migration). Do not assign a change to a specific commit without confirming its patch or blame. In particular, `HiddenLoadingZonesAppExt` existed before these commits (`f2014af`, 2026-11-01, is the most recent earlier Mary-O change found by `git log -S`).

## Required architectural outcome

1. Games request **body movement**, **room travel**, **loading-zone appearance**, **combat volumes**, and **portal simulation** through interfaces owned by those domains.
2. A game that does not offer portals has no source-level import of portal-specific names for routine body movement or room presentation. A standalone build should not select the portal mechanic unless another explicit dependency needs it.
3. All pose-changing authorities have a single, testable path that either admits a complete candidate or leaves the previous accepted physical state intact. Gameplay relationships, clocks, holds, and presentation consequences must follow the same admission result.
4. Portal simulation, portal construction/gun, portal rendering, and portal-authored LDtk conversion remain individually selectable to the extent that their actual dependencies permit. A host can support both portal-equipped and portal-free games without cross-room state leakage.
5. Keep the stronger functionality already implemented: canonical material transforms, exact aperture and source-curve queries, finite/numeric refusal, rollback-safe records, and per-body budgets. Do not replace the new math with unvalidated old approximations.
6. Use explicit, discoverable, domain-specific Rust names. Do not add generic module-level functions that duplicate method names elsewhere. Do not add compatibility re-exports merely to keep old `portal_passages` imports compiling; migrate consumers and remove the obsolete API location.

## Issue index

| ID | Priority | Evidence | Problem |
|---|---|---|---|
| P01 | High, architectural | Confirmed | Generic `PoseAdmission` lives under `portal_passages`; the Nov 9 commit migrated Mary-O and several generic domains into it. |
| P02 | High, correctness | Confirmed control flow; failure scenario needs test | Refused external poses can leave sequence, relationship, or hold state ahead of accepted motion. |
| P03 | High, architectural | Confirmed | Core movement uses `portal` / `portal_refusal` for admission and generic numerical faults. |
| P04 | Medium, correctness | Confirmed fallback; impact to verify | Missing per-body material snapshot is replaced by identity during geometry queries. |
| P05 | Medium, performance/ownership | Confirmed repeated work; no benchmark | Portal publisher reconstructs/copies body material and room/link data in multiple phases. |
| P06 | Medium, composability | Confirmed | Runtime `PortalSchedulePlugin` installs simulation **plus gun** and tests that as mandatory. |
| P07 | High, release gate | Recorded failures and open scope | Portal acceptance includes two failing app tests and several unfinished physics paths. |
| P08 | Medium, architectural, older | Confirmed | Generic loading-zone visibility is owned by `gate_portal_visuals` and global zone-name policy. |
| P09 | Medium, build layering, older | Confirmed manifests; closure must be measured | Mary-O opts into `all_capabilities`, which includes portal crates; other feature bundles also imply portal support. |
| P10 | High, integration | Design test gap | Verify portal state is isolated per live room and game within a mixed host, including publish/clear and capability sets. |

The priorities describe what an agent should inspect and implement first. Only the lines marked as confirmed have been directly established from code or the cited test receipt. A possible failure is not described here as an observed gameplay failure without a reproducer.

---

## P01 — External pose authority has portal ownership

**What was introduced:** `9f2a8fa` adds `PoseAdmission` in `crates/ambition_platformer2d_shared_tangle/src/portal_passages.rs:324-353`. The new `SystemParam` has three fields: `Option<Res<PortalPassages>>`, `LiveRooms`, and a query for `CarriedAcceleration`. Its `constrain(...)` method calls `ambition_platformer2d_core::movement::constrain_body_pose(...)` with `passages.motion_for(body, live.of(body))` when the optional resource exists. The underlying primitive is in `crates/ambition_platformer2d_core/src/movement/authority.rs:208-261`. That primitive constructs and validates a candidate sweep/material cast, then writes `BodyKinematics` and `SweepSample` only after success.

**Direct consumers introduced by that migration:**

- Mary-O: `game/ambition_demo_mary_o/src/flag.rs:595,667`; `cosmic.rs:247`; `castle_end.rs:377,527`.
- Generic engine: `crates/ambition_platformer2d_runtime/src/posed_limbs.rs:24`, `fall_rescue.rs:33`; `crates/ambition_platformer2d_actor_monolith/src/body/pet.rs:178,220,369`, `talk_spacing.rs:73`.
- Other capabilities: `crates/ambition_mount/src/lib.rs:754`; `crates/ambition_combat/src/capture/systems.rs:2808,2898,2923`; `crates/ambition_boss_encounter/src/conduct.rs:296,704`.

**Correct the precise diagnosis:** These consumers do **not** directly import the `ambition_portal2d` crate. `portal_passages` is in `ambition_platformer2d_shared_tangle`, re-exported through the facade. The problem is that generic pose admission is named, located, and assembled as portal-specific data. Renaming the import without changing the ownership and state inputs does not resolve it.

**Recommended shape:** The body/movement domain owns a stable `BodyPoseAdmission` (or another specific equivalent) `SystemParam` or service. It owns the complete pose operation and refusal outcome. The optional portal domain publishes **only the extra spatial admission information** consumed by that service. A portal-free composition supplies the neutral/empty admission context. The generic body transaction must work with no portal resources installed and for transformed bodies with valid material state. It must not require game content to query `PortalPassages` or determine which portal pairs are live. Portal math may remain in core where pure geometric processing is used by several mechanics; distinguish math primitives from the optional portal policy and Bevy resource.

Before defining a new provider interface, inspect `frame::thread::MotionAdmission`, `frame::material_cast::MaterialCast`, `movement::constrain_body_pose`, `LiveRooms`, and current `PortalPassages::motion_for`. Prefer one cohesive snapshot/value contract over a dynamic provider registry or additional shadow resource. The producer must preserve body/room specificity, carried acceleration, budget limits, and deterministic refusal semantics. Consider whether a neutral geometry-admission context can be shared by `step_motion` and external pose admission rather than built twice.

**Done when:** No generic Mary-O, mount, pet, capture, fall-rescue, or boss content imports a portal-named API merely to constrain an ordinary pose; portal-equipped movement still enforces the same apertures, budgets, and material maps; portal-free pose tests pass without a `PortalPassages` resource; old API and re-export are deleted (no migration shim).

## P02 — Pose refusal is not propagated through the owning gameplay transaction

The primitive is atomic **only for `BodyKinematics` and `SweepSample`** (`authority.rs:239-260`). It does not automatically roll back the caller's unrelated state. `PoseAdmission::constrain` logs failures at trace and returns `Result`; most consumers reduce that result to `is_err() { continue; }` or `is_err() { return; }`. Audit **every** caller for state changed before the call, for other bodies pinned in the same operation, and for state that remains active after a refusal.

**Concrete call-site cases:**

1. **Mary-O flag:** `game/ambition_demo_mary_o/src/flag.rs:624-635` runs `step_flag_sequence(&mut sequence, ..., time.sim_dt())` **before** `drive_her`; `drive_her` returns early on pose refusal at `:674-675`. The sequence can advance while the body does not. It has already made/released control holds earlier in the system. The state machine must commit its tick only when its required movement is admitted, or have explicit, tested refusal/recovery semantics.
2. **Mounts:** `crates/ambition_mount/src/lib.rs:795-854` pins the rider to a saddle; on refusal the `RidingOn` relationship persists and the rider does not follow. A rider's position, ground state, and AABB are updated only on success, which avoids one partial write but does not resolve the relationship failure. Choose whether to retain the previous pose, detach, defer, or change its authority state; test after moving the mount through an admitting/refusing boundary.
3. **Captures:** `crates/ambition_combat/src/capture/systems.rs:2826-2880` keeps `CapturedBy` on refusal, without moving the captive to the captor. The follow-on ground/AABB writes occur only on success; the relationship still asserts a hold that may be physically invalid. Keep captor/captive contact, hit boxes, and release semantics coherent. The comment at `:2828-2831` says a missing captor must be released elsewhere; do not make the projection system a second authority on release without changing the ownership design.
4. **Petting, two-body partial commit:** `crates/ambition_platformer2d_actor_monolith/src/body/pet.rs:428-435` removes `CommandedMove` and calls `stop_where_it_stands` on **petter and petted sequentially**. If the first is admitted and the second refused, the first body has changed while the interaction does not advance to `Gesture`. The current short-circuit expression is not atomic across the two bodies. Prepare and validate both candidates before committing either, or define an explicitly coherent fallback. Also check request setup at `:330-355` and ongoing gesture constraints.
5. **Boss held limb:** `crates/ambition_boss_encounter/src/conduct.rs:321-339` calls `claim_pose_hold(...)` **before** attempting an admitted pose. On refusal the hold may be claimed despite no accepted move. The `None` branch similarly releases the hold before attempting the falling-velocity update. `conduct.rs:726-744` has the same claim-before-admit pattern for a boss pose. A rejected request must not silently change the authority that the next tick observes.
6. **Mary-O castle/lava:** `game/ambition_demo_mary_o/src/castle_end.rs:549-590` skips sinking and splash on a refused velocity pin. `castle_end.rs:377` and nearby sequence/door logic also use the helper; inspect its state transitions. In `cosmic.rs:246-269`, a refused sky-ceiling pin leaves an out-of-bounds body for the next tick. These need defined behavior; do not claim every refusal is a defect without a scenario showing it matters.
7. **Other migrated producers:** `crates/ambition_platformer2d_runtime/src/fall_rescue.rs:88-109` admits the velocity change before spawning a ferry (good local ordering). `runtime/src/posed_limbs.rs:41-70`, `boss_encounter/src/conduct.rs:294-345,704-748`, `actor_monolith/src/body/talk_spacing.rs`, and pet `apply_pet_requests` must be inspected for prior claims or multi-entity updates.

**Recommended shape:** A caller prepares the *desired* pose and related state transition, receives a typed admission result, then commits both according to the relationship's single owner. For coordinated bodies, validate all necessary candidates before any commit; reuse the engine's core validation rather than cloning an independent portal-aware evaluator. A failed move is a **result that the owner handles**, not merely a trace. Decide per mechanic whether to retain current progress, cancel, detach, retry, or transition to a blocked state. Record refusal reason where it can help testing and diagnostics. Do not add an ad hoc `if portal` branch to each gameplay system.

**Regression tests:** Force `PieceBudget { max_hops: 0, .. }`, a material numeric refusal, and a room with no portal capability. Verify (a) the flag clock does not outrun accepted position; (b) a mounted rider and captive have a coherent relationship after denial; (c) two-body pet transactions are all-or-none or have a documented fallback; (d) rejected boss limb/boss poses do not change holds; (e) no invalid `CenteredAabb`, ground/contact fact, or `SweepSample` is published. Also test success after the restriction is lifted. This is about behavior and rollback, not only checking `Result::is_err()`.

## P03 — Generic movement APIs report portal-specific failures

**Evidence:** `crates/ambition_platformer2d_core/src/movement/kernel.rs:22-25` defines `MotionStepContext.portal: Option<frame::thread::MotionAdmission>`. `kernel.rs:174-202` tests candidate material travel and writes `events.portal_refusal` after an admission failure. `kernel.rs:206-235` sets **the same** `portal_refusal` for a numerical fault when authored motion parameters cannot be scaled, even if no portal link is present. `crates/ambition_platformer2d_core/src/movement/events.rs:91-95,117-123` defines and merges the field. Tests such as `movement/kernel/tests.rs:1884-1924` expect a numerical overflow under `portal_refusal`.

**Why this is the wrong seam:** A generic core motion step can fail because of material/numeric validity regardless of portal mechanics. A field named `portal_refusal` makes callers treat a broad motion validity result as one optional capability's event. Also, `MotionAdmission` currently carries portal `links`, `first`, `budget`, and `convention`: a name alone cannot turn this into a neutral provider interface.

**Recommended shape:** Give the core step a generic admission/validation result and typed reason that can retain the specific portal path/link when one was involved. Decide whether that belongs in `MotionStepResult` or `FrameEvents` based on whether the result is an authoritative step outcome or a downstream event. Preserve the distinction between a no-motion accepted step, a rejected candidate, and a semantic portal refusal. Audit both movement policies and all readers of `portal_refusal`, not only the producer. Keep the first-refusal merge order deterministic. A proposal must state which layer owns material validity versus optional aperture policy and how it handles numeric faults with **zero** active portals.

**Done when:** Generic movement uses generic names and type semantics, the portal plugin can attach precise portal refusal details without owning the whole core event contract, and tests cover both numeric failure without portals and a portal hop/budget refusal. No duplicate refusal stores or legacy aliases.

## P04 — Missing material snapshot silently uses identity

**Evidence:** `crates/ambition_platformer2d_shared_tangle/src/portal_passages.rs:232-259` handles `PartSource::Body` by looking up `self.materials[body]`, then using `.unwrap_or(Ok(Sim2::IDENTITY))`. `PartSource::World` correctly uses world material; `PartSource::Body` claims to use that body's material map. `portal_passages.rs:304-320` tests an explicit, populated transformed map but not the missing-body case.

**Possible failure:** A newly spawned, moved, resized, or rotated body may be queried before the publisher has added/updated its transform. The body source then becomes identity material without an error. Its volume/carry may be wrong in a portal cut, and lack of a key cannot be distinguished from a *known* identity transform. Do not claim this timing already occurs: reproduce from real query/schedule order first.

**Fix options:** Source the current authoritative material map from body state when needed, or represent a missing map as a typed error for `PartSource::Body`. Keep identity only when it is the actual transform or `PartSource::World` is requested. If a snapshot is required to avoid mid-frame mutable access, define its publication phase and prove no consumer can read stale/missing state. Test absent body, known identity body, transformed body, newly spawned body, and query after transit/room transfer.

## P05 — Full portal snapshot rebuild and duplicate derived state

**Evidence:** `crates/ambition_portal2d/src/passages.rs:30-73` assigns a fresh `PortalPassages` each publish, gathers `PortalPieceBudget` policies and all body materials, collects links by room, detects rings, and writes straddlers. `passages.rs:75-111` refreshes material maps and straddlers after transit; `write_materials` calls `clear_materials()` and writes every body again. `crates/ambition_platformer2d_shared_tangle/src/portal_passages.rs:60-75,94-100,125-166` stores a `BTreeMap<Entity, Result<Sim2,...>>`, room vectors, linear-search per-body budgets and straddlers. This is duplicated *derived* representation of material and live portal state.

**Status:** The repeated construction/iteration is confirmed. No cost or regression was measured in this review. A `BTreeMap` or vector search is not by itself a defect at current world sizes.

**Work:** Define who owns the canonical body-material transform and publication epoch. Measure `publish_portal_apertures`, `publish_portal_passages`, and consumers over representative body/room/portal counts, including a mixed-game host and portal-free room. Determine whether one immutable per-tick snapshot or a scoped incremental update reduces repeated work **without** invalidation bugs or rollback nondeterminism. Avoid an independent cache until its lifetime and update triggers are proven. Preserve ordering for numeric faults and portal pairs. Include data size and allocation counts as well as duration when possible.

## P06 — Runtime undoes the simulation/gun composition split

**Evidence:** `crates/ambition_portal2d/src/plugin.rs:15-35` already has `PortalSimulationPlugin` and `PortalGunPlugin` separately; `PortalPlugin` composes them. But `crates/ambition_platformer2d_runtime/src/portal_schedule.rs:20-28` installs `PortalPlugin` unconditionally when the runtime portal feature is chosen. `portal_schedule.rs:167-200` contains a test named `installing_portal_simulation_also_installs_the_gun_capability_it_can_construct`; it says the construction lane is compile-time gated under `portal`, so it insists on a gun schema. The source comment itself proposes a runtime capability token if simulation-only is supported.

**Problem:** A game with an authored pair or script-controlled aperture cannot use the standard portal runtime composition without installing the gun vocabulary, messages, construction schema, and maintenance. The existing lower-level API says simulation-only is supported, but the usual engine assembler does not expose that choice.

**Fix:** Separate *portal simulation selected* from *portal-gun construction selected* in runtime assembly, schema registration, construction dispatch, and authored-content validation. Use a capability token or comparable explicit declarative decision, carried into the construction context and content fingerprints. Do not merely remove the test while leaving a compile-time `PortalGunConstruction` arm available without its registered schema. Keep a full convenience composition for Ambition itself. Test simulation-only authored portals, simulation+gun, and no portals, including content-plan fingerprint equality across two runs.

## P07 — The portal acceptance envelope is still incomplete

**Direct source:** `docs/planning/engine/portal-parts-similarity-and-chirality.md:597-640,720-725,868-880`; `docs/planning/engine/portals-a-body-in-two-places.md:120-127`; the tests below. `9f2a8fa` records core 855 passing (one ignored), shared 354, portal 142, GNU-ton 39, and a broader app portal selection with **34 passing and 2 failing** tests. The two failures are:

- `game/ambition_app/tests/portal_translation_camera_continuity.rs:273` — `c141_to_c140_preserves_screen_position_and_continues_right`: discontinuity.
- `game/ambition_app/tests/portal_translation_camera_continuity.rs:496` — `thin_wall_walk_keeps_apparent_player_position_smooth`: no crossing.

The earlier record `dev/journals/portal-camera-continuity-regression-2026-07-21.md` mentions related test failures months before this work. Compare current baseline and Git history before assigning regression blame; the Nov 9 commit says baseline comparison is pending. A failed test is a release gate regardless of attribution.

The planning document also leaves open persistent root material rebasing, same-tick remaining motion after crossing, moving-host admission, physical righting, native actor/contact projection, some source curves and attack queries, unequal-aperture acceptance, and full replay/render/performance and PSIM acceptance. These are implementation tasks, not proven bugs of the last commit. The Nov 9 source work changed a wide set of consumers before all these gates were closed. Preserve prior accepted calculations while finishing the missing cases.

**Work:** Baseline the two failing app tests before and after the relevant commits; save exact commands/results. Reproduce with headless or probe harnesses and inspect assertions against intended world/screen continuity. Fix or explicitly re-scope the tests only when supported by the simulation contract. Track open work in its existing planning owner rather than copying a second task queue into this review; this review's checkboxes are acceptance requirements only. Cross-check the latest portal planning ledger when work begins because other agents may have advanced it.

## P08 — Generic loading-zone presentation lives in gate portal rendering (older issue)

**Evidence:** `game/ambition_demo_mary_o/src/scenery.rs:119-129` imports `ambition_platformer2d::render::rendering::gate_portal_visuals::HiddenLoadingZonesAppExt`, then calls `app.hide_loading_zone(zone.id)` for every Mary-O loading zone. `crates/ambition_render/src/rendering/gate_portal_visuals.rs:14-47` defines `HiddenLoadingZones(HashSet<String>)` as a global `Resource` and uses it alongside `GatePortalRegistry` to hide render entities. `crates/ambition_render/src/rendering/world.rs:54-74,82-90,172-181` consumes the same global set to **skip spawning** zones, including nameplates. The two paths have different lifetimes: initial spawn suppression versus hiding already created visuals.

**Correct attribution:** This existed before the Nov 8–9 portal motion change. It is a separate historical ownership problem exposed by the Mary-O grep. The type is in `ambition_render`, not `ambition_portal2d`.

**Problem:** Visibility policy for any loading zone is named after gate portals and keyed only by an unscoped `String` zone ID. Different game providers/rooms can reuse an ID; a global set may then suppress unrelated zones. The render system also conflates a gate sprite replacing a portal-zone picture with game-authored 'no picture' presentation. A `secret_` zone affects discovery semantics as well; do not collapse this into a generic `hidden` flag without evaluating those semantics.

**Fix:** Give each authored loading zone an explicit presentation policy (or room-scoped presentation configuration) owned by room/world/presentation code, not gate portal rendering. Gate portals can select their own render policy through the same data. Scope by game/room/zone identity where needed; prefer authorship in LDtk/content rather than a global registration loop. Preserve the existing distinct rules for visibility, nameplates, secret discovery, and dynamically opened/closed gate sprites. Remove `HiddenLoadingZonesAppExt` and its global type once migrated, with no obsolete alias. Test duplicate zone IDs in two live rooms, Mary-O's hidden sky/arrival zones, visible normal doors, a secret passage, and gate portal sprites.

## P09 — Mary-O's build advertises every capability (older issue)

**Evidence:** `game/ambition_demo_mary_o/Cargo.toml:35` and `game/ambition_demo_mary_o_app/Cargo.toml:47` both specify `default-features = false, features = ["all_capabilities", ...]`. `crates/ambition_platformer2d/Cargo.toml:73-95` defines `all_capabilities`, including `ambition_portal2d` and `ambition_portal2d_presentation`. `:147-168` has individual `portal_ldtk`, `portal`, and `portal_render` feature forwarders. `:117` defines `visible_web_base` to include `portal_ldtk`. `crates/ambition_platformer2d_ldtk/Cargo.toml:9-14` itself defaults to `ldtk_runtime` plus `portal_ldtk`. The actor monolith manifest explicitly says its `visible` capability should not add portals (`actor_monolith/Cargo.toml:98-111`), which makes the facade's broad feature set worth revisiting.

**Nuance:** Linking portal crates does not prove portal simulation runs inside Mary-O; runtime installation is feature-/capability-dependent (`runtime/src/lib.rs:683-687`, `runtime/src/portal_schedule.rs:144-162`). The facade default is still broad, and Cargo features unify across a build. Tests can accidentally force a feature by a separate dev dependency (`game/ambition_demo_mary_o/Cargo.toml:54-56` enables `test-support` with default features). Record **normal**, **test**, and assembled-host closures separately.

**Fix:** Replace Mary-O's `all_capabilities` with the actual named capabilities required for its world, input, rendering, content and host. Inspect indirect feature defaults and game-specific target features rather than guessing a short list. Confirm that no `portal_ldtk` or gun schema is required merely for Mary-O pipes, which are ordinary directed room travel. Do not change `all_capabilities` globally to fix Mary-O. If `visible_web_base` is intentionally a portal-bearing bundle, either document its purpose or split it into neutral web presentation plus explicitly selected portal authoring. Keep compile-only and runtime-capability tests distinct.

**Evidence command (run on real checkout, and record before/after):**

```sh
cargo tree -p ambition_demo_mary_o -e normal -i ambition_portal2d
cargo tree -p ambition_demo_mary_o_app -e normal -i ambition_portal2d
cargo tree -p ambition_demo_mary_o_app -e features -i ambition_portal2d
```

Check each package's actual target/feature closure and account for workspace feature unification; run `cargo tree --help` for target filters if required. An empty inverse tree in an appropriately isolated standalone selection is the desired result. If a shared binary also hosts Ambition, expect the binary to link portals; its **Mary-O session** must still not install/use portal mechanics.

## P10 — Per-room and mixed-game capability isolation

**Evidence:** `PortalPassages` stores `rooms: Vec<PassageRoom>` and uses `LiveRoomInstance` plus owner-specific budgets (`shared_tangle/src/portal_passages.rs:60-75,120-188`). `publish_portal_apertures` publishes live pairs by room and material maps for all bodies (`portal2d/src/passages.rs:30-73`). In the runtime, `PortalSet::{Pose,InputAdapter,InputWarp,WeaponAndProjectiles,TransitGuards,Frame,Transit}` are gated by `CapabilitySet(Capability::Portals)`, but the `Carves` and `Passages` publishers are intentionally not gated so they can clear stale state (`runtime/src/portal_schedule.rs:144-162`). This is a promising design but requires integration tests.

**Check:** In one host, run an Ambition portal-equipped room and a portal-free Mary-O room, and switch the controlled session/room. Verify that the portal-free game's generic movement authority works and ignores unrelated links/straddlers; published geometry for one room does not appear in another; portal state is cleared when the active topology changes; a stale `PortalPassages` resource cannot change gameplay in a room that lacks the capability. Test two rooms that reuse a zone ID and two bodies with different budgets. Confirm the expected behavior when the portal capability is in the binary but no portal entities/resources are installed in a room. Include rollback/replay and room transfer where feasible.

Do not assume `Option<Res<PortalPassages>>` is by itself a room capability gate. It only means the resource may be absent. Use the actual capability-selection and live-room model.

---

## Recommended implementation order

### Phase A — Establish the ownership contract

- [ ] Read ADR 0024 and map all pose writers/owners. List all `PoseAdmission::constrain`, `constrain_body_pose`, `carry_body`, `transit_body`, `MotionStepContext.portal`, and `portal_refusal` call/read sites. Include `game/`, `crates/`, tests, and plugins.
- [ ] Choose one body/movement-owned API for external poses. Specify its input, accepted result, typed refusal, sweep time, carried vectors, and optional spatial restrictions. State who constructs a per-room/per-body snapshot and how its validity is maintained.
- [ ] Decide how a group of externally constrained bodies preflights and commits atomically; use one movement authority. Do not introduce one-off `portal` conditionals in each game.
- [ ] Separate the geometric `MaterialCast`/partition math from portal-specific link policy and resource publication. State whether a generic motion-admission interface can be shared with the normal kernel without loss of geometric detail.

### Phase B — Migrate all clients and repair failure policy

- [ ] Move/implement the body-owned API and migrate **all** generic clients listed in P01. Remove old portal-name import/re-export, not merely forward it.
- [ ] Repair flag-sequence progress ordering and demonstrate a refusal/recovery scenario.
- [ ] Repair mount, capture, boss hold, and two-body pet semantics. Define behavior on refusal with tests of secondary state (`RidingOn`, `CapturedBy`, `PoseOwnedExternally`, `CenteredAabb`, ground facts, gesture stage).
- [ ] Audit fall rescue, talk spacing, scripted castle/cosmic movement, boss limbs, and remaining `is_err` handling. Explicitly classify any intentionally ignored refusals.
- [ ] Give the core motion step a generic admission result and retain portal detail as an optional typed cause. Migrate readers and tests; remove old `portal_refusal` contract.
- [ ] Resolve missing material snapshots according to a documented authoritative-source/epoch rule.

### Phase C — Composition and presentation

- [ ] Separate simulation and portal-gun runtime installation and authored construction gating, without breaking schema fingerprints.
- [ ] Replace Mary-O's `all_capabilities` with measured actual capabilities; inspect implicit LDtk/web feature defaults.
- [ ] Move loading-zone appearance into room-scoped/authored policy, migrate consumers, and remove portal-named generic visibility state.
- [ ] Test mixed-game host and per-room capability isolation.

### Phase D — Complete validation and review of retained portal math

- [ ] Baseline and resolve the two named app portal failures in P07. Keep a failure record if not yet resolved; do not mark the review fully complete.
- [ ] Inspect open acceptance items in the portal planning doc; select the relevant gates and avoid duplicating its implementation queue.
- [ ] Benchmark snapshot publication before optimizing; keep caching only with proven benefit and correct invalidation.
- [ ] Run targeted unit, property, and headless integration checks, then the assembled app and replay checks required by `docs/recipes/cheapest-sufficient-check.md`.
- [ ] Update planning/architecture docs only where the new API contract changes them. Do not edit a shared live queue to mark this review done. Record exact commits and tests in the final completion note.

## Regression checklist (must be executable, not inspection alone)

| Scenario | Expected outcome | Primary owner |
|---|---|---|
| Portal-free Mary-O headless/body pose | Valid pose admitted with no portal resource or API import | Generic body movement |
| Portal-free Mary-O compilation | No portal mechanic in standalone normal dependency closure | Facade/game manifests |
| Portal-enabled body with zero-hop budget | Rejected candidate changes neither body nor sweep | Admission authority |
| Refused flagpole pose | Flag phase/clock stays in an agreed state; no phantom progress | Mary-O flag sequence |
| Captor moves through denied material | Captive relationship and pose remain coherent; can recover | Capture |
| Mount crosses denied boundary | Rider relationship and pose remain coherent; can recover | Mount |
| Two pet participants, second refused | No partial first-body commit or abandoned gesture transition | Pet |
| Boss/limb request refused | `PoseOwnedExternally` holds and AABB remain consistent | Boss conduct |
| Non-portal numerical overflow | Reports a generic material/motion fault without a portal event label | Core kernel |
| Portal hop/budget failure | Keeps complete typed portal context and exact accepted history | Portal/motion |
| Missing material map on a Body source | Typed failure or authoritative resolution; never guessed identity | Material snapshot |
| Two rooms reuse a zone name | Visibility is scoped; one provider cannot hide the other | World/render |
| Simulation-only portals | No gun schema or gun installer, but scripted portals work | Runtime/construction |
| Full portal+gun composition | Existing gun and authored content continue to work | Portal plugin |
| Portal-equipped and portal-free rooms together | No link, state, or policy leakage across rooms | Host/capabilities |
| Portal camera and thin wall | Both named integration tests pass or exact unresolved failures documented | Portal app acceptance |
| Replay after refusal/room transfer | Deterministic accepted history and snapshot publication | Rollback/runtime |

The implementation agent should add narrow tests at the closest owning crate, then the assembled-app integration test where a multi-crate invariant is involved. `shared_tangle/src/portal_passages/pose_tests.rs:30-62` already tests per-body refusal and carried acceleration; extend equivalent coverage at the new owner. `core/src/movement/authority/pose_tests.rs` exercises the candidate-commit primitive. Do not delete good tests when moving the API; update their imports and run them.

## Search and verification commands

The first commands localize source; they do not need a build:

```sh
rg -n 'PoseAdmission|poses\.constrain\(|constrain_body_pose|portal_refusal|MotionStepContext' crates game
rg -n 'portal_passages|PortalPassages|PartSource|material_map|set_body_material' crates game
rg -n 'HiddenLoadingZones|hide_loading_zone|gate_portal_visuals|all_capabilities|portal_ldtk' crates game
rg -n 'PortalSimulationPlugin|PortalGunPlugin|PortalPlugin|PortalSchedulePlugin|Capability::Portals' crates game
git show --stat 9f2a8fa
git show 9f2a8fa -- game/ambition_demo_mary_o/src/flag.rs crates/ambition_platformer2d_shared_tangle/src/portal_passages.rs
git log --since=2026-11-02 --until=2026-11-10 --oneline -- crates/ambition_portal2d crates/ambition_platformer2d_core game/ambition_demo_mary_o
```

Follow `AGENTS.md` before running Cargo: verify the shared target bind mount; do not build or clean when it is absent. Freeze the tree during any gate and avoid concurrent builds on the shared target. Use the smallest relevant test population first. Examples (confirm exact target/test discovery locally):

```sh
scripts/setup/target_bindmount.sh --status
cargo test -p ambition_platformer2d_core --lib movement::authority
cargo test -p ambition_platformer2d_shared_tangle --lib portal_passages
cargo test -p ambition_portal2d --lib
cargo check -p ambition_app
cargo test -p ambition_app --test app_it -- portal
```

For the two specific camera/translation test names, inspect the real test target definition and run the exact target/filter (do not use `--exact` with a short name and accidentally run zero tests). `cargo test -- --list` or `cargo test -p ambition_app --test app_it -- --list` is useful for checking that selection. Run the project's required policy checks and platform-specific smoke gates when the changed modules demand them.

## Guardrails for completion

- Do not weaken or skip portal admission, budgets, numerical checks, or the authored content validation merely to make a test green.
- Preserve canonical material shape, parity/rotation, curved contact and time-aware source motion; do not substitute a bounding-box-only implementation.
- Preserve per-body/per-room simulation semantics, rollback determinism, and the one-body-one-path rule.
- No compatibility shims, parallel world-state caches, generic free functions duplicating method names, or game-specific portal bypasses.
- Use domain-specific public names so `rg` and IDE navigation find the true owner.
- Do not blame the portal agent for pre-existing loading-zone/global visibility policy or all-capabilities manifests without Git evidence.
- Do not confuse a linked crate, an installed plugin, and a room with the capability active.
- Do not overwrite other agents' work or alter the shared planning queue. This review is a standalone document; the implementation should make focused source commits and keep existing planning owners accurate.

## Completion report required from the implementing agent

Report (1) each P01–P10 ID as resolved, deferred, or confirmed not an issue with code/test evidence; (2) the new owner and public path for external body pose admission; (3) the exact failure policy for flag, mount, capture, pet, boss holds; (4) feature/dependency closure results for Mary-O, simulation-only portals, and mixed-game hosting; (5) measured publication cost if P05 was changed; (6) baseline and latest result of the two named portal app tests; and (7) the exact verification commands, test counts, and commits. If a gate is still failing, name it rather than saying the portal work is complete.

## Implementation receipts

First packet: `4afbf6100` (the first checkpoint). P03 and P04 are implemented. P01 has its body-owned
API; body snapshot publication still needs its owner move. P02 uses explicit
owner policies: flag and castle steps retry without progress; denied saddle
poses dismount; denied captive poses use the capture release owner; petting
prepares both bodies and cancels on denial; rejected boss poses retain holds.
Fall rescue creates no ferry before admission. Talk spacing retains progress
on denial. Sky and lava constraints retain physical state and dependent effects.

The public pose path is
`ambition_platformer2d_shared_tangle::body::pose::BodyPoseAdmission`.
The facade exposes the same body module. `prepare_body_pose` owns validation;
`PreparedBodyPose::commit` owns the physical commit. `BodyGeometry` is the
shared body geometry snapshot. Missing body material is a typed failure.
There is no old pose API alias.

Validation commands and results:

- `cargo check -j12 -p ambition_app --tests`: pass.
- Library batch for core, shared, boss, combat, Mary O, mounts and actors:
  856, 356, 246, 827, 205, 16 and 589 pass. Five tests remain ignored.
- `cargo test -j12 -p ambition_workspace_policy`: 35 pass.
- App `pet`, `grab` and `gnu_ton_fight` selections: pass; GNU-ton has 39 passes.
- `cargo test -j12 -p ambition_demo_smash_app`: 71 pass, six ignored.

Logs use `target/portal-review-a-`. At that checkpoint, P05–P10 remained open. This receipt
does not mark the full review or the portal implementation complete.

### Capability and publication packet

Code commit: `9e626f08b`. Map commits: `e7ad541` and merge `790e3ee`. The body domain now samples canonical body material before
motion and after chart transfer. The portal publisher adds only aperture policy,
links and occupancy. These are two publication epochs. The later sample must
include this tick's accepted motion. `BodyKinematics::material` remains the
canonical state; `BodyGeometry` is derived state and is rebuilt during replay.
No second cache or incremental invalidation protocol was added.

| Finding | Status | Change and guard |
|---|---|---|
| P01 | Resolved | `body::pose::BodyPoseAdmission` and `body::geometry` own admission and material sampling. Portal-free scaled and reflected pose tests pass. |
| P02 | Resolved | Packet `4afbf6100` commits sequence state and relationships after admission. Owner refusal and recovery tests pass. |
| P03 | Resolved | `MotionStepContext::admission` and `FrameEvents::motion_refusal` report generic motion failure. Typed aperture details and first-fault ordering remain. |
| P04 | Resolved | Missing body material returns `MissingMaterial`. Explicit world sources retain their identity transform. |
| P05 | Resolved | The two epochs have one body owner. The publication measurement below records their cost. Full rebuilds remain; no cache optimization is claimed. |
| P06 | Resolved | Runtime selects `PortalSimulationPlugin`. Ambition selects the gun separately. Construction consumes the same generation-bound schema catalog used for content identity. Missing gun schemas refuse authored guns. Repeat plan IDs agree. |
| P07 | Deferred | The two camera/walking failures remain release gates in the existing portal plan. The spark/wall case is fixed in `b2e8f750a`. The relevant-source baseline and latest results are below. No PSIM slice is closed. |
| P08 | Resolved | Each loading zone authors `Default` or `SceneArt`. The generic renderer owns this policy. Duplicate IDs in different rooms do not share visibility. Secret discovery and gate animation retain their owners. |
| P09 | Resolved | Mary-O selects its actual facade features. Standalone normal, test, visible, desktop and web trees have no portal crate. Isolated compilation and tests are recorded below. |
| P10 | Resolved | `PortalRooms` reads the existing per-game rules at the world owner. Publishers clear disabled-room links and occupancy. Physics, shots, held pairs and camera lists use room identity. Tests cover capability changes, equal coordinates in two rooms and a body transferred into a Mary-O room. |

The caller policies are: flag and castle retry without clock or phase progress;
mounts dismount at the last accepted pose; captives use the existing capture
release owner; petting prepares both bodies and cancels on refusal; boss pose
claims follow acceptance, and rejected release poses retain the hold. No client
gets a portal bypass.

The public rule paths are now `ambition_platformer2d_world::scoped_rules` and
`ambition_platformer2d_world::governing_rules`. Combat has no forwarding modules
or test-support bridge. These rules serve the world, combat and optional
capabilities through one room model.

The map tool authored the presentation field in twelve LDtk files. Semantic
comparison after editor normalization shows only the enum, field definition and
selected instance values. The editor also repaired existing Boolean defaults
in intro and Mary-O. Geometry is unchanged. The Science sign and two NPC edits
were excluded from the map commit.

Eleven map validations pass. Sandbox retains the existing error for
`scroll_lab_door`, which has no target or arriving zone. The Python repository
suite also retains an unrelated failure in
`test_the_shipped_ledger_and_page_agree`: the committed open-question page has
zero Q headings, and its checker refuses zero. The run was stopped after this
failure. No planning queue or question was changed to satisfy the checker.

#### Publication measurement

The ignored `passages::publication_measurements_tests::measure_body_geometry_publication`
test uses two live rooms with per-game capability rules. The second room disables
portals. Each publication time includes the body-owned material sample. Five
warm-up passes precede 100 measured publications at each epoch. The consumer
column is one batch over all bodies, averaged over ten batches. These are debug
build measurements on this VM, not release or target-device frame costs.

| Bodies | Pairs in enabled room | Before motion, us | After motion, us | Consumer batch, us | Material payload, bytes |
|---:|---:|---:|---:|---:|---:|
| 0 | 0 | 27.800 | 21.467 | 0.055 | 0 |
| 0 | 8 | 34.128 | 23.341 | 0.033 | 0 |
| 100 | 0 | 59.066 | 54.525 | 149.304 | 7,200 |
| 100 | 8 | 72.221 | 63.897 | 321.626 | 7,200 |
| 1,000 | 0 | 411.883 | 413.072 | 1435.263 | 72,000 |
| 1,000 | 8 | 427.146 | 421.185 | 3282.461 | 72,000 |

Payload bytes exclude tree nodes and vector capacity. Allocation counts were not
measured. The two samples have different required input states. This measurement
does not justify a shared mutable cache or a skipped second epoch. It does not
cover deep recursive cuts, moving-host load, rendering or GPU cost.

#### Portal acceptance that remains open

The relevant-source baseline uses `f7ad6e564988061c9034e7e8a838adcce56b5c92`,
the same authored lab and the source paths recorded in
`target/portal-camera-baseline-status.json`. That run passes the floor/ceiling
case and fails the same two walking/camera cases. It is a source comparison;
it is not a claim that the entire old commit was validated in a clean checkout.

The latest headless app run also fails:

- `portal_translation_camera_continuity::c141_to_c140_preserves_screen_position_and_continues_right`:
  the screen-space continuity assertion fails.
- `portal_translation_camera_continuity::thin_wall_walk_keeps_apparent_player_position_smooth`:
  the walk does not cross the pair.

The assertions were retained. Their owner remains
[the portal plan](../../docs/planning/engine/portal-parts-similarity-and-chirality.md).
Root rebasing, remaining motion, moving-host admission, righting, native actor
contacts, unequal apertures and full PSIM acceptance remain open there. This
review does not create a second implementation queue.

#### Verification and limits

The target bind was active before builds. Cargo gates ran in sequence. Source
files stayed fixed during each gate. Packet `4afbf6100` has the first admission
and caller tests listed above.

The capability packet has these results:

- Owning library batch: world 181 passes and one ignored; construction 10;
  publication 183; LDtk 81; renderer 522. The app library has 279 passes and one
  ignored. Portal has 143 passes and one ignored; runtime has 264 passes.
- Content portal selection: 64 passes. The full content library has 560 passes,
  one failure and two ignored. Its failure names the unpublished
  `history_afonso_letter` prop sheet.
- App mixed-game selection: 17 passes, including actual host capability
  declarations, room-local admission and body transfer into a Mary-O room.
- App portal selection: 35 passes and the two P07 failures. The c141 assertion
  reports delta `(-549.950, 29.325)`, length `550.731`, against tolerance `1.5`.
  These values match the relevant-source baseline.
- Broad app rollback selection before the baseline refresh: 82 passes, three
  failures and six ignored. The baseline failure is the intended rename from
  `PortalPassages` to `BodyGeometry` in the derived-resource row. No snapshot
  encoding changed. The other two failures are resource-census tests. They
  report `ambition_demo_smash::autoplay::SmashAutoplayDriver` and
  `ambition_platformer2d::content::sprite_reload::SpriteSheetWatchPolicy`.
  Their source files were not changed by this packet. No waiver was added.
- Art-dependent app selections remain red: declared art has five passes and
  three failures; registered character art has two passes and one failure.
  The generated library lacks history character sheets, manifests and portraits.
  This packet does not author or remove those assets.
- Smash standalone: 71 passes, six ignored; its doctest target has no tests.
- The enforced absence-contract check passes all 53 contracts. The interrupted
  Python run has 525 passes, 15 skips and the unrelated open-question checker
  failure described above. It is not a full Python-suite pass.

The body/portal owner batch was compiled with:

```sh
cargo test -j12 -p ambition_app -p ambition_platformer2d_runtime \
  -p ambition_portal2d -p ambition_content --lib --no-run --message-format=json
```

The emitted portal and runtime unit-test binaries ran in full. The emitted
content binary ran with filter `portal`. The emitted portal binary also ran
`--ignored --nocapture measure_body_geometry_publication`.

Other exact commands:

```sh
cargo check -j12 -p ambition_app --tests
cargo test -j12 -p ambition_app --test app_it -- a_game_governs_only_its_own_rooms
cargo test -j12 -p ambition_app --test app_it -- portal
cargo test -j12 -p ambition_app --test app_it -- rollback_
cargo test -j12 -p ambition_app --test app_it -- declared_art_resolves
cargo test -j12 -p ambition_app --test app_it -- registered_character_art
cargo test -j12 -p ambition_demo_smash_app
python3 scripts/checks/check_absence_contracts.py --check
python3 -m pytest scripts/tests/ -q
```

Logs and status records use `target/portal-review-`. This is a focused review
lane. It does not prove full workspace, Android, GPU capture, deep recursive
partition load or complete PSIM acceptance.

Mary-O validation is isolated from Ambition feature unification:

```sh
cargo test -j12 -p ambition_demo_mary_o -p ambition_demo_mary_o_app \
  --features ambition_demo_mary_o_app/visible --no-fail-fast
cargo check -j12 -p ambition_demo_mary_o_app \
  --target wasm32-unknown-unknown --features web
```

The web check passes. The native run has 205 content passes and one ignored,
ten app unit passes, and 139 visible integration passes with one failure and
three ignored. The failed test is
`her_spark_leaves_her_hand::a_spark_thrown_against_a_wall_does_not_pass_the_wall`.
A spark born inside the left wall is seen beyond its far side. The assertion
was retained. The later contact packet below fixes this case. The common
projectile collision resolver has no portal-feature branch. This earlier run
does not establish that the full standalone game is green.

All six standalone dependency-tree selections pass: content normal, app normal,
app normal plus dev dependencies, visible, desktop and web. They contain no
`ambition_portal2d`. A separate wasm-target tree also excludes it. Ambition's web
composition now selects `portal_ldtk` explicitly; the generic web facade does
not select it for portal-free games.

Final correction gates pass:

- `cargo test -j12 -p ambition_app -p ambition_portal2d --lib passages`:
  one pass and one ignored measurement.
- `cargo test -j12 -p ambition_app -p ambition_portal2d --lib measure_body_geometry_publication -- --ignored --nocapture`:
  one measured test passes. The table above uses this final run.
- `cargo check -j12 -p ambition_app --tests`: pass.
- `cargo test -j12 -p ambition_app --test app_it -- rollback_schema_baseline`:
  four pass after the intended derived-resource name refresh.
- `cargo test -j12 -p ambition_workspace_policy`: 35 pass. The portal dependency
  rule now names the world rule service. The timing probe is in a test file.
  No determinism exception was added.

The broad rollback selection was not repeated after the baseline text refresh.
The resource-census failures remain open. No full rollback-suite pass is claimed.
The policy, schema and app-check logs use `target/portal-review-correction-`.

The map push required a merge with remote `18e40ee`. The merge keeps its Science
layout and enemy data, then applies the four zone presentation values with fresh
schema UIDs. The committed semantic diff from that remote file contains only
presentation data. The working copy retains the original director sign and both
local NPC records exactly, including their IIDs, without duplicates. The local
identity delta stays unstaged. Science validation passes. The final content check
passes after the merge:

```sh
cargo test -j12 -p ambition_app -p ambition_content --lib worlds::tests::every_shipped_enemy_placement_can_be_built
```

The unchanged library commands for the earlier owning batch were:

```sh
cargo test -j12 -p ambition_app -p ambition_platformer2d_world \
  -p ambition_portal2d -p ambition_platformer2d_construction \
  -p ambition_platformer2d_publication -p ambition_platformer2d_runtime \
  -p ambition_render -p ambition_platformer2d_ldtk --lib --no-fail-fast
```

Documentation checks: intra-planning anchors pass (133 cross-file and 12
same-file links). The strict citation check reports 27 unresolved references in
other planning files. The link check reports 13 broken links in other planning
files. Neither checker reports this updated portal plan. The shared queue and
those other plans were not changed by this repair.


### Initial-overlap contact packet

Code commit: `b2e8f750a`. Native casts now have an explicit initial-overlap
policy. Projectile queries report occupied starts. Body escape queries permit
separation. This fixes the Mary-O spark that could leave a wall without impact.
The same policy applies to clipped material and turning obstacles. No game
branch or approximate collider was added.

Validation:

- Core library: 858 passes, one ignored. Shared library: 357 passes.
  Command: `cargo test -j12 -p ambition_platformer2d_core -p ambition_platformer2d_shared_tangle --lib --no-fail-fast`.
- The added shared test `projectile::collision::tests::an_overlapping_far_part_hits_before_the_shot_can_withdraw`
  passes separately. It checks initial overlap through a scaled pair.
- The exact standalone Mary-O test
  `her_spark_leaves_her_hand::a_spark_thrown_against_a_wall_does_not_pass_the_wall`
  passes with `cargo test -j12 -p ambition_demo_mary_o_app --features visible --test mary_o_it -- her_spark_leaves_her_hand::a_spark_thrown_against_a_wall_does_not_pass_the_wall --exact`.
- All 14 app tests selected by
  `cargo test -j12 -p ambition_app --test app_it -- projectile_portal_transit`
  pass. This includes the held-item transit tests.
- `cargo check -j12 -p ambition_app` passes.

Logs use `target/portal-goal-`. This lane does not prove a full standalone
Mary-O run, workspace acceptance, rendered output or complete PSIM acceptance.

The camera selection still has one pass and two failures. Its trace shows no
motion refusal or reset. The wall-walk case passes its intended entry without
transfer and later crosses the spinning shelf pair. The thin-wall case does
not transfer. Both place the feet at an aperture end. The canonical half-height
is `24.000001907348633`; its exact lower edge exceeds the aperture end by
`1.9073486328125e-6`, while the float collision box rounds it to that end.
Evidence is in `target/portal-goal-camera-probe.log` and
`target/portal-goal-walking-fit-numeric.json`. Temporary trace code was removed;
the original assertions remain. P07 and the native actor contact work remain
open in the existing portal plan.
