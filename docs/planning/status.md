# HEAD orientation

**Snapshot:** `HEAD` (see `git log -1`) (2026-08-28 local project date).

⚠ **this SHA goes stale within hours during an active run** — it names the tree
these paragraphs were measured against, not the tree you have. ⭐ **if it
disagrees with `git log -1`, trust HEAD and the ledger, and update this line
rather than reasoning from it.**

This page is a cold-start map, not an execution queue and not a completion
diary. [`queue.md`](queue.md) is the continuing
execution authority. [`tracks.md`](tracks.md) is the standing reservoir used to
replenish it. Focused plans own technical design.

If this page disagrees with current source or a focused open plan, update this
page rather than appending an archaeological correction.

⭐ **Reviewing rather than implementing?** [`../reviewer-guide.md`](../reviewer-guide.md)
is what the deep-review checkpoints (D237–D241) work from — role, what counts as
evidence, and how to start from current truth rather than from a previous agent's
summary. It was reachable from nothing until 2026-08-26.

## 2026-08-28, LATEST — three carves, and a census that was blind three ways

⭐⭐⭐ **THE ONE SENTENCE FOR D33: A CARVE CENSUS THAT COUNTS `crate::` PATHS IS
BLIND, AND IT IS BLIND THREE DIFFERENT WAYS.** The candidate table ranked by file
size and by `crate::` occurrences, and every one of its top entries was wrong.
What a census misses:

```text
a FACADE HOP     `crate::features::X` where X already lives in a peer crate.
                 Naming the owner took the boss module from 2 refs to 0 with no
                 code moved.
a GLOB           `use super::*` / `use super::super::*`. Measure by DELETING it
                 and reading the compile errors — read the WHOLE set, rustc stops
                 resolving early. Four module sets in a row supplied only bevy's
                 prelude and floor crates.
a `super::` PATH neither a `crate::` path nor a glob. Only the MOVE found these:
                 three reaches that kept `integrate_boss_bodies` behind.
```

✔ **CARVED TODAY:** `ambition_boss_encounter::ecs` + `::attack_moveset` (the boss
ECS tick and its moveset authoring), `ambition_combat::ledge_trump` (beside the
`ledge_trump_pop` rule it enforces), `ambition_combat::attack_support` (604 lines
following the melee path that left for `combat::moveset` long ago), and
`ambition_conversation::banter` (a bark is the shortest conversation there is).

⛔ **AND EVERY CARVE MUST GO LOOKING FOR TWO THINGS A COMPILER WILL NOT FIND**:
a SOURCE-SCANNING guard (`the_reaction_timer_clock_forks_on_purpose` walks one
crate's `src/` and wants two decay sites — widen the roots, never lower the count)
and a TYPE PATH HELD AS A STRING (`rollback_coverage.rs` waives resources by
literal path; poison-verified both ways).

⚠ **the line-count scoreboard cannot tell a carve from a feature** — 110,911 →
107,354 across three weeks in which six things left. Read the per-module OUTWARD
EDGE COUNTS instead.

## 2026-08-28 — four notes were the obstacle, not the code

⭐⭐⭐ **THE ONE SENTENCE: FOUR TIMES TODAY THE THING BLOCKING A ROW WAS A
MEASUREMENT A PREVIOUS AGENT WROTE DOWN, NOT THE TREE.** Same failure mode this
page already warns about, one level up: a `▢` on finished work is the cheap case,
and a recorded *reason* that is wrong is the expensive one, because it reads as
careful.

- **D245's last item was parked as a judgement.** The precedent that settles it
  was inside the row: `ambition_time` is equally a floor crate and has federated
  since 2026-08-26. `ambition_platformer2d_core` declares its own 25 rows now, and
  the runtime's `use body_clusters as bc` alias is GONE — it no longer names a
  single one of the floor's types. D245 CLOSED.
- **A recorded design COST was a test's own convenience.** The provider-action
  proof said a three-variant control-kind mirror was *"the honest price"*. One of
  the two blocking types is ours, three lines up, fieldless — `Hash` and `Reflect`
  derive for free. The mirror existed because the check lived inside a `#[test]`.
- **A twintrack bug note said *"one impulse at construction, not a walk"***, and
  pointed at the causal recorder as the next step. It is a walk, a twelve-line
  probe found it, and five tests that had failed since `a945c1de5` are green.
- **Two review measurements from 2026-08-26 were part stale**: the menu activation
  seam is not unadopted (both roads already share `PressArm`), and four parity
  rows said absent about work that shipped last week.

⭐⭐ **THE PROVIDER-ACTION ROAD IS OPEN** (D242's item, the one `tracks.md`
names). Register an action the engine has never heard of, bind it, press the key,
get a `SemanticActionPressed` back — no `Any`, no `TypeId`, no variant added to
the 35-variant enum. ⚠ what remains is PRESENTABLE, and re-measuring that changed
its shape too: `ControlSlot` and `TouchActionButton` are descriptions of hardware,
not arbitrary limits, so a provider action becomes presentable by being ASSIGNED a
slot rather than by widening one.

⛔⛔ **A QUEUED COMPONENT INSERT LEAVES ONE TICK UNDER THE OLD POLICY.** The
laboratory twin's whole bug: adoption QUEUES `DrivingParticipant`, the seat lands
a flush later, and one tick of her life runs as a seatless `Passive` stroller.
⚠ and the first fix sampled the wrong moment — correcting her inside the adoption
reads a body that has not taken the step yet, and prints a line that looks like
success. `Added<Marker>` is where that correction belongs.

⚠ **A SHIPPED USER SETTING CAN GUARD NOTHING.** `MenuTapMode` defaults to
`SingleTapWithDestructiveGuard` and its own doc names *"a stray touch on Quit"* —
and only the index-addressed helper consulted it, so the pause menu's Abandon /
Quit to Title / Quit to Desktop all fired on the first release. One policy now,
generic over an opaque row identity. ⇒ **when a setting has arms, count the
readers, not the writers.**

## 2026-08-26 — the mount is free, and four review rows were already finished

⭐⭐⭐ **THE ONE SENTENCE: FOUR OF THE SIX OPEN LEDGER ROWS I RE-READ TODAY WERE
FINISHED AND STILL MARKED OPEN.** D237, D238, D239 and D179 all closed by
READING them against HEAD — not by building anything. Two items in D237 had
shipped under D241 and were never marked; D238's clank fix and windbox contract
were both live in the code the row called open. ⇒ **the ledger went 18 open rows
→ 14 in one pass.** Re-read before you build; the four-session failure mode this
file warns about is a `▢` on work that already landed.

⭐⭐ **`ecs/mount/mod.rs` NOW NAMES ZERO MONOLITH PATHS**, which is D33's smallest
honest carve unblocked. Four outward edges went in one day: the brain rebuild
answers a `MountDied` mount was already writing; `ResolvedMotionFrame` was in
`shared_tangle` all along behind a re-export; `TemporaryControl` moved to
`shared_tangle` beside `Mass`; and the 26-column `ActorClusterQueryData` turned
out to be five columns, four of which live in other crates. The last one wanted
`ActorConfig`'s authored baseline, now `shared_tangle::body::SpawnBaseline` —
which also collapsed **three** hand-written `if is_aerial { 0.0 } else { 1.0 }`
sites into one recorded value. ⚠ `mount/tests.rs` still names 41 monolith paths,
so the remaining carve work is the FIXTURES.

⛔⛔ **THREE MECHANICS ARE GUESSING AT ONE MISSING FACT.** The initial dash, the
shield brake and body contact each bound themselves by a magnitude with the same
sentence — *"anything faster than its own run is somebody else's velocity"* — and
each fails on a DECAYED launch. Two ledger rows tracked them separately and a
third had already closed one as a feel question. All three are
[decision 34](awaiting-maintainer-decision.md) now, and the genre's answer is
TUMBLE, which this kernel fully implements and every shipped body leaves dormant.

⚠ **AND D168's OWN PRICING WAS WRONG IN FOUR PLACES**, each corrected by
measurement: the carve is three arms not all of `brain/`; the "253 lines of data"
is really 1.5–2.5k interleaved with behaviour; the enum split costs NO wire
format; and it buys EDIT COST (18 dependents → 10), **not** capability footprint —
every destination is already in the movement-only sentinel's closure.

## 2026-08-26 — a fortnight of stale markers, and three feature-gate holes

⭐⭐⭐ **THE ONE SENTENCE WORTH CARRYING FORWARD: A GREEN PER-CRATE `cargo test`
IS EVIDENCE ABOUT A FEATURE SET, NOT ABOUT A CRATE.** Three holes found in one
day, and each hid REAL red:

```text
ambition_demo_smash --lib   2 fixtures red since the winner-card fix; the
                            standing gate runs `-p ambition_demo_smash_app`,
                            which is a DIFFERENT test target
ambition_conversation       25 tests by default, 35 with `--features ui` — the
                            whole authored-command road is behind it, and a
                            DELIBERATELY BROKEN `Truth` arm came back GREEN
ambition_game_shell         45 by default, 72 with `--features basic_presentation`
                            — `mod pause_menu` is gated, so its ten tests never
                            ran AND could not pass: the fixture omitted a
                            message the shell plugin owns
```

⇒ when you poison something and it stays green, check the feature set before you
believe the poison.

⛔⛔ **AND THE SWEEP THAT FOLLOWED FOUND A FEATURE THAT DID NOT COMPILE AT ALL.**
`--features causal` had been broken across `ambition_combat`: three references to
`StocksMatchDecided.winner` after that message became `outcome: MatchVerdict`, a
`BodyReaction` construction missing a field, two fixtures on the old field, and
an expectation spelling a `HitSource` variant that no longer exists. ⭐ and the
repair was not mechanical — the instrument was still asking
`winner: Option<String>` with `None` meaning DRAW, which is exactly the
conflation `MatchVerdict` exists to remove, so an ABANDONED match had nowhere to
go but to impersonate a draw.

```text
ambition_input      56 default / 125 all-features   (pass — a visibility hole)
ambition_dialog     30 / 42
ambition_items      27 / 39
ambition_encounter  34 / 41
ambition_sfx         8 /  8                          (the only one with none)
ambition_combat    327 / 332 with `causal`           (DID NOT COMPILE)
monolith          1194 / 1202 with `causal`
```

⇒ **a non-default feature is where code goes to rot.** Anything gated is
compiled by nothing in the standing gate, so it drifts silently until somebody
turns it on.

⭐⭐ **AND THE OTHER HALF OF THE DAY WAS THE LEDGER DISAGREEING WITH HEAD.** Rows
were carrying `▢` over landed work, and one row's own proposed FIX could not have
repaired the defect it named:

* **D179(a)** parked as unreachable on a measurement that actually proved the
  proposed gate (`knockdown::owns_control`) is FALSE exactly when the defect
  fires — the three lines would have been dead code.
* **D170's biggest item** wanted a cross-room rollback snapshot; a rollback
  cannot cross a room boundary BY DESIGN (the commit rebases onto a new
  frame-zero baseline whose first `SaveWorld` overwrites every ring slot).
* **D125** said `MatchAbilities::apply` never receives the body's kit. It does.
* **D165's shield row one**, **items 4/5/11/31/36/39** and **29c**: all landed,
  all still marked open.
* **D72's own next-up table** disagreed with the inventory it calls canonical —
  Z-drop shipped on the very day the table was "re-read against" it.

⇒ **grep for the thing a row says is missing before working it.** It paid every
single time today.

⭐⭐ **WHAT LANDED, and the through-line is that a fact had no seam to live on:**

* **`MatchParticipant::body`** — a seat can state a body now, the movement twin
  of `action_set`. A catalog row's feel is that character's feel EVERYWHERE, and
  all 17 affected fighters are `tier: MainHall`, so a fighter self needed its own
  place to differ. `SmashFighterFacet` grew a matching `body` patch and
  `smash_roster` fills the seat from it.
* **17 of 19 grid fighters move on the wandering enemy's body** — measured on the
  shipped host, and the headline is not gravity: they build ground speed at an
  EIGHTH of the player's rate and cap their fall at 40%.
* **The select cursor could never snap to a portrait, on any device** — Jon's
  *"very hard to use with a gamepad"* was not a feel report. `nav` folds the held
  d-pad and held arrow keys, so it was non-zero on every frame an edge could
  fire, and the roam branch always won.
* **Two authorities on what `true` means** — authored dialogue had a
  byte-identical copy of the shared arg conversion, comment included.
* **The lock wall holds a `PreparedCondition`** instead of re-minting arguments
  every frame; the road was built for it (`ConditionCatalog::prepare`'s doc names
  it) and never adopted.
* **The pause menu is on `ListCursor`** — two menus in one game had disagreed
  about the end of a list.
* **D242 promoted**: nine participant/action items were reachable from
  `tracks.md` and from NO ledger row, and all nine now carry a measurement.

⚠ **WHAT IS BLOCKED AND ON WHOM**: #20 (dash/brake ownership) on waking `tumble`,
which no shipped body authors and which re-tunes knockback for the whole cast;
the recharge presentation (decision 33) and the six pirates' `standing_height` on
Jon; the 17 fighters' actual movement numbers on a session that runs the
repertoire probes before and after.

## 2026-08-25 — four review checkpoints, seventeen closed, five scoped

⭐⭐⭐ **THE ONE SENTENCE WORTH CARRYING FORWARD**, from the reviewer and
confirmed independently a dozen times today: *recent mechanics are locally tested
where they are AUTHORED, but their semantic distinction is LOST AT THE NEXT
SHARED GATEWAY.* Windboxes lose their reaction kind at `pending_launch`; shield
transitions lose their CAUSE at a bool; recovery helplessness loses its EPISODE
by deriving from a resource count; an input scope was set at spawn and never
re-derived; a body was mutated during PROPOSAL. **Look there first.**

⭐⭐ **AND A SECOND PATTERN THAT FOUND THREE DEFECTS**: a COMMENT stated the
correct rule and the predicate beneath it asked a DIFFERENT question — shield
drop lag ("you simply let go" vs every way a guard ends), roll endlag ("before
this becomes a gate, the roll needs its own timer" — since satisfied), the
untechable tech press ("it still spends the lockout below" — that road never
armed the timer). ⇒ **this repo's comments are load-bearing specifications, so
they work as a defect index.**

⛔⛔ **AND THE HARDEST-WON ONE: A GREEN SUITE IS ONLY EVIDENCE ABOUT WHAT IT
EXERCISES.** Today, FOUR fixtures omitted the state their bug lived in (the
sudden-death fixture had no `FighterStocks`; the timeout tiebreak's arms built the
side map BY HAND and never ran the fold; the impact-hitstop tests INJECTED the
victim's timer; the stale-decay test seeded an IDLE body). My own 68-crate
"sweep" checked COMPILATION, not passes — `ambition_render` and
`ambition_content` were red the whole time. And two of my own new assertions
turned out to be checks that could not fail.

⭐⭐ **BEFORE BUILDING ANY "GATEWAY" FIX, ASK WHETHER THE FACT IS ALREADY AT THE
SITE.** Windbox 29c was written up as needing a new `DefenseInteraction` enum
threaded to two seams; it needed neither. The producer already sets
`flinchless: hitbox.windbox.is_some()`, so "push, not strike" rides the knockback
to both damage roads, and `Option<GuardUnderFire>` already means "no guard
participates". **The channel existed; nobody had used it to say so.** That check
is cheap and it turned a schema-change-sized item into two conditions.

⇒ APPLIED TO EVERY REMAINING GATEWAY ITEM, 2026-08-25:

```text
29c  guard interaction     ALREADY THERE  → fixed, two conditions
29b  launch KIND           genuinely absent — `pending_launch` is a bare Vec2
                           on a SNAPSHOTTED struct
39   directional intent    genuinely absent — MovePlayback carries spec /
                           facing / was_grounded / t and no intent
24   helpless EPISODE      genuinely absent — BodyJumpState has the CHARGE only
4/5  resolved hit          genuinely absent — needs the new channel
```

### The open work, in the order I would take it

```text
1  resolved-hit split (D237 4/5)   the freeze fires for a CPU victim and not
                                   for the human; blast radius traced in full
                                   below; ⛔ do NOT reuse the causal
                                   BodyHitResolved — publish an unconditional
                                   fact and DERIVE the inspector from it
2  windbox 29b + 29c               `pending_launch` is a bare Vec2 so jab-lock
                                   and tumble come from SPEED ALONE; and no
                                   fact says "windbox" at the parry/guard seam.
                                   ⭐ latent — fix the primitive BEFORE content
                                   authors one
3  recovery-helpless EPISODE       body_is_helpless is pure resource state, so
                                   a hit that refunds the air dodge cannot lift
                                   it. Needs a BodyJumpState field → schema bump
4  clank simultaneity (26/27)      one-line fix WRITTEN AND REVERTED: nothing
                                   exercises arbitrate_attack_clanks at all
5  directional melee read model    delete attack_intent_from_move_id; carry the
                                   resolved AttackIntent on MovePlayback
6  Charge Shot                     ⚠ WAITING ON JON — a balance decision, in
                                   awaiting-maintainer-decision.md
```

⚠ Items 1, 3 and 5 each need a WIRE-FORMAT change; 2 and 4 each need a real
fixture. None of them is a tail-end job.

## 2026-08-25 — a 24-hour deep review, triaged end to end

⭐⭐ **START HERE: the one open item worth a session is the RESOLVED-HIT SPLIT**
(D237 item 4/5, and its design is named there). Everything else from the review
is closed or parked with a reopening condition.

**The finding, measured rather than argued:** `request_impact_hitstop_on_landed_
hits` reads the victim's `BodyCombat::hitstop_timer` in `CombatSet::Settle`, but
the schedule's own comment says player-victim hits go to a FIFO drained by a
resolver in NEXT frame's PlayerSimulation. ⇒ **the match freeze fires when a CPU
is hit and not when the human is.** Every test in that file INJECTS the timer on
the victim before firing the message, so none of them can see it.

⛔ TWO SHORTCUTS ARE BOTH WRONG, and checking them is most of the work already
done: carrying the payload's hitlag on `LandedBodyHit` would freeze on a hit an
INVULNERABLE victim ignored (the producer gates on self/corpse/faction/dedup/
geometry and a parry — no i-frame check); and adding the hold to
`apply_player_hit_events` enlarges a system whose own comments say TWICE that it
is at Bevy's param ceiling, which is exactly what the review's item 16 forbids.

⭐⭐ **AND THE GAP REVIEW CONFIRMS THE SHAPE — read this before reaching for the
existing `BodyHitResolved`.** Its correction to findings 4/5, verbatim in effect:
*do not wire gameplay to the current `BodyHitResolved`*. That type is
`#[cfg(feature = "causal")]`, its writer is `Option`, and its own comments
guarantee nothing in the simulation reads it. Depending on it would invert the
dependency — an OPTIONAL INSPECTOR becoming REQUIRED GAMEPLAY AUTHORITY.

⇒ **THE DIRECTION IS THE OTHER WAY ROUND**: publish an unconditional resolved-hit
fact that simulation consumes, and let the causal inspector DERIVE
`BodyHitResolved` from it. What is worth reusing is the resolution VOCABULARY
(`BodyHitResolution`, resolved damage, resolved hitlag, reaction kind, whether it
counts as an offensive connect), not the causal message.

**THE BLAST RADIUS, TRACED 2026-08-25 so the next session does not re-derive it:**

```text
1  define ResolvedBodyHit { victim, hitlag_s }   beside LandedBodyHit
2  add a `resolved` MessageWriter to BOTH writer bundles — the roads differ:
       player victim  -> BodyDeathWriters   (damage_apply.rs ~366)
       actor victim   -> FeatureHitWriters  (damage/mod.rs ~80)
   ⛔ NOT `Option` and NOT cfg(causal): the freeze is SIMULATION. The existing
   `BodyHitResolved` on that bundle is instrument-only and says so — reusing it
   would make an inspector load-bearing, which its own doc forbids.
3  publish right after `apply_body_hit_reaction`, where `combat.hitstop_timer`
   now holds the resolved value. Mirror `publish_reaction`'s shape: a helper
   with a no-op fallback so call sites carry no `cfg`.
4  impact_hitstop consumes ResolvedBodyHit instead of LandedBodyHit
5  register the message + clear_message_on_rollback (a new channel owes both)
6  its four tests INJECT hitstop_timer today — rewrite them onto the real road
```

⭐ NO DOUBLE-FIRE RISK: a hit takes exactly ONE of the two roads, by victim kind.

⛔⛔ **BUILT ONCE ON 2026-08-25 AND SET ASIDE — READ THIS BEFORE REBUILDING IT.**
The whole split works: `ResolvedBodyHit` carrying the resolved hitlag, published
from both roads, freeze consuming it, message + rollback clear + baseline + v104.
Gate green, every crate suite green. It was set aside for ONE reason, and it is
not a defect in the split:

⭐⭐ **ADDING THE WRITER PERTURBS THE MATCH BY ITSELF.** Two app fixtures went red
(`the_puppy_slug_forced_onto_the_stage_keeps_the_body_it_authored`,
`a_dash_less_fighter_presses_attack_out_of_a_run_and_gets_the_dash_attack`), and
they redden even with the FREEZE DISABLED — so it is not the new hitstop. A new
`MessageWriter` param changes Bevy's parallel scheduling constraints, which
reorders execution, which reshapes a chaotic match. Committed HEAD passes both;
the split reddens both.

⇒ **FIX THE TWO FIXTURE PREMISES FIRST, THEN LAND THE SPLIT.** Both are premise
guards, not assertions about the split: the slug reads 320 px/s against its
authored 80 while hitstun, hitstop and recoil are ALL ZERO and nothing was
disturbed — it is coasting or being shoved, and "is this velocity the body's
own" is a question no state check answers. ⚠ I tried four repairs (hitstop as
disturbance, start-from-rest, proximity, both) and none held; the fixture needs
a real look, not another guard.

⚠ AND MY FIRST ISOLATION WAS WRONG, which cost four probes: disabling the freeze
made three tests pass, so I blamed the freeze — but the slug fails with the
freeze off too. ⇒ **ONE ISOLATION RUN IS A HYPOTHESIS, NOT A RESULT**, and in a
chaotic sim a single differing tick reshapes the match.

⭐ **THE SEAM TO USE IS `ambition_combat::hit_reaction::apply_body_hit_reaction`**
— the one function all three roads (player, actor, boss) pass through, and where
`hitstop_timer` is actually written. Have it yield the resolved hitlag, publish a
`ResolvedBodyHit` from each road, and move the freeze onto that. That is item 4's
producer/resolved split arriving through its first real customer rather than as a
refactor for its own sake.

**Landed today, each measured → built → poisoned → verified:** the evade maneuver
/ i-frame clock split and roll ownership (Jon's playtest, 107px vs 33px); match
input device authority (the keyboard drove player one whoever claimed what);
double-jump cancel; windbox zero damage; bark determinism off `SimId`; the live
match clock counting scaled time; the special-turn proposal/acceptance boundary
and its fourth technique (wavebounce); wire format v103.

**Parked with conditions, not skipped:** the ledge-trump pop (its INPUT
`wall_normal_x` is itself world-X, so fixing the consumer alone moves the
inconsistency); steering-permission vs neutral friction (no move at HEAD both
roots steering and authors a carry); the special-turn input-ORDER recogniser.

## 2026-08-25 — the earlier review is closed out, and Smash gained four more mechanics

⭐⭐ **THE 2026-08-24 REVIEW IS FULLY CLOSED** — four P0s and four P1s, each with
a production-path poison, which was the review's own stated discipline. D210 is
✔ and D211–D214 carry the details.

```text
P0-1 clank            reads StrikeVolume, orders by SimId, ends the losing MOVE
P0-2 helpless         one derived rule gating move STARTS
P0-3 sudden death     the spent clock stops deciding a match already in it
P0-4 items            sleep is EXPLICIT (`SettledItem`), not read off velocity
D211 Exit Match       withdrawn once `StocksMatchSettled` says the match is over
D212 the match clock  ONE live clock, ceremony and pauses excluded, read by the
                      timeout AND the item cadence. Costs wire format; says why
D213 sim_random       a CONTEXT axis, so match two stops replaying match one
D214 sudden death     carries the tied LEADERS, not every survivor
```

✔ **AND FOUR PARITY MECHANICS, each chosen for having a customer already**:
dodge staling (every roll), the ledge-trump outward pop (Jon's "no way to knock
them off"), the untechable high-launch threshold (every hard hit), and the bark
rate (Jon: "not every time a character is hit"). The windbox primitive landed
too but is **UNADOPTED** — no move authors one, and which move gusts is a
character-design call sitting in `awaiting-maintainer-decision.md`.

⛔⛔ **THREE PLANNING FILES WERE LYING, and correcting them was real work.** Four
smash-parity rows claimed work that was already built (`WindowTag::Armor` and
`Invuln` ARE consumed, by `project_move_defense_windows`); D210 was ▢ on eight
items that had landed; and my own entries in Jon's observations file were
multi-paragraph write-ups that its header forbids. ⇒ **re-grep a row before
working it, and put reasoning in the ledger rather than in Jon's file.**

⭐ **THE PATTERN WORTH CARRYING: most remaining parity rows are blocked on
AUTHORED CONTENT, not on the engine.** The payload-field rows (extra shield
damage, unblockable, per-hit hitlag/hitstun/SDI multipliers, weight-independent
knockback) all need a move to author one, and this demo has shipped three
mechanics green and inert already. ⇒ prefer rows whose customer is every match:
that is why staling, the trump pop and the untechable threshold went first.

⚠ **STILL OPEN AND WAITING ON JON**: the shield roll (he confirmed the report,
will test — `cargo run -p ambition_demo_smash_app --bin roll_probe` is the
instrument, and ⛔ it does NOT yet fire the roll, which its module doc says at
the top); and which move should carry the windbox.

## 2026-08-24 — Smash gained four mechanics; the hit-unification is repaired and CLOSED

⭐⭐ **THE SMASH CAMPAIGN WAS MOSTLY ALREADY DONE, and finding that out was the
first job.** [`demos/campaigns/smash-fun-push-2026-08-22.md`](demos/campaigns/smash-fun-push-2026-08-22.md)
is D72's stated execution authority and
[`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md) is its
canonical feature truth — **they disagreed**. Every `O` slice plus `W1` and `W6`
read green in the inventory, so working the campaign's headings in order would
have rebuilt six shipped features. `W6` argued from a claim about the inventory
that the inventory never made. ⇒ the campaign is swept and marked; ⛔ re-grep an
inventory row before working a slice.

✔ **what this session then SHIPPED on it:**

```text
W2  autolink knockback   an intermediate multi-hit pulse HOLDS its victim
                         instead of launching. Kernel + authoring, schema 77→78
W3  the rising spin      Pointed Polygon's Up-B is four holding pulses then one
                         launch, via a shared `multihit` combinator
W5  respawn release      swinging SPENDS the respawn protection. It was a flat
    (the rule half)      timer nothing could end — a free hit every stock.
                         Schema 78→79
```

⛔⛔ **AND A CORRECTION PASS FOLLOWED (second GPT review), whose three findings
share ONE shape — read this before adding a feature, not after.**

```text
state initialized by a later INCIDENTAL event
    recovery_charges came from the landing refresh; `Default` is the SPENT
    state and both fresh-construction paths spelled fresh as
    `..Default::default()`. Invisible because anything that LANDED was right.

a grant claiming ownership it does not STRUCTURALLY have
    respawn protection borrowed `Empowered` — ONE component — so granting it
    overwrote the body's power-up and ending the beat removed every semantic
    in it. A marker cannot make a single slot into two grants.

a coordinate system rebuilt from the WRONG BODY'S facts
    autolink resolved its ATTACKER-local anchor from the VICTIM'S away-side
    and the VICTIM'S gravity. They coincide in the ordinary case.
```

⇒ ⭐⭐ **each was green under a test that looked reasonable**, and for the same
reason: the fixture could not reach the case the claim was about — two DIFFERENT
bodies for an ownership claim about ONE, front-contact same-gravity for a frame
claim, anything-that-lands for a budget filled by landing. **Ask what the fixture
would have to look like for the claim to be FALSE, and check the fixture is
that.**

⛔⛔ **THE TWO TRAPS WORTH CARRYING OFF THAT WORK.** (1) In a multi-hit the GAPS
between Active windows are load-bearing, not spacing: the runtime's re-hit rule
refuses a contiguous track, so touching windows land ONCE and the mechanic
silently does not exist. (2) `Empowerment::UNTOUCHABLE` is a CAPABILITY, not a
claim about who granted it — Sanic's super state and Mary-O's star hold it too —
so a ruleset ends only the grant it MARKED. Releasing by value equality is not
ownership.

## 2026-08-24 — the hit-unification was PARTLY WRONG; repaired and CLOSED

D200 is CLOSED (twelve correctness defects, then all five P2 consolidation
slices) and stays closed. What replaced it is **D203**, opened by Jon after the
ledge fix: *"the ledge damage issue sounds like a player actor unification we
need to at least log as a todo."*

He was right and the ledge was the symptom. A damaging hit resolves down
`apply_player_hit_events` or `apply_feature_hit_events` (and a THIRD road, the
capture throw), all three ending in one shared `apply_body_hit_reaction` — and
what drifted is everything each road does AROUND that call. In an arena the whole
roster is actors, so a player-only rule is invisible until somebody plays it.

⛔⛔ **READ THIS BEFORE MOVING ANYTHING ELSE INTO THE SHARED REACTION.** The
first slice moved `refresh_movement_resources_clusters` — air jumps, dash charges
AND the air dodge — in as *"the air options a hit gives back"*, reasoning:

```text
   player road does X   ·   actor road lacks X   ⇒   X is a fact of being hit
```

That inference is INVALID, and it shipped a false mechanic. In the genre a spent
double jump stays spent through an ordinary edge-guard hit — taking somebody's
second jump is a thing you do to them — and Ambition's traversal dash was swept
up without ever being named. Repaired at `2daa4fa05`: `AirBudget` deleted, the
reaction takes the ONE resource the rule names, and the jump went to its real
CAUSES (catching the ledge, being caught, landing). The test that enshrined the
wrong rule is gone with it.

⇒ **the classification table on D203 is the thing to use**, not "which road has
it". Ask whether a behaviour is intrinsic to an accepted hit, a launch
consequence, a ruleset policy, a cause-specific rule, or a road's own economy.

✔ **D203 is CLOSED**, and the last four rows closed by MEASUREMENT rather than by
moving anything: wallet armor is not partial (one shared resolver; only the
GRANTOR is narrow), the cling break is a motion-model policy no home avatar can
have, `safe_respawn_player` is gated on an authored hazard MODE and not a body
class, and `kill_disposition` is the ruleset's by the same argument the player's
respawn is the save file's. ⛔ a rule that is right on every road it appears on
stays where it is — do not open a third unification pass.

✔ what IS the reaction's, and correctly:

```text
knock_off_ledge      a hit takes the hang            (was both roads', separately)
air dodge returned   a hit gives the evade back      (one resource, not three)
hitlag               the freeze that makes it read as a hit
```

✔ **AND A DAMAGE-ONLY HIT IS STILL A HIT.** `knockback_velocity(None)` returns a
ZERO launch and the reaction wrote it over the body's own velocity, so a hazard
or a chip stopped a running player dead; the actor road had dodged that by
wrapping its whole reaction call in `if let Some(k) = knockback`, which cost it
every hit fact instead. Two roads, wrong in opposite directions. The reaction now
separates THE FACTS OF BEING HIT from THE FACTS OF A LAUNCH, and both roads call
it for every accepted hit.

✔ **The CPU can now reach a ledge to guard it** (`8d7dce964`). Both terms of the
corner test asked for the NEAREST edge, so the ledge you stand beside to punish a
hang read like the ledge you are backed against: a fighter walking out to
edge-guard flipped `EdgeGuard → Disadvantage` 90px from the lip and retreated,
every time. Retreat is away from the THREAT, so that is the direction it asks in.

✔ **A ground guard no longer rides into the air** (`11ffb209a`). `resolve_shield`
gated the raise and not the sustain, and a held Shield also fills the air-dodge
buffer once airborne — so walking off a ledge guarding produced the exact state
`air_guard: false` exists to forbid.

✔ **D202 is CLOSED.** Its double restriction is gone; its double PUBLICATION is
measured, judged and DECLINED — merging the two producers would drag five demo
systems out of the input phase to satisfy a diagram, and the condition that would
reopen it (a consumer needing finished control from both, before the gate) does
not exist. ⛔ do not re-derive that from scratch. Control is published twice — a
possessed body's in `PlayerInputSet::Brain`, an autonomous body's a phase later —
so every restriction over control was registered twice, and the pair was correct
only by an invariant nothing enforced: the first blank stopped the second sampler
crediting the same human press. `ControlGate` and `BodyMode` are re-parented into
`WorldPrep` after both publications, and one copy of each restriction gates
everybody. ⚠ the sets keep names that now lie about their phase, stated at the
enum.

✔ **D201 is CLOSED.** A hit takes the hang, the ledge lets go of a camper at 5 s
(`LEDGE_HANG_MAX_TIME` — the genre HAS a limit; the row's claim that it does not
was the false premise keeping this unbuilt), catching the ledge restores the air
recovery at the LATCH, and the CPU can reach a ledge to guard it. The regrab
COUNT and damage-scaled getup are recorded as decisions with their conditions,
not as gaps — do not build either without the symptom named there.

⭐⭐ **D33's carve price was measured on a bad count, and the correction is the
transferable part.** The row said one carve is possible and it is ~17,000
production lines, because `construction` and spawn are mutually dependent. Two
errors made that number: 25 of the 26 "survivors" holding the edge were in
`construction/tests.rs`, and a `features::ecs` grep could not see six more spawn
functions construction calls through the `crate::features::` RE-EXPORT. ⇒ **split
production from test AND resolve through re-exports before pricing any carve** —
this row's own laundering rule, which the row then failed against itself. One
real edge did die (two twenty-line inserts filed under "spawn" by topic, whose
only caller in the tree was construction), and two giant-creature wrappers
followed it; the edge THINS to four shared functions and does not vanish.

⛔⛔ **THREE WAYS A REFERENCE COUNT LIED IN ONE AFTERNOON, all on the same row** —
worth carrying to any carve, not just this one: it counted a TEST file as domain
coupling; it was defeated by a `crate::features::` re-export shell (two layers
deep) hiding the real `features/ecs` path; and a grep that EXCLUDED the defining
file made a symbol its own module calls four times look like somebody else's.
⇒ count production only, resolve through re-exports, and count IN the definition's
own file.

⚠ **D201's reference facts were WRONG and are corrected in the row.** Ultimate
does NOT allow an indefinite hang (6.5s under 100%, 5s at or above), its 6-grab
regrab limit is NOT the same mechanism as diminishing intangibility, and
damage-scaled getup is an OLDER-game rule that Smash 4 onward dropped. Do not
implement from the pre-correction text.

⛔⛔ **AND THE HABIT TO KEEP IS THE REVERTS.** Four changes were built, measured
and thrown away this pass, each buying a finding the ledger now carries:

```text
C5 frame-advantage guard   INERT — offered 0 times in 129 Advantage decisions
parry raise timing         buys 0–0–2 parries, sells 40% of shielding + a KO
ParryTiming::OnRelease     same mistiming at the other end
camera snap on teleport    a portal transit must NOT snap; the suite caught it
```

⇒ the parry pair explains itself: you cannot open a second window while already
holding shield, so parrying and blocking-early are mutually exclusive and a CPU
that cannot read its opponent should block. What is left of it is a risk
appetite, not a rule.

⚠ **one fitted constant moved and was RE-DERIVED, not re-fitted.** Shield in the
air is now the air dodge (`ShieldTuning::air_guard`, schema v77), and that made
`two_emmys_hold_a_mirror`'s 2× margin false: the air dodge aims in gravity-frame
axes, so a neutral stick zeroes velocity (independent pair 32% → 52%) and a
directional one breaks a shared-stream mirror (Emmy 100% → 84%). The mechanism is
written into the test beside the number.

⇒ **and the app gate's blind spot cost 1,157 tests.** The monolith's own test
target had stopped compiling. Swept the whole tier the same day: `cargo test
--workspace --lib`, 67 targets, all compiling and green. Run it after touching a
shared type.

## 2026-08-24 — ⛔⛔ A REVIEW REOPENED TWO ROWS. READ THE TRIAGE FIRST.

[`triage/gpt-review-2026-08-24-correction-pass.md`](triage/gpt-review-2026-08-24-correction-pass.md)
is the whole of it, verbatim, with four P0s. **Do not add a parity row until they
are closed with production-path tests.**

⭐⭐ **THE PATTERN, and it is the thing to carry forward:** several tests proved a
nearby SURROGATE road rather than the actual production authority. Verified at
HEAD for the worst one — `arbitrate_attack_clanks` queries
`With<HitboxLifetime>`, and `advance_move_playback` spawns authored volumes with
a comment reading *"NO `HitboxLifetime` on purpose"*. ⇒ **no authored Smash
attack ever entered the clank system**, and every clank test spawned a synthetic
box carrying the component production refuses. Rows reopened.

```text
P0-1 clank never reaches authored moves   ✔ FIXED  real attacks; stage declares it OFF
P0-2 helpless never reaches move starts   ✔ FIXED  one rule, asked by the move authority too
P0-3 sudden death ends on first hit       ✔ FIXED  spent clock ignored; stage half now in sim
P0-4 zero-velocity items float            ▢ VERIFIED  pickup/mod.rs:347 skips vel == ZERO
```

⛔ **THREE OF FOUR ARE FIXED; P0-4 IS NOT.** Evidence for each, and P0-4's
reverted attempt, is in the triage. ⚠ clanking's MECHANISM is fixed and proven on
the production road, and Smash declares the window `0.0`: turned on it re-tunes
the whole ground game, which wants a play session rather than another guess.

⛔ **THE DISCIPLINE THE NEXT PASS OWES: production-path poison before closing a
parity row.** A synthetic fixture is not proof of a moveset mechanic.

⚠ and the review preserved the rest: D204's rooting, D205's pogo, D206's Up-B,
D56, the recovery fix, `RespawnGrace`'s ownership, the defense presentation, and
`sim_random`'s stateless design are all called good and must not be reopened.

## 2026-08-24 — W8's four findings closed, then six mechanics

Jon played the demo. The full message is
[`demos/w8-playtest-2026-08-24.md`](demos/w8-playtest-2026-08-24.md) and the most
valuable half of it is the negative space: VFX refinement, HUD animation tuning,
cast-wide animation cleanup, juice adjustments and another presentation audit are
named and refused. ⛔⛔ *"merely could look nicer → defer."*

```text
D204 forward smash pre-movement   ✔ e7927cee2   NOT the ordering defect reported
D205 pogo is Robot v3's           ✔ 7346b6e86   floor → ceiling; census REPAIRED
D206 Up-B reads as a poke         ✔ 6946d72ba   disk; anchor x=0; sprite_spin_hz
D207 no way out of a match        ✔ 24cf7f08c   Exit Match → MatchVerdict
D56  Kernel Guide definition      ✔ 3d2f53018   identity only, no kit
```

⭐⭐ **D204 IS THE ONE TO REMEMBER, because the report pointed at the wrong
thing.** *"I should not effectively dash first and then Smash"* reads as an input
ordering bug; measured through the real key stack, the smash STARTS on the press
tick and then the fighter accelerates to the full run cap during its own startup
— 64 world px. Two causes: nothing said a grounded attack roots its owner
(`MoveGates::roots_steering`, set by `SmashRepertoire::GROUNDED`), and
**`integrate_home_body` never received the move motion scale at all**, so every
rule expressed as a motion lock was live for brain-driven bodies and silently off
for the road a human drives.

### Then six mechanics, in order

```text
clank + rebound     900377782 e8a29855f  two attacks meeting now TRADE
sudden death        55e610083            a level timeout CONTINUES
helpless            ddb3e3fa9            a spent recovery is final
sim_random          98b5d5414            randomness that survives a rewind
item spawning       30c7dfcb1 82e1afca3  built, and Smash declares it OFF
```

⭐ **`sim_random` is the one with the widest reach.** A rollback sim's problem
with randomness was never the generator — a STREAM is state. A draw is a pure
function of `(domain, tick, salt)`, so nothing registers, nothing rewinds, and
**schedule order cannot matter**. The fighter brain keeps its own stream and
should; this is for *"what does the world do this tick"*.

⛔ **ITEMS ARE OFF BY JON'S CALL, not by omission** — *"we don't need items in
smash right now. We eventually will."* One `None` in `apply_smash_match_rules`;
the machinery and its unit tests are live.

### What is still open, and it is Jon's

⚠ **STATURE.** He ruled there is no standard adult height, `ADULT_HEIGHT` must
not exist, and ambiguous characters wait for his eye. Nothing has been authored,
so **`robot_v3` still does not read as shorter than anything** — 38 of 45 remain
exactly 48.0, which is the state he called wrong. The next step is his: name the
characters, or approve starting with the few whose fiction is unambiguous.

⚠ **A RATCHET MOVED THAT NOBODY'S COMMIT EXPLAINS.** `COMPUTED_ID_TARGETS` 11 →
13: `officer.py` and `author.py` are new SVG-rigged targets in the renderer
checkout, UNTRACKED, so nothing in this repo can see them while the test scans
the directory on disk. Neither has a catalog row yet. Lower it again if either
goes away before it lands.

### Four inventory rows were STALE, and that is the pattern to watch

Fixed knockback, the spin Up-B, the Z-drop and "walk distinct from run" were all
marked absent and all shipped. ⭐ the last one is the instructive case: the row's
PREMISE was wrong (*"one continuum"*), measuring found the gait line already cuts
it, and the deliverable became the missing test plus a corrected row pointing at
the real gap — a digital input can only say 1.0, so a keyboard fighter cannot
walk. ⇒ **grep before building, including when the ledger is the thing making the
claim.**

## 2026-08-24 — THE GAME WOULD NOT BOOT, AND FOUR TESTS ALREADY SAID SO

Jon ran `./run_game.sh` and got a panic before the first frame: *"Error when
initializing schedule Update: schedule has 0 before/after cycle(s)"* — Bevy
detecting a strongly-connected component and then failing to name a single node
in it, which is the least useful diagnostic in the engine.

The cycle was fifteen hops and one new edge closed it. The merged defense
presentation selector asked to run `.after(activate_prepared_platformer_sessions)`
AND `.before(PresentationVisualSync)`, and the host's own frame chain already runs

```text
PresentationVisualSync → RoomTransitionCoverSet → Observe → Activity →
ActivitySignals → Drive → Input → Actions → process_shell_presentation_events →
Pending → Bridge → Providers
```

⇒ **`PresentationVisualSync` runs EARLIER IN THE FRAME than session activation**,
so "after activation, before the visual sync" is not late — it is impossible. Its
two siblings (`select_active_presentation_profiles`,
`select_active_hud_declaration`) only claim `.before(GameplayPresentationSet)`,
and dropping the extra edge is what the third one wanted too: the cue systems
read the policy published on the previous tick, exactly as they do.

⭐⭐ **THE GUARD WAS ALREADY THERE AND HAD NOT RUN.** Poisoned the edge back in:
four `boot_budget` tests go red immediately, because they boot the shipped
visible composition. They had not run because `cargo test -p ambition_app` was
failing to LINK on a stale artifact (`undefined symbol:
ambition_app::app::cli::run_shared_host_acceptance_cycle`) — cleared by touching
the crate source.

⇒ ⛔⛔ **`cargo check --all-targets` TYPE-CHECKS AND DOES NOT LINK.** That is a
second, different blind spot from the dependency-`cfg(test)` one below: this time
the test target existed, was in the gate's own package, compiled — and could not
be built into a runnable binary. A schedule that cannot be constructed is
invisible to every `check`, because nothing constructs it.

⭐ when Bevy names no node, dump the graph: `schedule.graph()` exposes
`dependency()` and `hierarchy()` publicly, and flattening set edges onto their
member systems before running Tarjan finds the cycle in about a minute.

## 2026-08-24 — D200's P2 consolidation, and two things nothing could see

D200's correctness half closed the day before; this pass worked the P2 list the
review sequenced after it. Every slice is one commit.

```text
§8a out-of-shield   ✔ 4a70be7e0  the rule was implemented TWICE — gate now in core
§8b capture pose    ✔ e1382cde2  one system registered twice → two named phases
§8c authoring fork  ✔ 3227a79f1  the fork hid nothing: one clip fallback, unreached
§8d fighter kits    ✔ f18cd26cb  a ruleset does not own a kit — moved to roster prep
§8e sheet identity  ✔ 6e0c37c15  assigning the key destroyed the rig target
D175 input bridge   ▢            no customer; its own doc names the condition
§4a control gate    ▢ 9df938a14  D202: control is published TWICE, so the pair is
```

⭐⭐ **THE THING TO CARRY FORWARD: TWO DEFECTS THIS PASS WERE INVISIBLE BECAUSE
NOTHING COULD SEE THEM, not because anything was wrong.**

`cargo check -p ambition_app --all-targets` is the gate and it does not build
another package's tests — **because it builds every DEPENDENCY AS A LIB, without
`cfg(test)`.** So the app's own lib, bins and tests are covered and every
dependency crate's `#[cfg(test)] mod tests` is not.

⛔⛔ **AND THIS PARAGRAPH DID NOT STOP IT HAPPENING AGAIN, later the same day.**
A field added to `MoveGates` compiled clean through the whole app gate while a
dozen `src/**/tests.rs` across the monolith were broken; it surfaced only when
something else ran `cargo check --all-targets -p <that crate>`. ⇒ knowing the
rule is not the same as running the command. After touching a type that CROSSES
CRATES, name them:

```bash
cargo check --all-targets -p ambition_platformer2d_core -p ambition_combat \
    -p ambition_characters -p ambition_platformer2d_actor_monolith -p ambition_app
```

⇒ **and it had already cost 1,157 tests once that day.** Under the same blind
spot the actor monolith's own test target had stopped COMPILING — a parameter
added to `apply_body_hit_reaction`, a membership row written as a 3-tuple, an
`AutolinkFollow` field renamed — and the one test that then failed had drifted
three ways behind the production chain. ⇒ when you touch a crate, run ITS tests;
the app gate is not a proxy for them.

And the fighter brain could not SEE a ledge hang. Jon: *"A character can just
stay on the ledge, and there is no way to knock them off."* Two separate causes,
both live: the generic ACTOR damage road never called `knock_off_ledge` (only the
PLAYER road did, and in an arena the roster is all actors — `c3d7cdba7`), and
`PerceivedActor` carried no hang fact at all, so the single most punishable state
in the genre classified as ordinary `Neutral` (`5cd004276`). ⚠ the second one
makes `Situation::EdgeGuard` live for the first time — it could previously only
fire against an opponent already past the blastzone — so CPU match distributions
will move, and that is the change, not a regression.

⛔ **D201's first draft named the wrong gap.** It said the getup vocabulary was
missing an attack; `ledge_grab/runtime.rs` has six options bound, none gated.
Written from the report's framing instead of from the source — grep first.

## 2026-08-23 — the smash correctness closeout landed, and six emergent tests lied

Two GPT reviews, twelve named defects, all closed. The ledger row is **D200**;
this is only the orientation a cold start needs.

```text
grab lock         ✔ abdc086a1  mutual capture deadlocked 28% of a mirror match
strike pulse      ✔ abdc086a1  one swing lands once across sibling volumes
buffered Special  ✔ abdc086a1  Up+Special kept its direction through endlag
charge payoff     ✔ d2004b335  the timeline road paid the FULL multiplier, always
CPU charge timing ✔ bbcf7b5a7  held Attack to the first HIT, not to the freeze
charge pose       ✔ ff4d06847  a held charge stood inside its own live hitbox
                  ✔ 56c480611  ...and all six smashes AUTHOR their pose now
throw edge        ✔ 3cebefd62  a direction held through the grab threw instantly
neutral dodge     ✔ 814c2d535  it never cost the charge; the revert was wrong
Mary-O i-frames   ✔ 7ef70de18  quasar AND the shared blink, stacked
capture matching  ✔ 829a7067b  no body is both captor and captive, order-free
impact hitstop    ✔ 56c480611  CPU-vs-CPU connects stop the world now
```

⭐⭐ **THE ONE THING TO CARRY FORWARD, and it is not on that list. SIX EMERGENT
MATCH TESTS MISATTRIBUTED A CHANGE IN ONE PASS.** `every_authored_route_gets_pressed`
blamed George's recovery for a charge-payoff change and again for a charge-pose
change; `the_cpu_charges_a_smash…` blamed the neutral dodge for a failure a CPU
cannot reach (its Dodge verb aims its stick, and the clause only fires on a
neutral one); `two_emmys_hold_a_mirror…` failed a PERFECT 856/856 mirror against
a sloppy 440/1376 one by comparing absolute frames across matches of different
lengths; and the jab-string probe accused the cancel chain when the human simply
had not landed yet. Every one is a match-DISTRIBUTION measurement read as a
MECHANISM failure.

⇒ **distrust any "the CPU stopped doing X" failure until a targeted fixture
agrees.** `a_fighter_brain_charges_a_smash_through_the_real_chain` is the shape:
brain → gesture → move → `MoveCharge` → frozen fraction, one motionless
opponent, no sampling in it.

⚠ **and a guard can be GREEN for the wrong reason.** The capture-chain test
survived deleting the clause it existed to protect (a chain's second edge is
refused by a different check), and its fixture had made two of its three bodies
allies — so with friendly fire off, the "chain" it named had been refused at
resolution and never existed. Poison the CLAUSE, not the file.

⭐ **the rollback wire format may grow now.** `rollback-wire-format-is-frozen` →
`rollback-wire-format-changes-are-declared`: drift is still caught in both
directions, but growth is legitimate when the baseline and
`GGRS_ROLLBACK_SCHEMA_VERSION` move together. The shrink-only rule was inherited
from `central-rollback-ownership-may-not-grow`, a MIGRATION constraint that
outlived its condition. Schema is at **76**.

## 2026-08-21 — D175 is CLOSED, and player two can fast-fall

```text
feel-clock latch  ✔ 889107010  SlotControlLatches — seat zero is row zero
pending input     ✔ 477fc8693  PendingSeatInputs — handle zero included
raw producer      ✔ 249af69b0  one system decides every seat's frame
raw SHAPING stage ✔ 0dab21479  SeatRawFrames — one row per seat
confirmed publish ✔ 0dab21479  SlotControls for everybody
```

**The global `ControlFrame` was the input bus**: a device wrote it, four systems
shaped it, one copy fanned it into `SlotControls[0]`. That is why shaping was
seat zero's *by construction* — every other seat went from its `ActionState`
straight into its own slot with no stage between — and why the secondary frame
producer could hardcode `fast_fall_pressed: false`. `SeatRawFrames` is that stage
for everybody; `ControlFrame` is now seat zero's OUTPUT MIRROR.

⭐ **the deletion ledger is where the size of the fork shows.** Six `ControlFrame`
entries in the workspace policy allowlist stopped holding the resource at all,
and the policy's `Bridge` vocabulary lost `FrameToSlot` for `SlotToFrame` —
**the direction reversed**, so the old category would now be a cycle. Gone:
`populate_slot_controls`, `accumulate_control_frame_latch`,
`publish_latched_control_frame`, the `handle == 0` branch in `publish_ggrs_input`,
`drive_slot_frame`'s last seat-zero arm, and three hand-written copies of "which
body drives this slot".

⛔⛔ **THE ROW'S OWN PLAN WAS WRONG ABOUT WHERE THE DERIVATION RUNS, and checking
that is what made the fix small.** It said the gesture derivation is on the FEEL
clock and must not move after publication. The `InputTimersAdvanced` set is registered
into the SIM schedule, which under a rollback host **is** `GgrsSchedule` — it
runs inside rollback, with `SlotInteractionState` as canonical rollback state.
The clock argument was sound; the placement claim was not.

✔ **and the row is CLOSED — the step I had written down as remaining was
falsified by one grep.** I had planned a `SlotGestures` split so the gesture
derivation could move into the `Update` device window. `bevy_ggrs-0.21.0`'s
`time.rs` replaces `Time<()>` with `Time<GgrsTime>` for the duration of
`GgrsSchedule`, and that clock is `advance_to(frame / framerate)` from
`RollbackFrameCount` — derived from the frame number and itself
rollback-snapshotted. ⇒ `Res<Time>` in the sim schedule is deterministic and
rewound, and the derivation is already in the right place; moving it to `Update`
would have put it on the WALL clock and created the desync the move was for.

⇒ what was left was a DEFINITION. `InputSet::Route` said *"every system that
WRITES the `ControlFrame` resource lives here"*, a rule identical to its purpose
only while one global frame WAS the input. Its real content is the ORDERING —
everything that shapes a seat's frame before the publication boundary — which is
why the gesture derivation and the interact buffer are still members although
neither touches that resource.

**Also this session.** The camera resolve stopped searching control authority — a
`DrivingParticipant` query had been folded into another parameter's tuple to fit
Bevy's 16-param ceiling, which satisfies the limit without reducing what the
system knows about; `ResolvedViewSubject` is a stage now and the resolve dropped
to 11 params. `drive_seat_frame(PRIMARY, …)` used to `return` silently, and the
SDK acceptance test asserted that it should — a test whose whole content is "it
did not panic" agrees with a function that does nothing. `run_game.sh` learned
`twintrack` (the fourth demo shell, previously reachable only by hand-writing the
cargo invocation), and `--ticks` stopped being ignored by the smash shell.

⛔⛔ **AND I SHIPPED D175 WITH 20 `app_it` TESTS RED.** The background job that
ran them printed two suites; I greped the output, read `test result: ok. 187
passed` off the FIRST one, and never saw the twenty `... FAILED` lines below it.
The task notification said *exit code 0* because the command ended in `| grep` —
the pipeline's status is the grep's. ⇒ **grep for FAILURES and read the LAST
result line**, and treat a piped test command's exit code as meaningless.

The defect underneath was one wrong predicate. `SeatRawFrames` is this tick's
input only on a frame-stepped host; a latch host has drained into `SlotControls`
already, and a rollback host had it published there by the SESSION. Every stage
that read the raw row inherited that, and the gesture derivation wrote what it
read back into the slot — so under rollback it overwrote GGRS's confirmed input
with a neutral row and both seats went silent through a rewind. Asked once now:
`another_authority_publishes`, with `seat_frame_this_tick` to read and
`shape_seat_frame` to write. Reads pick the authoritative table; writes go to
both, because which is authoritative depends on the host and which a shaped value
must reach does not.

⛔⛔ **THE RECURRING LESSON, and it is the sharpest kind of unfalsifiable check.**
Three Mary-O fixtures asserted *"a scripted press did not survive into the
simulation"* by reading `ControlFrame` — **the resource the scripted stage writes
directly.** None could fail for the reason all three named. They only became real
when that resource changed role, and immediately reported a one-tick lag they had
never been able to see. ⇒ ask of any liveness check: is the thing I read
DOWNSTREAM of the thing I drive, or is it the thing I drive?

✔ **both long-red `ambition_demo_mary_o_app` tests are CLOSED** (D181, D182) —
39 pass, 0 fail. One was a fixture setting a body down at a guessed height; the
other was the chase walking small Mary-O into a snake, where the "stall" was her
corpse and `halt_body` was doing its job.

⛔⛔ **AND FOUR MECHANISMS WERE PROPOSED ACROSS THOSE TWO ROWS; THREE WERE
WRONG.** Embedded in terrain (the collision layer is flat and she overlaps
nothing), pinned and immobile (a differential at two x-positions shows identical
walking and jumping — the "pin" was two-tick input latency read off a probe that
printed before it stepped), and rising through the block without striking
(`Head/Block` fires at exactly the underside). ⇒ **A DIFFERENTIAL BEATS A
THEORY**: every refutation came from running the same code at two inputs, never
from reasoning about one. ⛔ do not publish a mechanism in the same commit as a
fix unless the fix depends on it — this fix was correct under all three wrong
stories, and only the prose needed retracting.

⚠ **also long-red and outside the gate**: three `ambition_workspace_policy`
engine-policy rows (`body_step.rs` direct `kinematics.pos`/`vel` writes, and a
required path `time/feel.rs` that no longer exists), and the two consumer
fixtures under `fixtures/` are separate workspaces the repository gate cannot
reach — `minimal_game` had been red for eight days on a deleted catalog field.

## 2026-08-21, latest: four competing-authority defects, from a review of `f8ad04f9a`

⭐ **all four were CONFIRMED at source before any code moved**, and the two the
review got right about severity were the two nobody had a test for.

**Landed.**

```text
5902930a7  an input SEAT is not a match SLOT. The smash select screen keyed
           cursors by seat (right — a hand belongs to a person) and then used
           that index as the ROSTER CARD. With a CPU between two people,
           first_free_device gives card 2 device 1, so the second person drove
           the machine's card and their own was unreachable. Also: any human
           could grab any CPU token and nothing arbitrated, so two cursors
           carried one piece. SmashSelect::slot_driven_by + SelectCursors::
           try_grab; SelectCursor::grab is now private so it cannot be bypassed
79f465e62  two bodies may not both SPEND one gap. Every body_contact test was
           one mover against blockers standing still, so nothing saw two movers
           each granted the whole gap: 5 apart, 4 asked each, closed 8. And
           resistance could not save it — the free-gap part of a step is
           granted at full speed by construction, so it happened at 1.0. The
           gap is now DIVIDED in proportion to closing speed
3b804b947  a demo does not own where its cameras land on the glass. TwinTrack
           cleared MainCamera.viewport every frame against the generic owner
a50c7ea12  a claim given back by VALUE EQUALITY is not owned. DeclaredInputSeats
           and the InputAssignmentPolicy resource became one owned
           LocalSeatOffer; two Local<bool> flags and two "if it still equals
           what I wrote" tests are deleted with them
```

⛔ **the shape worth carrying forward, because it appeared three times in one
day:** a claim released by comparing the resource to the value you wrote is not
ownership, and no test whose successor claims DIFFERENT values can see it. Ask
*is this mine*. `SessionSeatingSource::release` had the right shape the whole
time, two lines below the broken code.

**Opened by the same review, and two closed the same day.** D178 landed
(`0ebcef4e4`): `ViewParticipant(PlayerSlot)` resolves a pane through the body
carrying `DrivingParticipant(slot)`, so a person's pane follows the person.
D177 got its fixture (`twintrack_split_has_two_viewports`, the first test of
TwinTrack's split on a real display) and then closed as NOT A DEFECT — see the
receipt in the ledger. **D179 is the one still open**: contact eligibility is
inferred from displacement MAGNITUDE rather than provenance, plus the
propose/commit residual the gap split could not reach.

⛔⛔ **the most expensive lesson of the day is in D177's receipt and it is not
about cameras.** Four eliminations in a row were unsound — each one's falsifier
was itself broken, and each was caught only by the next probe. When a bisect
keeps eliminating every candidate, the bisect is lying: stop and ASK THE TOOL.
`--features bevy/track_location` plus `Ref::changed_by()` named the writer in a
single run after four rounds of guessing. And the number that opened the row was
right while the sentence after it was wrong — it compared a live route against
an idle one and called the difference "view count".

**Deliberately not done:** D175 remains the largest named architecture item —
seat 0 rides the global shaped `ControlFrame` bus while seats 1+ publish
straight to `SlotControls`, so any new semantic shaper is primary-only unless
someone remembers to write it twice.

## Also 2026-08-21, later: content, guards, and the carve's PRICE

⚠ a second session ran beside the carve above. What a cold start needs from it:

**Landed.** Every fighter authors all four throws (`f3611b93d`), and the roster
ratchet grew 16 → 22 to cover grabs. Seat declaration moved out of the GGRS
backend into `ambition_input` (`d8994ce92`) — TwinTrack's second seat drives, and
the bug it fixed was *host-only*, invisible to every demo-binary test. The LDtk
`EdgeExit` reachability rule now reads the Collision IntGrid; it had been scanning
`Solid` ENTITIES and **could never fire** (15 levels vs 4, intersection empty).
A misspelt loading-zone `activation` is refused instead of silently becoming a
`Door`.

**Open and worth knowing before you pick a row:**

```text
Jon's interact-door bug   NOT reproducible in the sim harness, the demo
                          binaries, the shell host, under rollback, or with the
                          touch overlay. Five suspects eliminated by
                          measurement. Next probe is machine-local:
                          AMBITION_DATA_DIR=$(mktemp -d) ./run_game.sh
D174                      a 16px floor lip inside FIVE EdgeExit zones — you
                          must jump into the hub's contact exits. Content fix
                          or soften the contract; Jon's call
D175                      nine participant-input items, promoted from a doc no
                          ledger row could reach. Its first item is the SEAT-0
                          SHAPING BUS, which is why couch bugs keep recurring
TwinTrack                 PARKED, with a known limit: two panes render ONE
                          coordinate-time slice, so they can disagree about
                          optics and never about simultaneity
```

⛔ **and the carve's price, measured — read this before proposing one.** More
than half the monolith's modules hold rollback-registered types, so they cannot
move without rewriting the wire format. Worse, the cheap-by-coupling modules are
cheap *because* they are assembly or observers — which is exactly what the
monolith is finished AS. Five candidates were taken to a verdict; four were
refused by their destination's own stated contract. D33 carries the table, the
pre-flight command, and the reasoning.

## What moved on 2026-08-21: the monolith carve, by DOMAIN

Jon, that day: *"loc is the proxy. the real win is conceptual domain
separation."* D33 is where this lives, and the row now leads with that.

**Landed** — each with gate + 31/31 absence contracts + smash + `app_it` green:

```text
ActorSurfaceState            -> ambition_platformer2d_core   6c4592021
feel tuning                  -> ambition_combat              d6db434f4
hit reaction + stance        -> ambition_combat              403a32155
capture systems (1,922 ln)   -> ambition_combat              8669740f5
footstool (859 ln)           -> ambition_combat              00030e603
hit camera shake             -> ambition_combat              23755e201
boss animation               -> ambition_boss_encounter      9ea8ea2fa
PickupKindSpec               DELETED (was PickupKind)        d3bd6e95a
```

⭐ **the reusable part is the METHOD, not the moves.** A carve's coupling is not
what a file imports: strip comments, grep BOTH `crate::…` and `ambition_…::…`,
resolve each symbol to its defining crate. Three of these needed a retry because
that was done partially — and a whole 1,922-line domain turned out to be pinned
in the wrong crate by one `pub(crate)` function, not by any real coupling.

⚠ **and two scans produced confident-looking rankings of NOTHING** ("quest"
matching inside "request"; `super::super::` counts that never cross a crate).
Both are corrected in place with the negative recorded, because a list left
standing gets worked. ⛔ check the top hit by eye before believing any of them.

⇒ next: `attack.rs` and `limbs.rs` are both measured and BLOCKED — one on a
dependency decision costing five lockfiles, one on a three-way split across a
crate ordering constraint. Read D33 before picking either up.

## Major closure: D73 is finished

The authority-convergence campaign closed on 2026-08-13. The live architecture
no longer has an enemy `ArchetypeSpec` / `CharacterRoster` body authority or a
build-legacy-body-then-patch character road. Intrinsic body/capability facts come
from authored/prepared `CharacterDefinition`; placement, disposition,
controller, participant and ruleset facts remain contextual.

The migration working memory is archived under
[`../archive/planning-superseded/2026-08-13/`](../archive/planning-superseded/2026-08-13/).
Do not reconstruct deleted D73 representations because an archived review names
them.

## Current architectural direction

The successor umbrella is
[`engine/engine-1.0-architecture-program.md`](engine/engine-1.0-architecture-program.md).
The goal is a credible Godot/Unity-class 2D engine on Bevy while **Ambition
remains the flagship game and primary product driver**.

The highest-value successor fronts are, **in priority order** — ⚠ this list is ORDERED, and it was reordered on 2026-08-15 because the systemic-world
substrate had overtaken the two fronts printed above it:

1. **⭐ THE SYSTEMIC WORLD SUBSTRATE — the next major frontier, and PRIMARY
   CAPACITY GOES HERE** (D125). What a thing IS, which runtime occurrence it is,
   why it exists and how long it lasts; then item custody as the first demanding
   consumer, then capability-driven gating and reachability, then residency and
   persistent populations. Its seven focused plans are reachable from
   [`tracks.md`](tracks.md).

   ⭐ **status 2026-08-20: the substrate EXISTS** under names the plans do not use
   — `WornCharacter` (authored template), `SimId` (runtime occurrence),
   `SpawnOrigin` (provenance) and four ENFORCED lifetime scopes. Custody, item
   ownership, and all three persistence horizons (current world truth, the
   checkpoint/reset ledger, and durable save) are landed and distinct:

   * ✔ inventory ownership is settled (Jon's reviewer, 2026-08-15): the **body**
     owns its inventory and capabilities; `OwnedItems` is a migration/
     compatibility projection, not an undecided authority.
   * ✔ a held object's identity is the authority; the catalog only projects a
     count (`284ebd00d`). Held objects and pure-quantity items are disjoint
     populations, so a pickup can no longer mint a duplicate.
   * ✔ persistent occurrence continuity (`Placed` rows), the checkpoint/reset
     horizon, and durable save (`AmbitionGameSaveData` carrying
     `AuthoredOccurrences`, `CustodyBaseline`, `MintedItemBaseline`) are all
     landed. A durable description of a runtime-minted occurrence is exactly
     identity + `SpawnOrigin` + a definition reference — no position, no
     component snapshot (`88b611caf`). Headless compositions now install
     `DurableSaveHorizonPlugin` themselves, so an RL episode persists too.
   * ⛔⛔ a relation may not cross the durable horizon without its own authority
     (2026-08-20): `InCustodyOf` has two owners (item custody is durable,
     `PossessionState` is not), so the mirror now writes an `InCustody` claim
     only for occurrences the durable road can restore.
   * ▢ open: `Consumed` round-trips through the file with no live producer yet
     (load-bearing for `AuthoredOccurrences::rewind_argument` — a real open
     design item). The body resumes at the shrine while objects resume at the
     autosave's instant — two different times in one load, a deliberate
     first-slice trade, not an oversight.

   ⛔ **do not promote easy actor-monolith leaf carving ahead of this.**
2. **Simulation authority and determinism.** Decompose parameter-ceiling systems
   by phase/authority and invert rollback declaration ownership. See
   [`engine/simulation-authority-and-determinism.md`](engine/simulation-authority-and-determinism.md).
3. **⭐ NEW 2026-08-15 — deterministic authored gameplay logic and orchestration**
   (D127). Authoring is strong for **nouns** and weak for **verbs and
   relationships over time**; several independent partial condition → effect
   systems already exist in tree. **Rust extends the engine's vocabulary;
   authored content composes vocabulary that already exists.** See
   [`engine/authored-gameplay-logic-and-orchestration.md`](engine/authored-gameplay-logic-and-orchestration.md).
   ⛔ not scripting, not a rule VM, not a central effect enum — the substrate
   owns no universal sequencer, and boss patterns are the **template**, not a
   customer.

   ✔✔ **M1 IS MET FOR CONDITIONS, with two unrelated consumers.**
   `shared_tangle::authored_logic` owns the contract — `publish` is PRIVATE, the
   only way in is `PublishCondition for App`. Three domains publish
   (`custody.is_held`, `world.flag_set`, `inventory.holds`); a gated lock wall
   and authored `.yarn` dialogue both consume through one generic verb
   `condition("domain.question", <arg>)`, so publishing a condition makes it
   askable from dialogue with no edit to any bridge.

   ⛔⛔ **this also refuted the premise behind `YarnStateMirror`**: Yarn library
   functions CAN be Bevy systems and reach `&World` (`bevy_yarnspinner` advances
   the interpreter from an exclusive system; `SystemId<In<P>, O>` implements
   `YarnFn`). The mirror shrank to a projection rather than a feed.

   ⇒ **commands are a different shape than conditions, established rather than
   assumed**: a condition is safe to call from inside the interpreter precisely
   because it cannot change anything; a command mutates, so `<<give_item>>`
   records a REQUEST rather than granting. A `PublishCommand` contract owes
   authority, ordering and a ledger-shaped replay story, and generalises from
   `NarrativeInputPlugin<M>`, not from the condition catalog.
4. ⏸ **Ambition authoring + kinematic world objects — RESTING (D115, K2–K6 all
   closed).** Treat authoring/tooling as
   an engine product, improve LDtk as a first-class spatial compiler surface,
   and use moving platforms as the first vertical slice. See
   [`engine/authoring-and-tools.md`](engine/authoring-and-tools.md) and
   [`engine/ldtk-authoring-and-world-tools.md`](engine/ldtk-authoring-and-world-tools.md)
   and [`engine/kinematic-world-objects.md`](engine/kinematic-world-objects.md).
5. ⏸ **Ambition multiplayer + multi-view presentation — RESTING (D116).** Support local, online and
   mixed participants independently of shared/fixed/adaptive split-screen; grow
   toward multiple resident rooms when participants separate. See
   [`engine/multiplayer-and-multiview.md`](engine/multiplayer-and-multiview.md)
   and [`game/multiplayer.md`](game/multiplayer.md).

   ⏸ **D116 RESTS (2026-08-15), and M2 is only HALF done** — say it in two parts.
   ✔ **closed:** the presentation/projection sub-slice — per-view association and
   viewport application are proven by an assembled-host fixture, and both
   `PresentsView` writers that guessed are fixed. ▢ **deferred:** production
   two-view composition and layout — production spawns one camera and publishes
   one screen rectangle to every view **by construction**, and M2's own plan also
   names HUD ownership and input routing, which this slice did not touch.
   ⛔ do not expand into networking; the deferred half needs a real product need
   for a second view.
6. **Capability/runtime composition** (D136). Make optional capabilities honest
   in dependency and composition topology. See
   [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md).

   ⭐ **the through-line: each gap is a place where "who is this for?" was
   answered by whoever installed it first and never written down.**

   ✔✔ **`DeathRules` fixed (2026-08-16, `03d4c8d22`).** It was a bare `Resource`
   inserted at plugin-build time by three games, so the shell's Mary-O-after-
   Sanic composition order made every Smash match run under her 3.2s level
   replay. Fixed by declaring into `DeclaredDeathRules` under the rooms a game
   governs, using `runtime::mode_scope` (which already scopes a hosted game's
   systems and entities) — a second claim on one scope panics at build.
   ⇒ **the lesson: when a scoping concept exists, ask what KINDS of thing it
   scopes, not whether it exists.**

   ⛔ **the standing number, re-measured 2026-08-18: 44 crates linked, 17 a
   movement-only game never asked for** (`capability-footprint-may-not-grow`,
   printed by `check_absence_contracts.py` on every run — read the contract's
   own output rather than quoting a stale copy). The monolith is now off the
   `ambition_platformer2d_ldtk` holder list (production code names it zero
   times; the crate builds `--no-default-features`); the runtime is the
   remaining holder, and the footprint number itself has not moved because the
   dependency was already declared optional. ⇒ a slice claiming this front must
   run `cargo tree -i` for the crate it means to evict before picking what to
   carve, and must say what it did to the number or why the number is
   dominated by something it did not touch.
7. **Public SDK, authoring ergonomics, performance and iteration.** See
   [`engine/public-sdk-1.0.md`](engine/public-sdk-1.0.md) and
   [`engine/performance-and-iteration.md`](engine/performance-and-iteration.md).

⚠ **the browser is a TEST FIXTURE, not a front** (Jon, 2026-08-14). It is a
powerful architecture probe while the engine is decomposed — it found a shipped
composition that differed from desktop's and a developer instrument that was
load-bearing for gameplay input — but it does not decide which subsystem gets
built next. ⭐ **the test for any tempting performance task: would we want this
abstraction if the web target disappeared tomorrow?** Semantic asset readiness,
cross-platform phase telemetry, canonical asset publication, host-owned input and
an explainable load barrier all pass it. Brotli, wasm audio scheduling, Hall
streaming, a generic residency scheduler and byte shaving do not.

## Product and engine customers

- **Ambition:** flagship game. Its real content, authoring, multiplayer,
  persistence and presentation needs have first claim on product value.
  ⭐ its structural hub is [`game/ambition.md`](game/ambition.md) — the game and
  engine co-evolve, and it is **not** a thin demo waiting for a finished engine.
  From there: [`game/vision.md`](game/vision.md),
  [`game/open-world-roadmap.md`](game/open-world-roadmap.md),
  [`game/systemic-progression.md`](game/systemic-progression.md),
  [`game/multiplayer.md`](game/multiplayer.md). ⚠ nothing linked that hub until
  2026-08-15, which is how the flagship customer's own map went unreachable.
- **Super Smash Siblings:** active platform-fighter product push and possible
  future first-class game. Start at
  [`demos/super-smash-siblings.md`](demos/super-smash-siblings.md); the canonical
  current feature inventory is
  [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md), and the
  current execution campaign is
  [`demos/campaigns/smash-fun-push-2026-08-22.md`](demos/campaigns/smash-fun-push-2026-08-22.md).

  Core body-generic combat, shields, capture/throws, movement/contact, stocks,
  respawn, participant routing, and deterministic match paths are live. The
  remaining platform-fighter depth is now tracked feature-by-feature. Small
  reusable semantics may be added with the product feature; broader engine
  campaigns are called out explicitly rather than gating the whole Smash push.
- **TwinTrack:** ⭐ **a TWO-PLAYER game with a real split screen as of
  2026-08-20**, and the pressure test that paid for three engine seams. The
  laboratory twin is Emmy No-Ether, a constructed character body driven by seat
  one; the screen is split by construction, one gameplay `LocalView` per
  participant. What it bought the engine: `ambition_sim_view::ViewPlacement`
  (where a view sits, as a fraction of the gameplay rect),
  `ambition_sim_view::ViewSubject` (which body a view frames — the resolve
  answered that ONCE above the per-view loop), and `spawn_main_camera` declining
  to spawn a rig it cannot honestly bind. Both relativity read-models publish one
  row per observer and TwinTrack is their first adopter, so the two panes'
  numbers disagree because the physics does.

  ⛔ **two rules it cost to learn.** How many views there are belongs to the LIVE
  SESSION, not to what the binary links — composing a second view at plugin build
  time split every route in the game, and the symptom was `bevy_egui` panicking
  about schedules in 95 unrelated tests. And two seats is TWO statements:
  `DeclaredInputSeats(n)` makes seat entities, `InputAssignmentPolicy::JoinToClaim`
  gives them devices, and a surface that says only the first gets a dead second
  seat — measured on Jon's hardware.

  ▢ which pane shows which observer's aberrated sky is a presentation choice
  nobody has made; the seam is there.
- **Sanic / Super Mary-O / Hollow Lite:** retained acceptance customers for
  movement, classic platforming/content, and encounters/boss authoring.

An acceptance customer may eventually become a first-class game. That changes
its product investment, not the engine ownership rules.

## Durable architecture to remember

- one body, one path;
- character definitions own intrinsic reusable body composition;
- controllers provide intent rather than defining a body species;
- construction/preparation fails before partial mutation;
- deterministic simulation authority is explicit and snapshotable;
- views are local presentation over one simulation, not duplicate worlds — and
  **how many there are is a property of the live session, never of what the
  binary links**;
- **a capability is not landed until something ADOPTS it**, and a `Deref`-to-the-
  first-row fallback is how an unadopted split hides: every old reader compiles,
  reads identically while there is one row, and silently switches to whatever
  the publisher sorts first the day there are two;
- transport, control assignment, world residency and view layout are independent
  axes;
- LDtk is Ambition's preferred spatial authoring surface and should improve when
  real Ambition content outgrows it;
- the actor monolith is drained by coherent ownership, not line-count quotas;
- public APIs should expose game concepts rather than historical crate topology;
- **a relationship may not cross the durable horizon without its authority** —
  the save may only claim what the load can reconstruct, and a generic component
  gaining a second population enrols that population in every generic sweep,
  persistence included;
- **a set of lanes is a composed value, not a repeated one** — when a second
  customer of a federation arrives, the enrollment cost it MEASURES is the
  evidence for a composition owner; make it a plain struct whose every operation
  destructures exhaustively, so the carry list is one the compiler keeps, and
  keep the dynamic machinery out (`Any`, `TypeId`, registries, service locators
  trade a compile error for a runtime lookup).

## Explicitly deferred, not abandoned

- production online transport/Matchbox work should grow from an actual
  multiplayer slice rather than be built speculatively;
- Slower Light remains a future 3D relativity game;
- water/oil extensions to falling-sand remain desired deferred product ideas;
- the Leafwing clash-scan optimization remains trigger-based maintenance.

## Where to look next

1. [`queue.md`](queue.md) for execution order.
2. The focused plan named by the selected row.
3. [`JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`](JONS_OBSERVATIONS_BUGS_AND_ISSUES.md)
   for direct maintainer observations.
4. [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md) only when
   an actual product/feel decision is required.
5. [`tracks.md`](tracks.md) when replenishing the queue.
6. `docs/concepts/`, `docs/systems/`, `docs/architecture/` and `docs/adr/` for
   settled truth; `docs/archive/` for history.
