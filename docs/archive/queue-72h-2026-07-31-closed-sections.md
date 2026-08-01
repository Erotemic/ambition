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
decomposition trigger that never fired. Depending on `ambition` links 41 crates,
19 of which a movement-only game never asked for.

A first cut exists **unmerged and unverified**: branch
`worktree-agent-af39b56fa4add8fc2`, commit `26237cb3f`. 18 of 41 edges became
implicit crate features with `default = ["all_capabilities"]` preserving today's
facade exactly; `fixtures/minimal_game`'s measured closure moved **41 → 38**;
the footprint ratchet was reworked to measure the sentinel's RESOLVED closure
via `cargo tree --locked` (the old static walk counted optional edges regardless
of features, so it could never have moved).

Three verifications were still compiling at wrap-up and were never run:

1. `cargo check -p ambition` (default features),
2. `cargo test` in `fixtures/minimal_game` AND `fixtures/external_consumer`,
3. the red-probe of the new art-without-render refusal.

→ run those three, then merge or report the red. **A red result is a finding,
not a failure** — it is the first thing this slice has said that nobody knew.

⚠ the honest residue is already recorded on the branch: `render` is optional in
the facade but NOT for `minimal_game` (its windowed boot is a slice-B exit
criterion), `audio` stays unconditional, and the other 14 unwanted crates remain
linked because **`ambition_actors` brings them** — the §4 carve condition,
exactly as the baseline predicted. Do not cut edges until the number looks good;
that failure mode is named in the slice's own exit criteria.

**Guard:** the facade compiles with `--no-default-features`, and
`check_absence_contracts.py` (which owns the footprint ratchet) is green.

**⚠ VERIFICATION RESULTS, 2026-07-30 — and the first one is a trap about the
METHOD, not about slice H.**

| # | verification | result |
|---|---|---|
| 1 | `cargo check -p ambition` (default features) | ✔ green |
| 2a | `fixtures/minimal_game` | ✔ green, 16 passed |
| 2b | `fixtures/external_consumer` | ✘ 2 failed **IN THE WORKTREE ONLY — not a slice-H defect** |
| 3 | red-probe the art-without-render refusal | ✔ **RED, as claimed — and it found a second thing** |

**⛔ DO NOT RUN AN ASSET-TOUCHING TEST FROM A GIT WORKTREE.** The previous
session's handoff says to run these verifications *"from inside the worktree,
not from the main checkout"*, and for anything that reads generated art that
instruction is WRONG. `actors_desktop_asset_root()` resolves the engine tree to
`CARGO_MANIFEST_DIR/../ambition_actors/assets`, which is per-checkout, and the
generated sprite tree there is git-ignored — so it does not travel to a worktree:

    crates/ambition_actors/assets/sprites/   main: 972 files   worktree: 4

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
`ambition_actors` or `app_it`.


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
- The FOLD lives one layer up in `ambition_actors::features::combat_rules`,
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
  `crates/ambition_actors/src/features/enemies/mod.rs` (1039).
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
   now documents `generation()` and `ambition::rollback::stop`, including why a
   frame number cannot tell a restart from a rewind.
2. **The facade's first unit test created a feature job that ABORTS on this
   machine.** `crate_has_tests` is what admits a crate to the per-crate feature
   pass, so `crates/ambition` gaining `#[cfg(test)]` created a
   `-p ambition --features ...,profile,...` job — and `profile` forwards
   `bevy/trace_tracy`, whose static initializer aborts a test binary on a CPU
   with no invariant TSC. It fails before libtest lists a test: `--list` and a
   filter matching nothing fail identically. `ambition` joins `SKIP_FEATURE_JOB`
   for the reason already stated there — every one of its 17 extra features is a
   forwarder to `ambition_actors`, which is skipped for the same reason — and
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
`ambition_render` and merely REGISTERED by `ambition_host`.

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
The first version reached it through `ambition::renderer` — the raw crate
re-export — and `outlander-names-only-the-public-sdk` failed on `new=['renderer']`
in the suite. That contract exists because *"a consumer's imports encode our
implementation topology and we cannot move an implementation without breaking
them"* (ADR 0031), and "my test needed it" is exactly how such a leak gets made.
It lives in `ambition::view` now — *"what is drawn, as a game observes it"* —
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
capability, so `ambition::persistence` does not exist for it and there is no
settings resource to read. The quality resolve is inert there BY CONSTRUCTION —
slice H working as intended, not an omission. A consumer-level test for that
half belongs in a fixture that takes the capability, not in the one whose whole
point is taking as little as possible.


### ✔ S19. AC21 — the workspace builds with ZERO warnings (2026-07-31)

AC21 said AC17's *"remaining warnings are TEST-only and in
`ambition_engine_core`"* was already stale. It was, and the answer is now a
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
`ambition_engine_core` 448 and `ambition_characters` 379 tests green after the
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
