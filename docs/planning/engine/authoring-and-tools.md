# Agent-native authoring and tools

**State:** open; baseline `300004d601af1e633cfaee969f079cf9bb368ca8`.
**Doctrine:** [agent-native authoring](../../concepts/agent-native-authoring.md).
**Execution:** [the queue](../queue.md); technique admission/bounds packets A11/A12
are in [the frontier](actor-monolith-work-frontier.md).

## Goal

An author, including an LLM agent, can discover the engine's supported vocabulary,
inspect current content, change intent-level source, prepare/validate the result,
run a useful test, obtain a focused review artifact and explicitly publish/package
it. No visual editor operation or knowledge of monolith migration history is a
prerequisite. Optional visual tools consume the same authored semantics.

The authoring system is a control plane. Preparation and publication feed the
simulation; the author's model calls, filesystem operations and nondeterministic
planning are not part of a deterministic simulation tick.

## Current implementation: preserve these surfaces

The repository already has LDtk semantic inspection/editing/validation, world
queries and review bundles; procedural sprite authoring and published metadata;
MusicIR/SFXIR source and render diagnostics; provider-defined preparation and
public app builders. Use those surfaces rather than build another uniform CLI or
serialization format. See the root README and `docs/tools/index.md` for tool and
submodule ownership.

`TechniqueFlow` is also implemented and used by a shipped authored moveset:
`game/ambition_demo_smash/src/moveset.rs` assigns a flow to `read_and_seize`, and
`crates/ambition_combat/src/moveset/mod.rs` interprets it. The old assertions that
all flows are None and no interpreter exists are retired. Its contact signals
are per-move-occurrence latches, not arbitrary per-beat event subscriptions.

Two preparation gaps are source-established: the parameter-schema registry has
no production callers, and flow validation does not establish finite timeout or
u16 runtime-index bounds. F7/F8 in
[review findings](architecture-review-findings.md) define the exact limits; no
malformed shipped content or runtime hang is claimed without a reproduction.

## A1 - capability discovery

Domain owners expose list/describe/schema/support/diagnostic projections of their
actual installed or prepared vocabulary. Required categories include character
and action definitions, technique support, world entities/fields, sprite products,
music/SFX constructs and provider extensions.

A discovery record distinguishes known, installed, supported for this profile,
prepared and ready. It includes owner, schema/version, source/provenance and
relevant prerequisites. Do not advertise a schema as executable behavior. Do not
build a generic runtime service locator to implement a read-only catalog.

## A2 - semantic inspection before mutation

Inspection answers what exists, which source owns it, what it resolves to, which
capability executes it and which other definitions reference it. Runtime inspection
names the session/content revision/frame and separates authoritative values,
derived views and presentation projections. A value with no selected owner is
reported as unresolved, not a default from some other profile.

Reuse domain preparation/reference enumeration for both forward and reverse
queries. A parallel hand-maintained dependency graph would become another source
of truth. Stale generated indexes must identify their source revision, tool
version and generation status; regeneration failure is visible to the consumer.

## A3 - intent-level mutation and reviewable plans

Direct edits are appropriate for simple source formats. Fragile formats such as
LDtk use semantic operations for relationships, IDs and placements. A plan states
base revision/content identity, affected definitions, proposed operations, expected
references and validation/acceptance before applying changes.

Reject an apply against a changed base rather than overwriting concurrent edits.
Repeat application should be explicitly idempotent or return an already-applied
result. Multi-file edits need a staged/recoverable application protocol; do not
promise filesystem-wide atomicity because each individual file can be renamed
atomically. Publication of an accepted content revision is a separate operation
from editing source files.

The review artifact contains the semantic diff and validation result, not a
second manually curated copy of all source. Include source edits in the normal
version-control workflow; generated products carry provenance to those sources.

## A4 - preparation diagnostics and admission

Preparation rejects unknown/uninstalled references, conflicting definitions,
invalid parameter domains, impossible structural relations and known unsupported
nondefault fields. A diagnostic should identify:

```text
code and severity
provider / definition / field or node
source location or structured source path
selected profile and content revision
expected semantic kind / available support
actual value and repair guidance
```

This is an output contract, not a mandate to replace every domain's internal error
type. Compose existing diagnostics at the preparation boundary. Source spans are
used where available; compiled Rust-authored content may need owner/definition/
field provenance rather than an invented file offset.

F4's requires_facing, collected and persistent fields require a real support
contract. Q63 and the maintainer's Interact constraint still govern behavioral
changes. A validator can diagnose unsupported nondefault use without choosing the
gameplay policy or deleting authored data.

### Installed technique support is one declaration

A11 couples technique existence and parameter validation to the same capability
offer that installs its runtime handler. Register known paramless techniques
explicitly; absence of a validator must not mean both paramless and unknown.
A technique known to the source tree but not installed for this profile also
fails admission, with a different diagnostic from a misspelled key.

One domain-owned effect-reference traversal covers flow emits, timeline events,
sustained windows and applicable nested technique payloads. That traversal feeds
validation, discovery and dependency inspection. Handler systems remain ordinary
typed Bevy systems on their established schedule; the catalog must not become a
dynamic gameplay dispatcher.

Parameter checks include semantic limits/unknown fields where the schema requires
them, not only successful serde hydration. Freeze installed support before
publishing prepared content. Defensive runtime diagnostics remain for trusted
callers that bypass preparation.

Duplicate-key rejection does not require comparing function behavior. A key
already present can be rejected even when its validator is a function pointer.
Stable metadata supports provenance, not proof of executable equivalence. Use
explicit same-owner/revision or replacement rules only when their lifecycle and
invalidation semantics are defined. Last-write-wins is not forced by Rust's
function-pointer comparison limitations.

### Bound authored execution at preparation

A12 aligns flow graph indices with the runtime cursor, requires finite positive
waits, and specifies graph/work bounds. Existential reachability of Finish is not
a proof that every execution terminates; bound cycles and keep enclosing move
teardown explicit. Preserve proper-time semantics and existing per-occurrence
contact latches. Per-beat confirmations need scoped contact-event identity before
they can be advertised.

Do not add arithmetic, arbitrary queries, variables or a universal blackboard to
move-scoped flow as part of this work. Prepared programs should express existing
domain operations with explicit execution/cancellation limits. A new scripting
runtime needs a separate deployment/modding requirement.

## A5 - cross-domain content preflight

A meaningful content unit spans definitions and assets. Prepare a character's
body/actions/equipment, its sprite/portrait, writing, sounds and referenced
techniques together; prepare room geometry, paths, placements, encounters and
bindings together. Report all relevant missing inputs with the selected profile.

Validation does not require every asset to be resident on a GPU. Distinguish
semantic resolution, generated-product availability, device materialization and
activation readiness. Runtime simulation binds one accepted content revision;
late visual materialization cannot change its mechanical values.

Prepared metadata fingerprints are not executable-function fingerprints. An
acceptance receipt identifies the engine/build and provider implementation as
well as authored content. Same content metadata on different binaries does not
establish the same simulation behavior.

## A6 - concise review artifacts

Produce the smallest artifact needed for the decision: semantic room summary and
render; character sheet/hitbox strip; music/SFX preview with diagnostic report;
prepared-content/provenance diff; or a deterministic input/replay fixture. Human
judgment remains explicit for visual/audio quality. A passing automated check
does not certify subjective polish.

Artifacts identify source revision, provider, tool version, selected content,
settings and relevant runtime/frame context. Missing generated data is reported
as unavailable, not substituted under the requested artifact's name.

## A7 - one complete agent-authored acceptance slice

Use the existing moving-platform/LDtk slice to exercise world placement, path
references, preparation, dynamic geometry, rollback and review output. Add one
external-provider action/object slice to prove the same support/validation and
physical-input route outside named game content. These are customers of the
current engine, not invitations to build a second authoring framework.

For each slice, an agent must inspect, plan, edit, prepare, test, review and
publish through supported operations. Include a deliberately invalid reference
and unavailable capability that fail before play, then repair and rerun. Measure
successful completion and corrective loops; do not infer quality from command
count or the amount of generated prose.

## A8 - optional human visual frontends

A visual frontend is justified by a concrete editing need. It reads/writes the
same semantic source and invokes the same validation/publication pipeline. It
must not own a hidden scene/state authority that agents cannot inspect or test.
Do not make a GUI a gate for a headless content change.

## A9 - semantic dependency and reference graph

Support forward/reverse reference queries, structured unresolved references,
rename/delete planning and later rule-dependency inspection. Indexes derive from
prepared/domain references and carry a revision. A rename plan must report all
known affected sites and any opaque/unindexed domains; it must not claim complete
safety while ignoring a nested technique payload or external authored backend.

Across backends, apply with explicit expected revisions and a recoverable staged
protocol. Validate the complete new content revision before publication. An
incremental cache is an optimization of canonical preparation, not an alternate
acceptance authority; invalidation follows dependency identity and tool versions.

## Trust, publication and runtime limits

Validated authored data/programs have a bounded vocabulary and declared budgets.
Trusted Rust plugins/providers have application privileges; registration does
not sandbox them. Generated source receives ordinary review/build/tests, and
external tool execution is subject to the host's real trust boundary.

The construction executor's raw Commands cannot undo arbitrary mutation after a
failed verifier. Keep preparation rejection, fail-closed publication and future
isolated candidate activation distinct (F6/A10). Runtime model-backed characters
are remote intent participants with admission/deadline policy, not nondeterministic
callbacks inside simulation.

## Tool health and reproducibility

The ECS-inventory generator already has a focused executable health test:
`scripts/tests/test_ecs_inventory_can_actually_run.py`. Preserve its declared
parser-version compatibility and proof that output was actually produced.
Untracked `.agent` inventories are navigation aids, not authoritative current
counts. Check revision/freshness before using them in plans. The loading-zone
name ratchet remains a focused authored-presentation check, not a reason for
another repository-wide content scanner.

## Acceptance

One external and one flagship slice complete discovery, inspection, change,
preparation, behavioral verification, human-review artifact and explicit
publication/package without manual editor work or internal-module knowledge.
Invalid semantics fail at admission; diagnostics point to the real source;
repeating a request cannot accidentally duplicate content; stale plans refuse;
missing prerequisites remain visible. Current tool tests and Rust/GPU/platform
receipts are reported separately rather than combined into an unsupported pass.
