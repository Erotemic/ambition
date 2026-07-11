---
id: test-placement
aliases:
  - test organization
  - where do tests go
  - workspace policy tests
  - ambition_workspace_policy
last_verified: 2026-07-10
---

# Test placement

The binding rule: **a test lives at the narrowest scope that owns its
invariant.** AGENTS.md carries the one-paragraph version; this is the full model.

## Three homes

### 1. Local behavioral tests — the owning crate

- A small test that explains a **local implementation invariant** may stay inline
  (`#[cfg(test)] mod tests { … }` in the same file).
- A **large private test module** moves to an adjacent child module: `src/foo.rs`
  gains `#[cfg(test)] mod tests;` and the tests move to `src/foo/tests.rs`, which
  keeps private access via `use super::*;`.
- **Never widen a production API just to move a test.** If a test needs private
  internals, it stays crate-local (inline or adjacent) — do not make items `pub`
  to relocate a test, and do not externalize private behavioral tests into an
  integration crate.

**The 200-line trigger is a review proxy, not a verdict.** `scripts/check_agent_kb.py`
flags any inline `#[cfg(test)]` module ≥ 200 lines. The real rule is SEMANTIC and
the proxy is imperfect (a genuinely hard problem): line count alone never
establishes bad organization.

- **Genuine local behavioral tests** — ones exercising real, breakable logic
  (numeric folds, scoping, sequencing, serde round-trips, gameplay invariants) —
  may stay inline even when large, because co-location improves reviewability.
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
- **Dispositions and who sets them.** The agent-writable disposition is
  `maintainer-review-pending` (or `extract-pending` if the agent chooses to move
  it now). `maintainer-approved-inline` is a PERMANENT exception and is valid only
  when the path is in the maintainer-owned `MAINTAINER_APPROVED_INLINE` allowlist
  in `scripts/check_agent_kb.py`; an agent cannot self-grant it by writing a marker.
- **Weak or untrusted agents get the zealous default:** a new ≥ 200-line inline
  module fails until a reviewer allowlists it; an agent does not grant itself the
  exception. Operate in the spirit of the rule, not the zealous letter.

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

See `docs/architecture/architecture-boundaries.md` and the live migration matrix
in `docs/planning/engine/test-organization-migration.md`.

## Rules that survive every move

- **Poison tests stay with the policy/invariant they validate.** A grep lint that
  cannot fail is worse than no lint; never separate a poison test from its rule.
- **Non-vacuity checks stay with the harness whose execution they validate** — a
  scan that reads zero files must fail loudly, in the same place it scans.
- **A green empty scan is a failure**, everywhere.

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
