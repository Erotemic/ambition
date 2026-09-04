# Actor-monolith decomposition - executable work frontier

> **Verified against `06b25ee8772a7c5bdf934dce5d49a692ebc2f37b` (2026-09-03).**

**State:** ACTIVE TASK BRIDGE. This page exists to make D33 resumable by an
agent that should not have to reconstruct the whole decomposition history before
choosing one bounded task.

This page is deliberately narrow:

- [`../queue.md`](../queue.md) decides whether D33 is the work to run now.
- [`actor-monolith-decomposition.md`](actor-monolith-decomposition.md) owns the
  measurements, reasoning, carve history, and architectural evidence.
- [`controlled-character-actor-kernel.md`](controlled-character-actor-kernel.md)
  owns the target semantics of the residual actor/body kernel.
- **This page only turns the current measured frontier into executable task
  packets.** It does not replace any of the authorities above.

Do not copy historical investigation into this file. When a packet lands,
replace its current-state description with the new frontier instead of growing
an execution diary. Git history is the diary.

## How to use this page

When D33 is selected by the live queue:

1. Re-measure HEAD before touching code:

   ```bash
   python3 scripts/measure_kernel_module_graph.py --edges 20
   ```

2. Compare the result with the receipt below. If the named seam or dependency
   direction changed, update this page from the code and the focused plan before
   implementing an old packet.
3. Take the first **READY** packet. Do not implement a **DESIGN NEEDED** or
   **RE-MEASURE AFTER ...** candidate by guessing its owner.
4. Make one coherent authority/dependency cut. Do not choose a different task
   because it removes more lines.
5. Run the normal D33 post-carve checks in
   [`actor-monolith-decomposition.md`](actor-monolith-decomposition.md) and the
   D33 row of [`../queue.md`](../queue.md).
6. Re-run the module graph after the carve. Update this frontier before starting
   another D33 slice. A carve can change which next step is correct.

## Current receipt

At the verified SHA, `scripts/measure_kernel_module_graph.py` reports 53,489
production lines under the actor crate's `src/`. The largest strongly connected
module component contains 14 modules and 48,238 production lines:

`abilities`, `assets`, `avatar`, `character_runtime`, `character_sprites`,
`construction`, `control`, `features`, `items`, `projectile`, `schedule`,
`session`, `shrine`, `world`.

The strongest mutual edge is still:

```text
features -> construction : 30 references
construction -> features : 15 references
```

The focused plan has already measured the direction. `features -> construction`
is consumption of the construction protocol. `construction -> features` is the
protocol reaching upward to name concrete actor recipes. The next packet removes
that reverse dependency.

The line counts above are a receipt, not a score. The objective is a smaller
semantic cycle and cleaner ownership.

## READY - F1: invert actor construction recipe ownership

### Goal

Make actor construction a lower-level protocol/mechanism consumed by the actor
simulation, rather than a lower-level module that imports the actor simulation's
concrete recipe implementation.

Desired dependency direction:

```text
actor-owned recipe registration
            |
            v
construction protocol / registry
            ^
            |
      actor simulation consumes it
```

The important condition is simple:

> Production construction code must stop naming `features`.

At the verified SHA, the reverse dependency is concentrated in one production
file: `crates/ambition_platformer2d_actor_monolith/src/construction/mod.rs`.
The focused plan measures 15 code references to `features` there, plus the
corresponding test shape in `construction/tests.rs`.

The concrete names currently crossing upward include:

- `spawn_staged_actor_into`
- `spawn_runtime_minion_into`
- `spawn_enemy_with_faction_into`
- `spawn_boss_with_overrides_into`
- `is_limbed_host`
- `giant_hand_plans`
- `SpawnActorKind`
- `SpawnActorRequest`
- `GiantHandPlan`

The focused plan reduces the actual inversion to five recipe registrations.
Use the existing construction-domain/registry patterns and the already-landed
capability construction examples as precedent. Do not replace the dependency
with string dispatch, `Any`, a service locator, or another central switch.

### Required result

The packet is complete only when all of these are true:

- production `construction` has zero dependency on `crate::features`;
- concrete actor recipe behavior is registered from the actor-owning side;
- `features` may continue to consume construction vocabulary and plans;
- recipe identity, deterministic dispatch/fingerprints, refusal behavior,
  construction receipts, and reconstitution behavior remain stable;
- tests follow the new ownership instead of preserving the old reverse import;
- no new upward dependency is introduced to hide the old one;
- the module graph is re-measured after the change.

Moving the `construction` module into a dedicated lower crate is the expected
consequence once the inversion makes that move clean. Do not invent a package
name or force the extraction in the same commit if the post-inversion graph
reveals another unresolved owner. The authority inversion is the first hard
acceptance condition; the graph decides whether physical extraction is then
mechanical.

### Acceptance

At minimum:

```bash
python3 scripts/measure_kernel_module_graph.py --edges 20
grep -rn "crate::features" \
  crates/ambition_platformer2d_actor_monolith/src/construction \
  --include='*.rs'
```

The production portion of the second command must have no hits. Test references
must either move with the new owner or be justified as black-box test usage,
not as a way to keep production dispatch coupled.

Then run the D33 post-carve checks already owned by the queue/focused plan,
including generated module maps, planning citations, doc links, absence
contracts/capability accounting where affected, and the relevant Rust gates.

### Stop condition

After F1 lands, **stop selecting work from the candidate list below until the
module graph is re-measured and this page is updated.** The purpose of F1 is to
change the graph that chooses F2.

## Candidates - not yet executable packets

These are recorded so the next agent knows which questions are real without
mistaking them for approved moves.

| Candidate | State | What must be resolved before implementation |
|---|---|---|
| control / possession / body custody | **DESIGN NEEDED** | Decide the authority topology and final home of `PossessionState`; do not move the leftover `abilities/{possession,teleport,trapdoor,flyline}` family by directory name. |
| character materialization / presentation | **DESIGN NEEDED** | Decide ownership of `CharacterLoadStates`, then separate load/materialization, presentation, and live match activation along their real dependency directions. |
| world integration | **RE-MEASURE AFTER F1** | Re-count `world <-> features`, `world <-> construction`, and `world <-> session` after construction inversion before choosing an extraction boundary. |
| session / Ambition-game orchestration | **DESIGN NEEDED** | Name the composition owner above reusable actor/body domains before moving session, shrine, music/audio, or related policy glue. |
| remaining items adapters | **RE-MEASURE AFTER F1** | The world-item and held-item authorities already left. Re-measure the residue instead of treating the old `items/` line count as one domain. |
| low-coupling islands | **DEFER** | Do not choose these only because they are easy to move. Break the central semantic cycle first unless the live queue gives another reason. |

## Rules that prevent false progress

- **No LOC target.** Crossing 100k was a useful milestone; it is no longer a
  task-selection rule.
- **No wrapper carve.** A new crate that imports the actor monolith or leaves the
  same mutual authority cycle is not decomposition.
- **Move authority with lifecycle.** State, registration, scheduling, rollback
  declarations, tests, and public construction/SDK seams move with the domain
  when they are part of that authority.
- **Do not mistake ledgers for semantic coupling.** Broad registration files
  such as rollback/snapshot ledgers are expected to name many domains.
- **One graph-changing carve, then re-measure.** Do not pre-commit to F2/F3/F4
  from today's graph.
- **If the code contradicts this page, the code wins.** Re-measure, update the
  receipt, then continue. Do not implement a stale packet because it is marked
  READY here.

## Updating this frontier after a carve

Keep the update small:

1. stamp a new verified SHA/date;
2. replace the current graph receipt;
3. mark the landed packet complete in the live queue/focused plan as their
   contracts require;
4. promote exactly one next packet to **READY** only when its owner, dependency
   direction, production sites, and acceptance are measured;
5. leave unresolved candidates blocked rather than filling in an architecture
   from intuition.

A weaker agent should be able to open this page, re-run one measurement, and
know either exactly which bounded D33 task is safe to execute or exactly why no
next carve has been specified yet.
