# Authored techniques: installed support, checked flow and activation

**Status:** target implementation contract for A11/A12 and the definition-level
part of A10. Not implemented by this planning overlay.
**Baseline:** `300004d601af1e633cfaee969f079cf9bb368ca8`.
This page owns move-scoped technique admission and execution. It does not define
a universal encounter/dialogue sequencer or replace each domain's authoring
format. The [authoring tools plan](authoring-and-tools.md) owns the larger agent
workflow; the [queue](../queue.md) selects execution order.

## Decisions

An authored move becomes executable only after it is checked against the actual
selected capability profile. The same typed capability installation that supplies
a technique handler must supply its support/validation declaration. A human or
agent cannot make a misspelled key valid by omitting its parameter schema.

TechniqueFlow version 1 is a **bounded acyclic move-local program**. Its cursor
and contact latches belong to one MovePlayback occurrence. It emits requests to
existing domain owners; it never owns their body, projectile, capture or item
state. It has no dynamic code loading, arbitrary component access, recursive
program calls, blocking model calls or generic service lookup.

Publication first guarantees last-good **prepared definitions** on rejection.
Activation of a different mechanical revision initially occurs at an explicit
session/reconstruction boundary. Do not implement live arbitrary-world
replacement merely to let an LLM edit a move. Trusted Rust providers remain
trusted code; a typed installer does not sandbox them.

## Current implementation and the important correction

| Source | Established observation |
| --- | --- |
| `crates/ambition_entity_catalog/src/lib.rs` | EffectRef/ParamValue, the parameter registry, MoveSpec/HitVolume/FlowNode and structural flow validation |
| `crates/ambition_characters/src/prepared.rs` | Character preparation, final definitions, registry generation and registration lifecycle |
| `crates/ambition_characters/src/moveset_authoring.rs` | Move authoring transformation surface |
| `crates/ambition_characters/src/moveset_prefabs.rs` | Shared move-prefab expansion/authoring |
| `crates/ambition_combat/src/moveset/mod.rs` | Live flow interpreter, u16 cursor, move clock/contact latches and normal teardown |
| `crates/ambition_platformer2d_runtime/src/combat_schedule.rs` | Move/effect execution before hit-resolution feedback |
| `game/ambition_demo_smash/src/capture.rs` | A real native technique handler and explicit commentary about disconnected parameter validation |

The parameter registry currently accepts unknown keys and overwrites duplicate
registration. It has no production validation caller at this baseline. A
registered check is therefore neither proof that a handler is installed nor
proof that an authored reference was checked. Function-pointer equality is
irrelevant to rejecting a duplicate key: the key and claimed ownership already
suffice to report the conflict.

Current flow validation accepts in-range usize transitions without proving they
fit the u16 runtime cursor. Its positive-timeout check admits positive infinity,
and existential reachability of Finish does not exclude a cycle on another branch.
The runtime's per-tick node-count budget prevents an unbounded single interpreter
loop, but that node count is itself author-controlled and permits repeated Emit
cycles on successive ticks.

**Flow completion does not currently decide when a move ends.** MovePlayback's
`finished()` tests its timeline against MoveSpec duration, and the normal update
runs teardown when that condition is reached. Finish stops flow activity; it does
not remove the move's recovery. An unfinished Wait does not extend the move.
Existing charge, repeat-window and proper-time rules can delay when the timeline
reaches its end; that is different from a flow owning lifetime. Correct source
comments and planning claims that describe an unreachable Finish as necessarily
trapping a fighter forever. Do not "repair" them by changing the real lifetime
policy to match that prose.

### Current authored-flow census

The three concrete flow constructions found in game Rust source are:

| Source | Nodes | Structure |
| --- | ---: | --- |
| `game/ambition_demo_smash/src/moveset.rs`, read_and_seize | 3 | Wait Connected, Emit capture, Finish |
| `game/ambition_content/src/goblin_moveset.rs` | 3 | Wait Connected, Emit capture, Finish |
| `game/ambition_content/src/ninja_shadow_oni_leader_moveset.rs` | 4 | Wait Overlapped, Branch Blocked, Emit teleport, Finish |

All are acyclic and emit at most once. This is a source-constructor census, not a
claim to have executed all profiles or enumerated every external/serialized
input. The preparation test corpus must include dynamically produced/serialized
moves before shipping the new admission rule.

## Trust boundary and preparation pipeline

Use two explicit levels:

| Input | Trust and guarantee |
| --- | --- |
| Authored data and bounded TechniqueFlow | Must pass structure, installed-support, parameter and reference validation before activation |
| Native Rust provider/validator/handler | Has application privileges; its implementation is reviewed/tested, not proven safe by metadata |

The pipeline is:

```text
source decode and domain expansion
    -> collect final authored reference sites
    -> bind to a frozen installed-support profile
    -> structural + semantic validation
    -> private checked flow/technique representation
    -> candidate prepared character/content revision
    -> headless fixture and review receipt
    -> explicit activation at a supported lifecycle boundary
```

Raw decoding can happen before App composition. Installed-profile validation
cannot: its support set only exists after selected native capabilities declare
it. Place that stage after capability installation and before production body
materialization or active-definition publication. Do not force App/function
pointers into a file decoder, or publish a definition as fully checked while
promising to discover missing handlers later.

Character finalization/registry insertion must require the checked result.
Enumerate every production insertion/constructor path in `prepared.rs`, including
provider registration and already-prepared insertion. Direct native or test
construction must not accidentally acquire the same checked label without its
proof inputs. Keep test/raw constructors clearly separated; they are not a
second public path to activating unvalidated authored data.

## Installed support: concrete API and ownership

These are proposed type roles rather than existing public names.
<!-- cite-ok: proposed support API vocabulary -->

```text
TechniqueSupportBuilder   mutable only while selecting a profile
    offer(key, owner, schema_revision, parameter_policy,
          allowed_sites, delivery_class, nested_reference_policy)
    freeze() -> FrozenTechniqueSupport or diagnostics

FrozenTechniqueSupport
    immutable declarations for exactly this installed profile
    read-only discovery and lookup
    no gameplay-handler callback lookup

PreparedEffectRef
    validated key + unchanged structured parameters
    checked site and support-profile identity
    source provenance
```

Put the small declaration/validation vocabulary with the existing entity-catalog
schema only if it stays data-oriented. App/resource wrapping and typed handler
installation belong with the domain/runtime integration that already uses Bevy.
A schema package does not gain a Bevy dependency merely to offer `Plugin` methods.

Each concrete capability's typed installation does two things in one place:
installs its native translator/reducer in the declared schedule, and contributes
its declaration to the profile builder. Tests exercise that offer rather than
constructing an unrelated registry entry by hand. Profile finalization is an
explicit fallible operation after offers; do not depend on incidental plugin
insertion order or late global discovery.

Declarations contain:

- stable key, owning domain/provider, schema revision and source/provenance;
- either **Paramless** or a validator that hydrates the domain's parameter type
  and checks its semantic constraints, plus its nested-reference policy;
- supported reference sites and delivery semantics, including required contact
  context for on-hit use and whether sustained invocation is supported.

Paramless means the canonical empty parameter map, matching current ParamValue
Default. It does not mean accepting arbitrary ignored fields. Validators must
reject nonfinite numbers and invalid ranges relevant to their domain, missing
required data and unknown fields according to the declared schema. A successful
serde hydration alone does not establish these properties.

Duplicate keys fail profile finalization even if metadata matches. Do not compare
function addresses, treat repeated offers as overrides, or use last-write-wins.
Bevy plugin uniqueness or an explicit once-only typed installer handles legitimate
repeat installation. Selecting a replacement implementation is an explicit
composition choice that omits the original offer; it is not an author-controlled
runtime registration mechanism.

Freeze the builder before semantic preparation. A later capability change creates
a new profile and requires revalidation; it does not mutate the support behind
already prepared definitions. Canonical metadata fingerprints identify those
metadata, not native function behavior. Same-build identity remains a separate
input to replay/rollback compatibility.

### Availability and useful diagnostics

Unknown key, known-but-not-installed key, unsupported site and invalid parameters
are distinct errors. A documentation/discovery catalog may describe available
but absent capabilities; only the frozen **installed** set can authorize a call.
Do not install every game technique to make a generic engine profile validate.

The prepared value need not initially type-erase every possible hydrated payload.
Validation may retain the existing structured ParamValue, with the native domain
handler hydrating through the same declared schema. That is validation-before-
execution, not a claim of zero runtime decoding. Optimize a proven hot payload
inside its owner later; do not introduce an Any-based executable registry to
avoid a measured-as-yet-unknown cost.

## One exhaustive effect-reference traversal

Implement one typed visitor over the **expanded** MoveSpec representation. It
returns the reference, its contextual site and its provenance. The mandatory
current sites are:

| Site | Existing structure | Context |
| --- | --- | --- |
| Timeline effect | MoveEvent with Effect kind | Owner/move occurrence; no promised victim |
| Sustained window | MoveWindow.sustain_effect | Owner plus active-window identity; called under sustain timing |
| Hit consequence | HitVolume.on_hit, for every volume in every window | The accepted hit context supplied by its dispatch road |
| Flow emission | Every FlowNode Emit | Owner/move occurrence; contact latches do not automatically identify a particular victim |
| Nested domain references | Inside opaque technique parameters or expanded prefab inputs | Enumerated by that domain's declared schema, with explicit site mapping |

Run validation on final expanded moves, preserving original prefab/override
provenance so errors point to the source edit. Scanning only a prefab definition
misses invalid override results. Scanning raw RON for strings named `key` cannot
distinguish techniques from sound, animation, content or display keys and is not
an acceptable visitor.

A technique schema declares **no nested technique references**, or provides a
pure enumeration of them with paths and contextual sites. No implicit opaque
"probably none" case is allowed in a definition advertised as fully checked.
Use the same visitor for admission, forward dependencies, reverse-reference
inspection and discovery examples. Do not maintain four lists of effect-bearing
fields.

For this version, bound a move's expanded technique-reference traversal to 1,024
sites and nesting depth 16, rejecting cycles in the active reference-expansion
stack. These are explicit engineering safety limits, not measured performance
claims. Count occurrences, not only unique keys: the same effect in two windows
has two provenance sites. Memoization may avoid repeated definition validation
without discarding call-site diagnostics. Validate the current preparation corpus
against these limits before release; do not automatically raise them on failure.

When a schema adds an effect-bearing field or flow variant, its exhaustive visitor
match and poisoned-reference fixture change in the same commit. For each listed
site, inject an unknown key and invalid parameters independently and prove both
prevent publication. A single successful flow-Emit test cannot certify the other
sites.

## Prepared TechniqueFlow version 1

### Admission policy

The following are required, not options left to each implementation agent:

1. A present flow contains 1 through **256 nodes**, starts at node 0, and every
   node is structurally valid. An absent flow remains an ordinary timeline move.
2. All transitions are in range. Convert them with checked conversion into the
   prepared index type only after validating length and bounds. No `as u16`
   narrowing can create a prepared edge.
3. Every node is reachable from entry when both branches are considered. Reject
   unreachable nodes instead of preserving unchecked executable material.
4. The reachable directed graph is acyclic. Every path therefore ends at a
   Finish node when waits choose an edge. Reject a backedge even if another
   branch reaches Finish or a runtime per-tick budget would interrupt the cycle.
5. Every Wait timeout is finite and strictly positive. Move duration and the
   relevant charge/repeat timing parameters must satisfy their own finite domain
   constraints; do not claim flow validation alone validates all MoveSpec fields.
6. Every Emit passes installed-key, site, parameter and nested-reference checks.

The 256-node cap is a design budget, not a claim that 256 is an optimal frame-time
threshold. The current three-/four-node customers fit it. It supplies a small
checked execution representation and a bounded number of instantaneous dispatches.
A larger u16 address space would not be a useful execution-budget policy.

Version 1 rejects cyclic authored flows deliberately. Existing repeat windows
remain the supported mechanism for repeating timeline actions; they do not reset
the flow cursor. A later bounded repetition construct needs an explicit iteration
budget and lifetime tests. Do not smuggle it in as a backward node edge or add a
VM before a concrete move needs it.

### Prepared representation

Use a private checked representation adjacent to the current action schema and
executor, with validated indices and immutable effects. MovePlayback carries a
reference to the prepared revision and only occurrence-local mutable state:
cursor/done state, elapsed wait time and existing contact latches. It does not
clone the full graph each tick or reach into a mutable authoring registry.

No arbitrary public constructor can label a raw graph prepared. Preserve a
checked construction entry for native fixtures/providers and return diagnostics
rather than relying on every caller to invoke a separate `problems()` method.
It is acceptable to stage this representation change after installing the raw
validation gate; do not claim the gate is complete until all execution entrances
use it or are explicitly trusted/raw paths.

The interpreter has a defensive step guard equal to the checked node count, at
most 256, and treats guard exhaustion or a missing prepared node as an invariant
failure. Report the move/revision/occurrence and cancel through normal move
teardown. Do not hide corrupt execution as a successful Finish or emit again on
every later tick. Already emitted requests are not generically undone; the
prepared checks prevent this case for admitted authored data.

### Exact execution semantics to preserve

| Operation / condition | Required behavior |
| --- | --- |
| Emit | Emit once on entry through the ordinary move-effect lane; advance to its checked successor |
| Branch | Test the occurrence's existing latch snapshot immediately; choose one successor |
| Wait already satisfied | Take the success edge immediately, even if its timeout is also reached |
| Wait timeout | Take timeout edge only when the signal is not satisfied |
| Wait not resolved | Retain cursor and elapsed wait; return for this tick |
| Finish | Mark flow done; leave the timeline, recovery and normal cancellation rules in control of the move |
| Timeline reaches its end | Tear down normally even if a Wait has not completed |
| Interruption, landing cancellation, owner retirement | Existing move teardown; no detached flow continuation |
| Repeat window wraps | Existing timeline events/windows can rearm; the move-local flow does not restart |

Advance the flow's wait clock once per move update using the **effective** proper-
time delta after existing charge/hold adjustments. Do not substitute wall time,
raw fixed-step time or absolute timeline `t`, which can repeat. Preserve the
existing order: accrue delta, then follow instantaneous nodes; every taken edge
resets wait elapsed. A newly entered Wait does not also receive a second copy of
that tick's delta. Contact still has priority over timeout on that update.

The existing final update can have an effective delta greater than the remaining
clamped timeline duration. Preserve that behavior in the admission/representation
migration and test its timeout result; changing clock clipping is a separate
behavior decision. Hitstop/time dilation/charging must use the same delta selection
as the move, including zero advancement. Finite flow timeouts are proper-time
limits, not a promise that a paused game completes them in wall-clock seconds.

Contact signals are **latched for the move occurrence**. Overlapped includes the
existing connected/blocked contact classifications; Connected is not Blocked,
and parry must retain its existing classification. Two sequential Waits on the
same already-latched signal can both succeed. Do not clear a shared latch when
one flow node reads it. Fresh-per-beat contact requires a new scoped signal design,
not an accidental reset of another system's occurrence facts.

## Delivery phases and asynchronous domain lifetimes

A support declaration must state which producer sites reach which existing
consumer phase. The initial delivery classes are ordinary move effects,
sustained-window effects, and accepted-on-hit effects; the declaration cannot
make an unsupported combination work merely by saying it does.

Move/flow emission happens before hit resolution. A hit confirmed in Resolve
updates the move latch and can affect flow evaluation at its **next eligible
update**, not by recursively running the move again in the same tick. On-hit
requests use their existing later lane. A handler whose phase has already passed
must explicitly support next-tick delivery or refuse that authoring site during
preparation. Never depend on Bevy reader insertion order to deliver it this tick.

An Emit asks a domain to do an operation; it does not retain a callback into the
move. When an emitted operation can outlive the occurrence, its domain declares
one of two lifetime policies: independently owned after acceptance, or tied to
that move occurrence and cancelled with it. Carry the existing move-instance and
stable owner identity when that policy requires them. A late outcome for a
previous move cannot mutate the next move's latches because the owner entity and
move name happen to match.

Do not add per-projectile expiry, self-hit, control leases or arbitrary named
signals to the version-1 contact latch set. The proposed controllable-projectile
customer needs its own typed occurrence/lifetime protocol before it can be
represented honestly. Its absence does not justify inventing a universal signal
bus inside this small admission packet.

## Candidate revisions and explicit activation

A rejected edit leaves the active prepared registry, its generation, current
mechanical content binding and all live MovePlayback references unchanged.
Build the candidate off to the side; validation diagnostics are outputs, not
partial writes into the active registry. Discovery distinguishes available,
installed, candidate-prepared and active states.

An accepted candidate records its source identity, profile/support identity,
mechanical content identity, validation result and preparation version. Native
handler compatibility is constrained by the same-build policy; a metadata hash
alone cannot establish it. Existing moves continue to reference the exact
prepared definition they started with until a supported activation boundary.

For the first implementation, apply mechanical revisions at a new-session or
explicit reconstruction boundary that already replaces the authoritative
baseline. Do not hot-swap a registry underneath active moves or mix old mechanics
with new mechanical geometry. In rollback mode, the new revision begins a new
compatible baseline; ordinary snapshots from before activation are not restored
into the new revision.

Preparation rejection preserves last-good definitions. Trusted construction
failure after destructive mutation only guarantees fail-closed publication under
[the construction contract](immutable-content-and-transactional-construction.md).
Keeping an old live world running through arbitrary commit failure requires A10's
constrained inactive candidate and is not promised here. This separation permits
agent iteration without first solving general transactional ECS replacement.

## Implementation order

### A12a — DONE 2026-09-09

Add finite-time, checked-index, graph-size, reachability and DAG checks in the
entity-catalog flow validator. Add the three production flows as fixtures. Correct
comments about flow-owned move lifetime and document the deliberate rejection of
cycles. Keep the interpreter's valid-flow behavior unchanged in this commit.

**Landed.** `TechniqueFlow::problems` now rejects, each with its own
poison-verified case in `entity_catalog`'s `technique_flow` module:

- **A wait that never expires spelled as a number.** `f32::INFINITY > 0.0` is
  TRUE, so the mandatory-timeout check admitted exactly the unbounded wait it
  exists to forbid. (`NaN` was already refused by the same comparison; both are
  asserted so the pair cannot drift.)
- **A graph wider than `MAX_TECHNIQUE_FLOW_NODES` (256).** The bound is a CURSOR
  bound before it is a budget: `MovePlayback::flow_node` is a `u16` and the
  interpreter writes every transition with `*then as u16`, so node 65,536
  silently becomes node 0 — a terminating flow turned into a loop by a narrowing
  cast, with every edge still in range and nothing to report. Both sides of the
  boundary are tested; a limit checked only from above is a limit nobody has
  shown is reachable.
- **Any cycle reachable from node 0**, with the loop printed as the path that
  closes it. `reaches_finish` is EXISTENTIAL, so a branch that terminates on
  `then` and loops on `otherwise` passed it — and which road the fighter takes is
  decided at runtime by whether the strike connected.
- **A node nothing arrives at**, named by index.

**The three production flows needed no new fixture, and that is the better
answer.** `ambition_content`'s `every_authored_flow_in_the_shipped_rosters_validates`
already walks every authored flow in every shipped roster with an anti-vacuity
floor, and `ambition_demo_smash` validates its own; all still pass under the
stricter rules. A hand-listed trio would have been a third copy that goes stale
the day a fourth flow is authored. ⛔ That population guard's DOC BLOCK was
detached — sitting twelve tests above its own function, where it read as a second
paragraph of an unrelated test's rationale — and is re-homed.

**The lifetime prose is corrected, measured at the runtime rather than taken from
this page.** `MovePlayback::finished()` is `t >= spec.duration_s`: the TIMELINE
ends the move and teardown runs on that condition whatever the flow is doing. So
none of these failures traps a fighter, and three comments plus two diagnostics
said they did. What a broken flow actually costs is authored intent that never
runs — and, for a cycle, an `Emit` reached again and again inside the move's
window, a technique fired N times where the author wrote one. That is a smaller
and truer claim, and it is the one the error messages make now.

The interpreter is untouched: valid flows behave exactly as before.

### A11a: make profile support authoritative

Introduce the pure support builder/frozen result. Wire the actual capability
offers and handlers together. Add duplicate, unknown, disabled, paramless and
semantic-parameter tests using the installed capture/teleport customers, then
connect profile-bound validation to character preparation before insertion.
A metadata-only test does not prove an installed handler exists.

### A11b: close every reference position

Add the exhaustive expanded-move visitor and domain nested-reference policies.
Use it for validation and discovery. Poison each reference site separately and
verify unchanged active registry/generation on rejection. Inspect prefab overrides
and all public production insertion paths; remove duplicate check lists.

### A12b: install the checked immutable runtime representation

Make the prepared constructors private/fallible, convert edges once, pin prepared
revision on MovePlayback and eliminate the per-tick graph clone. Preserve timing,
latched-contact, occurrence and teardown semantics with differential traces of
the three current customers. Update rollback registration/checksum inputs for
changed occurrence fields; immutable definition identity follows existing content
binding, not pointer addresses.

### A11c: activation receipt through the public authoring route

Exercise a real provider-defined technique through edit, profile validation,
headless test, candidate review and explicit session-boundary activation. The
public route must reject a stale apply base instead of overwriting another edit.
Expose structured diagnostics and installed support from the same authority.
Do not add another generic CLI when an existing authoring tool can carry them.

## Required tests and diagnostics

| Fixture | Required assertions |
| --- | --- |
| Unknown / known disabled key | Different actionable diagnostic; no preparation or active-registry mutation |
| Duplicate key, same or different metadata | Profile conflict without function comparison or implicit override |
| Paramless with nonempty map | Rejected instead of ignored |
| Hydratable but invalid range/unknown field/nonfinite value | Domain semantic validation rejects with exact source path |
| Every effect-bearing site poisoned | Each prevents publication, including nested payload and prefab override |
| Oversized/recursive nested references | Deterministic bounded preparation failure; no partial prepared output |
| Empty, 256-node, 257-node flow | Explicit absent-versus-empty policy; legal boundary admitted, excess rejected |
| Target index 65,536 / out-of-range edge | Rejected before any narrowing; no cursor wrap |
| Finish on one branch, cycle on the other | Rejected despite existential Finish reachability |
| Unreachable node / timeout infinity or NaN | Rejected with node/path-specific diagnostic |
| Wait success and timeout together | Success priority preserved |
| Two waits after one contact | Existing latch reuse, no destructive read of shared contact state |
| Final-tick timeout / charge / repeat / time dilation | Existing effective-time behavior; flow does not extend or restart the move |
| Finish early / wait longer than remaining move | Recovery remains; normal move end still cleans up |
| Signal from Resolve | Read on next eligible flow update, not a same-tick recursive rerun |
| Late domain result after another move starts | Cannot mutate the new occurrence |
| Invalid raw/native runtime construction | Defensive invariant diagnostic/teardown, not a fake prepared value |
| Edit rejected during active play | Active revision, registry generation and playback reference unchanged |
| Valid revision activated at reconstruction | One coherent new baseline; no mixed old/new mechanics |
| Rollback over flow Emit/Wait | Same cursor, latch and effect occurrence behavior under established rollback policy |

Diagnostics include stable code, stage, provider/profile, source definition and
field/node path, expected contract, observed value and selected revision. Sort
them deterministically by source/site/code. A bare "invalid flow" or log followed
by default behavior is not an admission result an author can repair.

The bounds above constrain flow dispatch and reference preparation. They do not
bound arbitrary native handler cost or sandbox native code. Do not advertise
complete runtime resource isolation on the strength of a 256-node limit.
