# D126 — resolve order, and two capabilities nobody called (DISCHARGED 2026-08-14)

⚠ **EVIDENCE, NOT AUTHORITY.** Items 1, 2 and 3 are closed. ⛔ do not reconstruct
`step_kinematic` or `ActorControlFrame::drop_through` because this file names
them — both were REFUSED wires and deleting them was the payoff. The surviving
item (a one-way moving platform) moved to D115 in the live ledger.

- ▢ **D126 — The movement kernel's resolve order is unspecified, and two of its
  declared capabilities have no caller.**

⭐ **found 2026-08-14 while closing D115 K4, and promoted out of that row on
purpose** — a latent correctness defect in the most load-bearing kernel in the
repository should not live as a sub-bullet under a CLOSED item.

1. ✔ **RESOLVED 2026-08-14 — and the FIX IS NOT THE ONE THAT WAS PROPOSED.** The
   live function is `resolve_axis_repair`
   (`ambition_platformer2d_core/src/movement/collision.rs`); the `resolve_axis`
   this row first named died with item 2 below. It walked `&world.blocks` in Vec
   order applying each correction immediately, so **the last block in the Vec wrote
   the final position** — ceiling-first settled at `Vec2(400, 248)`, platform-first
   at `Vec2(400, 264)`, 16px apart, both stable fixed points.

   ⛔ **"sort by penetration depth and resolve deepest first" was REJECTED, and the
   reason is the whole lesson.** In that fixture the body physically does not fit —
   the ceiling demands centre ≥ 264 while the platform demands ≤ 248 — so **no
   valid position exists**. Sorting makes the answer deterministic without making
   it correct, and **a deterministic wrong answer turns the red test green while
   concealing the physical condition.** ⭐ this is an INFEASIBLE-CONSTRAINT problem
   wearing an ordering problem's clothes.

   ⇒ **the kernel now separates the two, and the derivation came out of the code
   rather than a sketch.** Each intersecting block already computed exactly one
   thing — its own correction — and that correction IS its demand on the body's
   centre, with its SIGN the whole classification: positive means a minimum,
   negative a maximum. ⭐ **and because `strict_intersects` admits only strictly
   non-zero corrections, claims in ONE direction are ALWAYS feasible and claims in
   BOTH are NEVER feasible.** The interval collapses to *deepest claim in the
   single direction* — a max over a set, order-independent by construction — or an
   empty interval. **No clamp, no tie-break, no epsilon.** A `OneWay` is forced
   `on_support`, so a platform can never be the opposing claim and cannot raise a
   false crush.

   ⇒ **`AxisConstraintConflict { axis, min_center, max_center, min_source, max_source }`**
   with `overconstraint()` — the penetration no position can remove, **16.0 for the
   pinned fixture, the same 16px the two orders used to disagree by**. It rides out
   on `FrameEvents.constraint_conflicts`. ⛔⛔ **THE CONSEQUENCE IS DELIBERATELY
   OPEN: NOTHING READS THAT FIELD.** Damage, death, a stock, respawn, forced
   displacement and immunity are all Ambition policy — the reusable mechanics layer
   reports what physically happened and the game decides what it means. The
   infeasible branch positions nothing, reports both contacts and zeroes the axis
   velocity, leaving the perpendicular axis untouched **so a crushed body can still
   walk out sideways**. ⚠ **a policy that LATCHES this — a crush timer, a
   "was crushed" flag, a death latch — IS rollback state** and owes registration
   plus `clear_message_on_rollback`; the field doc says so where its future
   consumer will read it. The conflict itself is not: it is per-step output,
   rebuilt from world geometry each tick and never read back.

   ⭐ **Jon's no-artificial-pushout refusal is PRESERVED, as a filter on
   ADMISSION**: a block whose own correction exceeds the body half-extent
   contributes no claim, so it cannot move the body, report a contact, *or*
   manufacture a conflict. Hoisting it is safe because it reads only
   `half_size()`, which does not depend on where the centre is.

   ⇒ **two pins now, and the second is the one that matters.** The original
   reproduction is GREEN and no longer `#[ignore]`d — it asserts the two orders
   agree, **that they settle at `start`** (the poison against "sort and call it
   fixed": agreeing on 248, 264 or a midpoint would satisfy the first assertion
   alone), and that the reported conflict is equal in both orders. And
   `two_solids_with_a_legal_interval_resolve_the_same_in_either_block_order` proves
   ORDER-INDEPENDENCE rather than crush detection — a body on a wide floor and a
   step, both supports, one legal interval — with `conflict == None` as its poison,
   which fails loudly if the detector fires on ordinary tiled geometry.

   ⚠ **re-baselining to expect**: any replay canary containing a body touching two
   solids on ONE axis in one frame. Single-contact frames — essentially all of them
   — should be bit-identical, and that was established by enumerating the writes,
   not asserted: same entry AABB, same admission test, the claim's WHOLE `Vec2`
   applied (projecting it onto the axis silently changes an OBLIQUE frame, which
   was caught and fixed), same pre-move contact, same grounding condition.

2. ✔ **`step_kinematic` — REFUSED (superseded), DELETED 2026-08-14.** Production
   callers: **0**. Test callers: **26 invocations, every one inside the module's
   own `kinematic/tests.rs`** — so not "test-only API" but *the only caller is its
   own test file*. The live kernel `movement::step_motion` is used from **31 files
   across 10 crates**. ⭐ **it read as live because of 13 stale comment lines in
   11 files across 5 crates**, three of which asserted "both sweeps" exist — a
   comment is a CITATION and citations go stale. −1366 lines
   (`kinematic.rs` + its tests), and every `collision_semantics` helper the dead
   sweep imported was checked to still have a live consumer. ⚠ two surfaces gave
   ACTIVE bad advice and were corrected: `dev/journals/lessons_learned.md` told
   future agents to *"call `ae::step_kinematic`"* — which would have rebuilt the
   very fork that lesson was written to prevent — and ADR 0017 used it as its
   physics example.
3. ✔ **`ActorControlFrame::drop_through` — REFUSED, DELETED 2026-08-14.**
   Producers: **0 brains set it.** `brain/player.rs` zeroed it under an explicit
   *"the engine owns the gesture"* comment; the mount forwarded rider→mount, i.e.
   `false` every tick. ⛔ **and wiring it was impossible without forking a rule**:
   drop-through is a DERIVED gesture — `descend > 0.35 && jump_pressed`, computed
   at the consumer so it stays gravity- and input-mode-relative — and `InputState`
   carries no boolean for it, so the field had **nowhere to map to**. ⇒ the field,
   its `clear_edges` line, its snapshot byte and the mount copy are gone; the
   refusal is recorded where the field sat. The replacement pin asserts *both
   ingredients survive the brain→engine bridge in one frame* and deliberately does
   NOT restate the `0.35` threshold, which is kernel-private — a copy would be the
   second spelling the test exists to prevent.
4. ▢ **A moving platform cannot be authored one-way — NOT DONE, and the reason is
   better than the task.** ⛔ it is not a dead declaration and **not a `bool`
   away**: `one_way_landing_from_previous_feet` compares the body's PREVIOUS feet
   coordinate against the block's CURRENT anti-gravity face. That is sound for
   static geometry and a **MIXED FRAME** for geometry that moves — a rising
   elevator would steal a landing off a stale feet line, a descending one would
   refuse one. `MovingPlatformState` already carries `previous_aabb()` for exactly
   this hazard. **That question must be answered before the field exists.** Cost
   if taken: a field on `MovingPlatformSpec` (5-arg positional `new`, 4 call
   sites) and on `MovingPlatformState` (5 constructors), which is serde-derived
   rollback snapshot state ⇒ another schema bump, plus a new LDtk `field_bool` and
   entity fieldDef; `MovingPlatformState` is referenced from **8 crates**.

⛔⛔ **A TOOLING FOOTGUN FOUND WHILE DOING THIS, and it silently destroys a
baseline.** `scripts/rollback_codec_shape.py` skips any path containing
`/.claude/`, so run from a `.claude/worktrees/` agent it sees **zero** codec files
and `--record` would **blank the baseline** rather than fail. The row was instead
hand-computed and falsified first by reproducing the recorded pre-edit values
exactly. ⇒ **record baselines from the MAIN tree, never from a worktree agent.**

