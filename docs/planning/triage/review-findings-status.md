# Review findings — the authoritative status ledger

**This file is the ONE place that says whether a review finding is open.** The
dated `gpt-review-*` files beside it are HISTORICAL EVIDENCE: what was claimed on
a date, what was measured, and why a fix took the shape it did. They are not
status, and a row's state in one of them may be stale the day after it is
written.

## ⛔⛔ Why this file exists

The 2026-08-30 re-review found the failure mode directly, and it is the one
`queue.md`'s own header warns about in a different register:

> the current documentation can cause an active bug to disappear simply because
> it wasn't mentioned in the later review's carry-forward paragraph.

Measured 2026-08-30, comparing the two dated files against the source:

```text
listed OPEN in gpt-review-2026-08-29 but LANDED       4 rows
    Sentry/Vortex anchors · Sentry/Vortex raw faction
    bomb "thrown" from velocity · submerged repair policy
listed as CARRIED FORWARD by gpt-review-2026-08-30    6 rows
still OPEN in 08-29 and named by NEITHER carry list   3 rows
    gravity authority/ordering · trapdoor UntilPressedAgain
    quadratic InputStreamRecorder
```

⭐ **The gravity row is the proof the bookkeeping mattered.** It was a P0 in the
08-29 file, was not in the 08-30 carry-forward paragraph, and was still live in
the source — nobody had decided to drop it, it simply fell out of every list a
session would regenerate. It was found again only because the next reviewer read
the SOURCE rather than the status.

⚠ **A dated file is never edited to change a verdict.** When a row lands, this
ledger moves; the dated file keeps whatever it said, because the evidence of what
was believed on a date is the thing that makes the next review's disagreement
legible.

## ▢ Open

| # | Finding | Rank | Evidence |
|---|---------|------|----------|
| O1 | Wire/Submerged can preserve an old `initial_dash_timer` — exclusive modes clear `dash_timer` and not this one. Wants one `interrupt_maneuvers_for_mode_transition()` authority. | P1 | [08-29](gpt-review-2026-08-29-rust-correctness.md) |
| O2 | Mary-O `follow_the_active_room`'s `Local<Option<String>>` treats a RESTORED historic room as a new transition and resets rollbacked `FlagSequence`/`MaryOLevelState`. | P0 | [08-29](gpt-review-2026-08-29-rust-correctness.md) |
| O3 | Quest and room-visit edge detection is knowingly non-rewinding; the repo's checksum tests carry these as positive controls. | P1 | [08-29](gpt-review-2026-08-29-rust-correctness.md) |
| O4 | Held ranged shots are attributed to slot zero (`PrimaryPlayerOnly.single()`). ⇒ fold held shots into `ProjectileSpawnRequest` and delete the parallel projectile simulation. | P1 | [08-29](gpt-review-2026-08-29-rust-correctness.md) |
| O5 | D199's anti-tunnelling ray is a CENTRE LINE against an AABB body and checks solids only. ⇒ swept AABB / Minkowski, policy-aware. | P1 | [08-29](gpt-review-2026-08-29-rust-correctness.md), `queue.md` D199 |
| O6 | Custom held-item abilities are singleton-controlled (`ControlledSubject`) while generic pickup/throw/fire iterate `DrivenBodies`. Wants one per-driven-body dispatcher. | P1 | [08-29](gpt-review-2026-08-29-rust-correctness.md) |
| O7 | Deterministic selection at the REMAINING sites: projectile victim ties, possession candidates, pickup-magnet ownership. `sim_selection` is where they go. | P2 | [08-30 rollback](gpt-review-2026-08-30-rollback-anchors.md) |
| O8 | The trapdoor's `UntilPressedAgain` says "any action press except movement/jump/dash" and checks only Attack or Special. ⭐ the useful framing: `SmashChargeSpec` wants to be a TIMELINE HOLD with a smash charge as one customer. | P2 | [08-29](gpt-review-2026-08-29-rust-correctness.md) |
| O9 | `InputStreamRecorder` owns a growing `Vec` and the whole resource is cloned into every snapshot, so frame N copies history `0..N` — quadratic. Keep the finalized recording outside rollback; rewind a cursor and the unconfirmed tail. | P2 | [08-29](gpt-review-2026-08-29-rust-correctness.md) |
| O10 | Fighter-brain L3 rollout — **confirmed bug, diagnosis pending.** The A/B is done and reproducible; what it does NOT say is which rollout veto converts l6's success into 45/45 unfought. Next step is a decision trace, not another sweep. | confirmed | [08-30 rollback](gpt-review-2026-08-30-rollback-anchors.md) |
| O11 | A universal `spawn_sim_entity` seam. Argued against; if it lands it wants an absence contract forbidding the raw spawn for rollback-registered bundles. | architecture | [08-30 rollback](gpt-review-2026-08-30-rollback-anchors.md) |
| O12 | The Switch Pro stick-range measurement on BOTH machines (`Shift+F6`, push to each corner, compare `PEAK`). Jon's to run; the outer-saturation fix depends on the number. | — | [08-30 sticks](gpt-review-2026-08-30-select-cursor-and-sticks.md) |
| O13 | `trap_probe`'s comments still imply a failure mode the reviewer WITHDREW (the Performer's down-Special releasing its own hold). The trace is right and the comments send the next reader down the withdrawn path. | P3 | [08-29](gpt-review-2026-08-29-rust-correctness.md) |

## ▣ Landed, with the row that closed it

| Finding | Closed by |
|---------|-----------|
| Sentry / Vortex outside rollback (no anchor, no registered state) | [08-30 rollback](gpt-review-2026-08-30-rollback-anchors.md) row 4 |
| Sentry / Vortex read RAW faction, violating possession's effective allegiance | [08-30 rollback](gpt-review-2026-08-30-rollback-anchors.md) row 4 |
| Bomb/grenade "thrown" inferred from velocity, wrong both ways | [08-30 rollback](gpt-review-2026-08-30-rollback-anchors.md) row 10 |
| Submerged collision repair contradicts submerged passability | [08-30 rollback](gpt-review-2026-08-30-rollback-anchors.md) row 11 |
| `PortalShot` / `FallingHazard` codec with no anchor | [08-30 rollback](gpt-review-2026-08-30-rollback-anchors.md) rows 2, 3 |
| `portal_fire_system` keeps only the LAST intent per tick | [08-30 rollback](gpt-review-2026-08-30-rollback-anchors.md) row 7 |
| Nearest-target ties on query order — sentry, pickup, world-item body/item | [08-30 rollback](gpt-review-2026-08-30-rollback-anchors.md) rows 8, 9 |
| Hardware quality seed runs in `PreStartup` and is overwritten in `Startup` | [08-30 rollback](gpt-review-2026-08-30-rollback-anchors.md) row 1 |
| **Gravity authority does not rewind, and zones resolve by query order** | [08-30 follow-up](gpt-review-2026-08-30-rollback-anchors.md#-follow-up-2026-08-30--the-second-pass) F3 |
| `WorldItem` sorted by an identity production never attached | [08-30 follow-up](gpt-review-2026-08-30-rollback-anchors.md#-follow-up-2026-08-30--the-second-pass) F1 |
| Two same-channel portal shots on one tick leave two portals | [08-30 follow-up](gpt-review-2026-08-30-rollback-anchors.md#-follow-up-2026-08-30--the-second-pass) F2 |
| Multiple vortex wells / sentries compose in query order | [08-30 follow-up](gpt-review-2026-08-30-rollback-anchors.md#-follow-up-2026-08-30--the-second-pass) F4 |
| The select cursor's speed, and analog folded into digital | [08-30 sticks](gpt-review-2026-08-30-select-cursor-and-sticks.md) |

## ⊘ Withdrawn

⊘ **The Performer's starting down-Special releasing its own `UntilPressedAgain`
hold.** The reviewer traced the acceptance path and withdrew it:
`ProposedVerb::Special::spend()` zeroes the special buffer before the buffered
intent can become the "new press" that releases the hold. The stale COMMENTS are
O13 above; the finding itself stays withdrawn.
