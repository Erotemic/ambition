---
status: current
last_verified: 2026-09-03
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
dead entries. For the room-feature raw-spawn gate:

```text
docs/architecture/architecture-boundary-allowlist.txt
```

Every scanned `spawn*.rs` file must appear exactly once and its recorded count
must equal the current raw `commands.spawn(` count.

⛔ **AND "SCANNED" IS NARROWER THAN THIS SECTION READS — MEASURED 2026-09-03.**
The gate (`tests/ambition_workspace_policy/src/custom/lifecycle.rs`) walks
`features/ecs` recursively but keeps only files whose **FILE NAME** starts with
`spawn`. Today that is exactly two: `spawn_actors.rs` and `spawn_static.rs`, both
allowlisted at 0.

⇒ **The directory named `features/ecs/spawn/` is therefore invisible to the gate
that exists to govern room-feature spawns.** Its seven files — `mod.rs`,
`portal_construction.rs`, `content_staging.rs`, `capability_lanes.rs`,
`gravity_construction.rs`, `character_spawn_plan.rs`, `tests.rs` — are named for
what they construct, so none begins with `spawn`. ⚠ **No production violation
today**: all six production files are at 0 raw spawns, and the single
`commands.spawn(` under that directory is in `tests.rs`. The gap is that a new
raw spawn in any of them would fail nothing.

⭐ **AND IT CLOSED BY REFACTOR, NOT BY DECISION.** `cdd0a0a0d` (2026-06-14)
split `features/ecs/spawn.rs` into `spawn/mod.rs` + `spawn/tests.rs`. Before that
commit the file was named `spawn.rs` and WAS scanned; after it, neither half
matched, and the allowlist has not needed to mention them since. The commit's own
subject is *"split 6 more test-heavy modules + fix source-scanner paths"* — so
scanner paths were on its author's mind, and this one still went. ⇒ Nothing
failed, because a name-matching gate cannot report the file it stopped matching.

✔ **FIXED THE SAME DAY.** The filter now tests the path RELATIVE TO the scan
root, so `spawn_actors.rs` and `spawn/portal_construction.rs` answer one rule and
splitting a file into a directory cannot undo it again. The allowlist grew from
two rows to nine — **coverage, not permission**: every production file added is
at 0 and was already at 0, and the single allowed raw spawn is `spawn/tests.rs=1`,
test scaffolding that builds a bare entity to drive this gate's own subject.
The vacuity assertion is now a FLOOR (`scanned >= 9`) rather than `> 0`, because
`> 0` was true throughout the three blind months and proved nothing.

⭐ **The blindness was demonstrated, not argued.** With a raw `commands.spawn(`
added to `spawn/portal_construction.rs`, the pre-fix gate — old filter, old
allowlist — reports `test result: ok`. The same tree with the path filter reports
*"1 raw commands.spawn calls; exact reviewed count is 0"*. ⇒ That pair is the
evidence this gate was worth widening; a green run over a planted violation is
the only proof a scanner's population was wrong. A removed file, missing row,
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
