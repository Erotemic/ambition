# Engine extension model

**State:** bounded extension model; runtime scripting/ABI stability remain
requirement-triggered. Baseline `300004d601af1e633cfaee969f079cf9bb368ca8`.

## Supported direction

```text
authored source / semantic operations
  -> domain preparation and diagnostics
  -> immutable prepared definitions or bounded domain programs
  -> typed domain requests and observations

trusted Rust Bevy provider / capability
  -> declares support, validation, installation and prerequisites
  -> extends the engine's vocabulary

host / game composition
  -> selects the installed offers and platform services
```

Authored content composes available vocabulary; Rust extends that vocabulary.
The preparation boundary validates the **installed** vocabulary for the selected
profile, not every technique whose name exists somewhere in the repository.

This model supports LLM authoring without requiring a new language or visual
editor. It does not make arbitrary Rust plugins safe to load from untrusted
sources. Sandboxing, dynamic ABI and user modding are separate requirements.

## Current evidence

Provider-owned catalogs, typed content preparation, semantic actions and the
public app builder have real customers. `TechniqueFlow` has both a live authored
customer in `game/ambition_demo_smash/src/moveset.rs` and an interpreter in
`crates/ambition_combat/src/moveset/mod.rs`. It is a move-scoped sequencer with
Emit/Wait/Branch/Finish and existing move-contact signals, not a general engine VM.

Preparation remains incomplete: the parameter-schema registry has no production
callers, unknown keys pass its standalone method, and flow validation does not
fully bound the runtime representation. A11/A12 in
[the frontier](actor-monolith-work-frontier.md) address those concrete gaps.
Do not reopen the already implemented interpreter as future work.

## Extension declaration and admission

A capability offer declares identity/owner/schema revision, actual handler
installation, parameter and reference validation, prerequisites, scope/lifecycle,
public phases and supported profiles. Keep validation metadata coupled to that
offer rather than a second catalog claiming handlers that were never installed.

A metadata catalog is useful for preparation and discovery; it is not permission
to invoke arbitrary behavior through a universal registry. Runtime translation
can remain ordinary typed systems/messages. Closed construction domains keep
typed dispatch and metadata-only registration. True independent provider seams
remain explicit and App-local, with deterministic ordering and conflict handling.

Reject duplicate keys by default. Function-valued entries can still reject a key
that already exists; no function comparison is required. Idempotence or deliberate
replacement needs its own owner/revision/invalidation rule. Metadata equality
must not be advertised as proof of executable equivalence.

## Time, scope and trust contracts

Declared programs use domain time and bounded execution/cancellation semantics.
A flow does not own the projectile, capture relation or health state it requests
operations on. A second Wait on an occurrence-latched signal is not a new event.
Adding per-beat signals requires explicit move/beat occurrence identity.

Runtime model calls remain outside deterministic stepping, arriving as admitted
remote-participant intents. Generated Rust is trusted source subject to ordinary
review/build/tests. An App-local registry or typed command does not impose a
security boundary on code with World/Commands access.

## Deferred choices

Runtime Lua/Wasm, downloadable behavior, hot-loaded native plugins and ABI
stability require a concrete modding/deployment customer. Decide save/schema
migration separately from Rust API and same-build rollback compatibility. A future
non-platformer or 3D customer can justify another simulation implementation;
it does not justify abstracting every current body operation over a backend now.

## Acceptance

An external provider declares and installs one action/object, discovers its
support metadata, prepares references and drives real behavior through the public
profile. Unknown, disabled and invalid-parameter uses fail before activation.
The same profile can omit unrelated capability offers. No engine key switch or
private module import is needed for that provider's legitimate extension.

Owners: [authoring](authoring-and-tools.md),
[authored orchestration](authored-gameplay-logic-and-orchestration.md),
[SDK](public-sdk-1.0.md), [composition](capability-and-runtime-composition.md).
