# Stable identifier centralization - semantic scope before shared syntax

The shared `string_id!` mechanism already lives in `ambition_load`; do not replay
the completed consolidation or infer that its panic/serialization policy should
apply to every identifier. Explicit, locally readable newtypes remain valid.
The inventory below is for semantic classification, not a mandate for a derive
macro or a universal ID crate.

⚠ **MEASURED 2026-09-16, because this page's motivation had gone stale in one
direction and its central question had been answered in another.** Over the 1601
tracked files under `crates/*/src` and `game/*/src`:

| | |
| --- | --- |
| `macro_rules!` in the whole workspace | 16 |
| …that generate an identifier type | **1** — `string_id!`, in `crates/ambition_load/src/id.rs` |
| its invocations | 11, across 3 crates (`ambition_load` 3, `ambition_game_shell` 5, `ambition_load_presentation` 3) |
| distinct id-suffixed `struct` declarations | 67 |
| identifiers minted by `format!` | 26 sites |

⇒ **THE SYNTAX CONSOLIDATION IS DONE AND THIS PAGE'S "several local `string_id!`
macros" WAS ITS PRE-STATE.** There is exactly one definition; the other two
crates `use ambition_load::string_id`. Its own doc records the three
byte-identical copies it replaced. So Options B and C below are not answering a
live duplication — they are answering whether the remaining 67 explicit newtypes
want the same treatment.

The [architecture reassessment](../engine/architecture-reassessment.md) distinguishes
authored content identity, live entity identity, occurrence identity,
construction-attempt identity, session/instance scope and rollback wire identity.
Two identifiers with the same representation need not have the same equality,
validation, persistence or retirement rule.

For new untrusted authored data, validation should produce source-local errors
rather than depend on a constructor panic. For an internal trusted invariant, a
panic may be intentional. Record which boundary the constructor serves; do not
mass-change behavior because a macro is available.

A8 requires two copies of the same room as the namespace witness before adding
instance qualification. A1 preserves existing rollback wire IDs during a pure
move. Canonical content metadata cannot identify executable function behavior;
see [registry protocol](ambition-registry-core.md). The dependency graph can locate
a shared syntax helper, but cannot establish the semantic owner of all IDs.

## The authority axis now has a mechanical owner, and it is not the declaration

⭐⭐ **THE FIRST AXIS IN THE LIST BELOW — "authority" — STOPPED BEING A TASTE
QUESTION**, because `ID-PEER` gave it a consumer that fails a build. The binary
that matters is not the six-way taxonomy: it is whether a value may enter what
two peers compare.

    local lifecycle / correlation identity   ≠   peer-stable canonical identity

`RollbackEntryKind::feeds_peer_checksum`, `in_peer_schema_identity` and
`HOST_LOCAL_IDENTITIES` (in `id_peer_audit.rs`) own that question today, and
`rollback-schema-baseline.json` member-diffs the peer-visible set.

⛔⛤ **AND THE MEASUREMENT SAYS THE DECLARATION CANNOT ANSWER IT.** All 11
`string_id!` types share one representation, one constructor policy, one panic
rule and one `Display`. Their authority is decided at the MINT SITE, and two of
them sit one line apart in the same file with opposite answers:

    LoadId       ← format!("shell.{route}.{next_load_transaction}")   host-local
    LoadWorkId   ← format!("{ROOM_ASSET_WORK_PREFIX}:{target_label}") content-derived

`LoadId` is in `HOST_LOCAL_IDENTITIES`; `LoadWorkId` is not, and both are
correct. ⇒ **A policy inventory keyed on the TYPE is looking in the wrong place
for this axis.** The 26 `format!` mint sites classify cleanly and the type name
does not predict the class:

| class | sites | shape |
| --- | --- | --- |
| content-derived | 9 | `AssetId` × 7 (`sprite.character.{name}`), `BrainPresetId`, `LoadWorkId` |
| authored-record-derived | 3 | `FeatureId` × 3 — `coin:{id}` from the defeated enemy's `config.id` |
| host-local lifecycle | 7 | `LoadId` × 2, `ShellRequestId`, `LoadPresentationOwnerId` × 2, `ShellHoldId` × 2 |
| test fixtures | 7 | not production |

⚠ **AND TWO OF THE FOUR HOST-LOCAL TYPES ARE NOT NAMED BY THE HAND LIST**:
`LoadPresentationOwnerId` (`room-transition:{sequence}` and
`shell:{route}:{load_id}`) and `ShellHoldId` (`session-publication:{activation}`
and `content-publication:{request}`). Both are DERIVED CARRIERS — the exact
failure mode `HOST_LOCAL_IDENTITIES`'s own comment says bit it twice, *"a
registered type never matches the name of the id it holds"*.

⇒ **IT IS NOT A DEFECT TODAY AND SAYING SO IS THE POINT.** Neither type is
rollback-registered, so the guard's verdict is unchanged and adding them would be
a true-but-vacuous widening. What the measurement establishes is that the
list is maintained along the wrong axis: the mint site decides, so the hand list
is a second authority over a question the mint sites already answer. A derived
census over mint sites is the shape that would not need updating by whoever adds
a carrier.

## Why this is in triage

The workspace contains many identifier-like newtypes, all now sharing one
`string_id!` where they share a policy. They repeat familiar operations:

- construction from strings or integers;
- `as_str` or raw-value access;
- `Display`;
- conversion from owned and borrowed values;
- transparent serialization;
- empty-value or format validation;
- ordering and hashing.

Some consolidation could prevent semantic drift. However, these types are easy
to understand when written explicitly, while a macro or derive can hide policy
from both human maintainers and coding agents. Saving twenty lines is not useful
if every future edit requires finding an expansion rule in another crate.

The problem to solve is **consistency**, not boilerplate at any cost.

## Questions that must be answered first

Inventory active identifier types and classify them along independent axes:

- **authority:** authored content ID, runtime identity, presentation key, local
  slot/index, protocol/session ID, or opaque handle;
- **representation:** `String`, `&'static str`, integer, composite value;
- **validation:** infallible wrapper, nonempty string, namespaced path, restricted
  alphabet, or domain parser;
- **stability:** serialized across saves/content, stable only within a process,
  or ephemeral test/presentation value;
- **construction:** public `new`, fallible parser, crate-private constructor, or
  generated value;
- **serialization:** transparent serde, custom codec, or deliberately absent;
- **interchange:** `From<String>`, `From<&str>`, `Borrow<str>`, `AsRef<str>`, or
  intentionally none;
- **error policy:** panic on programmer error, return a structured validation
  error, or accept all values.

Types should share machinery only when these policies agree. Similar spelling is
not sufficient.

## Candidate outcomes

The design review should compare at least these options.

### Option A — conventions only

Keep explicit newtypes in their owning modules. Add a short normative document
and a few reusable tests or review rules:

- stable authored IDs validate at their boundary;
- runtime IDs do not pretend to be authored IDs;
- serialized IDs state their compatibility contract;
- `Display` is not silently treated as a parser format unless documented;
- no domain parses identity from delimiters unless that grammar is the actual
  type contract.

This has the best local readability and no abstraction cost.

### Option B — a tiny declarative macro

Provide a deliberately obvious macro for the most uniform string wrappers. It
should expand to ordinary derives and small methods, with validation supplied
explicitly rather than hidden.

The invocation must communicate the policy at the use site. For example, a
reader should be able to tell whether empty strings are accepted and whether
serde is part of the contract without opening the macro implementation.

This could live in an existing low-level crate or a narrowly named crate. The
name `ambition_id` is a candidate, not a decision.

### Option C — a procedural derive

Use a derive only if the inventory shows enough genuinely uniform types that the
compile-time and discoverability cost is justified. A derive that secretly adds
constructors, validation, conversions, or serialization policy is disfavored.

The default bias is against this option until a pilot proves it remains obvious
to coding agents and humans.

### Option D — explicit types plus shared validation primitives

Centralize only stable validation and error vocabulary while keeping each
newtype implementation explicit. This may provide the consistency benefit with
less hidden machinery than generated implementations.

## LLM and maintainer legibility requirements

Any abstraction must pass a source-reading test:

- the policy of an ID is visible at its declaration;
- `rg` can find where construction and validation behavior comes from;
- compiler errors point to understandable code;
- generated methods do not surprise a reader;
- an agent can add a new ID correctly without copying an unrelated domain's
  policy;
- ordinary Rust remains available for exceptional identifiers;
- no abstraction encourages converting every string wrapper into the same
  semantic type.

A few repeated explicit implementations are preferable to a magical abstraction
that makes domain contracts harder to inspect.

## Proposed next step

⭐ **THE BOUNDED INVENTORY THIS ASKED FOR EXISTS FOR ONE AXIS AND IS ABOVE.** What
remains is the other seven — representation, validation, stability, construction,
serialization, interchange, error policy — over the 67 explicit newtypes, and
note that the axis already done is the one where a wrong answer causes a DESYNC
rather than mild inconsistency. That ordering was not planned; it is what having
a consumer does to a question.

Do not create a crate yet. Produce the remaining inventory and group only exact
policy matches. Then implement one pilot using either:

- conventions plus explicit code; or
- a small declarative macro whose invocation exposes all policy choices.

Compare:

- lines removed;
- clarity at the declaration site;
- quality of rustdoc and compiler diagnostics;
- ease of exceptional behavior;
- incremental compile cost;
- whether an unfamiliar coding agent can correctly explain and extend the type.

The pilot should be reverted if the abstraction primarily hides straightforward
code rather than centralizing a real invariant.

## Non-goals

This work must not:

- introduce a general utilities crate;
- unify semantically different IDs merely because they wrap strings;
- replace domain parsers with a universal delimiter convention;
- add a procedural macro before the policy inventory exists;
- move all identifier types into one crate;
- make serialized compatibility depend on generated behavior that is not
  documented at the declaration site.

## Promotion criterion

Promote a concrete implementation only after the inventory identifies a group
of exact policy matches and a pilot demonstrates better consistency without
making the code harder to understand. Until then, this document records a
question and evaluation method, not a chosen abstraction.
