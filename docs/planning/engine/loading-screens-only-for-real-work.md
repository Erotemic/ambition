# Loading screens only for real work

**State:** EXPLORATION, 2026-10-04. Planning input for an elegant fix; nothing
here is implemented. Companion to
[`room-transition-loading.md`](room-transition-loading.md), which owns the
transition's correctness (one plan, readiness, commit). This doc owns WHEN the
player waits and what the wait is made of. Asset demand and residency are
[`asset-preparation-and-residency.md`](asset-preparation-and-residency.md).

## The rule

Jon, 2026-10-04:

> We need to make it such that loading screens are not developer defined wait
> times, they only show up when actual work needs to be done to prepare a room
> before we can start the simulation properly. Ideally we can hide load times,
> but if we get to a point where we enter a door and we need to transition
> right now, but we aren't ready yet, that's when a loading screen should
> appear.

What follows from it:

- A loading screen is a symptom: the target was not prepared at the moment the
  player committed to entering it. It is never a scheduled event.
- Nothing on the transition path waits for a developer-chosen quantity: no
  ration that paces work behind a cover, no count standing in for a cost, no
  dwell time that outlasts the work.
- Preparation runs ahead of need, hidden. The door is where readiness is
  checked, not where preparation starts.
- What the screen reports is the work actually remaining.
- A door's visual treatment (a fade, a wipe) is a separate design choice from
  the loading screen. It is not a wait.

## What happened on 2026-10-04 (measured)

Jon's GPU host (`toothbrush`, RTX 3090), profiling build, hub to hall of
characters through the door. Bundle:
`dev/ambition_dev_measurements/profiles/desktop-timeline-run-20261005T025328Z`.

| stage | ms |
| --- | --- |
| construction preflight | 3.7 |
| asset manifest | 3.1 |
| **asset wait** | **2,840.8** |
| cover present | 16.1 |
| commit enqueue | 26.3 |
| commit to first frame | 23.3 |
| loading screen visible | 2,652.4 |

`prefetch_hit=false`, `covered=true`.

- **The wait was a ration, not work.** `MAX_CHARACTERS_MATERIALIZED_PER_FRAME = 1`
  with tier-based units (`materialization_units`: Full = 16, which is the whole
  ration) in
  `crates/ambition_platformer2d_actor_monolith/src/character_runtime/mod.rs`.
  The hall stages 137 characters, so it took about 137 frames. At about 20 ms a
  frame during the load, that is 2.8 s.
- **The work was about 0.5 s.** The census shows about 68 MP decoded during the
  wait. The decode times the log printed sum to about 0.5 s (images under 1 MP
  are not printed, so that is a floor). 38.5 MP of it is three baked boss sheets
  (`gnu_ton_boss`, `giant_gnu`, `mockingbird_boss`). The 137 part-drawn
  characters are a few hundred thousand pixels each.
- **The ration charged by tier.** It was sized on 2026-08-29 and 2026-09-01 for
  baked sheets (about 470 MB of RGBA per character). It charges a part-drawn
  character the same, and it applies behind a cover, where its own comment says
  pacing is not wanted.
- **The hall was not prefetched.** `NEIGHBOR_PREFETCH_ROOM_BUDGET = 4` (a room
  count) in `game/ambition_app/src/app/world_flow/room_transition_assets.rs`.
  The hub has more neighbours than that, logs the budget warning, and skips the
  hall as a whole room. So the door found nothing prepared.
- **The bar counts items, not work.** `observe_asset_progress(settled, total)`
  counts settled assets. Cheap items settle at once, so the bar opens near 80%.
  It then crawls while the expensive characters trickle in.

## Developer-defined waits and stand-ins found so far

A starting inventory. The plan should complete it by searching the whole
transition path, not only these files.

| what | where | kind |
| --- | --- | --- |
| `MAX_CHARACTERS_MATERIALIZED_PER_FRAME` and `materialization_units` | `character_runtime/mod.rs` | modelled-cost ration; applies behind a cover |
| `NEIGHBOR_PREFETCH_ROOM_BUDGET = 4` | `room_transition_assets.rs` | room count standing in for a decode/memory budget |
| `cover_required = presentation_available.is_some()` | `room_transition/loading.rs` | covers every transition a presentation exists for, ready or not |
| `minimum_visible = 300 ms` | `room_transition_presentation.rs` | dwell once shown; anti-flicker, but a wait that can outlast the work |
| `loading_reveal_after = 250 ms` | same | hysteresis before the progress UI shows (probably right: it hides short waits) |
| progress = settled / total count | `observe_asset_progress` | count standing in for remaining work |

## Ideas for the shape of the fix (not decisions)

1. **Readiness is a predicate, evaluated at commit.** At the door, the target is
   ready when:
   - its construction plan is prepared;
   - every asset its first frame draws is decoded and uploaded to the GPU;
   - the presentation can draw that frame.

   Ready means the transition commits with no loading screen. Not ready means
   cover, and keep the cover only until ready. The rollback host's frame-zero
   baseline install belongs inside "ready" (`room-transition-loading.md`).
2. **Prepare ahead by likelihood, budgeted by cost.** Priority goes to the door
   the player approaches or faces, then the room's other exits as the budget
   allows. Budget by resident bytes and by uncovered-frame headroom, not by
   room count. A hub with many doors then prepares the one the player walks
   toward.
3. **Pace only uncovered work, where the hitch happens.** The measured hitch was
   the render-world extract and upload of many images in one frame, not decode
   starts (decoding is off-thread). An uncovered frame gets a per-frame upload
   budget, adapted to measured frame headroom. Behind a cover, nothing is paced:
   the cover exists to absorb the burst.
4. **Report remaining work.** Progress is bytes (or estimated decode and upload
   time) ready out of required. A short wait never shows the UI (keep the
   reveal hysteresis). Whether `minimum_visible` survives is a design call:
   anti-flicker is real, but it must not be a reason a screen appears.
5. **Make the work small.** The part road cut the hall's character pixels by
   about 10x. What remains is mostly baked boss sheets, which are candidates for
   parts. Everything prepared ahead is work that never reaches a door.
6. **Observe every transition.** Log `ready_at_trigger` (and why not), prefetch
   hit or miss, and the wait split by stage. A loading screen with no named
   unfinished work behind it is a defect report.

## Open questions for the plan

- What "ready" means precisely on each host (eager, rollback), and whether any
  of it cannot run ahead (construction that mutates shared state).
- The memory budget for prepared rooms, and the eviction that pairs with it
  (`asset-preparation-and-residency.md` open work 4: character pages, FX and
  boss sheets have no retire).
- Characters that stream in while uncovered (open world, spawns in frame): the
  upload budget in idea 3 is their only protection.
- Whether doors keep a fade when ready (design, not loading).
- How to test it: a transition into a prepared room must show no loading screen
  on a rendered host. A headless preflight time is not a transition budget
  (T1 of `room-transition-loading.md`).

## Considered and parked

- **A ration in decoded pixels (2026-10-04).** It charges each realization the
  pixels it decodes, with one 4096 x 4096 page per frame. Monolith tests passed
  with it (1,343 of 1,343). It is parked because it still paces covered work by
  a model. Jon: "That is inelegant if true."

## Acceptance (draft)

- Hub to hall on a rendered host: no loading screen when the hall was
  preparable ahead. Otherwise the wait is no longer than the actual
  decode and upload work.
- No constant on the transition path paces covered work by a modelled cost.
- The progress bar moves in proportion to remaining work, and every loading
  screen names the unfinished work it waited on.
