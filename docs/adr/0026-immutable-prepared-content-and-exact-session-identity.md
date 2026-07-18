# ADR 0026: Immutable prepared content and exact session identity

## Status

**Accepted; implemented** (2026-07-18). This records Milestone A of
[`immutable-content-and-transactional-construction.md`](../planning/engine/immutable-content-and-transactional-construction.md).
The next executable milestone is explicit provenance plus one three-origin
`ConstructionPlan` vertical slice.

## Context

Provider preparation previously produced a `PlatformerSessionWorld` that mixed
prepared definitions with mutable live requests. Prepared publication moved that
bundle into the session root, snapshots authorized compatibility with provider
names plus room ids, and LDtk reload replaced world data without changing an
exact content identity. Equal names therefore did not prove equal definitions,
and registry insertion order remained an accidental authority in several seams.

## Decision

The authoritative lifecycle is:

```text
provider-owned fragments + prepared world source
    -> structured validation
    -> deterministic assembly
    -> immutable PreparedContent
    -> ContentFingerprint + ContentEpoch
    -> exact prepared-session publication
    -> lowering onto the canonical SessionRoot
```

The terms are distinct:

- **registration** records one provider/source-owned contribution without
  changing an already valid contribution on conflict;
- **validation** rejects malformed or conflicting authored input without live
  mutation;
- **deterministic assembly** normalizes semantically unordered input and emits
  canonical sections and diagnostics;
- **preparation** validates and assembles a complete candidate publication;
- **activation** consumes that exact publication and lowers its immutable source
  into mutable live components on the canonical session root;
- **immutable content** is authored/prepared definition data, never music or
  encounter requests, runtime cursors, readiness, handles, or presentation
  caches;
- a **content fingerprint** identifies behaviorally meaningful prepared
  definitions under an explicit fingerprint-schema version;
- a **content epoch** is the App-local generation assigned to a successfully
  published or committed prepared definition. Equivalent definitions may share
  a fingerprint but different preparations receive different epochs;
- a **snapshot-schema fingerprint** identifies the canonical registered codec
  schema, including entry kinds/types, message channels, dynamic anchors, and
  structurally derived declarations.

`PreparedPlatformerSource` owns immutable room/catalog/start-character/index
definitions. `PlatformerSessionWorld` is only the mutable live bundle produced by
`PreparedPlatformerSource::instantiate_live`. `PreparedContent` privately owns
the source and canonical fingerprint sections behind authoritative shared
ownership; it exposes no mutation API. The root carries both the exact prepared
object and an inspectable `PreparedContentIdentity`.

Fingerprints use BLAKE3 over versioned, length-delimited canonical sections.
They never hash `Debug`, insertion order, randomized maps, entity ids, handles,
addresses, timestamps, readiness, or mutable requests. Audio asset handles,
loaded waveform/presentation caches, and streaming readiness are excluded; the
provider-owned audio identity and authored expectations remain included.
Equivalent provider or registry insertion orders produce the same fingerprint.
Equal provider names and room ids do not imply equal content.

The first registry-lifecycle proof covers:

- `CharacterCatalogRegistry`: mature transactional deterministic assembly;
- `RoomContentStagingRegistry`: formerly permissive insertion-order behavior;
- `PlacementLoweringRegistry`: formerly assert/panic-based duplicate ownership.

These retain domain-specific types while sharing explicit owner/source/schema
metadata, transactional conflict behavior, canonical dumps, and explicit
fingerprint contributions. Idempotence is allowed for byte-identical character
fragments and exact placement-lowering registrations; opaque room-stager
closures reject duplicate ownership rather than pretending equivalence. There
is no implicit override.

Snapshots capture content fingerprint + fingerprint-schema version and snapshot
schema fingerprint. Restore checks session routing, then exact content and schema
identity, before room staging or any world mutation. Provider/room summaries
remain diagnostics, not compatibility authority.

LDtk reload builds a replacement `PreparedContent` candidate with the same
assembly path before commit. The candidate is normalized to the immutable
definition's original activation room so the mutable live-room cursor cannot
manufacture a content change. Invalid candidates cannot affect the active
object, fingerprint, or epoch. A materially changed successful reload commits
world and prepared identity together and advances the epoch. An equivalent
reload keeps both fingerprint and epoch, avoiding rollback-history invalidation
for a no-op. Ordinary room transitions never change content identity.

## Consequences

- Active sessions are pinned to one exact immutable prepared definition.
- Candidate reloads cannot be observed as active before commit.
- Snapshot incompatibility is an actionable preflight refusal.
- Deterministic dumps expose epoch, fingerprint, owners, sections, and snapshot
  schema for tests and developer inspection.
- Publication ids and load transaction ids remain lifecycle/routing identities;
  they are never content fingerprints.
- This is a **managed same-build contract**. Type names and an explicit global
  codec-schema version are meaningful within the build; this ADR does not claim
  universal cross-version save compatibility.
- Construction provenance, general `ConstructionPlan`, public prefab APIs, and
  broad room-construction conversion remain deliberately outside this milestone.
