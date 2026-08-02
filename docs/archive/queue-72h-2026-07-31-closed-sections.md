# Archived: closed sections of `queue-72h-2026-07-31.md`

**Moved verbatim on 2026-08-01, losslessly.** Every section here had ZERO open
(`▢`) rows at the time of the move; the live ledger keeps a one-line pointer
where each stood, and keeps every section that still has open work.

Why: `check_agent_kb.py` reports `docs/planning` at ~31.7k lines against a 10.5k
soft budget and says to *"archive sections that are long done rather than
trimming live plans"*. This is that, by a rule a reader can check — "no open
rows" — rather than by judgement about what still matters.

⚠ **Nothing here is edited.** If a row below turns out to be wrong, reopen it in
the live ledger rather than amending this file; an archive somebody edits is a
second source of truth.

⚠ **One section was deliberately NOT archived despite having no open rows**:
*"Done this run — do not reopen without new evidence"*. It is an instruction to
the next reader, not a record of closed work, and filing it away is exactly how
somebody reopens a settled question.

---

### ✔ S1. Slice H — merged and verified from the main checkout (2026-07-30/31)

**Why first:** it is done work rotting on a branch, and it is the one §4
decomposition trigger that never fired. Depending on `ambition_platformer2d` links 41 crates,
19 of which a movement-only game never asked for.

A first cut exists **unmerged and unverified**: branch
`worktree-agent-af39b56fa4add8fc2`, commit `26237cb3f`. 18 of 41 edges became
implicit crate features with `default = ["all_capabilities"]` preserving today's
facade exactly; `fixtures/minimal_game`'s measured closure moved **41 → 38**;
the footprint ratchet was reworked to measure the sentinel's RESOLVED closure
via `cargo tree --locked` (the old static walk counted optional edges regardless
of features, so it could never have moved).

Three verifications were still compiling at wrap-up and were never run:

1. `cargo check -p ambition_platformer2d` (default features),
2. `cargo test` in `fixtures/minimal_game` AND `fixtures/external_consumer`,
3. the red-probe of the new art-without-render refusal.

→ run those three, then merge or report the red. **A red result is a finding,
not a failure** — it is the first thing this slice has said that nobody knew.

⚠ the honest residue is already recorded on the branch: `render` is optional in
the facade but NOT for `minimal_game` (its windowed boot is a slice-B exit
criterion), `audio` stays unconditional, and the other 14 unwanted crates remain
linked because **`ambition_platformer2d_actor_monolith` brings them** — the §4 carve condition,
exactly as the baseline predicted. Do not cut edges until the number looks good;
that failure mode is named in the slice's own exit criteria.

**Guard:** the facade compiles with `--no-default-features`, and
`check_absence_contracts.py` (which owns the footprint ratchet) is green.

**⚠ VERIFICATION RESULTS, 2026-07-30 — and the first one is a trap about the
METHOD, not about slice H.**

| # | verification | result |
|---|---|---|
| 1 | `cargo check -p ambition_platformer2d` (default features) | ✔ green |
| 2a | `fixtures/minimal_game` | ✔ green, 16 passed |
| 2b | `fixtures/external_consumer` | ✘ 2 failed **IN THE WORKTREE ONLY — not a slice-H defect** |
| 3 | red-probe the art-without-render refusal | ✔ **RED, as claimed — and it found a second thing** |

**⛔ DO NOT RUN AN ASSET-TOUCHING TEST FROM A GIT WORKTREE.** The previous
session's handoff says to run these verifications *"from inside the worktree,
not from the main checkout"*, and for anything that reads generated art that
instruction is WRONG. `actors_desktop_asset_root()` resolves the engine tree to
`CARGO_MANIFEST_DIR/../ambition_platformer2d_actor_monolith/assets`, which is per-checkout, and the
generated sprite tree there is git-ignored — so it does not travel to a worktree:

    crates/ambition_platformer2d_actor_monolith/assets/sprites/   main: 972 files   worktree: 4

Confirmed by running the same target from the main checkout at the PRE-merge
commit: `fixtures/external_consumer --test gameplay` → **9 passed, 0 failed**.
Same code, same manifest, different checkout.

Both failures are "the engine's tree has no art"
(`the_umbrella_asset_install_gives_an_external_consumer_real_sprites`,
`a_consumer_owns_its_own_asset_tree_and_still_sees_the_engines`), which is
exactly what an empty sprite tree produces, and neither names a capability.
`external_consumer`'s manifest gained `default-features = false, features =
["ambition_render"]` on the branch — but on `main` it ALREADY said
`default-features = false` against a facade whose `default = []`, so before
slice H that line removed nothing and all 41 crates linked regardless.

→ merge the branch into `main` and re-run the fixtures from the main checkout,
which is the only checkout that has the art. Same for verification 3.

**⚠ VERIFICATION 3, RUN 2026-07-31. The commit message was right and the API
campaign doc's "unverified" was the stale entry** — building `minimal_game`
without `ambition_render` fails `the_minimal_game_boots_windowed` on exactly the
refusal's own message. The refusal is REACHABLE, which is the standing defect
class this repo keeps rediscovering, and it is not an instance of it.

**But the probe found something nobody claimed, which is what a probe is for:
the refusal MASKS every other composition problem.** Two further tests failed,
both asserting on DIFFERENT refusals and both receiving the render one:

    preparing_art_with_no_declared_cast_is_refused_and_names_both_fixes
    declaring_no_cast_and_a_starting_character_is_refused

Each reports `1 problem(s)`. So a consumer who has built without the render
capability AND declared no cast is told only about the capability — the composition
returns before the roster checks, which run after the capabilities are built. The
refusals are individually good and collectively a funnel: fix the first, rebuild
(~10 minutes), discover the second. That is the same shape as a compiler that
stops at the first error, and this codebase's own refusals are otherwise written
to name every fix at once.

→ ✔ **RESOLVED 2026-07-31, and the first framing of it was wrong.** "Accumulate
across the capability gate" is not possible: the second pass's checks need the
capabilities BUILT (a module may legitimately register its roster through one),
so a draft that does not assemble cannot be asked whether its roster exists. The
funnel is structural. What was fixable is that it was SILENT — the error said
`1 problem(s)` as if that were the whole list. `CompositionError` now carries a
`CompositionStage`, and a `Declaration` refusal says the capability-dependent
checks have not run yet. ADR 0032's promise is kept honestly instead of
overstated: "this is everything" and "this is everything I could see from here"
no longer look identical.


### ✔ S2. Match activation is a TRANSACTION (`activation_transaction`, 2026-07-31)

**Why:** three independent documents converge on this one gap — AD3/AA2 in the
24h ledger, the "lifecycle half" of
[character-preparation-finalization-plan.md](character-preparation-finalization-plan.md),
and API-campaign finding **(g)** (`participants()` is a declaration, the stage's
seating is an independent fact, and nothing reconciles them). It is the P0 both
GPT-5.6 reviews agree on and it is the load-bearing seam under couch-versus,
character select, and netplay.

Today fighters are constructed seat by seat across ticks and only the LATCH is
atomic. Seating runs on `sim`, which under a session IS `GgrsSchedule`, so a
rewind can land between two seats: seating returns early believing the match is
live while the fighters are pre-activation.

**The shape, already argued and agreed:**

    freeze topology
      → resolve every prepared character and spawn/adoption target
      → validate the COMPLETE roster
      → construct or adopt every fighter in ONE commit
      → publish ActiveMatch
      → start the countdown from that

Prefer completing activation BEFORE the rollback session begins. If it must
cross, the activation PHASE and the seat selections must themselves be
rollback-owned, keyed by **stable seat identity rather than `Entity`**.

⚠ **bounded today, and say so**: the versus route starts no rollback session, so
a player cannot reach this — a developer with the rollback observatory open on
`versus_gameplay` can. That is the reproduction.
⚠ **do not treat AD2 as closing this.** Registering the latch stops a rewind
stranding it; it does not stop a rewind landing between two seats.
⚠ **no `BodyUnderConstruction` marker** — refused in the preparation plan for a
reason that still holds: its safety would be `Without<..>` in every present and
future body query. Defer a REQUEST, never publish an incomplete body.

**Guard:** a test module named `activation_transaction` passing in
`ambition_platformer2d_actor_monolith` or `app_it`.


### ✔ S3. AC24 exists and was PROBED RED (2026-07-31)

**Why it comes with S2:** S2 needs a trustworthy acceptance harness and today
there is none. Probed: seating completes on the session's FIRST simulated frame,
so a `check_distance: 4` sync test issues its first load at frame 6 of frame ~1
and **frame 0 is never restored**. Both existing `rollback_match_activation`
tests passed with all four registrations REMOVED; their docstring says so.

Needs activation delayed more than `check_distance` frames after the session's
frame zero. Options: drive the real versus route in the test (bodies are
constructed several ticks after route entry, which does this naturally), or give
seating a seat that is deliberately unsatisfiable for N ticks.

⚠ two fixture arrangements were already wrong and are recorded in the file:
roster-as-frame-zero, and roster-inserted-mid-run-unrebased (an un-rebased
`world_mut` write is behind the cursor and the sync test correctly reports
frames 18–20 diverging).

**Guard:** `rollback_match_activation::rewinds_across_the_activation…` passing
in `app_it` under `--features rl_sim`.


### ✔ S4. The damage authority and the stocks loop (2026-07-31)

**⚠ JON RULED 2026-07-31, and it settles the open question this row carried:**
*"stocks are more important. maybe start the smash demo. versus can be a generic
fighter demo and test things smash doesn't."*

So `fighter_stocks: None` on the versus route is CORRECT and stays — versus is
the generic fighter, settling ROUNDS on health and testing what a health-based
ruleset tests. `game/ambition_demo_smash` is the stocks game, and it is the
loop's first real consumer: the engine owns the count and refuses to place a body
or decide what an ending means, and that split is only real once something stands
on the other side of it.

→ ~~**open on this thread:** the demo has its roster, respawn placement and
victory banner with tests, and no STAGE yet.~~ **STALE — closed later the same
day** (re-checked 2026-07-31 late). `smash_stage()` authors the room with a
`BLAST_MARGIN_PX` on all three sides, `SmashRulesPlugin::hosted()` is installed
by `SmashExperiencePlugin` and reads `FighterStockSpent`/`StocksMatchDecided`,
and `SMASH_SELECT_ROUTE` is the demo's home so a match is entered by choosing
fighters. The demo boots select → lock-in → stocks match end to end.

**Why it is architecture and not a game feature:** `DeathPolicy::Unbounded`
suppresses one consequence — a meter-kill stops killing — and does **not** make
the meter unbounded. `Health::damage` clamps at zero, so `damage_taken()`
saturates at `max`, knockback growth stops growing at 100%, `alive()` goes
false, and `resolve_body_hit` returns `Ignored`. **Selecting the variant today
buys an immortal punching bag.** Every consumer of "how hurt is this body" is
reading a meter that cannot express the thing the mode needs.

Part 0 of [smash-siblings-plan.md](smash-siblings-plan.md): one body-generic
damage authority separating **accumulated damage / threshold death / world
death**, with the consequences named — `DeathPolicy` has to travel with the
health component (so it moves crate-down), and the HUD needs a percent that can
print `188%`, which `ratio()` cannot express.

Then **part 1, stocks, as one complete slice** and not before: uncapped percent
→ knockback scales from it → blast-zone exit spends a stock → deterministic
respawn → temporary protection → zero stocks eliminates → last seat standing
ends the match → reset returns cleanly. `FighterStocks` is vocabulary only today
— no consumer, no rule, no respawn, no rollback registration, no elimination
flow. **All of it lands together, rollback-owned by the mode**: a stock count
that is not registered rollback state un-spends itself on a rewind.

⚠ a stock rule built on a saturating meter would look correct in every test that
never reaches 100%.

**Localized 2026-07-30 — do not re-derive.** The saturation is three lines and
one of them is the surprising one:

- `Health::damage` (`crates/ambition_characters/src/actor/mod.rs:94`) does
  `self.current = (self.current - amount).max(0)` **and returns early on
  `!self.alive()`** — so once the pool empties, later hits are not merely
  clamped, they are DROPPED.
- `BodyHealth::damage_taken` (`actor/body.rs:84`) is `max - current`, so it
  cannot exceed `max` by construction. There is no second meter to read.
- `Health::ratio` clamps to `0.0..=1.0`, so a HUD physically cannot print `188%`
  through it.

**The shape that follows:** `BodyHealth` grows an uncapped `accumulated: i32`
that every hit adds to unconditionally, `damage_taken()` returns THAT, and a new
`percent()` divides it by `max` without clamping. `health.current` stays the
derived pool for `HpDepleted`; under `Unbounded` the body simply stays alive and
the meter keeps climbing.

**`DeathPolicy` moves crate-down, and the direction is already legal:**
`ambition_combat` depends on `ambition_characters` and not the reverse
(`crates/ambition_combat/Cargo.toml:14`, no back-edge), so moving it from
`ambition_combat::components` to sit beside `BodyHealth` in
`ambition_characters::actor::body` is a downhill move with no cycle and no
orphan-rule problem. Re-export from the old path so consumers are not churned in
the same commit.

**Guard:** a `damage_percent` test module passing in `ambition_characters` or
`ambition_combat`; a `stocks` module passing in `app_it`.


### ✔ S5. AE6 — a match DECLARES its rules; `VersusBorrowedRules` is deleted (2026-07-31)

**Why:** `di_max_angle` and `FriendlyFire` are global forward-only resources
that the versus route mutates on entry and puts back on exit. Save/restore is
correct now (AE3) but it is correct **by discipline**: any other writer during
the match wins silently, and a crash between entry and exit leaves the borrow
outstanding. This is route lifecycle mutating global tuning and undoing it
afterwards — the shape, not the bug.

→ project a resolved combat tuning from the MODE/SESSION before the rollback
session begins, and let the stage read it instead of writing the world's.
`VersusBorrowedRules` should not exist afterwards.

**Guard:** `VersusBorrowedRules` has zero production callers, and a
`resolved_combat_tuning` test module passes in `app_it`.

**Built 2026-07-30.** `VersusBorrowedRules` is DELETED, not merely uncalled.

- `ambition_combat::rules` holds `DeclaredCombatRules` (what a match asks for)
  and `ResolvedCombatTuning` (what combat reads). The type lives there because
  `on_hit`, `hitbox` and the damage paths are its readers and a type must sit at
  or below its readers.
- The FOLD lives one layer up in `ambition_platformer2d_actor_monolith::features::combat_rules`,
  because its inputs do not both live down there — friendly fire is combat's own
  baseline, `di_max_angle` is this crate's feel tuning. Ownership travels down
  with the type; the projection happens where the facts are visible. It runs in
  `SandboxSet::WorldPrep`, ahead of every reader.
- All four `Option<Res<FriendlyFire>>` readers now take
  `Option<Res<ResolvedCombatTuning>>`. DI needed no signature churn: the two
  systems that copy `SandboxFeelTuning` locally overwrite `di_max_angle` on the
  COPY, so the hit kernel still travels as one tuning row and ~10 test call
  sites are untouched.
- Registered `declare_rollback_derived_resource`. Registering it as STATE would
  be the borrow again — a rewind would restore a rules value independently of
  the declaration that produced it, and the two could disagree for a frame.

⚠ **the tests changed shape, and the new shape is the stronger one.**
`leaving_versus_restores_what_was_there_rather_than_the_engine_default` became
`a_match_never_touches_the_world_tuning_it_plays_over`: same hostile non-default
starting values, but it now asserts the authored world is byte-identical BEFORE,
DURING and AFTER a match. The during case is the one the borrow could not
express at all — while a match was live, the world's tuning WAS the match's
tuning, so there was no moment at which the two could be compared.


### ✔ S6. `check_agent_kb.py` is GREEN (2026-07-30, `89e1c5426`)

**Why it is architecture:** it is the instrument that keeps every other planning
claim honest, and it was red the whole time the API campaign was being graded
against it (verified by running it at `c737ddfd6~1`). A standing-red instrument
is the same defect class as AF4a's always-red parity audit: a REAL finding
arrives as line eleven of a wall nobody reads.

Two mechanical items were fixed in the 07-30 pass; the two that needed review
rather than typing are done:

- **35 (not 34) source files carry a ≥200-line inline `#[cfg(test)]` module.**
  All reviewed and marked in `status.md` — every one is `behavioral-local` for
  the same reason: it exercises real behaviour through private constructors,
  private fixtures, or `super::` items outside the crate's public surface, so
  extraction would mean widening visibility the design keeps closed. Recorded
  `disposition=maintainer-review-pending`, because an agent may find and
  recommend but only a maintainer grants a permanent exception. ⚠ **two are a
  finding on SIZE regardless of placement** and want a maintainer's eye first:
  `game/ambition_demo_mary_o/src/lib.rs` (2896 test lines) and
  `crates/ambition_platformer2d_actor_monolith/src/features/enemies/mod.rs` (1039).
- **`AGENTS.md` was 227 lines, not the 193 `status.md` claimed** — the stale
  number was itself part of the problem. Now 179. Routed rather than trimmed:
  the Hall rationale became `docs/concepts/hall-of-characters-is-not-special.md`
  with the ⛔ rule and a pointer left in place; everything else was compression
  of routing prose that already pointed at a doc. No rule was dropped, and one
  gained content (git-ignored assets do not travel to a worktree).

**Guard:** `python3 scripts/check_agent_kb.py` exits 0. ✔ verified 2026-07-30.

⚠ still WARNING (non-fatal, and worth a row of its own eventually):
`docs/planning` is 30549 lines against a 10500 soft budget, and `tracks.md` is
1073 against 440. The fix is archiving finished sections, not trimming live
plans.


### ✔ S7. FB4b — the fighter brain emits inputs (2026-07-31) — ⚠ recorded gaps

**⛔ THE FIGHTER BRAIN WALKS OFF THE STAGE** (2026-07-31, measured over 60s of
real fighting in the smash demo). Seat 1 lost all three stocks WITHOUT BEING HIT
— it ran past its opponent and off the edge, repeatedly. The same overshoot is
visible in a short run: it travels from x=416 to x=165 past a foe standing at
247.

L1 has `Situation::Recovery` for a body already offstage, and L2's movement
scoring has no notion of a LEDGE — on an enclosed room "Approach" is always safe,
and every room in this engine was enclosed until the smash stage. So the brain is
correct and the world changed underneath it.

→ **PARTLY DONE 2026-07-31, and the residue is honest.** L2's movement scoring
now reads the supporting solid out of `WorldView::terrain` and penalises an
`Approach`/`Dash`/`Retreat` that would end within a body-width of its edge. Two
tests: closing toward a ledge scores below closing across open floor and below
backing away, and a view with NO terrain is unpenalised (an airborne body is not
a ledge question, and reading "I cannot see the floor" as "the floor ends here"
would freeze every brain in a composition that does not build terrain).

Measured effect in the demo: peak damage over sixty seconds fell from 40% to 24%.

⚠ **the fighter still loses stocks to the floor.** The penalty is -1.0 against
verbs scored 0.5–0.8, so `Approach` drops below `Jump` (0.3) rather than out of
contention — and a jump near a ledge carries you off just as well. The measured
distance is also a body-width, which is a stopping distance and not a braking
one: a fighter already moving at 200px/s needs to decide several ticks earlier.

→ **DONE 2026-07-31, and it needed no new variant.** L1 already had `cornered`
→ `Disadvantage`; it was only ever asked of `stage.distance_to_edge`. On an
enclosed room the room's edge and the floor's edge are the SAME LINE, which is
why nothing needed the distinction until the smash stage. `cornered` now also
reads `WorldView::floor_edge_distance`, so a fighter on the lip of a platform is
in `Disadvantage` — shield, retreat, back off — instead of `Neutral`.

`floor_ahead`/`floor_edge_distance` live on `WorldView` as ONE authority, asked
by L1 to classify and L2 to score. Two implementations of "where does the floor
end" would drift the moment one learned about one-way platforms; L2's private
copy is deleted.

**Measured, over sixty seconds of real fighting: peak damage 40% → 24% → 12%.**

⚠ **the fighter STILL loses stocks to the floor, and the remaining cause is a
third thing.** Travel rose to 469px as it oscillates at the lip — cornered →
retreat → not cornered → approach — and the KO itself is most likely a JUMP:
`Disadvantage` offers `Jump` at 0.4, jumping carries horizontal momentum, and no
verb-level ledge penalty applies to a jump because jumping in place is safe. A
fighter whose opponent cannot attack (a human seat with no controller) has no
other way to die, and it dies.

→ **MEASURED 2026-07-31, and the answer is no — for a structural reason worth
more than the number.** `ladder_probe` runs the same match at levels 1, 3, 5, 6
and 9 and counts the CPU's stocks. Its opponent is a human seat with no
controller and cannot attack, so **every stock lost is a self-KO**:

    level  stocks_lost
      1         3
      3         3
      5         3
      6         3
      9         3

Identical at every rung, including across the `rollout_depth: 0` → `12` boundary
at level 6. **L3 cannot help, because `refine_by_rollout` refines ATTACKS ONLY** —
it returns early on `options.attacks.is_empty()` and yields a `RefinedChoice {
move_id }`. Movement never enters a rollout. A self-KO is a movement defect, so
depth is structurally incapable of fixing it.

→ **HALF DONE 2026-07-31: the rollout now scores movement lines, and the shadow
model still cannot see the case that matters.** `rollout_value` gained a
SUSTAINED intent (it was hardcoded `Hold`, which is the whole reason a rollout
could only answer "which attack"), `RefinedChoice` carries
`suicidal_movement`, and the decision rig VETOES those verbs rather than
carrying a field nothing reads.

⛔ **It is still empty for a walk-off, for two independent reasons the probe
found:**

* `ShadowState` carries NO TERRAIN. There is no floor in the shadow world, so a
  body driven past a platform's edge does not fall — it walks on at the same
  height forever.
* the shadow's KO gate is `matches!(phase, Hitstun) && offstage`. That is
  DELIBERATE and defensible — a fighter offstage under its own power can still
  recover, and calling that dead would make the brain refuse every edgeguard —
  but it puts self-inflicted exits outside what any rollout can see, whatever
  terrain it gains.

→ ✔ **DONE the same day, and it took FOUR fixes to get one signal.** The row was
"the shadow model needs a floor, and a KO gate that distinguishes 'offstage and
reeling' from 'past the point of return'". Both were real, and neither was
enough alone:

1. **The floor got an extent.** `ShadowFighter::ground_span` is the supporting
   solid's span in the gravity frame, read off the view by a new
   `WorldView::supporting_floor` — the same authority `floor_ahead` already
   answered "how far to the edge" from, now exposed as the box so a SIMULATED
   body can use it. A grounded body outside its span stops being grounded.
2. **The KO gate dropped the `Hitstun` requirement.** The old reading conflated
   two situations: *offstage and reeling* is airborne past the floor's edge but
   INSIDE the envelope (dangerous, recoverable, correctly not a KO — that is what
   `distance_to_edge` scores), and *past the point of return* is outside the
   envelope, where on a platform stage the match rules delete you whether you
   were launched or simply strolled off. `StageView::is_known` keeps the empty
   default stage — for which `offstage` is true everywhere — from reading as an
   instant death at the origin.
3. **The veto stopped being gated on having something to swing.** Twice, in the
   same function: `attacks.is_empty()` short-circuited the whole rollout, and
   then `best.map(...)` dropped the veto again on the way out. `OptionSet::
   attacks` is empty in exactly one situation and its own doc names it —
   `Recovery`, "a body past the blastzone has exactly one problem". The body with
   one problem was the body the veto skipped.
4. **The horizon had to reach an edge.** `rollout_depth: 12` is 0.2 s at 60 Hz —
   right for "does this attack connect", and blind to a walk-off by a factor of
   ten. `MOVEMENT_HORIZON_MULTIPLE = 16` makes the veto's line 3.2 s, past the
   2.0 s it takes to walk from mid-stage to an edge at 160 px/s. 8× was tried
   first, reads as obviously enough, and is 1.6 s — SHORT. The test that pins the
   ratio is the multiplication, and it is what caught that.

**The measurement moved for the first time.** `ladder_probe`, whole level-9
profile fixed and only `rollout_depth` varied: **5.0 s to first self-KO at depth
0, 9.8 s at depth 12.** Before this the same table read `3 3 3 3 3` at every
rung — and that was a SATURATED metric (every level lost all three stocks, so the
column could not have reported an improvement if one had happened) over a blind
model. The probe now reports time, not stocks.

⛔ **This is not a fix.** Every rung still loses all three stocks inside ~16 s.
The brain kills itself half as fast, which is progress and not competence.

→ ⚠ **the spec-level question is HALF answered.** §8's ladder metrics are
survival % and damage ratio, and survival on a stage with edges is dominated by
movement — the one thing the rollout does not touch. Either the rollout learns to
score movement lines (a real extension: the shadow model already steps a body, so
rolling a WALK forward is the same machinery), or `l3_earns_its_depth` has to be
measured on a metric attack choice actually dominates. **The first branch was
taken** — the rollout scores movement lines now, and survival moved. What is
still true is the rest of it: survival on a stage with edges is dominated by
movement, so a survival number credits the VETO and says nothing about whether
attack refinement earns anything. Authoring a nonzero `rollout_depth` on a ladder
row is no longer blocked; authoring one on the strength of an ATTACK claim still
is.

→ ✔ **(b) was right, and it had a twin nobody had written down.** Both are the
HELD FRAME being inherited by something that never chose it:

* **All-vetoed applied nothing, and nothing preserves the fatal input.** `frame`
  begins each tick as `state.held`, so `find(...)` returning `None` left the body
  doing exactly what the veto had just called fatal, every decision tick, until
  it did. L2's verbs are Approach / Retreat / Jump / Dash — **none of them means
  "stop"** — so the empty case has to author a halt. It is not a rare branch:
  `ladder_probe` reaches it 164 times in five one-minute matches.
* **`apply_movement(Jump)` never touched `locomotion.x`.** So: veto Retreat,
  choose Jump, keep walking right — a direction this decision did not pick and
  the veto had just struck off. Found by writing the all-vetoed test and getting
  `1.0` where `0.0` was expected, which is the test failing for a better reason
  than the one it was written for. Every verb now clears lateral before speaking;
  facing is deliberately left held.

→ ⛔ **AND MY FIRST FIX WAS A PARALYSIS THAT READ AS A 3× IMPROVEMENT.** The veto
originally sustained a verb for the WHOLE 3.2 s horizon. At 160 px/s that is
512 px — wider than the 420 px stage — so every lateral verb was fatal from every
position, the veto fired on all 1337 decisions, and the "fighter" reasoned itself
into never moving. Survival went from 16.1 s to 44.5 s and every number said
success.

The quantity was wrong, not the size. A brain that re-decides every 5 ticks never
walks for 3.2 s, so asking "what if I did" answers a question nobody faces — and
answers it fatally in every direction. The verb is now sustained for
`commit_ticks` (exactly the decision interval) and the line COASTS for the rest of
the horizon. The long horizon is for the CONSEQUENCE — the fall, the body leaving
the world — which needs seconds; the input does not. Veto rate fell 1337 → 164.

**Where it stands** (level 9, whole profile fixed, only `rollout_depth` moved):

```text
  9/d0      5.2s to first self-KO, 11.4s survived
  9/d12     3.8s to first self-KO, 40.2s survived
```

Note the first column got WORSE. That is the honest shape: the rollout does not
stop the brain dying early, it stops it dying repeatedly. Reporting only the
survival column would have been the paralysis mistake a second time.

→ ✔ **the shadow got an air game, and the measurement went DOWN.** The row was
"the shadow model has no recovery — no double jump, no air drift, and
`MovementVerb::Recover` is the one verb `movement_intent` never judges". All
three closed:

* `SelfView::air_jumps_left` — the body already tracked
  `jump.air_jumps_available`; the fact simply never reached the brain. It sits
  with the other `can_*` capability flags, and answers the question the other
  flags cannot: not "may I press this" but "how many times more".
* `ShadowIntent::Recover` — an air jump that REPLACES downward velocity while
  the budget lasts, `AIR_DRIFT_FRACTION` of ground authority once it is gone.
  Deliberately pessimistic: a rollout that overestimates drift certifies
  recoveries that will not happen, and a fighter that dives off the stage on a
  promise is worse than one that never leaves it.
* **`Recover` steered by `-pos.x.signum()` — "toward the world ORIGIN".** Rooms
  in this engine start at (0,0) and extend positive, so the origin is a CORNER.
  Every body on the left half of every stage recovered by driving further left,
  into the blastzone it was escaping. It survived review because the demo's
  offstage-left is `x < 0`, where the wrong rule gives the right answer; a stage
  at x ∈ [1000, 1600] makes it backwards everywhere. Now asks the stage.

⛔ **AND THE FIRST VERSION OF THIS MADE THINGS WORSE, 40.2 s → 9.2 s.** With
`Recover` finally modellable the rollout could CONDEMN it — and `Recover` is the
only verb offered in `Situation::Recovery`, so an airborne body's option list
emptied, the halt fired, and a doomed recovery was replaced by a certain one.
**Standing still is only a fallback where standing still is survivable.** On the
ground it is; in the air it never is. The rollout now reports how long each fatal
line LASTS, and an emptied list falls back to the longest-lived one.

That made `halt` unreachable — every vetoed verb is a modelled verb, so an
emptied list always has a longest-lived line — so it is gone rather than left
standing as a refusal that cannot fire. The lateral clear moved to the caller,
where "nothing was chosen" means "nothing is pressed" structurally instead of by
every branch remembering.

**Where it lands** (level 9, whole profile fixed, only `rollout_depth` moved):

```text
  9/d0      7.2s to first self-KO, 13.4s survived
  9/d12     7.2s to first self-KO, 14.2s survived
```

The A/B has collapsed to noise, and the 40.2 s figure it replaced was the
paralysis. The `Recover` origin fix is what moved d0 from 5.2 s to 7.2 s — a bug
fix worth more than the whole rollout, which is its own comment on where the
remaining value is.

→ ✔ **that row was WRONG, and tracing the death rather than reasoning about it is
what said so.** The claim was "the deaths are recovery failures, and the veto has
no lever". `ladder_probe`'s per-tick trace of a level-9 fighter says otherwise:

```text
  t381  x=520  loco=1.0  on ground     <- walking right, veto silent
  t392  x=537  loco=1.0  JUMP
  t408  x=572  loco=1.0  vx=310        <- airborne, still driving right
  t430  x=733                          <- off the stage
```

It is not a failed recovery. It is a **jump held into a drift**, off a ledge the
veto had guarded correctly for six seconds of walking. And the rollout scored
that line as safe because `ShadowIntent::Drive` was gated on `on_ground`:

**a shadow body that jumped went straight up and landed exactly where it took
off.** The model had no air control at all. Every "walk to the ledge and jump"
line — the commonest death in the game — came back clean.

One `if` removed. The A/B, level 9, whole profile fixed:

```text
  9/d0      7.2s to first self-KO, 13.4s survived
  9/d12     7.9s to first self-KO, 22.7s survived
```

Both columns move, in the same direction, without the paralysis. That is what an
honest signal looks like next to the 40.2 s that was not one.

⚠ **and the first version of the test for it was VACUOUS.** It jumped, drove for
90 ticks, and asserted the body had travelled — but a jump's airtime is 36 ticks,
so the body landed and WALKED the rest. It passed with air control removed. The
window now closes at 30 ticks and asserts `!on_ground` at the assertion, which is
the only reason the assertion is about air control at all.

Also landed while looking for this: `ShadowTuning::dash_speed` / `dash_time`. The
shadow modelled `MovementVerb::Dash` as `Drive` — a 160 px/s grounded walk against
a 760 px/s impulse that works mid-air, 4.75× in the direction that matters. It did
not move the measurement (this fighter does not dash), and it is wrong regardless.

→ ✔ **`AIR_DRIFT_FRACTION` settled, by the engine's own constants and a
measurement** (2026-07-31). `ae::AIR_ACCEL` is 3100 px/s² against a SHARED
`MAX_RUN_SPEED` cap of 270 — a real body reaches the same top speed in the air as
on the ground, differing only in taking ~0.09 s to get there. So airborne `Drive`
at full ground speed is right for the steady state, and 0.6 is not a weaker air
game: it is the average over a ramp the shadow does not model, which sets
velocity instantly. The doc says that now instead of claiming physics.
⚠ **and moving it to 1.0 changes `ladder_probe` by NOTHING** — same first
self-KO, same survival, every rung. Whatever kills this fighter, out-of-jumps
drift is not on the path; do not tune this number hoping to move that number.

→ ⚠ **`ladder_probe` numbers are only comparable WITHIN one build, and this run
proved it.** Level 9 depth 12 read 2.7 s / 23.5 s before the stocks respawn
started going through `reset_body_clusters`, and 2.7 s / 5.4 s after. Bisected to
that commit. Nothing about the brain changed: the fighter had been carrying
maneuver state and a wrong body size through every knockout, and survival fell
when the respawn was FIXED. A cross-build comparison of this probe is a
comparison of two different games.

**⚠ TWO NAMESPACES SHARE ONE WORD, and it cost an hour** (2026-07-31, found by
drawing a match): `ControllerBinding::Cpu { brain_profile }` is NOT a catalog
brain preset. `spec_for_brain` looks the string up in the `CharacterRoster`
fragment's `by_brain` ARCHETYPE map and falls back to a default spec — whose
brain is `stand_still` — when it is absent. The catalog's `brain_presets`, which
`default_brain` and placement overrides resolve against, are a different
namespace a seated CPU never consults.

The field name reads exactly like the catalog's vocabulary, the failure is
silent, and the symptom is a fighter that stands still — which is
indistinguishable from a brain that was never installed. `BrainPreset::Fighter`
(added the same day, below) IS the authoring path, for a catalog-driven body; it
is simply not the road a MATCH SEAT travels.

→ ✔ **DONE the same day.** A CPU seat naming a brain profile the composition's
`CharacterRoster` does not have is now UNSATISFIABLE — the transaction refuses
and nothing is built — with a `debug_assert` naming the profile and listing the
keys that do exist. `resolve_initial_brain` already held this line for placement
overrides (*"never a silent fall back to the default"*); the seating path had the
opposite policy for the same class of mistake and now matches it.

⚠ **it caught the engine's own fixtures immediately, which is the finding under
the finding.** `seating/tests.rs` asked for `medium_striker` while its
content-free roster defines only `combatant` — so every seating test had been
building a `StandStill` fallback while naming a striker, and passing, because
nothing they assert depends on the brain. The helper names `combatant` now, which
is what those fixtures were really seating all along.

**Why:** ladder rows stay `rollout_depth: 0` until `l3_earns_its_depth` exists,
so the whole L3 rollout investment (FB6a–e, landed) is currently unexercised on
the ladder. Specced as §13 of
[engine/fighter-brain.md](engine/fighter-brain.md) and described there as
plumbing plus three careful pieces: `StateMachineCfg::Fighter`; the kit riding
`BrainSnapshot.attack_kit`; APM at the ONE emission point; one
snapshot-registered noise `u64`.

**Guard:** a `brain::fighter::decision` test module passing in
`ambition_characters`. ✔ 9 tests, 2026-07-31.

**The attack kit has a producer** (§13.2, closed same day):
`Option<&ActorMoveset>` joined the actor tick query and
`build_enemy_brain_snapshot` fills `attack_kit` from the contract's own moves via
`spec.frame_data()`. Declaration order, which `MovesetContract.moves` already is
— so the kit is stable across ticks and across a replay without a sort.

Built every tick like every other snapshot field. If profiling ever complains the
fix is to rebuild on moveset CHANGE, never to let it go stale.

⚠ the PLAYER's snapshot builder still fills an empty kit. That is correct today —
the player brain translates device input and never scores options — and it is the
line to change if a fighter brain is ever pointed at the primary player body.

⚠ **two spec facts did not survive contact and are recorded in the commit**:
`BrainSnapshot` was `Copy` and is not any more (19 mechanical errors, 3 files),
and `SnapshotCursor` is encode-only — a checksum projection, not a restore path,
so the clone restores `FighterState` and the cursor only makes divergence
visible.


### ✔ S8. AC22 — the exit oracle's COVERAGE moves with authored content (2026-07-31)

**Why it is next, by the review's own standard:** it is *a measurement that can
move*, which is the goal's ordering rule almost verbatim. The oracle is the
engine's determinism proof. AC18 already stopped a content edit from BREAKING
it — the assertion reads `lifetime_*` counters that survive a rebase — but what
the run COVERS still depends on the room: the authored enemy set decides whether
a confirmed Track-B lifecycle commit lands inside the rollback window, and
therefore whether the proof crosses a mid-window session rebase at all. Nothing
says which of the two runs happened. *"A proof whose coverage moves when a
designer edits a room is a weaker proof than it looks."*

**The bounded repair, and what it is NOT.** The row's first option — give the
oracle a fixture room it owns — is deliberately not taken; see the decision on
the row itself. What is taken:

1. **PIN the coverage.** `combat_equipment_switch_and_breakable_survive_forced_
   rollback_identically` asserts `sessions_installed`, so a content edit that
   changes whether the run crosses a rebase FAILS and says so, rather than
   silently changing what the proof covers.
2. **The deliberate rebase scenario already exists** and is better than anything
   this oracle would grow: `rollback_room_transition::a_transition_intent_is_
   recorded_then_committed_exactly_once` walks a body through an `EdgeExit`,
   proves the intent is recorded while predicted, committed exactly once on
   confirmation, bumps the session generation, and stays checksum-clean past it.
   The pin points at it by name, so a reader who hits the failure knows where
   the crossed-rebase case is covered.

**Guard:** `cargo test -p ambition_app --features rl_sim --test app_it
rollback_exit_oracle` green with the pin in place, and the pin probed by
asserting the other value.

**✔ DONE, and the first form of the pin was WRONG — the measurement said so.**
Pinning `sessions_installed == 1` failed immediately: the run reports **3**, and
none of the three is a lifecycle commit. Both setup helpers fold a `world_mut`
mutation into frame zero (`wear_oracle_armor`, `stage_player_on_arena_floor`)
and each fold INSTALLS A SESSION. `advance_runs: 2980` against
`lifetime_advance_runs: 2981` is the same fact from the other side: the current
session did essentially all the work, so nothing rebased mid-walk.

So the pin is a **delta**, captured after staging and compared at the end — a
statement about the WALK rather than about how the baseline was built. Pinning
the literal 3 would have made the oracle break the next time somebody adds a
setup step, which is the same class of coupling AC22 is about.

**Probed:** injecting one `rebase_rollback_history()` at frame 200 of the walk
fails the pin with *"the walk installed 1 further session(s) on top of the 3 the
setup built"* — and the stats it prints (`advance_runs: 1980`,
`lifetime_advance_runs: 2961`) are exactly the AC18 shape the pin exists to make
visible instead of mysterious. 7/7 `rollback_exit_oracle` tests green with it.


### ✔ S10. Three things the suite found about ITSELF (2026-07-31)

Running `./run_tests.sh` after the review repairs returned **3 of 24 jobs red**,
and none of the three was the work being checked. Recorded because two of them
are instruments failing in the way instruments fail — quietly, and in a shape
that reads as the code being wrong.

1. **The SDK reference ratchet did its job.** `RollbackHealth::generation` was a
   new public method the reference did not name, and
   `test_every_public_builder_method_is_documented` said so. `docs/sdk/api-reference.md`
   now documents `generation()` and `ambition_platformer2d::rollback::stop`, including why a
   frame number cannot tell a restart from a rewind.
2. **The facade's first unit test created a feature job that ABORTS on this
   machine.** `crate_has_tests` is what admits a crate to the per-crate feature
   pass, so `crates/ambition_platformer2d` gaining `#[cfg(test)]` created a
   `-p ambition_platformer2d --features ...,profile,...` job — and `profile` forwards
   `bevy/trace_tracy`, whose static initializer aborts a test binary on a CPU
   with no invariant TSC. It fails before libtest lists a test: `--list` and a
   filter matching nothing fail identically. `ambition_platformer2d` joins `SKIP_FEATURE_JOB`
   for the reason already stated there — every one of its 17 extra features is a
   forwarder to `ambition_platformer2d_actor_monolith`, which is skipped for the same reason — and
   the Tracy trap is written down where the next person to remove that entry
   will read it.
3. **⛔ A standing guard was FLAKY, and its flakiness was invisible.**
   `shell_host_lifecycle::the_full_multi_game_lifecycle_is_leak_free` failed in
   the full `app_it` binary and PASSED ALONE. Cause: `Messages<AppExit>` is
   double-buffered and dropped after two updates, and the test launched the Exit
   row, settled FOUR updates, ran one more, and then asked whether an exit
   message existed. It was asking about a buffer already cleared — green only
   when the host happened to answer late. It now WATCHES for the message across
   updates instead of sampling once at the end. 263/263 in the full binary.

⚠ the shape worth keeping: *a test that samples a Bevy message buffer some
updates after the event that fills it is not testing what it says it tests.*


### ✔ S11. The review's "constrain now" list, done as constraints (2026-07-31)

The review named four things to CONSTRAIN rather than solve, and said so
explicitly: *"do not spend a long campaign proving that a failed startup app is
reusable unless a consumer actually needs that behaviour."* Constraining them is
work, and it is the work that stops a provisional mechanism from being read as a
1.0 promise.

* **`install_into` mutating before validation completes.** The practical half is
  now done rather than promised: parsing a declared roster and building a
  silence fragment are PURE, and they used to run after the asset sources and
  the whole Bevy foundation were installed — so a typo in a RON string was an
  ASSEMBLY-stage refusal against an `App` the consumer had already handed over.
  Both moved into the declaration pass as a prepare step, in the same
  prepare/commit shape the rollback rebase and the room reset already use. What
  stays in assembly genuinely needs the built App (a fragment CONFLICT is a fact
  about what is registered). Held by
  `a_roster_that_does_not_parse_is_refused_before_anything_is_installed`, which
  asks the `App` whether `AssetPlugin` arrived instead of trusting the stage
  name — **with the control that makes that mean anything**: an assembly-stage
  refusal on the same path DOES have the foundation installed.
  And the limit is now STATED on `install_into`: a failed installation is fatal
  to that `App`; retry is not supported.
* **Opaque, order-sensitive capability closures.** `ModuleDraft::capability` now
  says what it is: no dependency resolution, no conflict detection, no claim
  that two modules declaring overlapping capabilities compose in either order.
  Provisional until a real second consumer produces an overlap — because a
  capability identity is the part a third party authors against, and that is the
  one thing that must not be guessed.
* **Settle ticks.** `RollbackPlan::settle_ticks` now says a tick count is not a
  readiness contract: it buys frames, it proves nothing, do not tune it to make
  a flaky test pass, and the durable version is a semantic barrier that belongs
  with atomic activation.
* **Consumer-supplied participant counts** — already handled by S-item 3 above:
  invalid counts are rejected and the effective count is reported. Full
  authority moves into prepared session topology with atomic activation, not
  as a facade cleanup.


### ✔ S12. A second biome had no sky in every composition but one (2026-07-31)

**The recurring class, found by following a limit rather than a bug report.**
S5's secondary-experience row named the startup room theme as "the one with a
visible consequence", and asking why led somewhere bigger: the parallax THEME
LOAD was app-local. `game/ambition_app`'s room-transition machinery calls
`ensure_parallax_layers_for_room`; nothing else does. So the shipped host had a
backdrop in every biome, and the demos, the external consumer and anything built
through `PlatformerApp` had the STARTUP room's theme and nothing else.

It failed the quiet way. `spawn_parallax_layers` skips a layer whose handle is
absent — `continue`, no warning — so a room in a second biome simply has no
background. Nothing in the world says the art was never requested.

⚠ **and the refresh was app-local too**, which is what made the fix a pair:
`refresh_parallax_layers_on_quality_change` is the system that turns a
`GameAssets` change into respawned layers, and it was registered ONLY in
`ambition_app`'s quality chain. Installing the load without it would have been a
fix that reads as a fix and changes no picture.

→ Both now live in `SessionRoomVisualsPlugin` — the plugin that SPAWNS the
layers, and the one the Ambition shell host adds directly — which is the same
resolution the world-label placement pass (AE1) reached three days ago, in the
plugin whose own comment says so two lines above. The app's duplicate
registration is removed; registering the despawn/respawn twice would run it
twice in one frame.

→ ⛔ **and the sweep found the SAME class one step further along, which is why
the sweep was worth running.** `sync_parallax_layers` — the system that MOVES
the layers with the camera — was app-local too. So a composition that got its
backdrop spawned still left it at the world origin forever: it slid out of frame
as the camera walked away, and the one thing a parallax layer exists for never
happened. Nothing about that reads as a missing system. The art is correct, in
the wrong place, and only when you walk. It now registers in the same plugin,
`.after(camera_follow)` — legal because `camera_follow` is DEFINED in
`ambition_render` and merely REGISTERED by `ambition_platformer2d_host`.

**Who this actually fixes, checked rather than asserted:** `ambition_demo_sanic`
authors ONE room and gives it the `skybridge` theme, so the startup bind already
had its art and the theme load never bit it — but its backdrop **never moved**,
on a demo whose entire subject is a camera travelling fast along a speedway. Both
demo apps install `PlatformerPresentationPlugin`, so both pick up the layer sync
(and the theme load, the quality resolve, and the projectile visuals through
`PlatformerHostPlugins`) from this work.

**The rest of the sweep, surveyed and deliberately LEFT** (`ambition_app` still
registers these engine systems by hand; each has a reason that is not "nobody
looked"):

* `refresh_entity_sprite_handles_on_game_assets_change` — pairs with
  `reload_visual_quality_assets_on_scale_change`, which is app-local because the
  settings menu is. A demo's `GameAssets` does not change after startup.
* `sync_health_overlays`, `sync_lock_wall_visuals` — generic code, but both are
  a GAME's choice (floating health bars; encounter lock walls). Installing them
  for every composition would make the engine decide a demo's HUD.
* `gate_portal_visuals::*`, `sync_portal_capture_parallax_layers` — portal is
  Ambition's feature and is feature-gated.
* `apply_placeholder_sprites_override`, `apply_hide_sprites_override` — dev
  tools, and the app owns its dev overlay.

**Guard:** `theme_load_tests::a_room_in_a_second_biome_loads_its_own_parallax_theme`
adds the PLUGIN rather than the system — the defect was never that the load did
not work, it was that only one composition installed it, and a test that wires
the system by hand cannot go red on that. **Probed:** with the registration
removed it fails on *"the active room asked for the `cave` theme and nothing
loaded it"*. Two more, both probed the same way: a theme already present must not
be re-derived every frame (a mutable deref alone marks `GameAssets` changed and
the refresh answers that by rebuilding every layer in the world), and a layer
must MOVE when the camera stands at x=900.


### ⛔ S14. The disk filled up, and the suite is what filled it (2026-07-31)

Mid-run, every build started failing with `ENOSPC`: **387G, 100% full**, 36M
free. Not a leak in the game — `/home/joncrall/ambition-target/debug/incremental`
alone was **110G**, against 167G of `deps`.

`scripts/run_tests.py` is the main producer and gains nothing from it: every
feature job is its own variant, each variant keeps its own incremental tree, and
a suite that runs a dozen times a day never reuses most of them. A job either
recompiles from a feature set nothing else shares — no cache to hit — or is
already fresh and does not compile at all.

→ the runner exports `CARGO_INCREMENTAL=0` (via `setdefault`, so
`CARGO_INCREMENTAL=1` still wins for anyone who wants it), and the 110G tree was
deleted. 72G → 110G free.

⚠ **for whoever runs the next long session:** watch `df -h /`. The failure mode
is not a clear error — the first symptom here was a cargo message about a
missing *work-product index*, which reads like a corrupt build, and the second
was a suite job dying with no useful output at all.

⚠ **and it came BACK within the hour: 98%, 12G free**, because a full suite plus
a `--features visible` fixture run rebuilds every variant, and `CARGO_INCREMENTAL=0`
only covers `run_tests.py` — a manual `cargo test` still writes one. The three
trees worth deleting first, measured: `outlander/debug/incremental` (8.7G),
`debug/incremental` (5.1G), `wasm32-unknown-unknown` (3.2G). 12G → 28G free.
⛔ **do not delete a target dir while the suite is USING it**: doing exactly that
is what made the `web_served_assets` job the only red in suite15, and re-running
it alone passes. Prefix heavy manual runs with `CARGO_INCREMENTAL=0`.

**Suite state, 2026-07-31 evening: 23/24, and the one red is the deletion above,
re-verified green on its own.**


### ✔ S15. AF4b's duplication is COLLAPSED (2026-07-31)

S9 GUARDED the last duplicated field (one id, two display names is a reported
conflict now). Collapsing it is the fix, and the question that made me defer it
has an answer: **`character_catalog::load_catalog()` is a pure function over an
`include_str!` constant.** No `App`, no plugin order, no asset load. So the
objection — "registration happens before the catalog is necessarily loaded" —
does not apply to Ambition's own content crate, which is the only place the
lineage lives.

→ `Incarnation` keeps `id` and `replaces` (reusable lineage COMPOSITION, which is
Jon's ruling) and loses `display_name` and `sheet`; `definition()` reads them off
the catalog row, the sheet through `CatalogEntry::manifest_target()` — the same
canonical projection the parity audit already compares with. Content owns the
facts, in one place.

⚠ hoist the parse: `definition()` is called per incarnation, and parsing the
whole roster three times at startup for three strings is the kind of thing that
looks free until the cast is fifty.

**Guard:** `every_incarnation_resolves_its_own_distinct_sheet` and
`every_incarnation_says_something` already exist and must stay green;
`the_shipped_cast_has_one_authority_per_character` becomes structurally true for
these three rather than checked.

**✔ DONE, and one of the guards had the SAME defect the voice field taught.**
`Incarnation` is two fields now — `id` and `replaces` — and `definition()` reads
the display name and the sheet target off the catalog row, the sheet through
`manifest_target()` (a row names FILES, a definition names a TARGET). The parse
is hoisted so registering the lineage reads the roster once.

⚠ `every_incarnation_resolves_its_own_distinct_sheet` was reading
`incarnation.sheet` — a Rust literal — so it would have stayed green while the
catalog said something else, which is exactly what
`every_incarnation_says_something` had to be rewritten out of on the voice field
earlier the same day. It asks the DEFINITION now. A new test pins the other half:
every incarnation presents under its catalog name, which makes
`DisplayNameDisagreement` unable to fire for these three by construction.

A missing row is a panic, not a fallback: inventing a name here would put the
duplication back one `unwrap_or` at a time. 5/5 lineage tests,
`the_shipped_cast_has_one_authority_per_character` still green.


### ✔ S18. The consumer asks whether the engine DREW it (2026-07-31)

S13's instrument catches the SHAPE — "no engine crate registers this system" —
by reading registration sites. It cannot see a picture. So the composition that
can answer honestly now does:
`fixtures/external_consumer/tests/the_engine_draws_what_it_promised.rs` builds
Outlander's own windowed app, walks the body for 180 frames, and asserts the
backdrop **exists and moved**. **Probed:** with `sync_parallax_layers`
unregistered it fails on *"every parallax layer is exactly where it was after
180 frames of walking"*.

⚠ **the component was PRIVATE, which is its own small finding.**
`ParallaxLayerVisual` sat behind a private module, so a consumer could install
the whole parallax family and had no way to ask whether a backdrop existed —
"is my sky drawn" was a question only `ambition_render` could answer. Exported,
with the consumer that justifies it.

⛔ **and I exported it in the WRONG place, which a guard caught the same run.**
The first version reached it through `ambition_platformer2d::renderer` — the raw crate
re-export — and `outlander-names-only-the-public-sdk` failed on `new=['renderer']`
in the suite. That contract exists because *"a consumer's imports encode our
implementation topology and we cannot move an implementation without breaking
them"* (ADR 0031), and "my test needed it" is exactly how such a leak gets made.
It lives in `ambition_platformer2d::view` now — *"what is drawn, as a game observes it"* —
which is the surface the question belongs to.

⛔ **the Sanic sibling was FLAKY for a day-old reason.** It passed alone and
failed inside the suite binary: a fixed-tick host without a pinned clock runs a
machine-speed-dependent number of ticks per `update()`, so 240 frames of holding
right covered a different distance under load. `act_completion` in the same crate
carries the same two `ManualDuration` lines with the same comment. This is the
second flake of this shape today — the first was `shell_host_lifecycle` sampling
a message buffer — and both were invisible to a single-test run.

⚠ **and the quality half of this file was ATTEMPTED and removed, which was worth
the attempt.** `ResolvedVisualQuality` reads `UserSettings`, and Outlander is
built `default-features = false`: it never took the `ambition_persistence`
capability, so `ambition_platformer2d::persistence` does not exist for it and there is no
settings resource to read. The quality resolve is inert there BY CONSTRUCTION —
slice H working as intended, not an omission. A consumer-level test for that
half belongs in a fixture that takes the capability, not in the one whose whole
point is taking as little as possible.


### ✔ S19. AC21 — the workspace builds with ZERO warnings (2026-07-31)

AC21 said AC17's *"remaining warnings are TEST-only and in
`ambition_platformer2d_core`"* was already stale. It was, and the answer is now a
number instead of a list: `cargo check --workspace --all-targets` emits **no
warnings at all**.

Five of them, and three were the interesting kind — `update_player_scratch`,
`update_player_control_scratch` and `update_player_simulation_scratch` in
`test_support.rs`: thin wrappers that pass `TEST_TUNING` into a
`_with_tuning_` sibling, with no caller anywhere. Test scaffolding for tests
that were rewritten out from under them. Deleted, not `#[allow]`ed — a helper
nobody calls is a helper that drifts out of step with the kernel it wraps, and
the next person to reach for it would find it compiling and wrong. The other two
were an `unused_mut` and an unused import.

⚠ **the value is the FLOOR, not the five.** 51 warnings under the
`web_served_assets` feature set are cfg-shaped and expected; what matters is
that the default `--all-targets` build is silent, because a build with five
warnings is a build where the sixth — the real one — arrives unnoticed.
`ambition_platformer2d_core` 448 and `ambition_characters` 379 tests green after the
deletions.


### ✔ S20. Sanic's own suite now says its sky moves (2026-07-31)

S18 put the question to the external consumer; this puts it to the demo where
the answer is most visible. `ov1_draws_the_world` already asks whether a demo can
draw its world at all — this is the same question one frame later, and the demo
whose entire subject is a camera travelling fast is the one that had no guard.

`the_speedway_backdrop_follows_the_camera`: settle until a layer exists (failing
with what an absent layer would MEAN, rather than comparing two empty lists),
then hold right for 240 frames and require the layers to have moved. **Probed:**
with `sync_parallax_layers` unregistered it fails on *"the sky is pinned to the
world while the camera runs down the speedway"*. 10/10 in that file.

---

### ✔ S24. The GPT 5.6 review of `7fb5c9a..a1b8da9` — two closed, one open (2026-07-31)

The review's verdict is that the interval's work landed; three current defects
were called out, and it asks for playtesting after them rather than another
audit.

* ✔ **1 (critical) — `BodyHealth` rollback dropped the meter and the policy.**
  Fixed; wire format v5 → v6. Both requested tests exist and both were PROBED
  against the original codec — where `rollback_health()` stayed silent for
  thirty ticks (both sides hashed the same incomplete encoding) and the VALUE
  assertion reported *"went into the rewind at 3760% and came out at 0%"*. That
  probe is the review's own point demonstrated: the checksum could not see it.
* ✔ **3 (medium) — Blink stayed pressed across held fighter frames.** Fixed at
  the seam rather than the site: `ActorControlFrame::clear_edges` said "all
  rising-edge flags" and cleared eight of fifteen, which is why the fighter's
  tick had open-coded three by hand and was right to. Probed: 5 of 5 ticks
  re-pressed.
* ✔ **2 (high) — the fighter scores a move and executes a generic attack.**
  CLOSED in two commits: the emission (`3361f5cdf`) and the acceptance test
  (`b8a133b3d`). The review's condition was specific and it is the one that is
  asserted — `MovePlayback.spec.id` after a scored choice, reached through the
  real kit builder, the real decision, the real emission, then
  `resolve_attack_gestures` + `trigger_moveset_moves`.
  ⚠ **the fixture's premise is REACH, not names.** A jab reaching 16 and an
  up-tilt reaching 84, so a 70-unit gap has one clear answer; two
  interchangeable moves would pass with the direction still discarded.
  **PROBED** against the original bug — the emission's axis forced back to
  `Vec2::ZERO` — which reports `left: "jab", right: "uptilt"`. And the
  close-range test is the CONTROL: it stayed green under that probe, so the pair
  pins that the direction is the scored one rather than a hard-coded `Up`.
  What it WAS, for the record: `RefinedChoice::move_id` was discarded —
  `pending_press` stored only a tick count, the emission set `melee_pressed`,
  and `trigger_moveset_moves` resolved whatever the neutral gesture mapped to.
  The reach / frame-advantage / rollout work decided WHETHER the brain attacked
  and never WHICH scored attack it performed, and a zero-reach move (a buff, a
  summon) selected by id still came out as a swing.
  The repair, and the two places it differs from the review's sketch:
  `AttackCandidate` carries the BINDING that invokes its move, the pending
  action retains it across execution jitter, and maturing populates the matching
  `ActorControlFrame` fields — one shared execution seam, not a fighter-only
  bypass. The pending choice is authoritative state and joins the fighter
  snapshot projection.
  ⊘ **POSTURE is deliberately NOT in the binding**, though the sketch listed it:
  the body's real grounded state decides that at press time, and a brain that
  could claim a posture it does not have would be reaching past the no-cheat
  contract to pick a move its body cannot perform.
  ⚠ **the kit ENUMERATES PRESSES** rather than listing `moveset.moves`, which
  the sketch did not ask for and which the same line required: a move no input
  can invoke had nowhere for its identity to travel. Every candidate is now
  executable by construction, and
  `every_candidate_in_the_kit_carries_the_press_that_invokes_it` is the guard.

---

## ✔ S45 — GPT 5.6's review of the couch slice: all five findings were right

Five findings, all confirmed against the code, all fixed. Three of the five were
**false green guardrails I wrote**, which is the part worth carrying.

1. ⛔ **reconnect swapped participants.** My test left exactly ONE seat vacant, so
   any returning pad filled the "correct" one — it proved vacancy filling and I
   wrote it up as restoration. With both seats vacant, pad B back first took seat
   0 and the two players swapped characters. Fixed with `PadIdentity` (OS name +
   USB vendor/product), which is what survives an entity dying. ⚠ two IDENTICAL
   controllers stay indistinguishable — the information does not exist — and that
   limit is in the type's own doc rather than left to be discovered.
2. ⛔ **a freeze protected the mapping from REPAIR as well as reorder.** A frozen
   seat whose pad returned kept pointing at a dead entity forever. The identity
   pass runs frozen or not now: handing a seat back its own controller is not a
   reorder. ⚠ the first attempt fell through to `None` and an existing test
   caught it — `None` means EVERY pad.
3. ⛔ **the Smash plugin stamped a global policy on every route**, including other
   games'. It writes only its own claim now and restores the default once, on the
   frame it leaves its own routes.
4. ⛔ **the keyboard-plus-pad test never pressed a key.** It drove only the pad
   and asserted the other body stayed still — which a seat wired to nothing
   passes. Both directions now.
5. ⛔ **"every declared field" compared two fields** and its rationale named four.
   ⚠ extending it to the other three would have been WRONG — health, box and mass
   come from each body's own character, not the match, so comparing them across
   seats fails on any asymmetric pair. Renamed to what it checks, and the third
   roster-declared field (`opens_suspended`) added.

⚠ **the pattern across 1, 4 and 5**: each test's NAME and its ASSERTIONS described
different things, and the name is the part people read — including me, a day
later, when deciding whether something was already covered. A test that proves
less than its name is worse than a missing test, because it answers the question
"is this checked?" with a confident yes.

  ✔ [closed: the sweep ran and came back clean] `None`-means-everything has now bitten twice in one session (leafwing's unset
  gamepad, both branches of `assign_local_seat_devices`). Worth a look at whether
  any other Option in the input path reads as "nothing" and means "anything".

✔ **the `None`-means-everything sweep came back clean, which is itself the
result.** Three sites in the input path treat an absent resource as permissive —
`smash/lib.rs:878` and two in `touch_input/bevy_plugin.rs` — and all three are
`Option<Res<SeatInputContexts>>` with the reason written next to them (*"a test
that wires no contexts is testing the screen, not the arbitration"*, *"composes
into apps that never install the participant-context resolver"*). Deliberate and
documented is a different thing from silent.

⚠ the two that bit were both `InputMap`'s gamepad association, where `None` is
leafwing's spelling for *whichever pad is found first*. That is a foreign API's
default, not a pattern in this repo — worth knowing, because the instinct after
being bitten twice is to distrust every `Option` in sight.

## S46 — the rollback harness has no input participants, so a device-level test there is vacuous (2026-08-01)

⛔ **Wrote a disconnect-under-rollback test, and it passed while proving nothing.**
It spawned a `Gamepad`, seated a two-human roster in a sync-test session,
despawned the pad mid-match and asserted the seats and the checksum survived. It
was green.

A probe at the moment of the unplug:

```text
[seat-probe] []
```

There are **no `InputParticipant` entities in the sim harness at all**. The pad
was never associated with anything, so the "disconnect" removed a controller
nobody was holding, and the test would have passed with the unplug deleted. It
was not committed.

⚠ **the reason is structural, not an oversight**: `seat_input_participants_for_roster`
and `assign_local_seat_devices` live in the HOST plugin's `Update` schedule, and
`Platformer2dSimHarness` is a sim-only composition. The device layer and the
rollback harness genuinely do not overlap — one is about which physical thing
authors a frame, the other about what happens to frames once authored.

⚠ **and this is why `two_local_seats_drive_independently_under_a_rollback_host` is
still sound**: it drives seat two through `drive_seat`, which writes the seat
LATCH — the rollback's own input path — and never touches an `InputMap`. It tests
the half that exists in that fixture. The disconnect test was reaching for the
half that does not.

  ✔ **BUILT 2026-08-01 — `game/ambition_app/tests/rollback_seat_devices.rs`.**
  The fixture that has BOTH. `Platformer2dSimHarness::app_mut()` is the new seam
  (systems, not just state: `world_mut()` could never have installed a schedule),
  and the test composes the host's four seating systems — primary spawn, freeze,
  seat, and the `PreUpdate` device assignment — onto a two-player sync test, in
  the host's own order. Two tests: two pads own two seats and keep them across
  every rewind; unplugging one leaves the other seat alone and the reconnect
  comes home.

  ⛔ **and it immediately found a live bug in the shipping path.**
  `assign_local_seat_devices` skipped its claim pass entirely for a FROZEN
  session — reasonable-looking, since the freeze decides the mapping. But
  claiming is also what RECORDS which physical controller a seat holds, so
  `remembered` stayed empty and the identity pass added for GPT 5.6's finding #1
  had nothing to match on. Unplug a pad mid-match, plug it back in, and its seat
  keeps pointing at the DEAD entity: not a swap, that player is deaf for the rest
  of the match.
  ⚠ **this was not an edge case, it was the only case.** Before a roster exists
  there is one participant, `players < 2` returns early, and no claim is ever
  made; then the match freezes. Ownership is empty at every real freeze, so
  identity-based reconnect had never once worked in a real session.
  → fixed by recording the identity of the pad the SESSION already chose
  (`topology.device_for_handle`), which repairs without letting the freeze be
  reordered.

  ✔ **and the claim below is now PROVEN.** `SeatDeviceOwnership` and
  `LocalDeviceOrder` are NOT rollback state: neither is registered, and the
  fixture runs a sync test — every frame saved, rewound, resimulated — with
  ownership stable across all of it. Which device authors a seat's frame is an
  input to the rollback, not a fact inside it; a disconnect takes away somebody's
  INPUT, not their character.

  ⚠ two things worth carrying forward, both found by building this:
  * seats are created in `Update` and assigned in `PreUpdate` (the host's own
    split), so on the frame a seat first exists NO assignment pass has seen it
    and every `InputMap` reads `None`. The first version of the test asserted
    there and "failed" against a topology that had already frozen correctly —
    it was measuring the gap between two schedules.
  * `cargo test -p ambition_input` runs **none** of the device-ownership tests.
    The module is `#[cfg(feature = "input")]`, so the per-crate invocation
    reports 55 passing while compiling out 29. `--workspace` unifies the feature
    on through `desktop_dev → visible → input`, so CI is fine — but 55 is the
    number a person sees while iterating, and it is silently partial.

✔ **and the sibling globals have a ratchet now.** `ActiveMatch` and
`DeclaredCombatRules` carry no `published_by` and are safe only because exactly
one writer touches each — the roster's exact position before Smash's select
screen published one from a different route.
`a-second-writer-of-a-match-global-must-answer-ownership` confines them to the
systems that own them.

⚠ it deliberately does NOT add an owner field. Adding one to two types with a
single writer each is a mechanism for a problem nobody has, and the eventual
owner's shape should be decided by the second writer's actual needs. The contract
makes that writer visible at the moment it appears, which is when the question
has to be answered rather than months later from a photograph.

✔ **the two-human re-seat across a rewind is checked.**
`two_local_seats_drive_independently_under_a_rollback_host` now also asserts, on
every tick, that the seat set is the same bodies in the same SLOTS after
resimulation — the two-human form of *"rewinding around the first active frame
reconstructs the identical roster"*. Probed by corrupting the expected set.

⚠ the slots matter, not just the count: a resim that rebuilt two seats both
numbered 0, or swapped which body wore which, keeps the count and loses the
match, and "two bodies exist" is exactly the assertion that misses it.

⚠ **and a process note that cost real work twice today**: `git checkout -- <file>`
to undo a probe DELETES the uncommitted change being probed. Commit the real
change first, or restore from a copy. Both times the injected break and the new
code went together, and the second time only the restore-check caught it.

## S49 — ▢ a BODY-LOCAL vector copied BETWEEN two bodies (2026-08-01)

The `locomotion` half of the S48 audit: 25 non-test assignments, and 24 are
sound. `player.rs` writes `local_axis` into a local field; `smash/emit.rs` uses
`side_face_toward_target()`, a local side sign; `state_machine` routes through
`snapshot.locomotion_for(..)`, whose parameter is *named*
`desired_local_velocity`; the fighter brain's five were fixed in S48.

⚠ **the twenty-fifth is `mount/mod.rs::steer_mount_from_rider`:**

```rust
mount_frame.locomotion = rider_frame.locomotion;         // BODY-LOCAL
mount_frame.velocity_target = rider_frame.velocity_target; // world — safe
```

`velocity_target` is world, so copying it between bodies is frame-safe.
`locomotion` is **controlled-body-local**, so copying it is only valid when the
two bodies resolve the SAME local frame.

✔ **normally they do**, and this took tracing to establish rather than assume:
`sync_riders_to_mounts` sets `rider.surface.gravity_scale = 0.0`, which zeroes
the MAGNITUDE and not the direction, so the rider keeps the room's gravity
direction — the same one the mount is in. `control_down` comes from the body's
own `resolved_frame.down()`, so both resolve equal. The copy is correct and the
reason is non-obvious enough to be worth writing down.

⛔ **the exception is named in the engine's own words.** From
`actors/update.rs`: *"a surface-walker's frame is its clung surface; everyone
else's is gravity at their position."* A surface-walking MOUNT therefore
resolves its frame from the surface it is clung to while its rider resolves from
gravity — and this line then hands the mount a vector authored in a different
frame. Its `locomotion.x` would drive the mount along the rider's side axis, not
its own.

✔ **CHECKED, and it is NOT reachable today.** Both preconditions were named and
both are now answered:
* `ControlGrant::Total` — **always**. `attach_mount_role` hard-codes
  `control_grant: ControlGrant::Total` for every archetype carrying a
  `mount_class`, so there is no configuration in which a mount does not take
  its rider's intent.
* a surface-walking mount — **none authored**. `character_archetypes.ron` has
  exactly two `mount_class` archetypes: the **shark** (`is_aerial: true`, so it
  steers by `velocity_target`, which is WORLD and frame-safe, and whose
  `locomotion` the smash brain zeroes) and the **giant** (*"no motion / no AI —
  the carried giant just stands"*).

→ so the action is the note, not the fix, and it is now AT THE LINE: why the
copy is sound (the rider keeps the room's gravity direction because
`sync_riders_to_mounts` zeroes the SCALE, not the direction), what would break
it (a crawler/adhesive mount), and what the fix would be then (convert through
each body's own basis rather than copy).

⚠ **the precondition is unenforced, and a new mount archetype is the trigger.**
Nothing fails if somebody authors `mount_class` on a surface-walker; the mount
simply drives along its rider's side axis. That is a good candidate for an
absence contract if a third mount ever appears — two is too few to be worth a
guard, and the note is where the next author will be standing.

⭐ **the general shape is worth naming**: a body-local quantity is only meaningful
with its body. Every OTHER cross-body hand-off in this file already uses world
quantities (position, the saddle offset rotated through `mount_frame.basis()`),
which is why this one line is the only place the question arises.

## S50 — ⛔ five first-draft instruments, all green through their own bug (2026-08-01)

Not five mistakes. One mistake, five times, in a single day of building guards —
and in every case the PROBE was the only thing that caught it.

1. **the tofu guard** swept for `TextFont {` literals, so it could not see the
   defect that has no literal: a `Text` with no `TextFont` at all, which is what
   the menu's row labels and tab labels had.
2. **`check_capability_ships.py`** matched `insert_resource(TypeName …)`. Its
   motivating writer builds a local and passes the binding
   (`commands.insert_resource(topology)`), so `LocalSeatTopology` came back with
   NO writer and fell into the "written nowhere, not our question" branch.
3. **the launcher-rebuild test** asserted after one `app.update()`. The command
   is consumed and the cursor moves that frame, but `render_basic_shell` has
   already computed its key — the rebuild lands on the NEXT frame, so restoring
   the bug still passed.
4. **the frozen-seat unit test** let an UNFROZEN claim pass run before freezing,
   which the shipping path never does, so `remembered` was populated either way.
5. **the rollback-baseline path guard** checked that the named crate still
   EXISTS. `ambition_platformer2d_core` was never deleted — `CenteredAabb` moved
   OUT of it into another live crate.

⚠ **the common cause is writing the guard from the MEMORY of the defect rather
than from the defect.** Each of those five is a faithful one-sentence
description of its bug and none is the bug at the level of code. "A `Text` with
no font tofus" does not tell you the failing site had no `TextFont` token to
grep for; "the topology is only written in dev code" does not tell you the write
is `insert_resource(local_binding)`.

→ **write the PROBE first.** Restore the real defect, watch the new check go red,
then believe the check. If restoring it is awkward, that awkwardness IS the
signal — there is currently no way to demonstrate the claim about to be made.

⚠ and a sixth of the same family, one level up: the geometry carve broke the
rollback schema baseline and **nothing caught it for four commits**, because
`cargo check --all-targets` compiles tests without running them and the only
authority is one integration test behind a full app build. Guard #5 above exists
for that gap and is deliberately weaker than the real test — see its docstring
for what it cannot see.


## S34 — ⛔ TWO authorities decide how many people are playing (2026-08-01)

Found while working the couch-multiplayer brief. This is the reason milestones 1
and 2 cannot be reached by adding a policy, and it is an authority conflict
rather than a missing feature.

**Rule A — the roster decides.** `seat_input_participants_for_roster`
(`crates/ambition_platformer2d_actor_monolith/src/schedule/input_systems.rs:100`)
states it outright:

> The roster is the authority on who is playing, so it is the authority on how
> many seats exist. Deriving seats from connected HARDWARE instead would mean a
> controller left plugged into a machine silently becomes a second player in
> every game on it.

**Rule B — the hardware decides.** `LocalSeatTopology::players()`
(`crates/ambition_input/src/local_seats.rs:125`) is `self.seats.len().max(1)`,
and `seats` is captured from `LocalDeviceOrder` — **the connected gamepads**. So
the count is derived from hardware, which is the exact thing rule A forbids.

And rule B is the one that reaches the session: `versus.rs:359` builds the roster
with `versus_roster_from(topology.players(), …)`, and the GGRS session sizes
itself from the same topology.

⚠ **it is wrong in both directions, which is why neither symptom alone found it:**

* a spare controller on the desk inflates the roster — precisely the failure rule
  A was written to prevent, live in the code that rule A's own layer feeds;
* a keyboard player plus one pad player DEFLATES to one player, because the
  keyboard is not a device row. That is Jon's milestones 1, 2 and 5 blocked, and
  no assignment policy can fix it from above — the count is already wrong before
  any policy is consulted.

### The resolution

`LocalSeatTopology` should freeze the **seat → source assignment for the seats
the roster declared**, not derive a seat count from devices:

* `players()` comes from the DECLARED seat count (rule A), not `seats.len()`;
* the frozen map is `seat → InputSourceId` (`sources.rs`), so a seat can be held
  by the keyboard, by a pad, or by nothing;
* `device_for_handle` keeps returning `Option<Entity>` — `None` for a keyboard
  seat, which is already the shape callers handle for an empty seat.

⚠ **this must not change solo play.** Under `UnifiedPrimary` the keyboard and
every pad collapse onto the primary participant, so one declared seat stays one
player however much hardware is attached — milestone 8. The keyboard becomes a
distinct source only when a seat CLAIMS it.

⚠ **and it touches GGRS session sizing**, which is why it is its own slice and not
a rider on the device-assignment fix already landed. `LocalSeatTopology` is
rollback-adjacent: the session freezes it once and every reconstruction — startup,
hot reload, proof-pulse restore — reads the frozen value. Changing what it freezes
changes what those three agree about.

Prerequisite already in place: `crates/ambition_input/src/sources.rs`
(`InputSourceId`, `InputAssignmentPolicy`, `keyboard_owner_for`), landed with the
pad-distribution fix and defaulting to today's behaviour.

---

## S36 — why couch multiplayer cannot be checked (2026-08-01, Jon's report)

Jon: *"even when we add a CPU player in smash there is only ever one player that
shows up in game, that will make it very hard to check that couch multiplayer is
working right."*

**Measured, not guessed:**

✔ **the seating is fine.** `a_two_participant_roster_actually_seats_two_bodies`
runs the real select flow, adds a CPU, starts the match and counts `MatchSeat`
entities: TWO bodies are seated. Every previous test in that file stopped at the
route and the session, so a roster of two putting one fighter on stage passed the
whole suite.

⛔ **the two fighters are built by different paths.** The census the test prints:
both carry 84 components and the SETS DIFFER — seat 0 is player-bodied
(`PlayerVisual`, `BodyPoseView`, `PresentedPose`, `Transform`, `GlobalTransform`),
seat 1 is actor-bodied (`ActorIdentity`, `Perception`, `RoomVisual`,
`RuntimeStagedActor`) with no transform and no pose view. That is the two-port
body fighter-unification exists to remove, surfacing as a user-visible symptom.

⚠ **and a headless test CANNOT settle whether seat 1 draws.** `BodyPoseView` is
the player-bodied read model; an actor-bodied fighter draws through the id-keyed
`ActorAnimIndex`, a resource rebuilt in the RENDER presentation plugin and
explicitly *"NOT the sim schedule … so a headless / RL build never pays for poses
it won't draw"*. The first draft of the test asserted seat 1 must carry a
`BodyPoseView` — a confident measurement of the wrong port, caught before it
shipped.

### ⛔ The instrument gap, which is the actual blocker

`capture_scene --route smash_gameplay` photographs the stage with **one fighter**
— correctly, because navigating straight to a route decides no roster. There is
no way to photograph a DECIDED match: `capture_scene` navigates, it cannot drive
the select screen to add a CPU and lock in.

So the only state anybody wants to look at — two fighters on a stage — is
reachable by neither the headless suite (blind to rendering by construction) nor
the capture tool (cannot reach the state). That is precisely Jon's "very hard to
check", and it is an instrument problem before it is a gameplay one.

  ✔ [closed: S36 — `capture_scene --press`] **give `capture_scene` a way to reach a decided match**: a `--smash-cpu N`
  style flag, or more generally a way to drive a route's own lobby before the
  shutter. The select screen already has headless drivers in
  `smash_in_the_host.rs` (`add_cpu`, `confirm`); the capture binary needs the
  same entry, not a new mechanism.
  ✔ [closed: S36 — photographed; both fighters draw once the roster survives] then photograph it, and the question "does the CPU fighter draw" answers
  itself in one image.

✔ **S36 instrument gap CLOSED, and the bug is now photographed.**
`capture_scene --press Down,Enter,Enter` drives a route's own lobby before the
shutter, reusing the same key taps the smash tests already use rather than adding
a second mechanism. One command reproduces Jon's report:

```bash
cargo run -p ambition_app --bin capture_scene -- \
    --route smash_select --press Down,Enter,Enter out.png 1600x900
```

The screen decides a real two-fighter match (`P1 — smash_duelist_a READY`,
`P2 — CPU · smash_duelist_b READY`), the stage loads, and **one** fighter stands
on the platform.

⚠ the first run photographed the select screen mid-"Starting…" — a confirmation
STARTS a route change, so the shutter caught the moment the presses were accepted
rather than the state they asked for. `POST_PRESS_SETTLE_FRAMES` fixed it, and
the distinction is the same one the route-camera grace draws.

  ✔ [closed: it was the roster clobber, not the draw path] **next, and now cheap**: the CPU fighter is seated and not drawn. It is
  actor-bodied, so it draws through the id-keyed `ActorAnimIndex` rebuilt in the
  render presentation plugin. Check whether that index has a row for the seated
  CPU's id — if it does not, the actor visual was never joined; if it does, the
  visual exists and is mispositioned (seat 1 carries no `Transform`). One
  instrumented capture run answers which.

⚠ **and an unrelated observation from the same photograph, worth recording**: the
select screen renders `·` (U+00B7) and `—` (U+2014) CORRECTLY — "P2 — CPU ·
smash_duelist_b". The menu-text tofu (Z1, nine dead hypotheses) is therefore NOT
repo-wide text rendering; this screen reaches glyphs `ambition_menu` cannot. That
is a live discriminator the tofu row did not have.

### ⚠ CORRECTION (same day): the photograph does NOT reproduce Jon's case

I wrote that `--press Down,Enter,Enter` reproduces "add a CPU and only one player
shows up". It does not, and a probe caught it before it became received wisdom.

Instrumenting the shutter frame in the capture binary:

```text
[stage-probe] seated: []
[stage-probe] feature visuals: 0 -> []
[stage-probe] roster=None active_match=false frames=245
```

**No `MatchSeat` entities at all, no roster, no active match** — and yet the
photograph shows a fighter on the platform. That fighter is the ordinary primary
player body, not a seated one. So the capture reaches the smash STAGE with no
match behind it; the image is a true photograph of a state, but not of the state
Jon described.

⛔ **and the divergence is the finding.** The same key sequence, through
`shell_host_app()` in `smash_in_the_host.rs`, seats TWO bodies — that is the
census test, and it passes. Through the `capture_scene` composition the roster is
gone by the time the stage is up. Two compositions of the same engine reach
different states from identical input, which is the composition-divergence class
this repo has been bitten by three times before.

  ✔ [closed: neither — the reconciler transferred ownership of the roster] **chase that first**: does the capture binary lack the plugin that publishes
  or activates the roster, or does the roster get published and then dropped on
  the route change? `active_match=false` with `roster=None` says activation never
  ran; whether that is a missing registration or a cleared resource is one more
  probe.
  ✔ [closed: superseded: the two-fighter stage is photographed] ⚠ **and it means the headless test is the ONLY thing currently proving the
  two-fighter seating**, in one composition. Until the capture path agrees, "two
  bodies are seated" is a fact about `shell_host_app`, not about the game.

The `--press` mechanism itself is sound and stays — it drove the select screen
correctly (`P1 READY`, `P2 CPU READY`, `Starting…`) and is what made this
measurable at all.

⚠ **and one candidate is already eliminated.** I assumed the capture binary
simply lacked the host composition — the plugin list visible at the top of
`capture_scene.rs` builds `AmbitionGameSimulationPlugin` and friends directly,
with no `compose_ambition_shell_host` in sight. **That list is the ROOM-mode
builder.** Route mode goes through
`build_visible_app(VisibleRenderMode::OffscreenGpu, /* shell_hosted */ true)`,
which does call `compose_ambition_shell_host` and `install_ambition_shell_visuals`.

So both compositions ARE shell-hosted, and the divergence is narrower and more
interesting than "a missing plugin": the same host, driven by the same key taps,
publishes a roster under `shell_host_app()` and not under the capture app. The
remaining differences worth probing, in order — the render mode
(`OffscreenGpu` vs none), `TimeUpdateStrategy::ManualDuration`, and whether 150
settle frames is simply too few for select → roster → activation when the tick is
manual.

⚠ that last one is the cheap check and should be first: raise
`POST_PRESS_SETTLE_FRAMES` and see whether the roster appears. A timing shortfall
and a composition defect look identical from one photograph, which is the whole
reason this row exists.

⚠ **the cheap check ran, and it did not answer the question — it found a second
limit instead.** Raising `POST_PRESS_SETTLE_FRAMES` to 600 makes the capture fail
outright:

```text
capture_scene: timed out waiting for texture readback after 691 frames
               (warmup 90 + grace 600)
```

The readback deadline is computed from the same budget the settle spends, so the
settle cannot be raised past it without the tool giving up before the shutter.
Reverted to 150.

  ✔ [closed: the press settle waits on readiness now] **so the settle has to wait on a CONDITION, not a frame count**: the route
  the presses asked for having become active, and — for a lobby that starts a
  match — the match having activated. `capture_scene` already draws exactly this
  distinction for the other end of the capture, in its own words: *"warmup is a
  duration, readiness is a fact"*. The press settle is currently a duration and
  should be a fact, and the readback deadline should extend while it waits rather
  than counting against it.

That is a small, contained change to the tool, and it is the thing standing
between Jon and a photograph of a two-fighter stage.

✔ **timing ELIMINATED** (2026-08-01). The press settle now waits on readiness
rather than a frame count — the presses end one capture and begin another, so the
route's own warmup and readback deadline apply. The photograph is unchanged: one
fighter, subject at the primary player's position, resolved after the route's own
90-tick warmup. The missing match survives a correct settle.

  ✔ [closed: explained: Versus rebuilt Smash's roster as its own] **what is left to explain the divergence**, now that timing is out: the
  capture app runs `VisibleRenderMode::OffscreenGpu` with
  `TimeUpdateStrategy::ManualDuration`, and `shell_host_app()` runs
  `MinimalPlugins` with real time. A match activation that depends on a
  `FixedUpdate`/tick cadence would behave differently under manual duration, and
  that is the first thing to check — it is one probe (does the select screen's
  confirmation reach the activation systems at all under the capture's clock).

### ⚠ the divergence is REAL and reproducible; timing and entry route are both out

Two entry paths (`--route smash_select`, and through the launcher with a `wait`
between screens), two settle strategies (fixed 150 frames, then readiness-based):
**every run reaches the stage with one fighter.** The probe at the shutter says
`roster=None active_match=false`, and no `MatchSeat` entities exist.

Eliminated, each by measurement rather than reasoning:

* ⊘ **settle timing** — readiness-based settle gives the same image, and raising
  the fixed count instead breaks the readback deadline.
* ⊘ **entry route** — going through the launcher, the way the passing test does,
  changes nothing.
* ⊘ **the clock** — route mode does NOT set `TimeUpdateStrategy::ManualDuration`;
  that is the room-mode builder. Both compositions run on real time.
* ⊘ **a missing host composition** — route mode calls
  `build_visible_app(OffscreenGpu, shell_hosted = true)`, which composes the shell
  host and its visuals.

  ✔ [closed: explained: the roster clobber, not the composition] **what is left**: `VisibleRenderMode::OffscreenGpu`, and whatever else
  `build_visible_app` installs or omits that the test's explicit
  `MinimalPlugins + PlatformerHostPlugins + compose_ambition_shell_host` list does
  not. The next probe is a diff of those two compositions' registered systems —
  `schedule-census` already prints a total (623 systems in the capture run), so
  printing the same census from `shell_host_app()` and diffing the NAMES is the
  measurement, not another guess.

⚠ **and the test is the weaker instrument here, not the stronger one.** It passes
in a composition no player runs. Whatever the diff turns up, the fix belongs on
the side that makes the SHIPPED composition seat two — not on the side that makes
the test agree with the capture.

## ✔ S36 CLOSED — Versus was rebuilding Smash's roster as its own

Root cause, from a per-frame probe through a real capture:

```text
route=smash_select    roster=2  ← the screen decides two fighters
route=smash_gameplay  roster=2
[roster-remove] on_versus=false mine=true published_by=Some("ambition_versus")
route=smash_gameplay  roster=-  ← gone, nobody seated
```

`versus_roster_from` stamps `published_by: ambition_versus`, so
`reconcile_roster_with_frozen_topology` rebuilding Smash's roster did not resize
it — it **transferred ownership**. `maintain_versus_stage` then did the right
thing with a versus-owned roster on a non-versus route and deleted it.

⛔ **the guard already existed one function over** and the reconciler never
learned it. `maintain_versus_stage`'s own comment states the rule — *"MINE, not
'a roster exists'… the resource is global and had no owner until it did."* One
`is_published_by` check in the sibling that was missing it. Photographed before
and after: one fighter, then two.

⚠ **and this makes S35 urgent rather than tidy.** The bug only bites where a
topology is FROZEN, and today the only thing that freezes one is the rollback
observatory behind `dev_tools`. `shell_host_app()` freezes none, so the
reconciler returned early and the headless test counted two seats and passed —
**about a composition no player runs**. Fixing S35 (freezing the topology in a
shipped build, where it belongs) would have SHIPPED this bug. The regression test
therefore freezes a topology deliberately; one written without it would have been
green and worthless.

  ✔ [closed: `a-second-writer-of-a-match-global-must-answer-ownership`] **the general form**: `MatchParticipantRoster` is a global
  resource with an owner field, and ownership is enforced by convention at each
  site. Two sites had the check, one did not, and nothing names the rule in one
  place. A `published_by`-aware accessor — or an absence contract that every
  `remove_resource::<MatchParticipantRoster>()` and every roster REPLACEMENT sits
  behind an ownership test — turns the third omission into a build failure
  instead of a photograph.

## S37 — a goal check that could not fail (2026-08-01)

Found while probing the new roster contract. `.goal/active.json` ran
`python3 scripts/check_absence_contracts.py >/dev/null` — **without `--check`** —
and the script returns `1 if args.check else 0`. So it printed `RED` for a
violated contract and exited 0, and *"S1 slice H: the absence contracts and the
footprint ratchet are green"* has been reported satisfied regardless of whether
they were.

Probed both ways with a real violation present: exit 0 without the flag, exit 1
with it. Repaired.

⚠ **the contracts themselves were never unguarded** — `scripts/tests` enforces
them through pytest, which is what the suite runs. Nothing rotted. What was
broken is narrower and worth naming exactly: the GOAL was reading an instrument
wired to always agree, so one of its twelve green rows carried no information.

  ✔ [closed: S37 — all twelve audited, two more repaired] **the class is worth a sweep, and it is cheap**: every `.goal` check is a
  shell command whose exit code is the whole signal. A command that cannot return
  non-zero — a script with an optional strictness flag, a `| head`, a `grep` whose
  failure is swallowed, a `; true` — is a row that is green by construction. The
  other eleven have not been audited this way. ⚠ note two already-known traps
  apply directly: `| head` closing a pipe under `pipefail` reports SIGPIPE
  (`reference_goal_check_sigpipe_footgun`), and a `pkill -f` matches its own
  shell.

✔ **S37 sweep done — all twelve audited, two more repaired.**

The `|| true` in six checks is CORRECT and not the smell it looks like: the
output is captured and a trailing `grep -E 'test result: ok\. [1-9][0-9]* passed'`
is what decides, so a compile error produces no matching line and the check
fails. The `[1-9]` also rejects `ok. 0 passed`, which is what a filter matching
nothing prints.

⛔ **but two checks ran TWO cargo commands and grepped the concatenation** (S2
match activation, S4 damage percent). A single `ok. N passed` from EITHER half
satisfied the grep, so the other half could fail to compile entirely and the row
would stay green — partial credit, in a check whose whole job is to say both
halves hold. Both now also require the absence of `test result: FAILED` and of a
leading `error[`/`error:`. Probed: an injected `FAILED` line flips the verdict.

Clean afterwards: checks 1 (inverted ledger), 2 (`cargo check`), 3 (`test -z`),
4 (`git grep -q && cargo check`), 7, 9, 11, 12 all fail properly — each ends in a
command whose exit code is the signal, with no swallowing.

⚠ **the general rule this leaves**, worth applying to any future goal: a check is
a shell exit code, so the LAST command must be the one that can say no. Capturing
output and grepping it is fine — grepping a pile assembled from several commands
is not, because the pile cannot say which half spoke.

✔ **and the sibling globals are clean — audited, not assumed.** The same clobber
shape was checked on the three other resources a second experience can write:

* `ActiveMatch` — two production removals, both in `versus.rs`, both inside
  route-gated arms (`(true, false)` clears it because a new roster is not yet a
  match; `(false, true)` is the teardown, now ownership-gated).
* `DeclaredCombatRules` — one removal, in the same ownership-gated teardown.
* `DeclaredInputSeats` — no production removals at all.

⚠ **but the asymmetry is the finding.** `MatchParticipantRoster` has a
`published_by` field, so an ownership question can be ASKED. `ActiveMatch` and
`DeclaredCombatRules` do not — they are safe today only because exactly one
experience touches them. The moment a second one does, there is no check to
write, and the failure will look like this one did: a match that quietly belongs
to nobody.

  ⚠ that is the argument for making the owner part of the resource rather than a
  convention per resource. Not urgent — one writer each — but it is the same
  design question the roster answered once and the others have not been asked.

✔ **both sides of the reconciler guard photographed.** Smash now seats two
(`Duelist B` on the stage beside the player); Versus is unchanged and correct —
two fighters, `Long Guard 60/60` / `Close Guard 52/52`, `ROUNDS blue 0 - red 0
(first to 2)`, `FIGHT`. A guard that fixed one game by breaking the other would
have looked identical in the Smash photograph alone.

## S38 — Ambition's HUD is drawn over a Smash match (2026-08-01)

Seen in the same photographs, and it is not a Smash HUD at all:

    smash_gameplay   →  HP 100/100 · MP 100 · $0        ← Ambition's adventure HUD
    versus_gameplay  →  Long Guard 60/60 · Close Guard 52/52 · ROUNDS · FIGHT

Smash is a PERCENT-and-STOCKS game — S4's own row says *"the damage meter can
EXCEED the pool (percent is not health)"* — so a health bar, a mana bar and a
money counter are three readouts that describe a different game. Versus, which is
the same fighting stack, has the right ones. The touch bezel differs too: the
Smash stage offers Blink / Fly Toggle / Ranged / Bubble Shield, which are
Ambition's protagonist verbs, not a fighter's.

⚠ **this is the app-only-presentation class again** — the HUD a route gets is
whatever the host installed, and nothing asks whether the ROUTE wanted it. The
declared-HUD seam exists for exactly this
([`project_declared_hud_seam_2026_07_21`]) and Smash is not using it, or is being
overwritten after it does.

  ✔ [closed: S38 — a MISSING declaration, not precedence] check which: does `smash_gameplay` declare a HUD that Ambition's overwrites,
  or declare none and inherit? One is a precedence bug and the other is a missing
  declaration, and the photograph cannot tell them apart.

✔ **S38 HUD half CLOSED.** Smash declares its own readouts now — `Duelist A 0% ·
3/3`, one gauge per fighter, seat-coloured — and Ambition's health/mana/money is
gone. The answer to the row's open question was **a missing declaration**, not a
precedence bug: Smash called `.with_hud(..)` nowhere at all.

⚠ **the sharpest part is what was already there.** `damage_percent()` is
deliberately unclamped with a test called
`damage_percent_is_unclamped_so_a_hud_can_print_188`, and `FighterStocks` keeps
`started_with` *"so a HUD can draw '2 of 3'"*. Two APIs were built FOR this HUD
and nothing ever consumed them — which is exactly why nothing was red. Every
piece worked; none was wired to a screen.

  ✔ [closed: S38 — it was the ability set, and the bezel was reporting it correctly] **the touch bezel is still Ambition's** — Blink / Fly Toggle / Ranged /
  Bubble Shield on a platform fighter, where Jab / Jump / Dash / Shield belong.
  Same class as the HUD (an inherited declaration nobody overrode) and probably
  the same one-line shape, but it is a different seam and is not assumed to be.
  ⚠ note the versus photograph shows a SHORTER bezel (Interact / Jab / Jump), so
  something already differs per route — find what decides it before adding a
  third mechanism.

### ⚠ CORRECTION: the bezel is a SYMPTOM, not a second declaration

The row above guessed the touch bezel was "an inherited declaration nobody
overrode, probably the same one-line shape" as the HUD. It is not, and the
mechanism says something more interesting.

Touch buttons are not declared per route at all. `touch_action_layout()` is ONE
fixed ten-button array, and visibility is decided per button by
`touch_action_available(action, &prompt)` where `prompt: Res<ControlPrompt>` —
the read-model of **what the CONTROLLED SUBJECT can do**. The versus stage shows
three buttons because its fighter can do three things. The prompt is working
exactly as designed.

⛔ **so the smash stage offering Blink / Fly Toggle / Ranged / Bubble Shield is
the prompt correctly reporting that seat 0 can do those things** — because seat 0
on that stage is Ambition's protagonist body, adopted, carrying Ambition's
ability set. The seat census in
`a_two_participant_roster_actually_seats_two_bodies` already showed it and I read
it as a rendering split: seat 0 carries `PrimaryPlayer`, `LocalPlayer`,
`PlayerEntity`; seat 1 is a constructed actor. The HUD calls it `Duelist A`
because the ROSTER says so; the BODY never became one.

  ✔ [closed: the ability levelling commit] **the real row: an adopted seat keeps the adopting body's abilities.** Player
  one fights as the exploration protagonist — blink, flight, projectiles, bubble
  shield — while player two is a duelist with a jab. That is not a couch-play
  balance question, it is two different games on one stage, and it makes "is
  couch multiplayer working" unanswerable for a second time.
  ⚠ do NOT fix this by filtering the bezel. The bezel is the only honest thing in
  the picture — it is reporting the defect.

## ✔ S38 CLOSED — and the bezel correction was itself corrected

Final state, photographed: two fighters, `Duelist A 0% · 3/3` / `Duelist B 0% ·
3/3`, and a bezel of Attack / Dash / Jump / Interact / Ranged.

⚠ **I was wrong twice about the bezel and the measurements are what settled it.**

1. First guess: "a missing declaration like the HUD." Wrong — buttons are one
   fixed layout gated by `touch_action_available(action, &ControlPrompt)`.
2. Correction: "seat 0 is Ambition's protagonist body." Also wrong — measured,
   `worn=smash_duelist_a`. It wears the right character.
3. What was actually true: it wears the right character and carried the wrong
   VERBS. seat 0 had every ability in the game (fly, blink,
   blink_through_hard_walls, glide, swim); seat 1 had run, jump, double jump and
   attack. The bezel reported seat 0 honestly the entire time.

Seating already levels this, gated on `roster.fighter_abilities`, with a comment
describing the identical failure found on the VERSUS stage in July. Versus
declares a set; Smash declared nothing, so the levelling never ran.

⚠ **and both named ability sets were wrong, each caught by re-measuring after
declaring it.** `basic()` would have removed double jump and attack. `sane_subset()`
— which reads like a fighter's kit in its opening lines — also grants fly, blink,
wall climb and pogo, so declaring it made the two seats agree that they could
both FLY. The set is spelled out now: run, jump, double jump, fast fall, dash,
attack.

  ✔ [closed: S38 — they come from the MOVESET, not abilities] `Ranged (V)` and `Interact (F)` survive on the bezel and no ability grants
  them, so they come from another source (a moveset or technique). Small, and
  now the only thing on that bezel not explained.
  ✔ [closed: `an_adopted_seat_and_a_spawned_seat_agree_on_every_roster_declared_field`, scoped to what the MATCH declares because health/box/mass are per-character] the four divergences seating has had to unify one at a time — health, box,
  mass, and now abilities — are all the same shape: *an adopted body keeps what
  the session gave it*. A fifth is likely. The comment listing them is the best
  record; a test that asserts an adopted seat and a spawned seat agree on every
  declared identity field would be better.

✔ **and the last two bezel buttons are EXPLAINED, not defective.** `Ranged (V)`
and `Interact (F)` do not come from the ability set at all — an action scheme maps
`ControlSlot::Projectile` to the `"ranged"` MOVESET verb and
`ControlSlot::Interact` likewise (`action_scheme.rs:238`,
`combat_from_moveset`). So the bezel is showing what the duelists' moveset
authors, which is a character-authoring question ("should a duelist have a
projectile?") and not a leak from Ambition.

⚠ worth stating because the same button would have read as another inherited-verb
bug: the bezel has exactly TWO inputs — abilities (movement) and moveset verbs
(combat) — and only the first was wrong.

✔ **the rule the four divergences share is now a test.**
`an_adopted_seat_and_a_spawned_seat_agree_on_every_declared_field` runs the real
select flow and asserts that every seat agrees on what the ROSTER declared,
scoped deliberately to declared fields — per-character differences are the point
of a fighting game, and "both seats are identical" would have to be waived the
first time somebody authors an asymmetric pair. Probed by deleting the ability
declaration: fails naming the seats, passes when restored.

## S39 — couch multiplayer: milestones 1–2 are expressible now (2026-08-01)

Three pieces, landed in order, each with the default preserving solo play:

1. `sources.rs` — `InputSourceId`, `InputAssignmentPolicy`, `keyboard_owner_for`.
   States who owns the keyboard, which nothing could previously say.
2. `assign_local_seat_devices` — pads go to seats that NEED one, skipping the seat
   already holding the keyboard. Fixes "the only pad goes to the person typing".
3. `seats_offered_under(devices, policy)` — the keyboard COUNTS as a source under
   `JoinToClaim`, so keyboard + one pad is two players. The smash select screen
   declares that policy while it is up.

⚠ **`UnifiedPrimary` is the default throughout** and every pre-existing test
passes untouched. Milestone 8 — a solo player driving one character with keyboard
or pad — is not something the couch work is allowed to charge for, and each of
the three pieces was written so that installing it changes nothing until a lobby
asks.

  ✔ [closed: S41 — the milestone-5 test passes] **milestone 5 is NOT proven**: no test drives a real keyboard press and a real
  pad press into two different seats in one match. That needs a fixture holding
  both source kinds, and it is the honest end of this slice — everything above is
  the lobby being ABLE to seat two people, not evidence that two people's inputs
  stay apart.
  ✔ [closed: S42 — milestones 6 and 7] **milestones 6 and 7** (disconnect keeps the seat, reconnect restores the
  same participant) are untouched. The current pass CLEARS a seat's association
  when its pad vanishes, which is right for the DEVICE and says nothing yet about
  the participant keeping its seat and its actor.
  ✔ [closed: S35 CLOSED] ⚠ and S35 still stands underneath all of it: the frozen topology that would
  make a MATCH stop re-sampling devices is created only by a `dev_tools` tool.

### ⛔ S39 addendum — the fix reintroduced the bug, and the test agreed with it

Attempting milestone 5 (a keyboard player and a pad player driving different
fighters) measured something the couch work had supposedly removed:

    slot 1 menu_select_pressed=true     slot 0 menu_select_pressed=true

One South press on one pad, arriving at BOTH seats. `assign_local_seat_devices`
gave the keyboard-owning seat `None` — and in leafwing an unset gamepad means
*whichever pad this finds first*, which the top of that same module documents as
the couch bug. The commit that fixed the couch bug reintroduced it one branch
over.

⚠ **and the test passed throughout**, because it asserted
`assigned(seat_one) == None` — written from the code rather than from the
property. `None` is the spelling that means the OPPOSITE of "this seat owns no
pad". Now `Some(Entity::PLACEHOLDER)`, which is what leafwing's own fallback
resolves to when no gamepad exists, with the property on the line.

After: `slot 1 true, slot 0 false`.

  ✔ [closed: S41 — the policy expiring at the route change was the last bug] **and milestone 5 is still not reached, one layer further along.** With the
  isolation correct, the pad's `MenuSelect` reaches slot 1's `ActionState` and the
  smash select screen still shows `seats: LockedIn Empty Empty Empty` — seat 1
  never joins. So the gap is now between a SEAT's action state and the select
  screen's per-seat menu frame (`populate_seat_menu_frames` /
  `SeatMenuFrames`), not in device ownership. That is a much smaller search than
  it was this morning, and the exploratory test that found it was removed rather
  than committed red.
  ⚠ the probe worth keeping in mind: print `menu_select_pressed` per slot at the
  moment of the press. It distinguishes "the input never arrived" from "the
  screen ignored it" in one line, and those two look identical from a photograph.

## ✔ S40 — couch milestones 1 and 2 are REACHED (2026-08-01)

A keyboard player and a gamepad player now take different seats and start a match
together. Measured, same scenario each time (keyboard confirm ×2, pad South ×2,
one pad connected):

    before   seats: LockedIn Empty        0 bodies
    after    seats: LockedIn LockedIn     2 bodies

Four things had to be built or repaired, and three of them were bugs found by
measuring the previous fix rather than by reading it:

1. `seats_offered_under(devices, policy)` — the keyboard COUNTS as a source under
   `JoinToClaim`. It was never a row in `LocalDeviceOrder`, so it could only be
   assumed, in prose, next to arithmetic that could not see it.
2. pads go to the seats that NEED one, skipping the seat already holding the
   keyboard. Before: the only pad went to the person typing.
3. ⛔ the keyboard seat must be associated with `Entity::PLACEHOLDER`, NOT `None`
   — an unset gamepad in leafwing means *whichever pad this finds first*, so the
   fix for (2) reintroduced the couch bug and its test agreed with it, because
   the test asserted `== None`, written from the code instead of the property.
4. ⛔ `drive_the_select_screen` iterated the UNIFIED count while the lobby
   declared the couch one. Two counts of the same thing: seat 1 existed, owned
   the pad, had the confirm in its `ActionState`, and was never read.

### ▢ milestone 5 is NOT reached, and here is the number

With both fighters seated, the pad's DPadRight moved BOTH — **14.19px** on the
keyboard player's fighter against **11.44px** on the pad player's.

So MENU input is seat-isolated (`MenuSelect` reaches only slot 1) and GAMEPLAY
input is not. Two different paths; one fixed. The next attempt starts here rather
than at the beginning.

  ⚠ the likely shape, stated as a hypothesis and not as a finding: seat 0 is the
  PRIMARY player, whose movement travels the global `ControlFrame`
  (`populate_control_frame_from_actions`) rather than the per-seat
  `SlotControls` path seat 1 uses. If so the isolation has to happen where the
  frame is populated, not in the input map — and the `InputMap` association fix
  is necessary but cannot be sufficient.
  ⚠ the test is written and NOT committed. A red test in the suite is worse than
  a written-down failure with its numbers.

## ✔ S41 — couch milestone 5 PASSES (2026-08-01)

`a_keyboard_player_and_a_pad_player_drive_different_fighters` runs the real
select-screen flow, seats a keyboard player and a pad player, moves one and
measures that the other does not follow. Milestones **1, 2 and 5** are reached.

⛔ **The last bug was mine and it wore a convincing disguise.** The policy was
`if on_select { JoinToClaim } else { UnifiedPrimary }`, so the assignment the
lobby made was undone the instant the stage loaded:

    during the match   slot 1 move_right=true   slot 0 move_right=true
    on the select      MenuSelect reached slot 1 alone

That reads exactly like *"menu input is isolated, gameplay input is not — two
different paths and only one is fixed"*, and the previous row said so. It was ONE
path under two policies. The wrong conclusion was specific, mechanical and
testable, and would have sent the next attempt hunting a second isolation bug in
`populate_control_frame_from_actions` that does not exist.

Jon's brief had the answer in a sentence: *"Before the match starts, freeze:
participant, session seat, control channel, input sources."* An assignment that
expires when the lobby closes is the opposite of frozen.

**Five pieces, four of them bugs found by measuring the previous fix:**

1. the keyboard COUNTS as a source (`seats_offered_under`)
2. pads go to the seats that need one, skipping the keyboard's
3. ⛔ the keyboard seat needs `Entity::PLACEHOLDER`, not `None` — unset means
   *every* pad, and its test agreed with the code because it asserted `== None`
4. ⛔ the select screen iterated the unified count while its lobby declared the
   couch one
5. ⛔ the policy reverted on the route change into the match

⚠ **the instrument that resolved four of the five** is one line per slot at the
moment of the press — `move_right=` / `menu_select_pressed=` per participant. It
distinguishes *the input never arrived* from *the surface ignored it* from *both
seats got it*, and those are indistinguishable from a photograph or a body
position.

  ✔ [closed: S42] **milestones 6 and 7 are untouched**: a disconnect must not reorder
  participants or transfer ownership, and a reconnect must restore the same
  participant. Today `assign_local_seat_devices` CLEARS a seat's association when
  its pad vanishes — right for the DEVICE, and it says nothing about the
  participant keeping its seat and its actor. That is the next slice, and it is
  where `LocalSeatTopology` being frozen (S35) starts to matter for real.
  ✔ [closed: S43 — both asserted in the milestone-5 test] milestone 3 (stable session seats) and 4 (distinct controlled actors) are
  implied by the passing test but not asserted separately.

## ✔ S42 — couch milestones 6 and 7 (2026-08-01)

⛔ **Milestone 6 was violated and a test blessed the violation.** Two seats, two
pads, unplug player ONE's:

    before   seat one → pad A     seat two → pad B
    after    seat one → pad B     seat two → pad B

`LocalDeviceOrder` forgets a pad that leaves, so every later seat shifts down one
and positional assignment redistributes the room. Invisible with a single pad,
which is how milestone 5 was reached without meeting it.

⚠ **`an_unfrozen_session_still_follows_live_discovery` asserted exactly this** —
*"with no session owning the seating, the live order is the authority"* — while
the test DIRECTLY ABOVE IT forbids the same promotion in the frozen case and
names the consequence: *"silently hand seat one's confirmed GGRS inputs to seat
two's physical controller."* Two tests, one hazard, opposite verdicts, differing
only in whether a topology happened to be frozen — and the unfrozen branch is the
one every player runs (S35).

`SeatDeviceOwnership` makes the assignment a fact that is REMEMBERED rather than
a position that is RECOMPUTED: a seat keeps its pad while that pad exists, a pad
that leaves frees exactly its own seat, and a free pad is only taken by a seat
that has none.

✔ **milestone 7 falls out of the same record.** A reconnecting pad is a new
entity — Bevy moves the generation, so remembering "seat one had 3v0" would never
match again — and what restores the participant is that the seat which lost a pad
is the one still holding none.

**Couch status: milestones 1, 2, 5, 6, 7 pass; 8 (solo unchanged) is pinned by
every pre-existing test plus an explicit one. 3 and 4 are implied by the passing
milestone-5 test and not asserted separately.**

  ⚠ ⚠ **three of the last four bugs were in code written the same day to fix the
  previous one**, and each was found by a measurement rather than by rereading:
  `None` meaning every pad, the select screen counting seats differently from its
  own lobby, the policy expiring at the route change, and now positional
  reassignment on disconnect. The pattern is worth naming — a couch input fix
  that is not measured under TWO sources is not known to work, because every one
  of these was invisible with one.
  ✔ [closed: S35 CLOSED] S35 still stands: the topology freeze that would make a MATCH stop
  re-sampling devices is created only by a `dev_tools` tool. Everything above
  makes the unfrozen path correct, which lowers the urgency and does not remove
  the hole.

## ✔ S43 — every couch milestone is now asserted by a test (2026-08-01)

| # | milestone | where |
|---|---|---|
| 1 | keyboard joins one participant | `a_keyboard_player_and_a_pad_player_drive_different_fighters` |
| 2 | a gamepad joins a second | same |
| 3 | both receive stable session seats | same (seats 0 and 1) |
| 4 | both select distinct controlled actors | same (two bodies) |
| 5 | both produce independent control frames | same (move one, the other does not follow) |
| 6 | disconnecting does not transfer ownership | `unplugging_one_pad_does_not_hand_its_seat_the_other_players_pad` |
| 7 | reconnecting restores the same participant | `a_reconnecting_pad_comes_back_to_the_seat_that_lost_one` |
| 8 | single-participant Ambition unchanged | `without_a_declared_policy_the_pad_still_goes_to_seat_one` + every pre-existing seat test |

⚠ **milestone 4 is NOT "distinct characters", and the first draft asserted that
and failed.** Both players joined with the cursor at slot 0 and picked
`smash_duelist_a`. That is a mirror match, which every platform fighter allows —
a test banning it would have been a rule nobody asked for, enforced by a test,
discovered by a player. The note stayed in the test because the wrong reading is
the tempting one.

  ⚠ what the milestone list does NOT cover, stated so it is not mistaken for
  done: online play, a rebinding UI, every device backend, cross-build protocol
  compatibility (Jon's brief excludes all four), and — from this side —
  **anything about a rollback session**. Every one of these tests runs on the
  shell host with real time. A GGRS match with two local seats re-samples nothing
  today because `LocalSeatTopology` is never frozen outside `dev_tools` (S35),
  and the couch work has made the unfrozen path correct rather than removed the
  need to freeze.

✔ **the MODULES.md check is wired** (S31's open ▢). `modules_md.py` has had a
check mode the whole time and appeared in no runner. It is a `scripts/tests` case
now — that suite already runs first and cheaply in the backbone, and its own
comment gives the reason this was needed: *"a guard nobody executes is not a
guard."* Probed with a real new module.

⚠ **that is the third instrument this run that existed and was never called**:
`modules_md.py --check`, `check_absence_contracts.py --check` (the goal ran it
without the flag, so it could not fail), and the `.agent` index's prune (which
did not exist, but its absence had the same effect — a generated artifact nobody
verified). The pattern is worth a sweep of its own: **a repository that writes
its own tooling accumulates checks faster than it accumulates callers.**

  ✔ [closed: S44 — the sweep ran; four guards had no caller] sweep for the rest: every `scripts/*.py` with a `--check`/`--verify` mode or
  a non-zero exit path, cross-referenced against `run_tests.py`, `scripts/tests/`
  and `.github/workflows/test.yml`. Anything in the first list and none of the
  other three is a guard that has never run.

## ✔ S44 — the uncalled-guard sweep (2026-08-01)

Cross-referenced every `scripts/*.py` with a non-zero exit path against
`run_tests.py`, `scripts/tests/` and `.github/workflows/test.yml`.

**Four checks existed and nothing called them:**

| guard | state |
|---|---|
| `modules_md.py` (check mode) | ✔ wired — found 19 stale maps + 3 missing |
| `check_absence_contracts.py` | ✔ the GOAL called it without `--check`, so it could not fail |
| `check_doc_links.py` | ✔ wired — I had been running it by hand all run |
| `check_roadmap_evidence.py` | ✔ wired |

⚠ **`check_doc_links.py` is the instructive one.** It was green the entire time
*because a person was running it manually after every doc edit*. That is exactly
how a check with no caller stays green until the person stops — and the greenness
is evidence about the person, not about the repository.

Deliberately NOT wired, because they are generators or one-shots rather than
guards: `audit_git_media_history.py`, `ecs_inventory.py`, `non_ecs_inventory.py`,
`regen_music_registry.py`, `render_line_profiles.py`. Named so the next sweep
does not re-derive the distinction.

  ✔ [closed: S44 — `test_every_guard_has_a_caller`] **the pattern deserves its own guard.** Everything above was found by a
  five-line shell loop; a repository that writes its own tooling accumulates
  checks faster than callers, and this will recur. A `scripts/tests` case that
  fails when a `check_*.py` exists with no caller would close it permanently —
  ⚠ and needs the generator/guard distinction above encoded, or it fires on
  `ecs_inventory.py` forever and gets waived.

✔ **and the class is closed.** `test_every_guard_has_a_caller` fails when a
`scripts/check_*.py` is named by no runner. The `check_` prefix is the rule
deliberately — the first draft used "any non-zero exit path" and instantly needed
carve-outs for the generators, an allowlist that would need maintaining and would
be waived the first time it got in the way. Probed with a fresh orphan guard.

