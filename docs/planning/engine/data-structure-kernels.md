# Shared data-structure kernels

**State:** PLANNED, 2026-11-21. Reviewed production source `b91b29d35935`
after planning commit `d150d69d`. No implementation, build or new timing
result is claimed. [Execution specialization](execution-specialization-and-work-selection.md)
owns the semantic/reference/production distinction; this page applies it to
reusable collection operations. [The queue](../queue.md) selects the work.

## Decision and package boundary

Make a small set of collection operations independent optimization targets.
Give each operation a precise result contract, a direct implementation and
measured alternatives. Gameplay supplies the selected input snapshot and
consumes the same result whichever implementation is selected.

Use existing foundation libraries for the first two families:

- `ambition_geometry::spatial` owns bounds-query algorithms. Extract the
  private range grid from core `Layered` into this family. Retain the existing
  bounds tree and add a direct scan under the same operation contracts.
- Add a generic `unique_lookup` module to `ambition_binding` for lookup over
  a frozen relation: missing, exactly one row, or ambiguous. Its kernel uses
  ordinary Rust values and imports no ECS or lifecycle types. Native owners
  supply their keys and values and interpret the result.

`ambition_geometry` already depends on Bevy math/ECS, Parry and serde for its
other modules; `ambition_binding` depends on `tracing`. Neither is currently
a dependency-free crate. Their kernels can still be measured without the
game host. Keep the new module separate from authored `Resolver`/`Bound`:
their duplicate policy differs. State this additional read-snapshot role in
the binding crate's module map when implementing it.

No new catch-all `ambition_utils` or collection replacement layer is selected.
A separate leaf crate remains a packaging option when a real consumer or
measured dependency cut needs it. Moving code alone has no runtime speed claim.
The selected benefit is reusable algorithms with stable meanings and isolated
measurements, not a new repository or submodule.

## Existing mechanisms and independent customers

| Source | Existing mechanism | Shared work to expose |
| --- | --- | --- |
| [Geometry spatial module](../../../crates/ambition_geometry/src/spatial.rs) | Rebuilt `SpatialTree<T, S>`, nearest bounds, overlap queries and conservative summaries | Scan/tree alternatives with the same query meaning |
| [Core Layered](../../../crates/ambition_platformer2d_core/src/layered.rs) | Shared immutable base, hidden/added rows, separate lazy current/swept grids | The grid is a query algorithm; the overlay retains storage and visibility ownership |
| [Script contact sampling](../../../crates/ambition_combat/src/extension/contact/facts.rs) | Material-piece envelopes indexed through `Layered` | Use the range snapshot directly; remove its unused overlay wrapper |
| [Combat target selection](../../../crates/ambition_combat/src/targeting/spatial.rs) | Per-room nearest queries, team/faction summaries and canonical scan fallback | Select scan or tree before unnecessary index preparation |
| [Object references](../../../crates/ambition_platformer2d_shared_tangle/src/lifecycle/object_refs.rs) and [behavior calls](../../../crates/ambition_platformer2d_runtime/src/object_behaviors.rs) | Both build `BTreeMap<K, Option<Entity>>`; any second key occurrence marks ambiguity | One frozen relation/cardinality reducer, with scan or indexed lookup |
| [Authored placement lookup](../../../crates/ambition_platformer2d_shared_tangle/src/lifecycle/authored_objects.rs) | Buckets of optional references distinguish absent, unpublished, unique and ambiguous roots | Reuse row cardinality while retaining publication and occurrence checks |

The existing [geometry comparison tests](../../../crates/ambition_geometry/src/spatial/tests.rs)
already compare tree queries with exhaustive evaluation. The library program
starts from those production operations, rather than a catalog of containers
that might one day be useful.

## The result contract comes before interchangeable storage

The same method names do not establish interchangeable behavior. Specify the
following for each selected operation:

| Part of the contract | Required distinction |
| --- | --- |
| Input | Snapshot, chart/cut, admitted numeric domain and key equality |
| Output | Exact result or conservative candidate set; duplicate treatment |
| Ordering | Unordered set, canonical row order, or stable ranked winner |
| Failure | Empty result, invalid input, unknown bound and incomplete read |
| Lifetime | Snapshot-local row versus durable identity; replacement invalidation |
| Mutation | Immutable query view versus an explicitly maintained live relation |
| Limits | Representable capacity versus a gameplay output/work limit |

Use narrow operations. Range lookup, nearest accepted bounds and unique-key
lookup do not need one trait with every container method. A range grid need
not implement nearest search to qualify as a range backend. Backend-specific
iterators, node IDs, buckets and mutable storage must not escape into callers
that need to exchange implementations.

Keep a facade at the owning snapshot boundary. A concrete choice, a private
enum or static specialization can select the implementation once per query
or preparation. Do not add a virtual call per entry by default, or propagate
an implementation type through every gameplay system. Retain generics where
payloads, filters or summaries need them; measure code generation before
adding another specialization axis. A bounds/row-index kernel can often
avoid specializing the heavy algorithm for each gameplay payload.

## K1 — snapshot bounds queries

### Exact bounds results and conservative candidates

A range result is exact with respect to the supplied closed AABBs. Touching
edges count. It is still only a candidate for exact shape or material contact.
An internal grid or tree can visit different conservative supersets, but the
shared boundary must exact-filter them before an ordered or bounded consumer
acts on the result.

The current tree's range traversal order differs from `Layered`'s visible
list order. Preserve that distinction in the API. An unordered exact-set
operation is sufficient only for a consumer that cannot observe its order.
An ordered adapter returns canonical snapshot rank before collision processing,
early exit, result truncation or a potentially refusing narrow-phase call.
Do not sort every internal traversal when order is unused; do not let an
ordered caller accidentally receive backend traversal order.

Contact sampling indexes material pieces, then canonicalizes body identities
before its `max_touchers` limit. Preserve both steps. Deduplicating by body
inside the index can discard another relevant piece; limiting raw backend
candidates can choose different bodies or suppress an incomplete read.

Nearest means the accepted bounds with the minimum declared squared distance,
then the stable input rank. Combat supplies its semantic identity order.
Bounds-distance is not center-distance. A grid nearest search needs a lower
bound for every unvisited region; the first populated cell is not proof of
the nearest result. Retain the current invalid-query scan/refusal policy at
its owner.

### Filters and numerical meaning

`SpatialFilter` and the nearest closure allow mutable state. Their acceptance
must nevertheless depend on the snapshot and query, not on traversal order,
call count or previous accept/reject calls. Pure-result memoization and
diagnostic counters can use mutation. Combat's existing team/faction cache
fits this law. Backend exchange does not promise identical callback traces.
Summary pruning must remain conservative under the same filter.

Declare the production arithmetic and tie policy. With query zero and two
finite `f32` coordinates `2e20` then `1e20`, both current squared distances
overflow to infinity, so rank breaks the tie. A wider arithmetic reference
can select the second. This is an arithmetic witness, not a runtime report.
That wider reference is useful, but it is not a drop-in implementation of
the current operational result. SIMD, FMA or parallel reductions must obey
the same selected production contract or accompany an explicit domain change.

### Unknown bounds and snapshot ownership

The finite-only tree rejects nonfinite/reversed bounds and empties a failed
rebuild. World geometry deliberately uses an unbounded envelope to retain a
row when no finite motion bound is known. Contact sampling also retains
refused material for an incomplete-read result. These are different inputs.
The shared range adapter needs an explicit always-candidate path for unknown
bounds; it must not feed them to a finite-only backend and erase the snapshot.
Malformed data, proved absence and an unknown envelope remain distinct.

The query can also be deliberately global: `Layered::near(UNBOUNDED)` is an
existing all-rows read. Preserve it in the adapter even when a finite-only
tree rejects nonfinite query boxes. A global query is not a NaN or reversed
query, and an always-candidate item list alone does not handle that distinction.

The domain supplies conservative bounds in one stated chart and motion cut.
The index does not infer portal image ownership, swept material or frame
conversion. A faster lookup cannot repair an under-conservative envelope.

Rows belong to one immutable snapshot. A base index, visible ordinal or tree
slot is not an occurrence identity and cannot survive replacement by itself.
Keep `Layered`'s shared `Arc` base and small hidden/added layer. Its current
and swept grids have different inputs. Whole mutation retires both caches;
an overlay query must not flatten the base or rebuild it for every body.
Contact partitions can own a plain range snapshot because they need none of
that overlay behavior.

Retain the grid's large-row and large-query fallbacks. Check representability
before narrowing row IDs or cell coordinates. Never truncate an admitted
population to fit an implementation. A valid finite coordinate outside a
backend's cell-key range takes a conservative/global fallback; it must not
become a new gameplay refusal. A compact structure's capacity is not the
gameplay output limit.

## K2 — ambiguity-preserving frozen lookup

The shared operation asks how many input rows have a key. Its result is
`Missing`, `Unique(value)` or `Ambiguous`. This is a proposed kernel result,
not a replacement for existing domain error enums.

Both object and behavior indexes currently count rows, not distinct values.
A repeated identical `(key, value)` row is still ambiguous. Preserve this law.
For authored placements, `Unique(None)` means one unpublished root; it must
remain different from `Missing`. Session legality, retirement, publication,
scope visibility and method availability are interpreted outside the kernel.

A direct scan is the reference and can be the cheapest production query for
a small relation. An indexed build can use the existing tree map, a sorted
table, or a private hash table when measurements support it. Lookup must not
expose hash iteration order. Stable diagnostic enumeration, if needed by a
customer, is a separate ordered operation. The owner can supply borrowed or
prepared keys; the utility must not require string creation at every lookup.

Keep this operation separate from two existing laws:

- [Authored Resolver](../../../crates/ambition_binding/src/lib.rs) allows
  repeated aliases for the same declaration, retains a first slot for
  conflicting declarations, and reports duplicates separately.
- [Registration classification](../../../crates/ambition_registry_core/src/lib.rs)
  treats an equal registration as idempotent and a different one as conflict.

Neither is the row-cardinality law above. Standardizing storage cannot choose
one of these policies for all three.

The first kernel is frozen. An ambiguity tombstone loses membership: after
two rows collide, removing one cannot establish which value remains. A later
incremental backend must retain sufficient membership/counts or rebuild at
the owner cut. It cannot claim correct removal from `Option<V>` alone.

The [addressed behavior plan](addressed-behavior-inbox.md) still owns cut-level
view reuse, births, retirement, reconstruction and replay visibility. Replacing
a map does not remove repeated whole-population captures. `resolve_authored`
already captures once per nonempty request batch; do not claim it builds once
per placement. Its nested occurrence capture and separate completion captures
are the existing preparation work to address there. ECS entities are values
in a native view, not new durable IDs supplied by this utility.

## Select by the complete workload

For a fixed published input, compare total cost over its useful lifetime:

```text
publish/build + updates + queries + exact filtering/order + result consumption
```

Include retained bytes, scratch allocation, cold publication and invalidation
cost. Query count and lifetime can decide the winner even at the same row
count. A scan is both a simple reference and a valid shipping fast path.

The tree already scans small leaves. Choosing a scan after building room
groups, summaries and the tree does not remove that preparation. Select
before optional setup. Preserve stable input collection that either path
requires. A lazy snapshot can delay indexing until actual demand, provided
the scan and indexed paths have identical outcomes and cache warmth cannot
change admission or consume a different logical work budget.

Choose policies from target measurements and workload shape: population,
queries per publication, extent distribution, changed membership, large or
unknown rows, layers and filter selectivity. Static target profiles and
input-derived thresholds are acceptable. Timing-based autotuning must not
decide omitted candidates, completion, refusal or gameplay approximation.
Device choice can change storage and work; the execution contract still
governs numerical/replay equivalence.

Keep optional alternatives out of unrelated shipping compositions when that
reduces code size or compile cost. Target feature selection is not permission
to change the query contract. Do not promise a universal winning structure.

## Independent measurements with production meaning

Existing receipts support this direction:

- The [grid query receipt](../../../dev/ambition_dev_measurements/journal/2026-11-18-spatial-grid-query-cost.md)
  compares production `Layered::near`. A hash-table trial helped larger worlds
  but slowed small-world local queries by about 5–12%; a sorted cell table was
  selected. The warm-query result excludes initial build and game frame cost.
- The [target selection receipt](../../../dev/ambition_dev_measurements/journal/2026-11-20-spatial-targets-and-geometry-publication.md)
  includes moving snapshots and rebuilds: some 32-body cases regress while
  larger populations improve. It does not establish release/mobile thresholds.
- The [contact observer receipt](../../../dev/benchmark-candidates/scripted-contact-observer-scaling.md)
  shows that query count changes amortization at a fixed body count. It is a
  local debug result, not a device budget.

Put focused benchmark targets beside the kernel and retain receipts in the
existing measurements submodule. Use compact inputs captured at the owning
API boundary plus small deliberate edge cases. Exercise the production entry
point and consume its results. A benchmark-only copy of the algorithm is not
evidence about the shipped kernel. A permanent generic workload DSL is not
required.

Separate three comparisons: correctness against the direct operation, isolated
kernel cost, and a representative engine consumer including preparation and
consumption. Include local/global queries, sparse/clustered extents, update
ratios, query counts, unknown bounds and output density where the contract
uses them. Measure one promising alternative at a time on a named target;
do not multiply every algorithm/profile/platform into a mandatory test matrix.

Existing exhaustive spatial tests and lifecycle witnesses supply the main
correctness comparisons. Extend them for the replacement operation's actual
risks: ordering and ties, unknown/invalid inputs, duplicate rows, unpublished
values, generation replacement and replay. Run expensive measurements on
demand. This plan adds no policy/source checker or test framework.

## Delivery and deletion criteria

1. **K1 range extraction:** move the existing grid algorithm into geometry's
   query family, with a direct reference scan and declared ordered adapter.
   `Layered` delegates while retaining sharing and overlay semantics; contact
   partitions use the range snapshot directly. Delete the private grid and
   contact's unused `Layered` wrapper. Preserve current fallback and refusal
   behavior before choosing another representation.
2. **K2 frozen lookup:** place the row-cardinality kernel in the binding
   foundation. Migrate occurrence and behavior indexes, then authored placement
   cardinality. Delete their repeated ambiguity reducers. Keep domain errors
   and E3 view lifetime with their owners. Do not migrate authored Resolver or
   registry classification under the wrong duplicate law.
3. **K3 measured selection:** compare direct and current indexed implementations
   with full preparation/consumption. Where measurements justify a small-workload
   path, select it before optional setup; record target-specific crossover
   evidence. A range tree, refit tree, hash
   layout, SIMD kernel or alternate key index follows only a workload that
   justifies it. This is an extension point, not a commitment to implement all.

Graph search remains with navigation until its arrival-state cost model is
correct. Active mailbox/deadline structures remain with E3 until freeze,
active-time and wake semantics are fixed. Standard collections stay appropriate
where only storage, not a repeated operation, is shared. These limits keep the
library's concepts small enough to explain and optimize independently.
