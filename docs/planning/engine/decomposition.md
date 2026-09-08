# Decomposition doctrine

**Standing package/dependency rules:**
[package and capability boundaries](../../architecture/package-and-capability-boundaries.md).
**Current target:** [architecture reassessment](architecture-reassessment.md).
**Execution:** [the queue](../queue.md).

This path is also cited by workspace policies. Before deleting or renaming it,
inspect `source_doc` references as well as Markdown links and preserve the argument
behind each rule. Do not edit policy links merely to make a path checker green.

Decompose by ownership and dependency value, not by line count. A move should
reduce the state, policy and lifecycle knowledge a consumer needs, or deliver a
specific installation, SDK or measured build benefit. It does not automatically
improve frame time, startup time or memory use.

## Decomposition has two dimensions

**Authority decomposition** asks who owns a fact, which writers may change it,
which invariant coordinates them, when other systems observe the result, and how
it is created, restored and retired.

**Capability composability** asks whether a consumer can select or omit a major
capability with only its declared prerequisites, without inheriting unrelated
resources, systems, content or host assumptions.

Authority decomposition is necessary for sound composition but does not prove
composition. A well-factored crate can still be mandatory in every application.
Conversely, a feature flag around badly divided authority does not repair it.

Measure composability along separate axes: runtime installation/lifetime,
resolved Cargo dependency and feature closure, public API, and deployment asset
requirements. They do not imply one another. A no-render plugin configuration may
still compile renderer dependencies. An ergonomic facade can still expose
internal module topology. These are independently falsifiable properties.

### The ordering is not negotiable

Establish semantic authority and dependency direction before inventing extension
machinery. Then expose the smallest intentional seam and let higher layers select
capabilities. Partial progress is allowed: a module-level ownership repair can
land before independent installation or a crate extraction.

This does not require finishing every engine authority before improving any
capability. Reassess readiness per boundary; a global declaration that all
prerequisites are crossed hides unresolved ownership in other regions.

### What moves with an authority

A packet accounts for state and writers; construction/defaults; stable identity;
queries/read models; private systems and public scheduling phases; run conditions;
deferred-command visibility; rollback declaration/encoding/checksum; teardown;
session/match/attempt/body/stock lifetime; and external effects.

A type plus a re-export is not that transfer. Nor is a new `SystemParam` that
requires the caller to assemble every old dependency. A source move can preserve
behavior while moving an interface in the wrong semantic direction; the actor-
spawn correction is the concrete example to remember.

### The user model

Provide an opinionated default composition and a small set of documented minimal
or capability-selected profiles. Major capabilities can be intentionally absent;
not every implementation crate needs to be optional or publicly named. Avoid an
unbounded Cartesian matrix of feature combinations that nobody can test.

The target consumer can construct and advance a body/world without named game
content, then add the capabilities they need. The exact passing profiles are
recorded by executable fixtures, not inferred from names such as core or runtime.

## Boundary selection test

Before introducing a trait, message, adapter, registry or shared value, answer:

1. Which knowledge of the other domain disappears from the caller?
2. Is the dependency semantically downward already, making a direct call simpler?
3. Does this operation require same-tick execution, deferred observation, rollback
   replay or confirmed external delivery? Would the new seam change that?
4. Who owns failure, cancellation, stale identity and lifetime cleanup?
5. Which supported customer needs substitution or independent installation?

If the only answer is a smaller SCC or fewer imports, do not add the abstraction.
A cycle inside one accepted control or custody authority can be correct. A module
called features containing unrelated authorities cannot be legitimized by calling
the whole cycle one package.

## Absence, readiness and defaults

A supported absent capability must not require dummy sibling resources. Validate
real prerequisites at installation/preparation, or explicitly represent supported
absence in the consuming API. Do not blanket-wrap every system parameter in an
Option and let required work disappear.

Represent absent, pending, ready and invalid states when their distinction affects
behavior. A plausible default camera, difficulty ladder or authored value must
not impersonate a prepared result. A presence check for AssetPlugin is not a
render-stack readiness check; name the actual prerequisite the system uses.

Scope readiness to the selected content revision, session and relevant device
materialization. A ready resource left over from a retired session is not evidence
that the new session is ready.

## Scheduling and integration

Capability owners install their private systems and order their internal phases.
Composition may order independent capability phases. Preserve phase ancestry,
run conditions, deferred flush behavior and the read/write relation; two identical
`.before` strings can behave differently under different parent sets or gates.

A semantic phase should describe an observable guarantee such as custody settled
or body geometry published. Do not create one public set for every private
function just to disguise function-level coupling. Do not collapse the whole tick
into a chain to make ordering failures disappear.

## Registration and extension

Explicit App-local provider registration is valid for real independently authored
providers. A closed recipe set may have metadata registration without executable
runtime discovery. Backend-neutral rollback registration is justified because it
separates domain declarations from a real backend adapter.

None of these establishes a need for a universal service locator, generic
message bus, type-erased dependency injection or global plugin discovery. Shared
vocabulary must have semantic meaning and a bounded interpreter. Two consumers
using a type is insufficient reason to put it in shared_tangle.

## Public API, naming and completion

Retain the ergonomic facade while migrating its consumers away from internal
namespace mirrors. Do not rename a mixed implementation container to a polished
capability name before its ownership is proven. Transitional warning names may
remain useful; they must have a documented exit, not become permanent excuses.

Completion evidence consists of a simpler responsibility map, fewer unrelated
writers/assumptions, a production behavior witness and the relevant profile/API
benefit. Source counts and SCCs are supplementary diagnostics. A module-level
move can succeed even if the SCC size is unchanged.

Related owners: [actor decomposition](actor-monolith-decomposition.md),
[composition](capability-and-runtime-composition.md),
[SDK](public-sdk-1.0.md), [work frontier](actor-monolith-work-frontier.md).
