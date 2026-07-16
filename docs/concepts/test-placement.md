---
id: test-placement
aliases:
  - test organization
  - where do tests go
  - workspace policy tests
  - ambition_workspace_policy
last_verified: 2026-07-11
---

# Test placement

The placement guideline: **a test lives at the narrowest scope that owns its
invariant.** AGENTS.md carries the one-paragraph version; this is the full model.

## Three homes

### 1. Local behavioral tests — the owning crate

- A small test that explains a **local implementation invariant** may stay inline
  (`#[cfg(test)] mod tests { … }` in the same file).
- A **large private test module should normally move** to an adjacent private
  child module (the default): `src/foo.rs` gains `#[cfg(test)] mod tests;` and the
  tests move to `src/foo/tests.rs`, keeping private access via `use super::*;`.
  Staying inline when large is the EXCEPTION (below), not the norm.
- **Never widen a production API just to move a test.** If a test needs private
  internals, it stays crate-local (inline or adjacent) — do not make items `pub`
  to relocate a test, and do not externalize private behavioral tests into an
  integration crate.

**The 200-line trigger is a review proxy, not a verdict.** The repository
inventory flags inline `#[cfg(test)]` modules ≥ 200 lines for review. The real rule is SEMANTIC and
the proxy is imperfect (a genuinely hard problem): line count alone never
establishes bad organization.

- **Genuine local behavioral tests** — ones exercising real, breakable logic
  (numeric folds, scoping, sequencing, serde round-trips, gameplay invariants) —
  are OWNED by the implementation. Ownership does not decide layout: they may
  remain inline even when large ONLY when a maintainer explicitly approves that
  co-location materially improves reviewability; otherwise the default above (move
  to an adjacent private child module) applies. Ownership is satisfied equally by
  `equipment/tests.rs` / `flag/tests.rs` as by an inline module.
- **Structural / guardrail tests** — shape checks, signature checks, ratchets,
  module-size or architecture policy, anything whose main job is to constrain
  machine-generated changes — belong in `tests/ambition_workspace_policy` or a
  dedicated integration location, NOT interleaved with implementation a human is
  trying to read.
- **A `kind` finding is not a layout decision.** An agent may inspect a flagged
  module and record its `kind` — `behavioral-local` (real local behavioral tests)
  or `guardrail` (shape/signature/ratchet/policy). `behavioral-local` settles
  semantic OWNERSHIP: the tests belong with the implementation. It does NOT settle
  physical LAYOUT — hundreds of behavioral test lines may still read better in an
  adjacent private child module (`foo/tests.rs` via `#[cfg(test)] mod tests;`,
  keeping private access with `use super::*;`) than inline in the same file.
- **Dispositions and who sets them.** An agent may classify a module's `kind` and
  RECOMMEND a disposition — `maintainer-review-pending` (or `extract-pending` if it
  chooses to move the module now). `maintainer-approved-inline` is a PERMANENT
  exception: the path must appear in the maintainer-owned
  `MAINTAINER_APPROVED_INLINE` allowlist, and
  repository policy reserves that entry to the maintainer. (An agent with write
  access could edit the allowlist; policy, not a mechanism, forbids it granting its
  own exception.)
- **Exceptions are maintainer authority, not agent self-service.** No agent may
  grant a permanent inline exception to its own or another agent's work; a new
  ≥ 200-line inline module needs a maintainer-owned allowlist entry. Operate in the
  spirit of the rule, not the zealous letter.

### 2. Public crate / assembled-system behavior — that crate's `tests/`

Public crate behavior stays in the crate's `tests/` directory. Game scenarios,
replay, collision oracles, desync canaries, dialogue/content validation, and
boss lifecycle/scenario behavior remain owned by the relevant crate — they test
runtime contracts or authored content, not repository structure. A later
system-contract campaign could create a contracts package; until then they stay
put.

### 3. Workspace policy — `tests/ambition_workspace_policy`

Source scans, dependency boundaries, module-size limits, architecture ratchets,
forbidden-name checks, and workspace-consistency rules live ONLY in the
sequestered workspace-policy package. It inspects the workspace as data (parsed
manifests + source walking) and links no production crate, so running it never
compiles `ambition_app`.

- Declarative, repetitive rules are DATA (`policies/*.toml`).
- Unusual semantic scanners stay as readable custom Rust (`src/custom/`), not a
  generic DSL.
- Scoped and independently filterable: `repository_policies`, `engine_policies`,
  `game_policies`.

See `docs/architecture/architecture-boundaries.md`; the completed test move is
recorded historically in `docs/planning/engine/test-organization-migration.md`.

## Rules that survive every move

- Test reusable policy tooling with representative positive and negative cases.
  Individual declarative rules do not automatically need their own poison test.
- Keep a harness-level non-vacuity check where an empty scan could otherwise pass;
  do not multiply equivalent checks per rule.

## Commands

```bash
# Local implementation work
cargo test -p <owning-crate> --lib

# Policy scopes (independently filterable, one shared compiled runner)
cargo test -p ambition_workspace_policy engine_policies
cargo test -p ambition_workspace_policy game_policies
cargo test -p ambition_workspace_policy repository_policies
cargo test -p ambition_workspace_policy            # all scopes + self-tests

# Handoff / merge gate
cargo test --workspace
```

Local iteration runs the owning crate plus the relevant policy scope; full
`cargo test --workspace` remains the handoff/merge gate.
