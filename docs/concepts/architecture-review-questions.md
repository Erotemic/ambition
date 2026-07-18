---
id: architecture-review-questions
aliases:
  - architecture review checklist
  - fresh agent review questions
  - ownership and lifecycle questions
last_verified: 2026-07-17
related_docs:
  - docs/recipes/fresh-agent-navigation.md
  - docs/planning/decision-principles.md
  - docs/concepts/autonomous-decision-making.md
---

# Architecture review questions

Use these questions when entering an unfamiliar subsystem or reviewing a large
proposal. They are intentionally independent of current filenames and system
names, so they should remain useful while the implementation changes quickly.

The questions are not a process gate. They are a compact way to detect the
failure classes that repeatedly create expensive parallel architecture.

## Authority

- Which layer owns the authoritative state?
- Is the value authored, observed, derived, cached, or presented?
- Who is allowed to mutate it?
- Does another controller, actor category, platform, or lifecycle path already
  perform the same operation elsewhere?
- Would this change create a second authority or compatibility path?

If two mechanisms answer the same authoritative question, unify them rather
than merely making their behavior similar.

## Lifecycle

- When is the state created?
- When does it first become valid?
- Is an uninitialized value distinguishable from a measured negative value?
- When is it frozen, invalidated, rebuilt, and destroyed?
- Does reset, transition, hot reload, restore, and runtime spawning use the same
  lifecycle contract?
- Which schedule or deferred-command boundary makes the mutation observable?

A default value is not automatically a physical observation. Initialization
must not masquerade as a gameplay transition.

## Inputs and semantic transitions

- Is this system reading device input, semantic intent, body state, or
  presentation state?
- Is a controller-specific distinction actually data?
- Does the owning simulation layer publish a semantic transition, or are
  consumers independently inferring edges from raw booleans?
- Can human, brain, replay, and network sources reach the same body-level seam?

Physical facts and semantic transitions should be established once, then
consumed by gameplay and presentation.

## Transactionality

- Can validation and dependency discovery finish before live-world mutation?
- If execution fails, is the previously authoritative state still valid?
- Can work happen in an immutable plan, staging scope, or disposable world?
- Is commit authorization exact and one-shot?
- Does retry repeat preparation safely rather than continuing a half-applied
  mutation?

Expensive preparation may be incremental; authoritative replacement should
remain atomic from the game's perspective.

## Identity and reconstruction

- Is identity explicit data or inferred from a string convention?
- Can the entity or resource be reconstructed from the same recipe that created
  it normally?
- Are authored, provider-staged, and runtime-dynamic origins distinguishable?
- Does restore use exact authoritative state or deliberately rederive it?
- Is compatibility checked against the relevant content and schema identity?

Do not promote a successful family-specific reconstruction trick into a public
contract until the provenance model is explicit.

## Scheduling and scale

- Does correctness depend on incidental system, query, or hash-map order?
- What work scales per entity, per peer pair, per room, or per frame?
- Are inactive or stand-still entities still paying full simulation costs?
- Does a timing span include deferred-command application and the first frame in
  which consequences become visible?
- Is the proposed optimization attacking measured work or merely hiding it?

## Public seams

- Is this API exposing a proven ownership boundary or an internal transitional
  mechanism?
- Can a different game add content or behavior without editing the core?
- Does the facade reveal the supported abstraction while keeping implementation
  details replaceable?
- Are duplicate registration, override, ordering, and failure semantics explicit?

Use an external consumer as evidence before declaring an unstable internal seam
part of the SDK.

## Navigation discipline

Before inventing a new abstraction:

1. use `scripts/agent_query.py` to locate likely owners and parallel paths;
2. inspect the relevant generated ECS crate shard;
3. read the crate's `MODULES.md`;
4. inspect direct callers and lifecycle siblings;
5. search engineering memory for the invariant or symptom.

The goal is not exhaustive reading. It is finding the true authority quickly
enough to avoid extending the wrong path.
