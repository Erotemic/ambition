# Prevalidated content artifacts and fast runtime admission

**Priority:** Deferred; outside the execution queue.
**Value test, after authorization:** A measured startup, reload, or content-loading cost justifies it, or a release/mod packaging customer needs a portable prepared artifact. A measurement alone does not authorize implementation.
**Scope:** Content validation, preparation, and artifact admission. Do not change simulation authority or the asset residency model.
**Source baseline:** Ambition snapshot `102cbf1`, 2026-10-16. Recheck paths and behavior against the implementation HEAD before work starts.

**Execution gate (maintainer ruling, 2026-10-16):** Do not implement, prototype, benchmark, add dependencies, add a crate, or schedule any work from this document unless one of these conditions is true:

1. The maintainer explicitly asks to implement **this specific proposal**.
2. All other work in the `docs/planning/` engineering queue is finished. There must be literally no other work left to select. A blocked, paused, or temporarily unassigned queue item does not count as finished.

Before case 2, check the current [`docs/planning/queue.md`](../queue.md) and its linked open work. Do not use a measurement, an adjacent task, or a promising experiment as permission to start. Research and planning edits alone do not authorize code, tool, content, or test changes. The work slices below are a record of possible future work, **not** an autonomous work order.

## Goal and decision

Compile authored content once, ahead of play when practical. Let a compatible runtime load the compiler's prepared result without repeating source parsing, semantic validation, reference resolution, and lowering. Keep one set of validators. Keep a small, explicit runtime admission step.

A validation *certificate* alone is not the target. A boolean or digest that says "valid" saves little if the runtime must parse and lower the same text again. Prefer a **portable prepared artifact** with provenance and evidence that the exact artifact passed the owning validators. The runtime checks the evidence and then hydrates that artifact into the existing prepared values.

This is an optimization and packaging extension of the [extension model](extension-model.md), [immutable content contract](immutable-content-and-transactional-construction.md), and [content generation and reload contract](content-generation-and-reload.md). It is not a new compiler, source format, content registry, or publication protocol.

## Starting point

- `ambition_content_pack::compile` is the common parse, check, resolve, canonicalize, and lower path. Capability owners register schema handlers. It produces `PreparedContentPack`.
- `PreparedContentPack` holds canonical content, required capabilities, schema versions, resolved references, source provenance, a semantic fingerprint, and schema-specific lowered values. The lowered map uses `Arc<dyn Any + Send + Sync>`. Do not serialize this map or use its Rust type identity as the portable format.
- `ambition_content_pack::artifact::ContentArtifact` already provides an envelope with section versions and admission checks. Its current sections contain opaque text payloads. It is **not yet** a serialized, admitted `PreparedContentPack` and does not by itself prove validation.
- `game/ambition_content/src/pack.rs` compiles the shipped content on first selection and shares the result within the process. `compile_pack` uses `AssetsUnchecked` so missing generated art can remain a visible fallback during development. A certified pack must not claim strict binary-asset validation from that mode.
- `game/ambition_app/src/app/resources.rs` runs `validate_content_graph` and then creates a room set. `validate_content_graph` first calls `LdtkProject::validate` and then `to_room_parts`; `to_room_parts` validates again as part of conversion. `to_room_set` validates and converts again, this time against the composed vocabulary. Some work repeats, but the vocabulary difference can change the result. Do not remove a pass until the two contracts have one explicit shared input and result.
- `docs/planning/engine/content-generation-and-reload.md` records roughly **15 ms** for a cross-content graph check on 2026-10-07. `docs/recipes/validating-a-content-pack.md` records roughly **17 ms** for an older 17-source pack. These are different workloads. Neither is a current full startup cost or proof of a useful cache speedup.

## The proposed path

```text
Authored sources + declared dependencies + schema/compiler identity
                             |
                  ONE owning compiler
         parse -> validate -> resolve -> lower
                             |
                portable prepared artifact
       canonical sections + provenance + dependency seal
                             |
            release package or local content cache
                             |
               cheap runtime admission
       integrity + compatibility + resource bounds
                             |
         schema-owned hydration -> prepared generation
                             |
          existing candidate readiness/publication
```

The compiler and runtime may live in different processes. The artifact must not require the same process, allocator, or Rust `TypeId`. The host may still refuse it if the selected providers, installed capabilities, schema versions, or active-session facts do not meet its requirements.

## Trust model: what the certificate proves

Call an artifact **prevalidated** only when the runtime has a trusted way to bind it to the exact validator result and all inputs that result used.

- **Release bundle:** The package builder validates the inputs and seals the generated bytes in the shipped package manifest. The trusted package/build boundary establishes that the artifact is the output of that validation. A cryptographic digest detects changed bytes; the host must have a trusted expected digest or authenticated manifest. A digest stored beside arbitrary untrusted bytes is not a trust anchor.
- **Local development cache:** The compiler writes a result under an exact cache key derived from input bytes, declared dependencies, schema/compiler identity, and relevant preparation policy. On reuse, check the key and recheck changed input/dependency bytes. Use atomic write/rename, refuse partial entries, and detect an interrupted or stale build. The local cache is a performance aid, not a security boundary against a hostile local writer.
- **External pack or mod:** A pack-provided "valid" marker is not enough. Validate normally unless a recognized trusted publisher or local compiler result authenticates both the artifact and its validation context. A signature scheme is optional future work; do not require it to support ordinary local authoring.

The current 64-bit semantic `ContentFingerprint` tracks content identity. It is not a collision-resistant security digest. For the artifact's byte integrity or a source-byte cache key, choose a collision-resistant digest such as SHA-256 or BLAKE3. State its purpose. Do not confuse semantic sameness with byte provenance.

**Certificate fields (candidate):** artifact format and section versions; source byte digests and source declarations; transitive external dependency digests; schema/validator revision or compatible semantic revision; compiler protocol version; target preparation profile; required capabilities and ports; canonical mechanical identity; asset-check policy (`unchecked`, `advisory`, or `strict`); prepared section digests; optional source-qualified diagnostics. Keep timestamps and absolute developer paths outside the mechanical identity.

A cached result is valid only for the checks it performed. For example, strict file-existence checks do not certify future GPU uploads or that an audio device can play a file. A content graph checked against one LDtk project and character catalog does not certify a different project or catalog.

## Reuse existing Rust packages, not a new serialization stack

These are **candidates to assess after the execution gate opens**, not approved new dependencies. Check the actual workspace and locks at that time. The 2026-10-16 source snapshot already has `serde` 1.x, `bincode` 1.3.3, `blake3` 1.8.7, and `sha2` 0.10.9 in `Cargo.lock`. Use the existing `ContentArtifact` and schema-owned validators first.

| Option | Useful role | Caution |
| --- | --- | --- |
| **Existing** [`serde`](https://docs.rs/serde/latest/serde/), [`bincode` 1.x](https://docs.rs/bincode/1.3.3/bincode/) | Encode typed, schema-owned prepared sections without inventing a new codec. | An explicitly versioned outer envelope and stable data schema are still required. Serialized Rust structs are not automatically a durable file format. |
| **Existing** [`blake3`](https://docs.rs/blake3/latest/blake3/) or [`sha2`](https://docs.rs/sha2/latest/sha2/) | Hash exact source bytes, dependencies, and artifact sections. | A hash stored by the untrusted pack itself is **not** proof that the owning validators ran. The expected digest must come from a trusted build or cache context. |
| [`postcard` 1.x](https://docs.rs/postcard/latest/postcard/) | Small Serde binary encoding with a documented stable wire format. | Compare it to existing `bincode` on real domain data; avoid adding a second format without a measured benefit. |
| [`rkyv` 0.8](https://docs.rs/rkyv/latest/rkyv/) | Archived data with possible zero-copy reads and built-in byte validation. | Higher complexity and separate archive layout/version rules. Use checked access and bounds for untrusted bytes, not `access_unchecked`. Never equate byte validity with content semantics or certificate trust. |
| [`zstd`](https://docs.rs/zstd/latest/zstd/) | Optional compression when release artifact size or I/O is material. | Decode and allocation also cost time. Set strict decoded-size limits; do not add compression by default. |

The default candidate is **existing domain codecs + Serde/bincode + an existing hash crate**. Compare `postcard` or `rkyv` only against a reproducible baseline with real prepared data and explicit compatibility checks. The goal is faster admission, not a new generic cache or package manager.

## Representation and ownership

1. **One validator implementation.** Keep schema validators with their capability owners. Both the source compiler and the prevalidated path must agree on accepted semantics. Do not create a second "quick validator" with looser rules.
2. **One portable IR.** Extend `ContentArtifact` or its domain-owned sections with codecs for canonical, validated values. Only serialize pure domain data. Keep Bevy entities, handles, closures, `Any`, installed executable callbacks, and device resources out of the artifact.
3. **One lowering authority.** Where a prepared value is portable, the schema can encode and hydrate it. Where it is not, encode the checked input to that stage and perform a bounded, deterministic bind/materialization step after admission. Do not maintain independent parsing of authored source in the runtime.
4. **Explicit required sections.** Extend admission to distinguish required from ignorable optional sections. Fail closed for a required unknown section. The current envelope's ability to carry an unknown section does not imply that a host can silently omit one that changes gameplay.
5. **Bounded inputs.** Bound artifact bytes, section count, section sizes, nesting, strings, and resource claims before allocating expensive objects. Deserialize with ordinary safe checks; never use unchecked zero-copy access to untrusted bytes.
6. **App-local hydration.** An artifact has stable provider/content IDs, not process-global installed handles. Resolve installed services and bind them within the selected App/experience.
7. **One publication road.** Hydration yields an existing prepared candidate. Session activation, retained foreign registry entries, save compatibility, ready-state gates, cancellation, and rollback generation pinning still use the current content transaction.
8. **No binary payload fiction.** A metadata certificate must not assert that ignored sprite, SFX, or music outputs exist or decode. Keep their strict packaging validation, source digests when relevant, and runtime residency/readiness checks on their existing roads.

## Work slices (not scheduled)

### V0 — Measure before changing data flow

**This is not authorized until the execution gate opens.** Add narrowly scoped timing spans and a repeatable report for:

- pack source I/O, manifest parsing, schema lookup, facet checks, reference resolution, aggregation, canonicalization, and lowering;
- LDtk JSON load, each validation call, room conversion, cross-content graph checks, and any repeated conversion;
- artifact decode, hydration, candidate readiness, and first visible use;
- fresh process startup, warm filesystem-cache startup, and changed versus unchanged content reload.

Report total wall time **and** per-stage time, target hardware, build profile, input sizes, source count, room count, selected provider, and HEAD. Take repeated release-profile measurements. Include at least one weak desktop/mobile-class target. Do not mix the older 15 ms graph figure with new measurements as though they came from one run. Attribute disk I/O, CPU work, and asset-device materialization separately. Store detailed evidence in the measurement area, not as a second live execution queue.

**Decision gate:** If admission/validation is already a small part of startup and reload, defer disk-cache and serialization work. Prefer eliminating a proved duplicate pass first.

### V1 — Share LDtk validation and conversion

Introduce an owned prepared-world result that carries validated room parts and the data the cross-content judge requires. Apply the exact selected vocabulary and manifest to the shared result. Make the cross-content checker consume that result, not independently convert the same project. Check that invalid authored links, fields, and missing zones still fail on the same path.

A local shared-result refactor is useful even if there is never an on-disk certificate. Measure it on the same fixture before considering it complete.

### V2 — Compile portable prepared sections

Pick one small, pure content family that already has a schema handler and lowered value. Add its portable section codec with an explicit section version. Round-trip through a separate process; compare the hydrated prepared value, canonical fingerprint, diagnostics, and downstream behavior with the normal compile result. Then extend to a cross-reference-heavy family, a room/world family, and finally the actual shipped pack.

Do not make `PreparedContentPack` itself a serializable bag of trait objects. Evolve the domain-owned section protocol and keep unknown required sections as refusals.

### V3 — Release fast path and trust check

Build the artifact during packaging through the existing validator and strict package guard. Include expected digests and complete compiler/section requirements in the release manifest. At startup, check integrity and compatibility, hydrate, and use the normal admission/publication path. No authored source reparse is needed when those checks pass. If a release artifact is corrupt or incompatible, fail with a clear refusal; do not silently activate an unchecked or stale pack.

Development can instead recompile from local authored source on a cache miss. On a reload failure, retain the active generation. Never publish part of a candidate because its cache entry was present.

### V4 — Optional incremental cache and tooling

Only if measurements justify it, cache prepared sections by exact dependency sets. Rebuild downstream sections when a referenced source, LDtk world, schema, external catalog, or compiler rule changes. Share dependency tracking with the content-generation protocol; do not build another invalidation graph. Expose a CLI report that says which sections were reused, why, and what was checked.

## Invalidation and admission examples

| Change or condition | Required response |
| --- | --- |
| Source whitespace changes only | A source-byte cache key changes; recompilation may prove the same semantic fingerprint. Never assume byte equality from semantic equality. |
| Authored value or reference changes | Recompile and revalidate the affected dependency closure before publication. |
| Schema/validator behavior changes without a format change | Invalidate results unless explicit semantic compatibility has been established. |
| LDtk world, character catalog, or music registry changes | Invalidate any cross-content judgment that read it. |
| A required capability or section version is absent | Refuse hydration/activation, even if byte hashes match. |
| Installed runtime services or save state change | Recheck runtime admission; a prior static certificate cannot authorize the new state. |
| Generated image/audio changes or is missing | Check the proper asset policy and materialization/readiness; do not reuse an unchecked asset claim as a strict claim. |
| A source watcher sees a partial write | Keep the active generation; accept only a complete validated candidate. |
| Two different Apps select different providers | Resolve and hydrate per App; no process-global "validated" singleton for both. |

## Acceptance evidence

- **Parity:** A full source compile and a verified-artifact hydrate produce the same typed prepared facts and the same gameplay-relevant content identity. The tested domains include real references, not just an empty fixture.
- **Refusals:** Corrupt bytes, altered dependency bytes, a changed validator, missing/unknown required sections, oversized payloads, missing capabilities, and a wrong artifact profile never reach publication.
- **Asset policy:** `AssetsUnchecked` cannot become "strict" by serialization. A missing binary remains diagnosable and cannot be hidden by a cached metadata verdict.
- **Lifecycle:** Rejected reloads leave the old generation active. A second App and a sequential provider switch cannot inherit a first App's hydrated handles or content.
- **Replay:** The exact admitted mechanical identity is pinned with the generation used for replay and restore. A certificate does not authorize cross-generation migration.
- **Measurement:** Record cold/warm startup, reload, decode/hydrate overhead, artifact size, and cache hit rates. Show a reproducible improvement on a real target before enabling a more complex cache by default.
- **Build and license:** Generated artifacts remain reproducible from authored inputs. Do not check in large generated payloads only to make the fast path work.

## Open questions at promotion

- Which target and cold-start budget actually motivate the change?
- Which schema families contain values that can be serialized without losing their meaning? Which require runtime rebinding?
- Does the existing RON envelope remain suitable for fast hydration, or should its versioned sections use a compact binary encoding? Measure both before choosing.
- How should publisher trust work for optional third-party packs? No signing service is needed for the first local/release implementation.
- When are cross-content dependencies captured: during pack compilation, world preparation, or the existing candidate gate? Preserve one owner for each rule.

**Priority rule:** V0 through V4, **including the V1 duplicate-LDtk cleanup**, stay deferred. A measured cost, a release need, or a seemingly safe cleanup does not authorize work. Only an explicit maintainer request for this proposal or literal exhaustion of all other `docs/planning` queue work can authorize implementation. After authorization, require evidence before selecting later stages.
