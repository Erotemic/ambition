# ADR 0032: Authoring is declarative — content is a value, capability is declared, only the engine lowers into `App`

## Status

**Accepted** (2026-07-30), after the API 1.0 campaign implemented it across six
slices and proved it against six consumer-matrix categories. Its central claim —
"module inclusion is a MERGE, not an ordering" — was tested by two real modules
in slice D, and the identity half ("Sanic standalone and Sanic embedded produce
the same content and rollback-schema identities") is
`sanic_has_the_same_identities_standalone_and_embedded`.

Extends [ADR 0026](0026-immutable-prepared-content-and-exact-session-identity.md)
(immutable prepared content and exact session identity) from the *internal*
lifecycle to the *public authoring surface*, and extends
[ADR 0017](0017-rust-behavior-ron-content-ldtk-space.md) (Rust holds behavior;
RON authors content) to cover federated capability schemas.

Reached across four rounds of external review; see
`../archive/reviews/claude-reply-2026-07-30-api.md` (docs/archive/reviews/claude-reply-2026-07-30-api.md — removed from the checkout 2026-09-05; still in git history)
§8–§11. The historical executable API campaign is archived at
`../archive/planning-superseded/2026-08-13/engine/api-1.0-campaign.md` (docs/archive/planning-superseded/2026-08-13/engine/api-1.0-campaign.md — removed from the checkout 2026-09-05; still in git history).
Current authoring-product work is owned by
[`../planning/engine/authoring-and-tools.md`](../planning/engine/authoring-and-tools.md).

## Context

The obvious shape for a game-authoring API is a builder holding the app:

```rust
pub struct ExperienceBuilder<'a> {
    app: &'a mut App,
    // …
}
```

It is the shape most Bevy-adjacent libraries use, and it is wrong here for a
reason specific to this engine.

**ADR 0026 already fixed the lifecycle, and it puts authored input at a stage
before live mutation:**

```text
provider-owned fragments + prepared world source
    -> structured validation          (rejects bad input WITHOUT live mutation)
    -> deterministic assembly         (normalises semantically unordered input)
    -> immutable PreparedContent
    -> ContentFingerprint + ContentEpoch
    -> exact prepared-session publication
    -> lowering onto the canonical SessionRoot
```

A builder that mutates `App` collapses validation, assembly, preparation and
activation into "whatever the provider's `build()` did." It puts the public API
one stage past the point the engine's own contract says authored input is still
inert.

**Today's type is already pure.** `PlatformerExperienceAuthoring` is a plain
struct — ids, `AuthoredCatalogFragments`, optional specs, no `App`. Handing
providers a mutable `App` would be a regression, not an improvement.

**Order-independence is required and unobtainable from a mutation stream.**
ADR 0026: fingerprints *"never hash `Debug`, insertion order, randomized maps,
entity ids, handles, addresses, timestamps, readiness, or mutable requests …
Equivalent provider or registry insertion orders produce the same fingerprint."*
You cannot compute that over a sequence of `App` mutations without first
buffering them into a value and canonicalising it — and that buffer *is* the
pure definition. `&mut App` does not remove it; it hides it in a staging
resource and makes "is content complete yet?" unanswerable. That is not
academic: `RollbackSessionContract` compares content identity every frame and
invalidates the session when it changes.

**The decisive evidence is the machinery `Plugin::build` already forced us to
write.** `CharacterPreparationPlugin` needs three mechanisms, each paid for in a
real defect:

1. `fn finish()`, to fold the staged cast after every provider's `build`;
2. a `PreStartup` backstop, because **`App::update` does not run `finish`** and
   this repository drives `update` by hand nearly everywhere — every headless
   test, the external fixture, the rollback harnesses, the tools. Its own
   comment records the symptom: *"every character silently falls back to the
   host's compatibility kit, so a consumer's peaceful wanderer comes out
   swinging the protagonist's sword … it is what the outlander fixture reported
   within an hour of the barrier landing"*;
3. an idempotence flag, because **`App::finish` re-runs every plugin's `finish`
   every time it is called** — without it, a second call republished an *empty*
   registry.

All three exist to reconstruct *"the complete set of contributions"* from a
stream that has no completion signal. `Plugin::build` cannot tell you it was the
last one.

## Decision

**1. Content is a pure value. Authoring never mutates `App`.**

A module's content methods accumulate into an inert draft. Nothing a provider
writes is live when its `define` returns. The engine — which calls `define` —
seals the draft, validates it, assembles it deterministically, and fingerprints
it before anything is installed.

**Completeness becomes a `->` in a signature.** No `finish`, no `PreStartup`
backstop, no idempotence flag, no ordering hazard.

**2. Capability is code, and code is *declared*, then lowered by the engine.**

You cannot serialise a Bevy system into a content document, so capability
registration must eventually touch `App`. It does so through named methods that
record declarations, and an engine-owned installer applies them in a canonical
order. Preparation therefore produces two products:

```text
PreparedContent          — validated, canonical, fingerprinted authored data
PreparedCapabilityPlan   — ordered plugin installation + declared schema
```

**Ambition owns this plan explicitly**; a Bevy `PluginGroup` is its *lowering*,
not its substitute. `plugin_group` supplies ordered `Box<dyn Plugin>` installation
and nothing else — it has no notion of stable capability ids, dependency and
conflict resolution, duplicate-contribution policy, simulation-phase
declarations, rollback fragments, facet handlers, a schema fingerprint,
structured diagnostics, or a pre-session freeze boundary. The plan owns those and
lowers through a plugin group afterwards.

**3. The `App` restriction is scoped to module construction, not to the process.**

Inside `define`, mutation is staged. In the game's own `main`, the consumer has
an ordinary Bevy `App` and may do anything — its own asset loader, render
pipeline, inspector, any third-party plugin. Per
[ADR 0031](0031-public-facade-is-the-compatibility-boundary.md), `PlatformerApp`
is a plugin group, not a runtime that owns your app.

**4. Declarative is a property of the value, not a mandate for one file format.**

ADR 0017 still gives us a useful default: spatial layout belongs in a spatial
authoring backend, broad data-oriented content benefits from data files, and
reusable behavior belongs in code. D73 sharpened the important boundary,
however: **pure Rust construction of an inert authored value is still
declarative authoring.**

A provider may build a `CharacterDefinition`, rules document, generated world
fragment, or other content value in Rust when composition, reuse, procedural
authoring or type-safe capability references make that the clearest source. What
this ADR forbids is using authoring as an excuse to imperatively mutate `App`,
bypass preparation, or create a second runtime authority.

The choice is therefore not "RON good, Rust bad". It is:

```text
inert/provider-owned authored values
        -> validate / resolve / prepare
        -> deterministic installation / runtime projection
```

versus an imperative mutation stream whose completeness depends on plugin/order
history.

Broad cast metadata may still live efficiently in provider data such as
`character_catalog.ron`; character-specific intrinsic composition may also be
expressed by registered `CharacterDefinition` values. Preparation is where those
sources become one complete runtime character rather than parallel authorities.

Facet schemas remain federated: capabilities register the schemas they own so
third-party content can extend the authored model without editing a closed
engine enum.

**5. A raw string is never a runtime authority.**

Static Rust consumers use references generated from validated content
(`content::characters::MALLORY`). Dynamic content — tools, editors, mods — uses
authored identifiers converted at validation:

```text
UnresolvedContentRef<T>  --(pack validation)-->  ResolvedContentRef<T>
```

Resolution happens at ADR 0026's **validation** stage: before assembly, before
the fingerprint. Generated constants are a convenience over an already-validated
graph, never the safety mechanism — content authored by a tool or loaded as a
mod must be as safe as content someone wrote in Rust.

This repository deleted its String-keyed lookups (`row_index_of` and both
String-keyed art maps) for exactly this reason: **a bad id fails silently.** The
shipped gate for the same class is
`game/ambition_app/tests/declared_art_resolves.rs`, whose notes record why: *"a
declared image naming no file is indistinguishable from a bolt nobody skinned."*

**6. Content transactions are a first-class verb, not a later addition — and
they are NOT session transitions.**

The lifecycle is not `compose → run`. This engine already revises content
*during* a run: LDtk reload builds a replacement `PreparedContent` candidate
through the same assembly path.

So the API exposes `candidate → validate → commit → new epoch` from the start,
and **the draft type used at composition time and at commit time is the same
type.** Otherwise there are two content paths, which is the failure the
construction campaign exists to end.

⚠ **Two different transactions, sharing a confirmed commit boundary:**

```text
ContentRevision:    ContentDraft → validate → PreparedContent → new ContentEpoch
SessionTransition:  existing PreparedContent + current session
                        → confirmed lifecycle commit → new live baseline
```

A room transition **selects from existing prepared content**. It does not edit a
draft and does not publish a new content fingerprint. The first draft of this
ADR cited `PendingLifecycleCommit` and `commit_confirmed_lifecycle` as evidence
for the content verb; that was wrong — they are the *session* transaction, which
is why a room change rebases the rollback session without producing new content.
The two share a confirmed frame boundary and nothing else.

Content is immutable *within* a session and revised *transactionally* between
epochs. It is not frozen after compilation.

## Consequences

**Deletion criteria — the campaign is not done until these are true.** A new
abstraction earns its place by making an old compensating mechanism unnecessary:

* the `PreStartup` character-preparation backstop is **deletable**;
* provider plugin ordering no longer determines content completeness;
* repeated `App::finish()` cannot republish or alter prepared content;
* headless and visible hosts consume the same prepared-content fingerprint;
* Sanic standalone and Sanic embedded produce the same module-content and
  rollback-schema identities;
* no runtime character consumer reads a fallback authoring catalog.

**Errors arrive once, structured, at a known point.** A draft yields one build
error listing every conflict in the experience. `&mut App` yields a
resource-missing panic three plugins later — the failure `ShellComposition` was
created to end.

**Module inclusion is a merge, not an ordering.** `include(SanicModule)` over a
draft is a merge with transactional conflict detection, which the registries
already implement (byte-identical fragments idempotent; opaque room-stager
closures reject duplicate ownership). Over `&mut App` it is "did Sanic's plugin
run before Mary-O's".

**One open question this ADR does not close**, deliberately, because it is
unproven: **what happens when a content document names a facet schema that no
installed capability claims.** *Ignore* recreates the prepared-but-unconsumed
portrait field at scale, and "state that looks accounted for and is not" is this
repository's single most recurring defect class — six of eleven fixes in the last
campaign. The campaign's working answer, stated as a contract on the AUTHORED FACET rather
than on the schema registry:

* every authored facet resolves to exactly **one compatible installed handler**;
* the capabilities it requires are installed;
* it validates and canonicalises;
* it has a declared **prepared disposition and runtime projection**, or is
  explicitly marked authoring-only;
* ambiguous ownership is rejected unless explicitly supported.

⚠ **An installed schema with zero authored instances is VALID** — a capability
installed and unused is ordinary. The first draft of this ADR said "a registered
schema no consumer reads is an error", which conflated *no handler* with *no
instances* and would have failed every optional capability. The portrait-field
hazard is covered by the prepared-disposition bullet, which is where it belongs.

A pack declares a **minimum required capability set**, not one exact named
profile. That becomes ADR material once a slice has proven it.

**A version-space obligation.** A versioned facet schema (`ambition.body.sprite@1`)
must be deliberately related to the existing version spaces —
`SnapshotSchemaFingerprint`, `GGRS_ROLLBACK_SCHEMA_VERSION`, and ADR 0026's
fingerprint-schema version — because that relation decides whether yesterday's
save loads today. Ask it before `@1` is minted; the first migration is when it
gets expensive.

## Alternatives considered

**`ExperienceBuilder { app: &mut App }`** (the conventional shape). Rejected:
contradicts ADR 0026's validation stage, regresses from the pure
`PlatformerExperienceAuthoring` that exists today, makes order-independent
fingerprinting impossible without a hidden buffer, and keeps every piece of the
`finish`/`PreStartup`/idempotence apparatus.

**Pure content *and* pure capability — no `App` access at all.** Rejected: a
system is not data. Restricting capabilities to an engine-known closed set would
buy purity by forbidding third-party capabilities, which is the federation
property the whole design depends on.

**Imperatively enumerate content into `App` from Rust.** Rejected. Rust-authored
*values* are allowed; a provider walking a cast and mutating runtime resources or
registering order-dependent side effects as the authoring model is not. Large
row-oriented content should still prefer data/generated sources when that
improves iteration, but compile avoidance is an ergonomics consideration rather
than the definition of declarative authoring.

## Current implications for agents

- **Inside `define`, never mutate `App`.** Content accumulates into an inert
  `ModuleDraft`; capability is *declared* (`capability(plugin)`) and lowered by
  the engine in canonical order. The consumer's own `main` keeps its ordinary
  Bevy `App` — the restriction is scoped to module construction.
- **Gameplay systems a capability installs go in `app.sim_schedule()`, never
  `Update`.** `Update` puts gameplay outside the rollback simulation entirely;
  asking for the schedule is what makes one plugin correct on both hosts. See
  "Where your gameplay systems go" in `docs/sdk/api-reference.md`.
- **Completeness is a return value.** Do not add `finish()` hooks, `PreStartup`
  backstops, or idempotence flags to reconstruct "all contributions" from a
  mutation stream — that apparatus is what this ADR exists to delete, and its
  deletion criteria in §Consequences are the scoreboard. Work claiming to
  advance this ADR should say which criterion it made deletable.
- **A demand a module cannot meet is refused with a structured error naming
  the fixes** (`characters(ron)` vs `no_characters()`), never satisfied by a
  silent default — a silently substituted empty catalog is how a game ships
  its bosses drawn as the fallback body.
- **A raw string is never a runtime authority.** Unresolved refs resolve at
  validation, before assembly and the fingerprint; generated constants are a
  convenience over an already-validated graph, never the safety mechanism.
- **Content revision and session transition are different transactions.** A
  room transition selects from existing prepared content; it does not edit a
  draft or mint a content fingerprint. Do not cite the session lifecycle
  machinery as the content verb — that conflation was corrected in this ADR
  once already.
