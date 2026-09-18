# Actor residual-kernel decomposition

**Baseline:** `300004d601af1e633cfaee969f079cf9bb368ca8`, 2026-09-08.
**State:** active, authority-first migration. The old P2/P3/P4 sequence is
superseded by the [reassessment](architecture-reassessment.md).
Execution order is in [the queue](../queue.md); concrete conditional packets are
in the [work frontier](actor-monolith-work-frontier.md).

## Goal

Retain a coherent body/control/action execution authority and remove unrelated
session restoration, preparation, world-object behavior and content integration.
Do not treat the residual crate as the only monolith: combat, characters, core
and runtime also contain mixed responsibilities.

The [responsibility map](architecture-responsibility-map.md) defines the target
logical owners. It does not mandate a crate per owner. The
[edge ledger](actor-monolith-hard-core-edge-ledger.md) records current evidence,
accepted dependencies and holds instead of postponing ownership analysis until
an expected SCC size is reached.

## Current graph and limits

The measured nontrivial SCCs are:

```text
largest cyclic component   10 modules
second                      2 modules
```

⛔⛤ **THE MEMBER LIST USED TO BE HERE AND IS NOW OWNED IN ONE PLACE.** This page
carried its own enumeration of the nine, which went stale when the membership
moved — `shrine` LEFT (A1b took its checkpoint half) and `avatar` +
`character_runtime` joined, so the count went 9 → 10 while containing one repair
and two new problems. ⇒ [The reassessment](architecture-reassessment.md) owns
the members and the diff, and makes the argument that *"the count cannot tell a
repair from a regression, and here it contained both."* Re-measured 2026-09-18
and unchanged since its 2026-09-17 reading. **This page owns the rules and the
cuts; ask that one which modules.**

Reproduce with:

```bash
python3 scripts/measure_kernel_module_graph.py --scc --cuts --edges 80
```

⭐⛤ **AND NO PAGE HAS EVER RECORDED WHICH EDGES THE `--cuts` HALF NAMES.** Every
page that names this instrument passes `--cuts` and then quotes only the
component sizes. ⚠ The OUTCOME was known — the
[edge ledger](actor-monolith-hard-core-edge-ledger.md) says in its own opening
that it records decisions *"at the current nine-module SCC, rather than assuming
that a future six-module SCC is the right unit of design"* — so the six has been
in the corpus all along with nothing saying where it comes from. Measured
2026-09-18, the single edges whose removal shrinks the largest component:

```text
-> 6    1 ref    abilities -> features
-> 7    3 refs   control -> abilities
-> 8    1 ref    avatar -> control
-> 9    1 ref    character_runtime -> avatar
-> 9    1 ref    features -> projectile
-> 9    4 refs   construction -> world
```

⇒ The first line is the striking one: **ONE production reference collapses the
largest component from ten modules to six.** It is
`abilities/thrown/puppy_slug_gun.rs:113` calling
`crate::features::spawn_runtime_minion`, and `abilities/mod.rs:17` already names
that seam in its own module doc.

⛔⛔ **AND THIS IS RECORDED AS A FACT, NOT AS A MANDATE — READ THE NEXT SENTENCE
BEFORE ACTING ON THE TABLE.** ⭐ The edge ledger already made this call, which
is the strongest evidence it is the right one: it chose to record decisions at
the ten-module cycle *"rather than assuming that a future six-module SCC is the
right unit of design."* The frontier page states the same stance outright:
*"A1–A12 name responsibilities, not SCC scores"*
([work frontier](actor-monolith-work-frontier.md)), and this page's own
paragraph below says the spawn extraction proved *"the same SCC result can
accompany both a wrong and a corrected ownership boundary."* ⇒ A one-reference
cut that moves a number by four is precisely the change this repository has
decided NOT to make for the number's sake: `puppy_slug_gun` spawning a minion IS
an ability using a spawn service, and inverting it to satisfy a graph metric
would buy a worse boundary and a better score.

⇒ **So why write it down at all?** Because the alternative is that the next
reader runs `--cuts`, sees `1 ref` beside a four-module win, and re-derives the
enthusiasm from scratch — and because if a RESPONSIBILITY argument ever lands on
that seam, this table says what it would also be worth. The number is the
consequence, never the reason.

This is a textual intra-crate module graph. It does not see Cargo-crossing query
ownership, shared writers, message timing, plugin prerequisites or all Rust
import forms. The spawn extraction demonstrated that the same SCC result can
accompany both a wrong and a corrected ownership boundary.

The settlement move and corrected spawn extraction stay landed. Do not reopen
live actor views, provocation, fighter-ladder projection or dismounted-rider
rebuild as spawn responsibilities. Keep
`scripts/tests/test_actor_spawn_boundary.py` as the existing focused boundary
check, without treating it as proof of all runtime actor semantics.

## Current design choices

- A1 moves checkpoint restoration to session; the pending lifecycle slot stays
  with its coordinator, not in shared_tangle.
- A2 resolves geometry/contact disagreement before removing projectile knowledge
  of target families. A marker plus unresolved broad damage query is insufficient.
- A3 moves actor-aware placement lowering to construction; immutable spatial world
  vocabulary stays at the world owner.
- A4 may retain an internal possession/control cycle when it enforces one accepted
  driver relation. `features` is not a coherent retained domain.
- A5/A6/A7 split destructibles, prepared character/materialization responsibilities
  and item/custody integration only after their writer and lifetime maps are known.

These decisions are justified in the reassessment and implemented through bounded
packets. Do not reproduce a second ordered queue here.

## Rules that every packet must obey

### One packet, one ownership claim

Separate a correctness change from its following mechanical move. Include only
callers, tests, registration, scheduling and facade changes needed for that
responsibility. Explicitly list non-goals and behavior that remains unchanged.

### The old internal path must disappear

Move consumers to the actual owner. Do not keep internal compatibility re-exports
solely to avoid editing imports. A curated public facade may re-export a supported
semantic API; that is different from making a temporary internal path permanent.

### State and lifetime travel together

For each moved state value, account for construction, writers, scope, retraction,
rollback/checksum, reconstruction and observation. A state definition without
those operations is not a complete authority transfer. Unknown writers or
lifetime are a hold on that part of the packet.

### Preserve behavior and wire identity in pure moves

Move snapshot implementations with their owning type where Rust coherence permits;
update domain declarations and backend adapters. Preserve existing wire IDs and
encoded meaning in a behavior-preserving move so that churn is attributable.
The repository's same-build policy does not promise cross-version compatibility;
a separately reviewed format change may change the format explicitly.

### Preserve scheduling semantics, not just function order

Record public phase membership, ancestor gates, ordering, deferred flushes and the
population that runs. A missing optional sibling may otherwise remove a phase or
skip initialization. Startup checkpoint restore is the immediate example: moving
it under a gameplay gate would prevent loading-time restoration.

### Tests witness consequences

Use a focused production-path behavior fixture plus an appropriate absence or
external-consumer fixture. Existing source guards are useful for known boundary
regressions, not a substitute for behavior. Do not require an unrelated poison
campaign for every move or use a zero-test filtered run as acceptance.

## Packet receipt

Record current base/new head, owner and moved operations, source paths removed,
new dependency direction, state/registration/lifetime changes, preserved phase
visibility, tests actually run and remaining limitations. Record SCC/reference
changes as diagnostics without a target score. Refresh citations and policy
arguments for moved source in the same commit.

## Satellite and coherent-package decisions

`assets` and `character_sprites` remain a separate coupled preparation question.
A grouped module/package can be right there; it is not on the immediate critical
path. `character_runtime` being out of the SCC does not certify its mixed
preparation, device residency and match-activation responsibilities.

An accepted residual cycle needs an explicit common invariant and a bounded state
owner. The fact that world and session call each other is insufficient to merge
spatial query algorithms and lifecycle coordination into one permanent runtime
package. Conversely, splitting every control mode into services can make one
control relation harder to enforce.

## Exit

D33 exits when the residual package owns only the documented body/control/action
kernel, other responsibilities have explicit owners and consumers no longer need
its historical internal topology. Any retained cycle has a concrete shared
invariant. Independent capability/profile acceptance is a separate C2/SDK exit;
D33 cannot declare it complete from source placement alone.

## Post-carve safety map

A carve that moves one of these owner files must update the corresponding absence
contract in the same commit. This table belongs here rather than in `queue.md`
because it is durable decomposition doctrine.

| If your carve moves… | Update these absence contracts |
|---|---|
| `crates/ambition_combat/src/moveset/mod.rs` | `ending-a-move-goes-through-the-one-teardown-path` |
| `crates/ambition_characters/src/brain/fighter`, `crates/ambition_characters/src/brain/state_machine/mod.rs`, `crates/ambition_characters/src/snapshot_impls.rs`, or `crates/ambition_characters/src/brain/mod.rs` | `the-generic-brain-does-not-grow-new-platform-fighter-edges` |
| `crates/ambition_platformer2d_actor_monolith/src/character_runtime/match_activation.rs` or `game/ambition_app/src/app/versus.rs` | `a-second-writer-of-a-match-global-must-answer-ownership` |
| `crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs` or `game/ambition_app/src/dev/rollback_observatory.rs` | `the-seat-topology-has-one-engine-side-creator` |
| `game/ambition_app/src/app/versus.rs` or `game/ambition_demo_smash/src/lib.rs` | `the-global-roster-is-retired-only-by-its-owner` |
| `tools/ambition_ldtk_tools/ambition_ldtk_tools/ldtk/paths.py` or `tools/ambition_ldtk_tools/tests/test_ldtk_core_helpers.py` | `the-worlds-path-is-confined-to-ldtk-paths` |
| `crates/ambition_characters/src/prepared.rs`, `crates/ambition_combat/src/worn_kit.rs`, or `crates/ambition_characters/src/actor/character_catalog/mod.rs` | `the-catalog-default-action-set-is-confined-to-one-file` |
| `crates/ambition_platformer2d_actor_monolith/src/character_runtime/presentation.rs` | `the-provider-resolver-is-confined-to-one-file` |
| `crates/ambition_characters/src/prepared.rs`, `crates/ambition_platformer2d_actor_monolith/src/avatar/starting_character.rs`, or `crates/ambition_characters/src/actor/character_catalog/mod.rs` | `the-catalog-axis-tuning-is-confined-to-one-file` |
| `crates/ambition_platformer2d_actor_monolith/src/avatar/starting_character.rs` or `crates/ambition_platformer2d_actor_monolith/src/avatar/mod.rs` | `the-movement-tuning-resolver-is-confined-to-one-file` |
| `crates/ambition_platformer2d_actor_monolith/src/avatar/starting_character.rs` | `the-motion-model-resolver-is-confined-to-one-file` |
| `crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs`, `crates/ambition_platformer2d_shared_tangle/src/lifecycle/session.rs`, or `crates/ambition_platformer2d_actor_monolith/src/world/rooms/stage.rs` | `only-the-candidate-builder-hides-a-root` |
| `crates/ambition_platformer2d_shared_tangle/src/construction/mod.rs`, `crates/ambition_platformer2d_actor_monolith/src/world/rooms/transaction.rs`, or `crates/ambition_platformer2d_provider/src/lifecycle.rs` | `only-the-publication-authority-publishes-a-candidate` |
| `crates/ambition_platformer2d_provider/src/authoring.rs` or `crates/ambition_game_shell/` | `one-registrar-installs-the-session-teardown` |
