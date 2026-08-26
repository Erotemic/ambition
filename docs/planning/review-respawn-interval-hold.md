# For review: keep, rebase, or drop `respawn-interval-holding`?

> ## ✔ ANSWERED 2026-08-25 — **verdict 3: re-do on current main**
>
> - **D194 is CLOSED** by `3a2100d86`. The evidence is stronger than "fixed one
>   grab bug": it measured the exact D194 matchup in the full app and went from
>   40 captures / 2028 capture-ticks (28%) / 0 pummels / 0 throws to
>   2 / 74 (1%) / 2 / 2, with the duel actually ending. The mechanism was
>   same-tick mutual capture making both bodies captor **and** captive, which put
>   the real capture policy out of reach. The old hold precondition is satisfied.
> - **D192 is still needed.** `respawn_delay_ticks` is absent from main and
>   `game/ambition_demo_smash/src/lib.rs` still places the body on the same tick.
>   `KnockoutsView`'s `LastSeenBodies` cache exists *because* of that.
> - **Do not merge or rebase the branch.** It was built on rollback schema 73;
>   main is 104. A conflict-resolved rebase means answering "how do I reproduce
>   what schema-73 code did" hunk by hunk. The right question is "what is the
>   smallest correct respawn-interval representation in schema 104".
> - **All three branches are pushed and frozen as reference.** Salvage behaviour,
>   tests, measurements and the KO-attribution lesson; do not port schema-73
>   registrations or old staging shapes. Delete them once the replacement lands.
> - **Re-evaluate, do not port, the knockout-velocity message change.** If KO
>   position/velocity belongs anywhere it is `BodyKnockedOut` — where the event
>   occurs — not `FighterStockSpent`, a later rules consequence.
> - **The acceptance guard** is the test the hold could never run: the repaired
>   D194 mirror *with* a 60-tick interval enabled. Structural, not pinned to 74
>   ticks — no body both captor and captive, pummels and throws occur, captures
>   stay in the repaired regime.

**The judgement asked for:** three branches hold D192's respawn-interval mechanism
and its follow-on work. They have not been merged for **two days and 271 commits**.
Decide whether they are still worth carrying, and if so whether the merge happens
now or the work is re-done on current main.

Nothing has been deleted. This asks for a verdict, not permission.

## What is actually on them

`respawn_delay_ticks` appears **nowhere in main's source** — the mechanism exists
only here.

| branch | ahead | pushed? | unique content |
|---|---|---|---|
| `respawn-interval-holding` | 3 | ✅ `origin/respawn-interval-holding` | D192 respawn beat, knockout VELOCITY on the message, schema 73, duel-guard instrumentation |
| `d194-and-respawn-verified` | 6 | ⛔ **local only** | superset, plus `a53699ed5 "Finish the knockout VELOCITY: the producer half never landed"` and a WIP guard |
| `attrib-beat-only` | 4 | ⛔ **local only** | superset, plus `819e48e14 "vel producer, for attribution only"` — reads as a probe |

Two of the three exist on exactly one disk. That is worth fixing before the
verdict, whatever the verdict is.

## Why it was held, and why that reason has decayed

`docs/planning/queue.md:405` (D194), verbatim:

> D192's mechanism is landed, guarded and proven, and is **HELD UNMERGED on branch
> `respawn-interval-holding`** … merging it turns this guard red, and setting the
> demo's knob to 0 instead turns the beat's OWN two guards red … ⇒ fix this row,
> then merge that branch unchanged.

So the hold was deliberate and had a stated precondition: **fix D194's grab lock
first.** Since then `3a2100d86 "D194: two grabs on one tick made two holds, and
neither could act"` landed on main — 124 lines in
`crates/ambition_combat/src/capture/systems.rs` plus 85 in tests. The other D194
commit, `ef23cd925`, is a one-line ledger entry.

⛔ **The D194 row still reads `unstaffed`.** Nobody updated it when that fix
landed, so the ledger cannot answer its own precondition.

**Question 1 — does `3a2100d86` satisfy the precondition?** It fixes one specific
sub-defect (two grabs on one tick). D194 as written is broader: a top-rung mirror
spending 28% of a match in a grab before any beat exists. If the row is closed,
the note says merge unchanged. If it is not, the hold still stands and should say
why in one line.

## The mechanical state, which is new

The blocker is no longer only semantic. `git merge-tree main
respawn-interval-holding` exits 1 with **12 conflicted files**:

```
crates/ambition_combat/src/snapshot_impls.rs
crates/ambition_combat/src/stocks.rs
crates/ambition_platformer2d/src/lib.rs
crates/ambition_platformer2d_actor_monolith/src/character_runtime/prepared_match.rs
crates/ambition_platformer2d_actor_monolith/src/character_runtime/staging.rs
crates/ambition_platformer2d_runtime/src/rollback/registry.rs
game/ambition_app/tests/rollback_schema_baseline.txt
game/ambition_app/tests/smash_cpus_damage_each_other.rs
game/ambition_demo_smash/src/lib.rs
scripts/baselines/rollback-schema-baseline.json
scripts/tests/rollback_codec_shape.txt
docs/planning/queue.md
```

The rollback registry, snapshot impls and **all three schema baselines** conflict.
`smash_cpus_damage_each_other.rs` conflicts because main's own D194 fix edited the
same test. This is the least forgiving place in the repo to carry a stale branch,
and the cost rises every day.

**Question 2 — rebase or re-do?** A 701-insertion / 25-file diff whose conflicts
are concentrated in rollback schema may be cheaper to re-implement against current
main than to reconcile. The reviewer is better placed to judge that than the
conflict count is.

## What is NOT established

- Whether D194 is closed on main. Not determined; it needs someone who knows
  whether the grab lock is one defect or several.
- Whether the branch's schema 73 is still coherent with main's current schema.
- Whether `a53699ed5`'s "producer half" is the same missing half D194's row names,
  or a different one.

## To reproduce

```bash
git log --oneline main..respawn-interval-holding
git merge-tree main respawn-interval-holding        # exit 1 = conflicts
git diff --stat main...d194-and-respawn-verified
grep -rn respawn_delay_ticks --include='*.rs' crates game   # nothing on main
```

## The verdict wanted

One of:

1. **Merge now** — D194's precondition is met; reconcile the 12 conflicts and land it.
2. **Keep held** — the precondition is not met; record in one line what still blocks
   it and who owns that, because "unstaffed" on a row that gates a held branch is
   how this decays.
3. **Re-do on main** — the mechanism is worth having, the branch is not; keep it as
   a reference and write the change fresh.
4. **Drop** — superseded by main's KO-beat and respawn work; delete the branches
   and close the rows.

Whichever it is, push `d194-and-respawn-verified` and `attrib-beat-only` first.
