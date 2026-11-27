# Reusable kernels and infrastructure: ranked source audit

**Status:** Triage, not selected work. Review of `f4e6ab713b864cebdedeb4ff9df9e5e4e413711a` (November 26 local time / November 27 UTC, 2026). This page proposes work for the planning owners to assess. It does not change the queue, approve a crate split, or record new maintainer rulings.

## Purpose

Reduce the number of implementations that maintain the same rules. Prefer small, reusable mathematical kernels, collection operations, and Bevy utilities. The maintainer wants code that can serve other Rust and Bevy projects, with Ambition-specific conventions outside the reusable operation.

Use [data-structure kernels](../engine/data-structure-kernels.md), [execution specialization](../engine/execution-specialization-and-work-selection.md), and [architecture](../engine/architecture.md) as the existing owners of this direction. Do not start a second utility program with different rules. Do not reopen K1-K3. Do not change active renderer or portal work without coordination with its owner.

**Finding:** Recent work has removed duplicate mechanical authorities. It has also exposed repeated infrastructure operations and new duplicate geometry. The best next extractions are narrower than the earlier proposals for generic containers and universal search drivers.

**Evidence:** Source inspection and comparison of `bc3a8d30613a` and `f4e6ab713b86`. This is not a build, runtime test, performance measurement, or complete third-party library survey. Re-run the evidence commands against HEAD before selecting work.

## Ranked findings

Confidence refers to the evidence for a useful shared operation, not to a verified implementation or a fixed API.

| Rank | Finding | Confidence | Initial disposition |
| --- | --- | --- | --- |
| 1 | Point bounds and convex half-plane clipping | Very high | Pilot in the geometry foundation |
| 2 | Replay-aware Bevy messages | Very high | Extract an existing useful mechanism |
| 3 | Host and guest input-history operations | Very high | Share two small operations |
| 4 | Stable hashing primitives | Very high | Share the algorithm, preserve each protocol |
| 5 | Snapshot and rollback foundation boundaries | High | Use the time crate as the acceptance consumer |
| 6 | Confirmed-frame diagnostic buffers | High | Consolidate two traces first |
| 7 | Parametric slab intersection | High for the mathematics; medium for the interface | Compare the callers before a pilot |
| 8 | Keyed retained-product reconciliation | High | Active rendering owner to assess and implement |
| 9 | Bounded numerical interval search primitives | Medium-high | Share narrow operations with the portal owner |
| 10 | Prepared scoped collection queries | Medium | Reuse existing grouping, measure before indexing |

These ranks are recommendations, not a new execution order. The first six have the strongest case for implementation. The others need coordination or a small experiment. Sections below state the evidence, possible owner, and acceptance or promotion condition.

## Progress that changes the earlier recommendations

The new portal implementation already has `RootTransfer`, `StepSpan`, `MotionAdmission`, `ChartChange`, and `MetThrough`. `RootTransfer` gives the movement kernel and portal transfer machinery a common position and velocity transformation. Commit `3efcd700` removes the old transit authority that could move a body root after its movement step. This is an example of useful abstraction without a general framework. Preserve that direction.

M1 and M2 have implementation receipts for multiple stages. Do not treat all their former tasks as open. Reconcile the remaining scope against [moving frames](../engine/portals-are-moving-frames.md) and the current source. The latest crushing work also has obstruction, admission and response code. Its one-way policy, authored witnesses and remaining collision cases need their own owner, not a parallel utility project.

The geometry helpers in the new carried-obstruction code warrant immediate review. The latest movement code does not yet justify a general integrator, a general constraint solver, or a common physics engine for all movers. Separate the established mathematical operations from the movement laws that use them.

Evidence:

```bash
git show --stat 3efcd700
rg -n 'struct (RootTransfer|StepSpan|MotionAdmission|ChartChange|MetThrough)' crates
rg -n 'M1[abc]|M2[abcde]|SUP1' docs/planning/engine/portals-are-moving-frames.md
```

## 1. Geometry foundation

**Owner candidates:** `ambition_geometry`; [data-structure kernels](../engine/data-structure-kernels.md). Coordinate with [moving frames](../engine/portals-are-moving-frames.md) and [crushing and obstruction](../engine/crushing-and-obstruction-response.md).

### 1a. Bounds of points

**Finding:** Multiple modules independently reduce points to coordinate extrema. They do not all use the same rule for empty or nonfinite input. A shared operation can give callers one numeric contract, without selecting their failure policy.

| Source location (under `crates/`) | Operation |
| --- | --- |
| `ambition_geometry/src/combat_volume.rs` | `bounds_of_points` |
| `ambition_platformer2d_core/src/layered.rs` | `bounds_of` |
| `ambition_platformer2d_actor/src/avatar/trail.rs` | `polygon_bounds` |
| `ambition_platformer2d_runtime/src/portal_world/carried.rs` | `bounds` |
| `ambition_render/src/rendering/occlusion.rs` | `bounds` |
| `ambition_render/src/folded_mesh.rs` | Mesh `bounds` |
| World collision and actor geometry update | Inline polygon bounds |

Start with the pure operation that returns extrema. Inspect the existing `Aabb` constructors and any suitable Bevy math or Parry operation first. Make empty input, finite bounds, nonfinite input, and degenerate geometry explicit. An empty set is not an invalid set. A point or segment can have valid bounds even when it has zero area. A caller can choose conservative unbounded bounds, refusal, or another documented result. Do not silently map invalid geometry to an empty result.

Do not require every caller to allocate an intermediate point collection. Avoid unnecessary conversion from extrema to center and half-extents if the result can retain extrema directly. Check large coordinates, negative values, and overflow in conversions. Keep representation-specific adapters small.

**Trigger to promote:** Confirm at least two live consumers with the same extrema operation. Migrate them and remove their former implementations. Compare representative empty, degenerate, invalid and large inputs with the intended old behavior. Add only tests that protect the selected contract.

### 1b. Convex half-plane clipping

**Finding:** Two live clipping paths perform the same edge/plane operation with different precision and area rules. A third public helper has no Rust caller in this snapshot.

- `ambition_platformer2d_core/src/frame/thread.rs`: `clip_convex` and `clip_into`. The material partition path uses `DVec2` intermediate calculations and positive-area checks.
- `ambition_platformer2d_runtime/src/portal_world/carried.rs`: `clip_behind`. The moving-portal obstruction path uses a separate single-precision implementation and checks the number of vertices.
- `ambition_portal2d/src/pieces.rs`: `clip_halfspace` and `bound_clipped`. The repository search found no caller of `clip_halfspace` outside its definition. Verify re-exports and intended API customers before retirement. No current caller is not by itself a reason to delete a useful concept (Q74). Do not count this path as a third production customer.

Extract or adopt the common operation for clipping a polygon by a half-plane. Let the callers select plane orientation, coordinate chart, precision policy, and whether they require a closed-set result or positive area. Preserve the distinction between intersection geometry and bounds of that geometry. A conservative AABB is not an exact material polygon.

Use the strongest existing numeric implementation as a reference, not as an automatic replacement for every caller. A change from `f32` to `f64` can change contact decisions. Inspect intersection rounding, points exactly on a plane, duplicate vertices, collinear edges, empty output and nonfinite inputs. Preserve conservative bounds and refusal conditions where the physics contract requires them.

**Trigger to promote:** The live material and carried-obstruction consumers share a narrow clipping operation without leaking portal state into the geometry foundation. Delete the replaced loop. Resolve the unused helper by intent and maintenance cost. Do not introduce a general polygon Boolean library.

### 1c. Parametric slab intersection (secondary)

**Finding:** The following operations all intersect parameter intervals for individual coordinate axes:

| Source | Function | Domain policy |
| --- | --- | --- |
| `ambition_geometry/src/geometry.rs` | `slab_sweep` | Finite sweep; contact normal; near-zero threshold |
| `ambition_portal2d/src/placement.rs` | `ray_interval` | Ray interval; reciprocal direction arithmetic |
| `ambition_platformer2d_world/src/platforms/obstruction.rs` | `time_of_contact` | Forward contact, potentially beyond the proposed displacement |

The differences include initial overlap, stationary axes, near-zero motion, grazing contact, parameter bounds and tie-breaking of the contact normal. Compare the direct reciprocal calculation for a ray on an AABB boundary with the explicit stationary-axis branches in the other implementations.

A small axis-interval or interval-intersection operation may be useful. A universal sweep API with flags for every caller is not the goal. Preserve caller-specific contact policy. Do not report an incomplete or invalid calculation as proof of absence.

**Trigger to promote:** A small pilot replaces the repeated numeric work and makes the callers clearer. Keep independent implementations if the only common code is trivial or the shared interface obscures their contact laws.

Evidence for 1a-1c:

```bash
rg -n 'fn (bounds_of_points|bounds_of|polygon_bounds|bounds)\b' crates
rg -n 'fn (clip_convex|clip_into|clip_behind|clip_halfspace|bound_clipped)\b' crates
rg -n 'clip_halfspace|bound_clipped' crates game
rg -n 'fn (slab_sweep|ray_interval|time_of_contact)\b' crates
```

## 2. Replay-aware Bevy messages

**Finding:** `ambition_platformer2d_core/src/replay_messages.rs` already implements a useful general Bevy operation. It imports only `bevy_ecs`. It suspends the live `Messages<M>` resource, installs a private replay stream, and gives each `ReplayMessageReader` a separate live and replay cursor. A replay sequence identifies when to reset the private cursor. The live stream, message IDs, and reader positions survive the replay.

At the reviewed revision, 69 Rust files under `crates/` mention `ReplayMessageReader`. This count includes definitions and tests; it is not a count of independent consumers. The module already has a test with two readers at different positions.

The extraction is mainly a dependency and interface task, not a new generalization. This is the strongest direct candidate for a Bevy ecosystem contribution.

Before extraction, fix or document the following contract details:

- The current `isolate_messages_for_replay` removes the message resource before the nested-replay assertion. Reject nesting before changing resources. Also check sequence exhaustion before mutation, or restore state on failure.
- Define whether an absent message resource is an allowed no-op. Define the order of isolation and restoration for multiple message types.
- Preserve different live reader positions, message IDs and pending payloads across one or more replays. Replay traffic must not advance the live cursors.
- State how buffer maintenance and ordinary `MessageReader` systems interact with isolation. Do not imply safety for a reader that does not use the replay mechanism.
- Establish restoration on failure at the host boundary. Do not promise unwind safety until the actual schedule and resource ownership support it.

`ambition_platformer2d_runtime/src/external_effects.rs` also swaps message resources, but it captures outputs for publication. Keep this separate from replay-reader isolation until common semantics are demonstrated.

**Owner candidates:** rollback infrastructure and [island rollback](../engine/island-rollback-and-compact-peer-input.md). A small independent Bevy ECS package is a reasonable option when the dependency cut needs it. Compare the public API and existing rollback libraries before external publication; investigate `bevy_ggrs` integration. Do not extract the entire Ambition host or island policy.

**Trigger to promote:** The existing test and selected failure-boundary tests pass after extraction. A minimal Bevy ECS example builds without Ambition gameplay or physics crates. Real Ambition readers use the same implementation. No new competing message reader remains.

```bash
rg -l 'ReplayMessageReader' crates -g '*.rs' | wc -l
rg -n 'isolate_messages_for_replay|restore_messages_after_replay|SuspendedMessages' crates
```

## 3. Host and guest input-history operations

**Finding:** `ambition_hosted_world/src/prediction.rs` and `room_rollback.rs` duplicate the same ordered-map lookup and pruning rules.

The requested tick uses the exact recorded input if one exists. Otherwise, the nearest earlier input supplies `levels_only()`. If no prior input exists, use the default input. A one-shot edge must not repeat at later predicted ticks. When old records are removed, keep the last record strictly before the retention boundary so that it can define the first retained tick that has no exact record.

Use a common domain operation for predicted input and, if useful, a small generic ordered-map function for predecessor-preserving pruning. For example, `predicted_input` and `retain_from_with_predecessor` describe the operations. These are names for discussion, not required interfaces. Do not introduce `SparseTimeline<K, V>` merely to wrap `BTreeMap`.

Preserve host and guest rules for session identity, input arrival and exact input edges. The history of snapshots does not need the same retention rule as held input. A caller that needs ordinary pruning should continue to use ordinary pruning.

**Owner candidates:** hosted-world prediction and rollback; use the existing input contract owner for the shared prediction rule.

**Trigger to promote:** Both host and guest use one prediction law and one predecessor-preserving operation. Cover an empty timeline, exact boundary, gaps, old held values, and a one-shot edge on the preceding tick. The change must not create a new network protocol or alter prediction behavior.

```bash
rg -n 'levels_only|range\(\.\.=|last_before|\.retain\(' \
    crates/ambition_hosted_world/src/{prediction,room_rollback}.rs
```

## 4. Stable hashing and stateless random primitives

**Finding:** A textual search finds the FNV-1a 64-bit offset constant in 26 Rust files under `crates/`. Inspect the matches: some are genuine independent implementations, others are constants, tests or intentional protocol code. Known consumers include snapshot hashing, extension SDK digests, SFX and resource identifiers, portal links, combat and simulation randomness.

Share byte hashing, incremental mixing, and common known-vector tests. The `fnv` dependency is already present in `Cargo.lock`; assess its API and exact output before adding another implementation. A small local `const fn` may still be appropriate for compile-time identifiers. Do not assume `std::hash::Hash` or a default hasher gives a portable byte representation.

Keep each caller's framing, integer byte order, reserved identifiers, digest projection, and namespace policy. `PeerDigest` already owns a significant part of peer checksum projection; extend it where relevant, do not create another checksum authority. Preserve every existing intentional hash output and random sequence during consolidation. This is a migration constraint, not a general promise of permanent external compatibility.

`ambition_platformer2d_core/src/sim_random.rs` provides stateless random draws from a domain, context, tick and salt. It is a useful deterministic operation. Extract the common mixing primitive first. Document tuple framing and index selection before advertising the whole operation as a general random library. Modulo index selection and a deterministic mix alone do not establish statistically independent or unbiased draws.

**Owner candidates:** data-structure kernels and the snapshot foundation. The existing [stable identifier triage](stable-identifier-centralization.md) cautions against combining distinct identity policies. This work concerns hash algorithms, not a universal identifier system.

**Trigger to promote:** Group exact algorithm matches, compare against existing implementations, and migrate several confirmed duplicates. Use known vectors and representative old protocol outputs, including compile-time values. Do not change a saved ID or peer checksum as a side effect.

```bash
rg -l -i 'cbf29ce484222325|cbf2_9ce4_8422_2325' crates -g '*.rs'
rg -n 'StateHasher|PeerDigest|fnv1a|splitmix|sim_salt_for_name|sim_random' crates
rg -n '^name = "fnv"$' Cargo.lock
```

## 5. Snapshot and rollback foundation separation

**Finding:** Some otherwise reusable crates depend on the platformer core to declare rollback state. `ambition_time` is a particularly good acceptance consumer. Its direct imports from core are concentrated in `rollback_registration.rs`, `snapshot_impls.rs`, `gameplay_elapsed_snapshot.rs`, and `time_control/mod.rs`.

`ambition_platformer2d_core/src/snapshot.rs` owns `SnapshotState`, `SnapshotCursor`, `SnapshotResolve`, `StateHasher`, `PeerDigest` and `RollbackRegistrar`. The snapshot code also has canonical projections and existing macros. These concerns have different dependencies:

| Concern | Desired lower boundary |
| --- | --- |
| Stable hash and value encoding | Rust primitives; optional type adapters |
| Canonical projection and digest | Explicit value and identity contracts |
| Replay-aware message streams | Bevy ECS |
| Rollback registration | Bevy ECS and simulation-neutral registration vocabulary where feasible |
| Room membership, stable game identities and commit authority | Ambition simulation owners |

Do not require a fully engine-independent snapshot store in the first step. Trace actual type dependencies and orphan-rule constraints. Separate the vocabulary that domain crates need from host operations. Prefer an existing leaf package or one justified dependency cut, not a new catch-all utility crate or a compatibility facade that perpetuates the dependency.

Suggested order:

1. Extract replay messages (finding 2).
2. Identify the encoding and registration vocabulary that time actually needs. Move the smallest coherent part.
3. Preserve deterministic byte encodings and checksum projections. Canonical float behavior, including signed zero and normalized NaN payloads in the relevant projection, must not change accidentally. Preserve explicit enum codes, stable reference handling and decoding bounds.
4. Move `ambition_time` to the new boundary. Inspect resolved feature dependencies to ensure it no longer pulls platformer core indirectly.
5. Assess codec boilerplate only after the boundary works. Compare existing macros, existing serializers and a possible derive. Do not create a procedural macro solely to replace straightforward handwritten code.

**Owner candidates:** [compiler boundaries](../engine/compiler-boundaries-and-build-products.md), [island rollback](../engine/island-rollback-and-compact-peer-input.md), and the time owner. Preserve the existing renderer-free simulation build. Independent clock utilities could be another useful Bevy contribution after this cut. Do not export Ambition's clock and rollback policies wholesale.

**Trigger to promote:** A dependency walk identifies a small feasible extraction. Accept the first campaign when `ambition_time` builds without platformer core under its supported features, existing snapshot/checksum tests agree, and the registration and replay contracts retain their meaning. Record actual dependency changes; no speedup is claimed without a measurement.

```bash
rg -n 'ambition_platformer2d_core' crates/ambition_time
rg -n 'SnapshotState|SnapshotCursor|SnapshotResolve|RollbackRegistrar|PeerDigest' \
    crates/ambition_platformer2d_core/src/snapshot.rs
rg -n 'canonical|signed_zero|nan_payload' crates/ambition_platformer2d_core/src/snapshot
cargo tree -p ambition_time
```

## 6. Confirmed-frame diagnostic buffers

**Finding:** `ambition_gameplay_trace/src/buffer.rs` (`GameplayTraceBuffer::record_oob_assessment`) and `actor_trace.rs` (`ActorTraceBuffer::record`) repeat session/frame bookkeeping for speculative diagnostics. A corrected frame must replace the old assessment, including with an empty assessment. Confirmed results are published in order. An old session must not affect a new one. Without rollback, the diagnostics can publish immediately.

Extract the smallest shared pending-frame mechanism, leaving each trace's diagnostic policies, bounded output ring, warmup, arming, and dump format in its owner. A map keyed by frame with a session identity may suffice. Establish replacement, confirmed-prefix extraction, correction and reset semantics first. Do not imply exactly-once publication beyond the contract established by the caller's confirmation and generation rules.

`ambition_platformer2d_runtime/src/external_effects.rs` has related but more complex semantics. It can retract outputs for one room while preserving other rooms' results, and append corrected room outputs. Do not force that journal into a diagnostic buffer. After the trace migration, assess whether it can share ordered replacement or confirmed-prefix extraction without changing cross-room order and effect ownership.

**Owner candidates:** gameplay trace and the rollback output boundary. A later independent Rust utility is reasonable if the small interface proves useful.

**Trigger to promote:** Consolidate the two traces; retain their existing corrected-frame and session tests. Add only missing boundary cases. The new code must express their common lifetime law more clearly than the duplicated implementations. Decide on the journal separately.

```bash
rg -n 'Pending.*Assessment|pending|confirmed|session' \
    crates/ambition_gameplay_trace/src/{actor_trace,buffer}.rs
rg -n 'retract|confirm|room|replay' crates/ambition_platformer2d_runtime/src/external_effects.rs
```

## 7. Coordinated or conditional findings

### Retained ECS products (rank 8)

The 2.5D pilot has several independent maps and repeated create/update/hide/retire logic in `ambition_render/src/rendering/card_view.rs`. Vessel products and retained debug drawings are related customers. The rendering agent has a separate discovery brief. Let that agent select this work under [presentation order and 2.5D](../engine/presentation-order-and-25d-rendering.md); do not duplicate its implementation.

Distinguish source absent, temporarily pending, ready, and unchanged. Preserve scope, product identity, external despawn recovery, and teardown. Investigate Bevy relationships before designing a map wrapper. A generic diff of desired keys against retained products can be pure Rust; Bevy realization is an adapter. Do not create a declarative renderer or UI framework. Promote when two migrated production systems remove repeated lifecycle bookkeeping. Reassess publication after the migration.

### Interval search and velocity reference (rank 9)

`core/src/frame/crossing.rs`, `frame/material_cast/moving.rs`, `frame/material_cast/curved.rs`, and `frame/material_cast/sampled/curved.rs` contain related bounded chronological subdivision. They do not all have the same search guarantees. Reuse existing `interval_center` in `frame/motion.rs` where its contract fits. Compare precision termination, chronological traversal, and work accounting. Keep the conservative envelopes, rejected-contact continuation, initial overlap, winding, and refusal rules in the solver. An exhausted search is not proof of absence.

The current movement continuation, held-item motion and projectile motion also convert between stored velocity and the crossing-time velocity. A precise time-reference conversion could help, but only for the integration law that supports it. Constant acceleration is not a valid general model of nonlinear friction, support changes and controller acceleration. Coordinate with the portal owner. Promote narrow mathematical operations only after the assumptions and existing results agree. Do not start a universal integrator or branch-and-bound framework.

### Scoped preparation (rank 10)

`portal_world/carried.rs` prepares portals for relevant bodies. Compare repeated room filtering with existing `PortalsByRoom` and `AdmittedPortals`. Also inspect `TickObstructions`, `CarriedObstructions` and `PrefetchedByRoom<T>` for repeated preparation laws. Prepare the relevant population once when multiple queries warrant it. Preserve room instance identity, snapshot lifetime, canonical order, refusal, and allocation bounds. Do not cache across an invalidation boundary. K3 already establishes that scans can be faster for small populations. Use existing grouping before adding a generic index. `FrozenLookup` is a cardinality operation, not a one-to-many grouping API.

```bash
rg -n 'by_owner|by_art|by_body|by_mesh|by_source|by_panel|by_glow|RetainedDrawings' crates/ambition_render
rg -n 'interval_center|RootTransfer|StepSpan' crates
rg -n 'PortalsByRoom|AdmittedPortals|TickObstructions|CarriedObstructions|PrefetchedByRoom' crates
```

### Crushing: investigate correctness before abstraction

The new crushing code has pairwise motion admission, resistance, ejection and compression. It is too early for a general constraint solver. The portal-carried AABB approximates transformed material. Multiple obstructing bodies, both moving endpoints, curved surfaces and game-specific one-way policy still affect the contract. Authored motion allowance is not a physical force model.

Compare local and carried clearance accounting: the local calculation deducts prior penetration from its allowance, while the reviewed carried calculation does not make the same deduction. Check whether the policies should agree. Inspect the carried publisher's `Entity` sorting and its peer-independent-order claim. Entity allocator order need not agree across peers; verify whether the reduction is order-independent or requires semantic identity. These are review questions, not reproduced defects. The crushing owner should resolve them. A common operation for intersecting admitted fractions may be useful later, after real local and portal cases establish its contract.

## 8. Ideas not selected in this pass

| Earlier idea | Disposition and reason |
| --- | --- |
| Generic sorted-vector map/set | Defer. The domain collections have different mutation, ownership and revision laws. Compare an existing library or a small migration before committing. Binary search alone is not enough duplication. |
| Generic bounded history | Defer. Capacity eviction, explicit loss, cursor and rollback correction semantics are different. Consolidate only proven shared operations. |
| General graph library | Reject a new graph library. Prefer existing `petgraph` and small deterministic adapters when repeated closure or cycle diagnostics justify them. |
| Generic overlay collection | Investigate later. `Layered` has promising shared-base and sparse-overlay storage, but motion bounds, indices and geometry visibility are coupled to it. Do not disrupt current portal geometry without a concrete second storage customer. |
| Universal constraint solver or movement integrator | Not selected. Extend the established motion and crushing owners instead. |
| Full numerical search driver | Defer. Start with the small numerical operations above. |
| Universal generation cache or work-budget framework | Defer until independent consumers establish matching lifetime or budget rules. |
| New snapshot derive macro | Defer until the dependency cut and serializer comparison show remaining value. |
| New standalone stable hashing library | Low publication need; reuse existing libraries or local primitives. Internal consolidation remains high confidence. |

The earlier audit was too quick to name generic data structures. Replacing a three-line collection operation with three lines of adapters does not reduce maintenance. The common semantic contract must be clearer and more useful than the code it replaces.

## 9. Suggested implementation packets and triage outcome

Select at most three coherent workstreams initially, subject to the current queue and owner coordination.

- **Geometry:** point bounds and polygon clipping first; review obsolete helpers; then the slab interval pilot. Keep portal-specific geometry and material policy with their owners.
- **Deterministic infrastructure:** replay messages first, stable hashing primitives, then the dependency cut demonstrated by `ambition_time`. Make each stage useful on its own.
- **Temporal operations:** shared host/guest prediction and retention; then the two trace buffers. Assess the external-effect journal independently.

Rendering and portal agents own the related conditional findings. No need to block their progress for the other packets. Do not create new worktrees. Do not schedule a broad utility-library program.

For each selected change:

1. Re-measure the finding and inspect HEAD and existing owner plans. If the change already landed, record that disposition instead of repeating it.
2. Define the shared operation independently of its current consumers. Include numeric domain, error and refusal, order, identity, lifetime and mutation where relevant. Compare existing Rust and Bevy facilities. Using an existing implementation counts as success.
3. Choose the narrowest shared function, data type or package boundary that serves the operation. Prefer pure Rust where the operation does not need ECS. Keep necessary Bevy adapters separate from Ambition policy.
4. Migrate real consumers and remove the replaced paths. Compare before and after caller clarity and dependency edges, not only LOC. One consumer can justify a dependency extraction when there is a clear independent use; do not manufacture a second consumer or retain obsolete helpers to meet a count.
5. Verify intentional behavior, determinism and rollback at the changed boundaries. Use small, high-value tests. Benchmark when performance or a storage choice is part of the justification. Do not claim a speedup from moving code.
6. Stop or revert a pilot when the API needs excessive flags, the callers become harder to read, or new dependencies outweigh the benefit. Do not count a new utility beside unchanged competing implementations as consolidation.
7. Route accepted work into existing owner documents and the live queue only after triage. Preserve useful contracts, not a second queue. Record deferred or rejected work only where its owner needs the decision. Remove this dated intake page when its findings have been resolved or routed, subject to inbound references.

For external reuse, prioritize replay-aware messages, independent clocks after the dependency cut, and retained ECS products after the rendering migrations. Geometry and temporal collection operations can also be useful, but first compare existing library coverage. Publish or contribute a narrow operation with a standalone example and explicit contract, not the Ambition game that motivated it. No new public package is required for the internal consolidation to succeed.
