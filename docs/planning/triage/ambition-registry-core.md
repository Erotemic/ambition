# `ambition_registry_core` — one protocol for canonical registries

> **State:** TRIAGE — PROPOSED DIRECTION, 2026-07-22.
>
> The direction is chosen: introduce a small, dependency-light
> `ambition_registry_core` crate for the protocol repeated across Ambition's
> registries. The exact Rust API remains an implementation decision and should
> be proved through one or two migrations before broad adoption.
>
> **Not a queue card:** this document records the direction only. Promotion into
> [`../tracks.md`](../tracks.md) should happen when the current construction and
> room-transaction campaign has a safe insertion point.

## Problem

Ambition has several independently useful registries whose implementations keep
repeating the same protocol:

- stable registration keys;
- owner, source, and schema metadata;
- rejection of empty or malformed identity fields;
- idempotent re-registration;
- structured conflict reporting;
- deterministic ordering independent of provider registration order;
- canonical dumps for diagnostics and tests;
- stable contribution to prepared-content or snapshot fingerprints;
- tests for order independence, idempotence, and transactional conflict failure.

Examples include construction recipes and relations, rollback registration,
placement lowering, content staging, character catalog fragments, boss catalog
fragments, encounter registration, and quest registration. Their domain values
are different, but their surrounding protocol is increasingly the same.

The cost is not primarily literal duplicate lines. The larger cost is semantic
drift. Recent work repeatedly found registries that differed on what counted as
identity, whether function addresses participated in equality, which metadata
entered a fingerprint, and whether a conflict left the old registry unchanged.
Those are protocol decisions and should not be rediscovered per domain.

## Decision

Create a workspace crate named:

```text
crates/ambition_registry_core
```

Its job is to provide a small shared vocabulary and canonical mechanics. Domain
crates continue to own:

- their key and value types;
- their storage maps;
- their executable functions and runtime dispatch;
- their override or layering policy;
- their Bevy resources and `App` extension methods;
- their domain-specific diagnostics and validation;
- the decision about which registrations affect which content identity.

This is **not** a universal generic registry container. It is a shared protocol
for registries that remain domain-owned.

## Proposed responsibilities

The first version should be intentionally small. Candidate responsibilities are:

### Registration metadata

A common metadata value for stable declarations such as:

```rust
pub struct RegistrationMeta {
    pub owner: String,
    pub source: String,
    pub schema_id: String,
}
```

The exact storage types are not decided here. The important behavior is:

- fields have explicit meaning;
- required fields are validated consistently;
- equality is semantic and stable across builds;
- function addresses and process-local values never define registration identity;
- implementation behavior changes require an appropriate stable schema change.

### Registration outcomes and conflicts

A shared distinction between:

- a newly inserted registration;
- an idempotent equivalent registration;
- a conflicting registration for an already-owned key.

The conflict representation should retain both existing and incoming stable
metadata. Domain registries may wrap it with their own key and context.

### Canonical row emission

A small canonical row or section writer that makes it difficult to accidentally:

- depend on insertion order;
- omit a field from the diagnostic dump but include it in the fingerprint, or
  vice versa;
- use ambiguous separators or ad hoc formatting;
- fingerprint process-local function addresses.

The core should frame stable rows; domain registries decide the row vocabulary.
For example, construction may emit recipe and relation rows while rollback emits
component and resource rows.

### Fingerprint section framing

A shared way to turn a canonical registry dump or row stream into a named
fingerprint section. The crate does not own the application's complete content
fingerprint. It provides the stable section-level mechanics used by the owner.

### Small validation helpers

Only validation that is truly protocol-level belongs here, such as required
nonempty metadata fields and deterministic duplicate/conflict classification.
Domain-specific key and value validation stays with the domain.

## Explicit non-goals

`ambition_registry_core` must not become an `ambition_utils` grab bag.
Specifically, it should not:

- own one generic `Registry<K, V>` used by every subsystem;
- erase domain types behind `Any` merely to share storage;
- register executable provider callbacks for closed domains;
- infer schema changes from function pointer identity;
- depend on Bevy unless a later proof shows a genuinely shared Bevy adapter is
  worth the fanout;
- own complete prepared-content assembly;
- absorb stable-ID design, test fixtures, serialization helpers, or unrelated
  collection conveniences;
- force registries with intentionally different layering semantics into one
  policy.

A useful test for every proposed addition is: *does this encode a registry
protocol invariant, or is it merely code two registries happen to use?*

## Dependency boundary

The preferred crate is dependency-free or nearly dependency-free. It should sit
below the domain registries and change rarely.

Bevy-facing conveniences should initially stay in the owning crate. A broad,
frequently edited foundation crate would increase rebuild fanout across an
already large workspace and would defeat the purpose of a stable core.

The crate must not depend on registries that depend on it. Fingerprint integration
should use stable bytes or rows supplied upward to the lifecycle owner rather
than pulling domain registries downward into the core.

## First migration

Do not migrate every registry at once. Choose two registries that already share
the intended semantics but exercise different domains:

1. the construction recipe/relation registry;
2. the rollback registry or one provider-fragment registry.

The pilot should answer:

- Is `RegistrationMeta` sufficiently expressive without becoming domain-aware?
- Can canonical dump and fingerprint bytes be generated from the same rows?
- Can conflict errors remain more informative than they are today?
- Does the abstraction remove policy drift rather than merely move lines?
- Is source code still obvious to a maintainer or coding agent reading one file?
- Does the crate avoid dependency cycles and excessive rebuild fanout?

If the second registry requires awkward adapter code or weaker diagnostics, stop
and narrow the abstraction rather than forcing adoption.

## Migration phases

### Phase R1 — inventory and invariant table

Before code, inventory active registries and record for each:

- stable key;
- owner/source/schema metadata;
- idempotence policy;
- conflict policy;
- ordering rule;
- canonical dump format;
- fingerprint consumer;
- executable/runtime fields that must remain outside stable equality;
- provider layering or override rules.

This table is migration evidence, not a permanent source-of-truth document. It
may be deleted when all decisions are represented by code and tests.

### Phase R2 — build the narrow core

Implement only the metadata, outcome/conflict, canonical-row, and section-framing
pieces demonstrated by the pilot registries.

### Phase R3 — migrate two registries

Move the selected registries without changing their public domain behavior.
Retain or improve their existing poison tests:

- registration order does not change canonical output;
- identical registration is idempotent;
- conflicting registration is rejected transactionally;
- stable schema changes move the relevant fingerprint;
- runtime function addresses do not affect canonical identity.

### Phase R4 — evaluate before expansion

Measure source reduction, diagnostic quality, compile fanout, and API clarity.
Only then decide which additional registries should migrate. Some registries may
correctly remain independent because their policy is genuinely different.

## Acceptance criteria for promotion

Promote this direction into the execution queue only when a bounded card can
name:

- the two pilot registries;
- the exact shared invariants being extracted;
- the crate's dependency ceiling;
- tests that distinguish semantic identity from runtime function identity;
- a no-behavior-change migration strategy;
- an explicit list of registries not being migrated in the first pass.

The work is successful when registry implementations become smaller **and** it
becomes harder for them to disagree about canonical identity, conflict handling,
and fingerprinting. Line-count reduction alone is not sufficient.
