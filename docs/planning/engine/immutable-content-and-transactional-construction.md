# Immutable content and transactional construction

The owner contracts are [construction and reconstitution](construction-and-reconstitution.md),
[asset preparation](asset-preparation-and-residency.md) and
[room transition loading](room-transition-loading.md). This page fixes the meaning
of their common vocabulary.

## Prepared content

[Authored technique admission](authored-technique-admission.md) owns the checked
move representation and installed profile rules. The 2026-09-11 source inspection
finds installed checks and staged cast retention already implemented. The
[extension model](extension-model.md) extends that road to portable artifacts and
coordinated code/data/schema generations; it does not create a second validator.

An active simulation uses a validated immutable content revision. Source decoding,
semantic validation, handler availability, mechanical geometry, referenced content
and construction preflight precede activation. Device residency can complete on
a separate road, subject to the declared reveal policy; a visual tier must not
change authoritative collision geometry.

A canonical metadata digest identifies the metadata included in it. It does not
prove that two function bodies, builds or schemas behave identically. Same-build
rollback and cross-build content compatibility are separate promises. Do not make
an executable registry serializable by recording callback names.

## Construction guarantees

The current typed domain construction interface provides:

| Stage | Established responsibility | Guarantee it does not establish |
| --- | --- | --- |
| Plan and prepare | Resolve typed parameters and references; validate before committing | Arbitrary callbacks are pure, or the future command stream cannot fail |
| Commit | Emit domain construction operations through the current Bevy interface | An undo log for arbitrary resource mutation, despawn or external effects |
| Verify | Check the resulting supported entity/relationship invariants | Roll back every command already applied |
| Publish | Expose only an admitted, verified room/baseline to normal consumers | Preserve the old room after destructive work unless separately staged |

Evidence: `crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs`,
especially the raw `Commands` context, `commit_entity` and post-commit verifier.
Use **prevalidated, verified publication** for this existing contract. Do not
promise ACID-style atomicity or generic rollback. Finding F6 and packet A10 in the
[frontier](actor-monolith-work-frontier.md) describe hardening and its limits.

A supported construction failure must be contained by the current session's
failure/admission policy. Stronger all-or-nothing replacement needs a concrete
customer, staged candidate ownership and a constrained set of reversible effects;
it does not follow from renaming a function `transaction`.

## Boundaries to preserve

World lowering translates authored world data into domain-owned construction
parameters. Actor/catalog-specific placement lowering belongs at that integration
boundary, not inside generic geometry. Keep typed construction lanes; no universal
recipe enum, string/TypeId callback registry or custom snapshot engine.

Entity construction, resource-only reset, checkpoint routing and device asset
hydration can share a lifecycle coordinator without becoming one operation.
Checkpoint restoration belongs with session; rest-point healing/capture remains
content. [The checkpoint protocol](checkpoint-restoration-protocol.md) requires a
pinned snapshot used by both preparation and commit; a raw reset broadcast is not
sufficient authority for domain mutation. A rollback room transition waits for admitted confirmation and creates a
new frame-zero baseline. Existing snapshots do not cross room boundaries.


## Required definition-level retention versus optional world isolation

Rejected authored edits must leave active prepared definitions, generation and
existing playback references unchanged. This is a required A11 contract and does
not wait for A10. Initially activate changed mechanical content at a supported
session/reconstruction boundary. An inactive **prepared value** can be validated
without building an inactive ECS world. Preserving the old world after arbitrary
destructive native construction failure is the separate, stronger A10 guarantee.

## Bundle publication is not world undo

I3 in [fast iteration](fast-iteration-implementation.md) validates the whole
candidate generation, seals its base epoch and installed profile, then activates
at the supported lifecycle boundary. A character, module and schema do not
publish independently. Rejection before activation preserves the old generation
and timeline. Once destructive world commit begins, retain the failure semantics
above unless A10 has actually delivered stronger isolation. Content publication,
rollback replay, and durable-save migration are three different contracts.
