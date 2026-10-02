---
status: current
last_verified: 2026-10-02
related_docs:
  - docs/concepts/engine-mental-model.md
  - docs/concepts/content-and-provider-boundaries.md
  - docs/architecture/engine-architecture.md
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
dead entries. The room-feature raw-spawn gate
(`tests/ambition_workspace_policy/src/custom/lifecycle.rs`) reads:

```text
docs/architecture/architecture-boundary-allowlist.txt
```

The gate scans two roots: files under
`crates/ambition_platformer2d_actor_monolith/src/features/ecs` whose path
relative to that root starts with `spawn` (so `spawn_static.rs` and
`spawn/portal_construction.rs` match one rule), and every file under
`crates/ambition_platformer2d_actor_spawn/src/actor_spawn`. Every scanned file
must appear exactly once, and its recorded count must equal its current raw
`commands.spawn(` count.

- A removed file, a missing row, or an excess allowance is a failure.
- Reduce counts by moving creation through the canonical scoped construction
  seam.
- Increase a count only when a raw spawn is intentional, cannot use that seam,
  and the same patch explains why.
- The gate asserts a scanned-file FLOOR, not `> 0`. Only a deletion should lower
  it. A filter that stops matching is how a name-matching gate goes blind.

### Adding or removing a crate touches files that name crates

A dependency allowlist goes stale when a shared floor crate is extracted beneath
it. Widen the allowlist when the new crate satisfies the policy's rationale (for
example, a floor crate that depends only on `bevy` keeps a "host-free" crate
host-free). Do not keep a private copy of a floor concept to avoid the edge.

Find every file that names crates instead of remembering them:

```bash
git grep -l <an existing sibling crate name> -- '*.toml' '*.lock' '*.py'
```

Then run both gates after you add or remove a workspace crate:

```bash
cargo test -p ambition_workspace_policy --test policy   # read the per-policy lines
python3 scripts/check_absence_contracts.py | tail -5    # confirm a verdict per contract
```

A stale sentinel lockfile (`fixtures/minimal_game/Cargo.lock`) is reported as
`capability-footprint-sentinel-lockfile-is-stale`.

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
