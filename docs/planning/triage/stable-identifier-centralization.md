# Stable identifier centralization — inventory first, abstraction pending

> **State:** TRIAGE — DESIGN DECISIONS PENDING, 2026-07-22.
>
> Ambition should have one place that keeps stable identifier conventions
> straight, but no decision has been made to introduce a derive macro, a shared
> newtype crate, or generated boilerplate. Explicit, locally readable Rust is a
> valid outcome.

## Why this is in triage

The workspace contains many identifier-like newtypes and several local
`string_id!` macros. They repeat familiar operations:

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

Do not create a crate yet. Produce a bounded inventory of identifier types and
group only exact policy matches. Then implement one pilot using either:

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
