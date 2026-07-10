# 0023: Same-build determinism is a contract, enforced by lints

## Status

Accepted; ENFORCED (2026-07-09, netcode N0.3). Decided by Jon (2026-07-06,
answering the Q4 decision brief in
[`docs/planning/engine/netcode.md`](../planning/engine/netcode.md)).

## Context

Determinism means "same inputs ⇒ same simulation states". How strong a promise
the engine makes decides how much discipline every future sim system carries.
The Q4 brief laid out three levels:

1. **Canary only.** Bit-identical replay tests exist but may be re-baselined
   freely. Zero ongoing discipline; rollback netcode, rewind mechanics, and
   reproducible RL training each become a separate retrofit later, against code
   that was never held to the rule.
2. **Same-build contract.** The SAME binary, on the SAME platform, fed the SAME
   per-tick input stream, produces identical sim states. `f32` math is fine —
   floats are deterministic on one binary; we just cannot reorder operations
   between runs, which same-binary guarantees.
3. **Cross-platform bit-exact.** Identical states across different OS / CPU /
   compiler builds. Requires software-float or fixed-point math, no `std` trig,
   audited transcendentals — a deep tax on every kernel.

## Decision

**Level 2 is the contract.** It is the knee of the curve: it buys replay and RL
reproducibility as product features, desync forensics, same-build online lockstep
AND rollback (both peers run the same binary — the normal case for an indie
game), rewind mechanics, and the fighter brain's forward rollouts. It costs a
permanent-but-light discipline tax that the lint set below makes mostly
automatic.

**Level 3 is explicitly NOT promised — but the architecture must not be coded
into a corner against it.** Every rule below is chosen so that cross-platform
determinism stays reachable without a rewrite, should it ever be wanted.

### The rules

Each is enforced by the `engine.determinism` / `game.determinism` policy
([`tests/ambition_workspace_policy/src/custom/determinism.rs`](../../tests/ambition_workspace_policy/src/custom/determinism.rs)
+ config [`policies/determinism.toml`](../../tests/ambition_workspace_policy/policies/determinism.toml);
migrated 2026-07-10 from the retired `crates/ambition_runtime/tests/determinism_lints.rs`),
which greps every non-test source in the SIM crates.

1. **No ambient randomness.** Sim randomness is a seeded, snapshot-registered
   resource — a per-owner or per-tick seeded stream. No global RNG, no thread
   RNG, no OS entropy. A seed is reproducible today and portable to level 3
   later. An unregistered RNG is a determinism bug the N0.4 desync canary will
   catch, after it has already cost a debugging session.
   *(Enforced on both the manifests and the sources. A `rand` **dev**-dependency
   is fine: a fuzzer that generates inputs proves determinism rather than
   breaking it.)*

2. **No wall-clock reads.** The sim advances on `WorldTime` (ADR 0010/0011) and
   is indexed by `SimTick`. `Instant::now()` in a sim system makes the trajectory
   depend on how fast the machine ran, which is the opposite of a replay.
   Under fixed tick, `Res<Time>` *inside* the tick is Bevy's fixed clock and is
   therefore deterministic; this rule is about `std::time`.

3. **No hash-order semantics.** `std::collections::HashMap` / `HashSet` use
   `RandomState`, seeded **per process**: iteration order differs between two
   runs of the same binary on the same inputs. Where the order is observable —
   spawn order, message order, who acts first — the sim is not replayable.
   Iterate a `BTreeMap`/`BTreeSet` (sorted, and portable to level 3), or a
   `bevy::platform::collections` map (`FixedHasher`, deterministic same-build),
   or keep the hash set as a *membership filter* and iterate the source sequence.

4. **`Entity` is never an ordering key.** Bevy entity ids are allocation details
   — index plus generation, reused from a free list. Sorting by one makes sim
   order depend on spawn/despawn history rather than on the world. Order by a
   STABLE id (`ActorConfig.id` / LDtk iid, a `PlayerSlot`, a spawn sequence
   number) — the identity vocabulary `SimSnapshot` (N3.1) and rollback both need.

Bevy `Query` iteration order is not stable either. An order-sensitive system
resolves ties by a stable id rather than by `Query::iter().next()` — see the
`AMBITION_REVIEW(determinism)` notes in `features::ecs::save_sync` and
`features::ecs::actors::update`, and the lowest-`PlayerSlot` fallback in
`player::queries` (F4.4).

### The escape hatch

Some hash iteration genuinely cannot be observed: a pass whose steps write only
their own key, or an index that is derived state, excluded from the snapshot and
read only by presentation. Mark those `AMBITION_REVIEW(determinism)` on the line
or at the head of the comment block above it, saying *why the order cannot be
observed*. The marker is grep-able, and
`reviewed_determinism_exceptions_are_listed` prints the whole set, so an auditor
reads every such claim at once.

## Consequences

- **What this buys.** Replay fixtures and RL trajectories are reproducible as a
  product feature. Desync forensics has a ground truth. N2 lockstep and N3
  rollback become engineering, not archaeology. `SimSnapshot`'s identity
  vocabulary is the same one rule 4 forces.
- **What it costs.** New sim code must pick `BTreeMap` over `HashMap` when it
  iterates, and thread a seed instead of reaching for `thread_rng`. Both are
  free at the point of writing and expensive to retrofit — which is the whole
  argument for deciding now.
- **What it does not cover.** The lints are greps, not a type system. They catch
  the shapes that have actually bitten (a `HashSet<Entity>` iterated into spawn
  order — found and fixed by this ADR's own slice, in
  `features::ecs::attack::start_body_melee`). They cannot see an iteration whose
  receiver is only typed several lines above, and they do not police iteration
  inside a `bevy::platform` map, which is legal at level 2 and would not be at
  level 3. The N0.4 desync canary — two sims, one input stream, a state hash per
  tick — is what catches the rest, and these lints exist to keep its failures
  rare enough to be worth investigating.
- **Formats stay open.** The input-stream (N0.2) and snapshot (N3.1) encodings
  are versioned, explicit in field order, and free of platform-width-dependent
  types, so level 3 remains reachable.

## Current implications for agents

Writing sim code (`ambition_engine_core`, `platformer_primitives`, `time`,
`entity_catalog`, `world`, `characters`, `combat`, `projectiles`, `portal`,
`encounter`, `items`, `cutscene`, `interaction`, `sim_view`, `actors`,
`runtime`):

- **Iterate a `BTreeMap`/`BTreeSet`**, or a `bevy::platform::collections` map, or
  don't iterate. `std::collections::HashMap`/`HashSet` seed per PROCESS, so two
  runs of ONE binary diverge. Keeping a `HashSet` as a membership *filter* and
  iterating the source sequence is fine.
- **Never order by `Entity`.** Ids are allocation details (index + generation,
  reused from a free list). Order by a stable authored/spawn id or a
  `PlayerSlot` — the identity vocabulary `SimSnapshot` (N3.1) needs anyway.
- **Never read the wall clock.** The sim advances on `WorldTime` / `SimTick`.
- **Never reach for a global RNG.** Sim randomness is a seeded,
  snapshot-registered resource.

`ambition_runtime/tests/determinism_lints.rs` enforces all four with an explicit
allowlist and a failure message naming the file, the line, and the fix. When an
occurrence genuinely cannot affect sim order, mark the line (or the comment block
above it) `AMBITION_REVIEW(determinism)` and say why;
`reviewed_determinism_exceptions_are_listed` prints the whole set for audit.

**The same file family now holds a second invariant.**
`ambition_runtime/tests/control_frame_lint.rs` (refactor-chain R5) pins who may
hold the global `ControlFrame` — one player's device frame, so a sim system that
reads it is silently slot-0-only. Both lints are greps with justified allowlists,
both are poison-tested, and both exist because a measured "this is already true"
is not an invariant until something can make it fail.
