# Execution specialization and selection of active work

**State:** PLANNED, 2026-11-21. The maintainer requires full expressive
semantics and fast gameplay implementations. The previous portal review
named fast cases but did not specify their cost boundary sufficiently.
**Reviewed source:** production code at `b91b29d35935`; planning continuation
of `aef73be10f3d`. This review ran no Rust build, game or device benchmark.
Source counts and mathematical counterexamples are not measured speedups.
**Execution:** [the queue](../queue.md) selects the bounded domain packets
below. [Performance and iteration](performance-and-iteration.md) owns cost
measurement; each linked domain page owns its implementation contract.

## Decision

Keep a full semantic model. Run the cheapest implementation whose conditions
hold for the requested operation. Prepare relations that are already fixed,
and select work from actual demand or pending state. Installing a capability,
adding dormant objects, or having a distant portal must not make every actor
execute that capability's general algorithm.

This extends [prepared facets](capability-linking-and-prepared-facets.md).
There is no new universal execution graph, cache service, interpreter or
registry. A prepared clip property, an entry dispatch table and an indexed
live occurrence have different owners and lifetimes. Each removes a named
runtime reconstruction at an existing consumer.

## Semantic contract, reference evaluator and production implementations

| Layer | Purpose | Cost and authority |
| --- | --- | --- |
| Semantic contract | Shapes, material, motion, ordering, time, scopes, permitted effects and supported limits | Defines what an operation means; does not prescribe one algorithm |
| Reference evaluator | Direct evaluation, exhaustive eligible candidates, replay diagnostics and small offline comparisons | May spend much more work; optional in shipping compositions |
| Production implementations | Prepared lookup, closed-form geometry, local temporal queries and bounded general evaluation | Select from the actual admitted input; all live writers use the same selected result |

The existing curved solver is production code with useful caches. Its breadth
does not make it an independent proof oracle. An exhaustive candidate mode
can test a candidate filter while retaining a shared narrow phase; that only
tests candidate selection. A higher-precision or independently implemented
reference can establish additional claims, with explicit scope and numerical
error bounds. Keep both claims accurate.

Reference evaluation may run in existing offline inspection, replay and
focused test tools. Do not run a shadow general solver on every gameplay
query, put it in every release dependency, or build a second authoring format.
An offline reference that supports more work or wider numerical ranges may
resolve a production refusal; that does not prove that the refusal was false.
Separate supported material policy, numerical work limits and a proved miss.

The semantic model may remain more expressive than any one fast case. A
runtime case outside a specialization uses the next supported implementation.
That transition must be explicit and local. A production work limit needs an
admitted, deterministic continuation or truthful refusal; elapsed wall time
cannot silently change collision or discard a gameplay operation.

A deliberately simpler gameplay representation can also be part of the domain
contract. An authored collision hull need not follow every visible contour;
the navigation cost model need not claim a fastest physical trajectory. State
that approximation at its owner and compare implementations against that
contract. Machine load, cache misses and visual quality tiers cannot silently
select a different gameplay representation. Full reference expressiveness
does not require every authored object to pay for every supported feature.

### Numerical and replay equivalence

Two production paths need the same observable semantics: hit order, finite
aperture rules, stable ties, accepted state, emitted effects and refusal policy.
Algebraic equivalence alone does not prove floating-point equivalence. If a
cache hit and miss, preparation order, or task completion order can select
different arithmetic, preserve the existing deterministic commit contract.
Use one operational result, or an owner-defined canonical conversion that is
itself verified. An unexplained tolerance is not permission for peers to
commit different states.

A high-precision reference can compare contact intervals, material coverage
and error bounds without requiring its raw floats to equal production bits.
Production replay checks still apply to production. A reference with a larger
work budget is not the fallback chosen merely because a machine is faster.

Cache warmth and eviction must not decide whether an operation exhausts its
work allowance, defers or commits. Required prepared values stay available for
their admitted lifetime, or the cold path uses the same deterministic outcome
and publication policy. Logical work accounting must not change merely because
a repeated calculation was physically reused. Avoid moving an unpredictable
cache miss into authoritative gameplay as a different refusal.

## Binding time is the first optimization

| Fact | Prepare or update when | Runtime reads |
| --- | --- | --- |
| Clip semantic roles and track joins | A sheet/clip generation is prepared | Dense tracks and prepared clip properties |
| Extension entry, supplier and reply relationships | A selected module generation is admitted | Entry handles and ordered invocation spans |
| Object occurrence and private behavior binding | Construction, retirement, reconstruction or binding changes commit | The address through the operation's visibility/authority view |
| Pending delivery and deadline work | Journal/lifecycle changes update membership; active waits/retries advance and due conditions wake | Receivers with relevant work |
| Portal query coefficients and local candidates | Published geometry/motion changes; one source interval is selected | Shared preparation and the narrowest applicable solver |
| Navigation motion envelope and retained graph | Their exact movement/frame/probe or geometry inputs change | Reachability and routes over the selected immutable value |

Do not hash, sort or rebuild the full input merely to ask whether a prepared
value changed. The owning publication supplies identity/revision with the
immutable input. Small transient values can remain inline. A preparation
lookup must not cost more than the simple operation it replaces.

Cache retention does not confer semantic ownership. A value still named by
live state or retained history remains available under its owner's lifetime
contract. A derived index can be rebuilt; it cannot become a second journal
or let an old ordinal select a new generation's value.

Ownership, residency and replay entry/exit update the operation's scope and
visibility projection. They need not rebuild an unchanged occurrence-address
relation. Likewise, advancing a tick must not reclassify every mailbox:
advance the existing active work and due conditions. An acknowledged but
unfinished call remains in deadline/lifetime work; freeze retains that
membership without spending active time.

## Bound the unit of work

**Spatial queries:** dispatch by the source shape, interval, active occupancy
and conservatively reachable geometry. Distinguish an unsearched root from a
searched root with no crossing. Keep one prepared leg for world contact,
recipients and its accepted prefix where their input meanings agree. Retained
motion needs compact legs and crossing facts, not a stored copy of every
solver cell for every body.
The [portal preparation contract](portal-parts-similarity-and-chirality.md)
owns prefix reuse: preserve its original clock, initial overlap and boundary
ownership. An accepted earlier stop cannot require compiling a refused suffix.

**Presentation:** distinguish compiled clip facts, sampled pose, source pixels,
finishing effects and view placement. A finishing-only change must not force
source-part rasterization. An atlas page is storage; it is not automatically
the semantic invalidation unit. Gameplay pose and interpolated presentation
remain separate samples, even if they use the same prepared program.

**Behavior:** distinguish available vocabulary, live addresses and active
journal work. One addressed call must not search every installed entry or
reconstruct every live object's address. Adding a fact type must not install
another identical mailbox scan. An active projection preserves unresolved
calls, deadlines, retry and freeze semantics; an empty mailbox is not the
only inactive case.

**Navigation:** use the state that the cost law actually reads. Surface identity
is enough for the current boolean reachability relation; cost also reads the
arrival position on that surface. Retaining that position allows fixed edge
costs and reusable route work. Merging states before they are equivalent can
make both the result and its cache wrong.

## Selected domain packets

These are bounded changes within the existing product order. Portal execution
stays in P2/P4/P6; this table does not create a second portal queue.

| Packet | First customer and owner | Work removed | Required distinction |
| --- | --- | --- | --- |
| E1: prepared portal queries | [Portal delivery](portal-parts-similarity-and-chirality.md), core cast preparation and native callers | Repeated root search, duplicate source-cell builds, irrelevant pose evaluation | Local fast cases, bounded refusal continuation, compact retained history |
| E2: compiled visual facts and staged images | [Runtime part animation](runtime-rigged-sprite-animation.md), sheet preparation and renderer | Whole-clip invariant scans, stable track joins, source rasterization caused only by finishing changes | Exact sample/space, complete appearance key, cell versus page dependency |
| E3: prepared dispatch and active delivery | [Addressed behavior](addressed-behavior-inbox.md), extension admission, lifecycle and inbox owners | Entry-by-trigger matching, repeated occurrence capture, duplicate and idle mailbox scans | Entry-major serial order, fresh observations, replay visibility and active-time deadlines |
| E4: prepared navigation states | [Navigation](platformer-navigation-and-reachability.md), motion envelope and route owners | Repeated environment-independent probes and graph reachability work; incorrect merging of arrival states | Current graph cost model versus physical optimality; complete publication independent of worker timing |

Each packet identifies the old interpretation or repeated operation to delete.
Start with a production consumer. A new generic wrapper with the same scans,
allocations and full-domain invalidation is not completion.

### Existing wins remain in place

Preserve portal candidate filtering, fixed-pose reuse and bounded refusal
recovery; retained clip/material/image storage; whole-character raster removal
from ordinary part publication; shared admitted metadata and codec maps;
indexed event recipients and the unsubscribed early return; shared room walls,
parallel independent-body integration and the recent spatial target index.
Their historical cost receipts are not measurements of the proposed changes.

Resource slot binding, repertoire preparation and schema/native dependency
cuts already have owners. Repeating those plans is not another efficiency
finding. The [portable prepared-artifact hold](prevalidated-content-artifacts.md)
remains in force. In-memory preparation and offline reference use here do not
select that separate serialization, certificate or shipping-cache proposal.

## Evidence needed for a bounded packet

Record removed work as well as total time. Compare the same selected input
with cold and warm preparation, ordinary traffic and the case that actually
requires the general implementation. Existing profiling and owning tests
supply the instruments; this plan adds no permanent source-policy checker.

Useful controls target the proposed boundary:

- Add unrelated portal motion while keeping the queried material local.
- Add unsampled clip frames while keeping the selected sample, pose and
  compiled whole-body eligibility identical; the latter still depends on
  the complete clip at preparation.
- Change only opacity/HSV/warp finishing while source parts remain identical.
- Add installed entries and dormant receivers around one addressed invocation.
- Reuse one motion profile across rooms, or one graph among many querying bodies.

Include invalidation once: geometry or clip replacement, a changed admitted
module, occurrence reconstruction, room replay/visibility, or a changed
navigation profile. An invalidation probe must exercise the actual owner and
consumer; do not test a duplicate cache-key function in isolation.

Use a small number of high-value comparisons against a general/reference
path. Preserve gameplay, replay and appearance semantics. Measure simulation
per tick separately from ticks per displayed frame; separate CPU preparation,
GPU source/finish passes, retained bytes and cold publication cost. No packet
is complete merely because its inner loop became faster while its activation
or invalidation work became a new hitch.
