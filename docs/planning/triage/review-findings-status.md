# Review findings — routing, not a second status ledger

Dated `gpt-review-*` files in this directory are historical evidence. They record
what a reviewer believed at a point in time and are not updated as fixes land.

There is no separate permanent review-status authority. A finding that still
needs work must be routed to one of the normal planning authorities:

- executable now -> [`../queue.md`](../queue.md);
- worthwhile but deferred/trigger-based -> [`../tracks.md`](../tracks.md);
- genuine Jon/product judgement ->
  [`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md);
- direct maintainer observation ->
  [`../JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`](../JONS_OBSERVATIONS_BUGS_AND_ISSUES.md).

This avoids the failure mode where a dated review, a review-status file and the
queue each carry independent verdicts and one becomes stale after the next
commit.

## Routing of the 2026-08-29/30 open findings

| Former finding | Current owner |
|---|---|
| Wire/Submerged preserved `initial_dash_timer` | **landed** in `720d16b1`; no open planning row |
| Mary-O active-room `Local` can remember future transition across rewind | `queue.md` **D-SIM-LOCAL** |
| quest/room-visit non-rewinding edge history | `queue.md` **D-SIM-LOCAL** |
| held ranged shots attributed through primary slot | `queue.md` **D-CONTROL-ITEM** |
| D199 centre-line anti-tunnelling ray | `queue.md` **D199** |
| custom held-item abilities use singular controlled subject | `queue.md` **D-CONTROL-ITEM** |
| remaining deterministic projectile/possession/pickup selections | `queue.md` **D-SIM-SELECT** |
| trapdoor `UntilPressedAgain` semantic mismatch | `queue.md` **D-TRAP-HOLD** |
| quadratic rollback copying in `InputStreamRecorder` | `queue.md` **D-INPUT-RECORDER** |
| fighter-brain L6 rollout regression | `queue.md` **D-FIGHTER-L6** |
| universal `spawn_sim_entity` proposal | **not a task**; explicitly rejected as a generic solution in `simulation-authority-and-determinism.md` |
| Switch Pro outer-range question | `queue.md` external measurement + `awaiting-maintainer-decision.md` note |
| stale `trap_probe` comments for withdrawn self-release diagnosis | folded into `queue.md` **D-TRAP-HOLD** |

## Reviewer rule

When a new review lands:

1. verify each finding against current HEAD;
2. fix/reroute findings that have already landed rather than carrying them as
   open because an older review says so;
3. promote still-live work to the normal planning owner immediately;
4. leave the dated review unchanged as evidence;
5. do not create another carry-forward/status list.

A later review may still cite an older finding for rationale, but present-tense
status comes from the queue/tracks/decision authority and current source.
