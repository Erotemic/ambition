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

---

## 2026-08-29 — the fourth branch, and a sync sweep

`scripts/sync_status.sh` now answers *"is everything merged and synced"* in one
command. Run against every branch, worktree and submodule it found **four**
unmerged branches, not three. The fourth is `specials-are-real-moves`.

### ✔ `specials-are-real-moves` — **do not merge; superseded, and measured**

Two commits from 2026-08-27, 566 behind main, ten files. It is the FIRST
iteration of the Performer's trapdoor, and that move has since been rebuilt on
main twice over.

⛔ **The branch does not contain what main contains.** `TrapdoorVisual`,
`sync_trapdoor_visuals` and `stays_over_its_surface` are all on main and none of
them is on the branch. Merging would not add the mode; it would drag an earlier
shape of it back underneath the current one.

⭐ **Its five extra test arms ARE the salvageable part, and three of them
contradict a rule Jon asked for.** Ported onto main verbatim and run:

```text
a_submerged_body_does_not_fall                 ok
a_submerged_body_with_no_stick_stays_put       ok
a_submerged_body_moves_under_a_rooted_move     FAILED  (400 -> 400)
a_submerged_body_travels_the_way_the_stick_points  FAILED
a_submerged_body_passes_through_solid_ground   FAILED  (stopped at 400)
```

All three failures are the same fact: their fixture suspends her in OPEN AIR at
`y=600` with the floor at `y=852`, and main's `stays_over_its_surface` refuses
every step for a body with no surface above it. That rule is Jon's, verbatim
(2026-08-28): the trapdoor *"can only move along a ground surface (i.e. it can't
go over a ledge)."* The branch predates it, so its arms assert the mode it
replaced.

⇒ the two arms that pass are worth having and want a fixture that puts her UNDER
a platform, which is a rewrite rather than a port — the same verdict the three
branches above already carry, arrived at independently and with numbers this
time.

### What the sweep actually changed

* `dev/ambition_dev_measurements` was BEHIND its recorded pointer and held one
  uncommitted append — a workspace run from 2026-08-29 that existed on one disk.
  Committed inside the submodule, rebased onto its own main, pushed, and the
  superproject's pointer moved to it. ⛔ **both repositories had to move**: a push
  with no pointer commit leaves the data unreachable, a pointer commit with no
  push leaves a sha nobody can fetch.
* Nothing anywhere was unpushed. All four stale branches are on `origin`, so the
  reference copies the verdict above depends on are safe.

### ⚠ Still open, and it is a decision rather than a task

Three worktrees hold uncommitted work that is **not mine to commit or discard**:
`agent-worktree1` (1 file), `agent-worktree2` (32, detached at a commit already
in main), `sidework` (34). Sampled, it is a partial application of a refactor
that has since LANDED on main by another route — `HazardFeature` moving from the
monolith to `ambition_combat::hazard_runtime`, which main already spells the new
way. So it is very likely abandoned scaffolding rather than work in flight, but
"very likely" is not a thing to run `git checkout --` against.

⇒ **for the maintainer:** if those three worktrees are idle, they can be reset
and the four stale branches deleted, at which point the sweep reports clean. If
any agent is still live in one, leave it.

---

## 2026-08-29 — deleted, and where the shas live now

Jon: *"We can just delete them if neither of us remember if they are useful at
all. They probably got superceded. Maybe we fixed the trapdoor thing and then
redid it again."* He is right about the trapdoor: `specials-are-real-moves` is
the first iteration of a move main has since rebuilt twice.

⭐ **THE PRECONDITION THIS DOC SET WAS ALREADY MET.** The hold said *"delete them
once the replacement lands"*; D192's replacement landed as `cefbfde55` on
2026-08-25. `respawn_delay_ticks` is absent from main not because the mechanism
is missing but because the shipped representation is `RespawnInterval` +
`DeathInterlude` + `OutOfPlay` — the components that already existed.

| deleted branch | tip | last commit | age at deletion | behind main |
|---|---|---|---|---|
| `specials-are-real-moves` | `88000c757` | 2026-08-27 | 2 days | 568 |
| `attrib-beat-only` | `819e48e14` | 2026-08-23 | 6 days | 1226 |
| `d194-and-respawn-verified` | `54d97e05b` | 2026-08-23 | 6 days | 1223 |
| `respawn-interval-holding` | `238d59bfb` | 2026-08-23 | 6 days | 1226 |

⛔ **"SIX DAYS OLD" UNDERSELLS IT, AND THE COMMIT RATE IS WHY.** Main took
118–297 commits a day across that window, so the three 08-23 branches diverged
**1,223–1,226 commits** ago. That is not a stale branch, it is a different
codebase — which is exactly why the 2026-08-25 verdict said re-do rather than
rebase, and why a merge was never the cheap option it looked like.

⭐ A tip sha is enough to resurrect a branch (`git fetch origin <sha>`) until the
remote garbage-collects it, which is why they are written down here, in the D200
queue row, and in the commit that did the deleting.
