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
