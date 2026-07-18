---
status: current
last_verified: 2026-07-18
related_docs:
  - docs/concepts/engine-mental-model.md
  - docs/concepts/content-and-provider-boundaries.md
  - docs/planning/engine/architecture.md
---

# Architecture boundary guardrails

Architecture policy turns durable dependency/ownership rules into fast source
and manifest checks. It is not a hand-maintained mirror of the current crate
tree.

## Policy home

The authoritative suite is the sequestered workspace member:

```text
tests/ambition_workspace_policy/
```

It treats the repository as data and links no production crate. Declarative
rules live under its `policies/`; semantic scanners live under `src/custom/`.
Each diagnostic carries a stable policy ID, owner, rationale, source document,
and offending location.

Use the generated repository map before changing a policy:

```bash
python scripts/agent_query.py "architecture policy <boundary>"
python scripts/agent_query.py crate ambition_workspace_policy
```

## Durable direction

The exact packages will evolve, but these arrows should remain one-way:

```text
foundations and stable data contracts
    -> shared platformer vocabulary and focused domains
    -> unified simulation composition
    -> observation/read models
    -> presentation

reusable engine/runtime/provider interfaces
    <- provider-owned named content
    <- thin host/app composition
```

Representative rules:

- Reusable engine crates do not depend on Ambition's named content or app.
- Foundations do not depend on orchestration, presentation, or host policy.
- Provider/game crates may register content through typed public seams; engine
  crates may not reach upward to discover it.
- Presentation reads stable observation/effect interfaces rather than mutating
  live simulation for convenience.
- Human, brain, RL, and replay controllers converge on one actor-local
  action/body path.
- Room/session entity creation uses lifecycle-scoped construction helpers.
- Tests do not widen production APIs or force app compilation into repository
  policy checks.
- Process-global registries do not become hidden App/session authority.

[`../concepts/engine-mental-model.md`](../concepts/engine-mental-model.md) is the
human explanation; policy IDs should point to a durable source doc rather than a
completed migration ledger.

## Exact allowlists

Allowlist files are exact reviewed inventories, not ceilings that can accumulate
dead entries. For the room-feature raw-spawn gate:

```text
docs/architecture/architecture-boundary-allowlist.txt
```

Every scanned `spawn*.rs` file must appear exactly once and its recorded count
must equal the current raw `commands.spawn(` count. A removed file, missing row,
or excess allowance is a failure. Reduce counts by moving creation through the
canonical scoped construction seam. Increase a count only when a raw spawn is
intentional, cannot use that seam, and the same patch explains why.

## Changing a boundary

1. Identify the durable ownership rule, not just the current cycle.
2. Check active planning and ADRs for intended direction.
3. Prefer a declarative manifest/source rule; use custom Rust only when semantic
   analysis is genuinely clearer.
4. Add a harmful fixture/poison case for reusable scanner behavior.
5. Update the source doc and policy data in the same patch.
6. Delete obsolete waivers/allowlist rows immediately.

## Run

```bash
./run_tests.sh -p ambition_workspace_policy
# During policy development, direct focused cargo filters are also useful:
cargo test -p ambition_workspace_policy engine_policies
cargo test -p ambition_workspace_policy repository_policies
cargo test -p ambition_workspace_policy game_policies
```
