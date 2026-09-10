# Advanced fighter brain — evaluation, difficulty and regression contract

**State:** ARCHITECTURALLY STABLE / PRODUCT CALIBRATION OPEN — the major brain
architecture regressions have been diagnosed. Remaining work is representative
evaluation, explicit difficulty/roster decisions, and product tuning backed by a
trace that names the responsible decision.

This page is the current brain/evaluation contract. The large fixed-seed tables,
null-control runs, superseded ladder hypotheses and investigation chronology are
preserved by git history and measurement artifacts rather than in the live plan.

## Scope

This page owns:

- fighter difficulty/profile semantics;
- evaluation-rig representativeness and provenance;
- deterministic regression methodology;
- no-cheat constraints;
- rollout/shadow-model requirements specific to fighter decision quality;
- current brain calibration decisions.

Generic navigation/recovery capability is owned by
[`platformer-navigation-and-reachability.md`](platformer-navigation-and-reachability.md).
Performance cost outside fighter decision architecture belongs in
[`performance-and-iteration.md`](performance-and-iteration.md).

## Current architecture

### The brain acts through normal perception and actor control

Higher difficulty may improve what the brain decides and how reliably it commits.
It may not bypass actor control, use privileged future state, ignore physical
constraints, or directly scale combat outcome merely to make a rung win.

The brain receives semantic resolved facts rather than reimplementing core body
rules from lower-level state wherever possible.

### Difficulty and player-facing gameplay difficulty are different systems

Brain difficulty/profile parameters may change:

- reaction delay;
- decision cadence/APM;
- commit probability;
- accuracy/noise;
- policy/scoring weights;
- planning breadth/depth where enabled.

`ambition_persistence`'s player-facing gameplay `Difficulty` may legitimately
change damage multipliers. Do not confuse that product setting with the no-cheat
rule for fighter AI profiles.

### Recovery uses the shared body capability

The level-6 recovery regression was an integration/decision bug, not evidence
that fighter AI needs a private navigation engine. Keep the reusable recovery
probe/`RecoveryLens` authority and integrate fighter decisions against it.

### Rollout/shadow simulation must call canonical rules

A shadow state may not guess a future maneuver when the real body resolves that
maneuver from cooldown/budget/grounding state.

The `Dodge` lesson is the durable rule: if a semantic verb can resolve to roll,
air-dodge, dash, etc., the shadow path should call the same resolver on
shadow-stepped state or decline to model it. Do not expose hidden inputs merely
so a second implementation can re-derive the rule.

## Evaluation-rig contract

A table is not evidence about the shipped fighter unless the run states the
inputs that define the fighter and scenario.

Every report must print/record:

- exact Git HEAD;
- fighter ladder/profile source path;
- fighter/roster identities and movesets;
- stage/fixture;
- paired/unpaired design;
- difficulty/rung;
- decision/scoring weights or source thereof;
- seed set/count;
- bout clock/time limit;
- relevant feature/profile toggles.

Defaults must appear in the output even when the caller did not pass them.

A parse/load failure for an explicitly requested shipped ladder must fail the
run rather than falling back to a different ladder.

## Regression methodology

### Trace before tuning

Do not tune rollout depth, heuristics, APM, reaction, accuracy or utility weights
until a trace identifies the responsible decision/path.

Required workflow:

```text
reproduce
-> trace decisions/options/facts
-> identify the decision seam
-> make one targeted change
-> run matched control + changed arm
-> re-run representative scenarios
```

### Use null controls

When a rig change, seat assignment or comparison design is under question, run a
mechanically identical/null pairing. Seat or fixture bias that survives the null
must be separated from fighter-quality claims.

### Use enough clock for the outcome being measured

A short clock that leaves many unresolved bouts measures pace/partial damage,
not final match strength. When the product question is “which fighter wins,” use
the shipped/appropriate match clock and report unresolved bouts explicitly.

### Scenario premises must be real

A static opponent does not measure reaction delay. An unarmed stand-in does not
measure special-move choice. A recovery fixture must include the relevant body
capabilities.

If the premise is missing, fix the fixture rather than interpreting the table.

## Settled findings that still constrain work

### Level-6 recovery integration regression is closed

The trace identified the fighter decision integration as the defect; the shared
recovery capability itself was not the culprit. Preserve that split.

### Level-1 platform-floor regression is closed

Do not reopen it as a generic “low difficulty is broken” item without a new
reproduction.

### The shipped ladder and the engine fallback are distinct authorities

The repository has carried both an authored ladder and an engine/default floor.
Measurements can describe the wrong one unless the run names the source.

The maintainer decision about which authority should remain lives in
[`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md).
Until resolved, every calibration receipt must name the ladder source explicitly.

### Authored `read_weight` is not a normal shipped-axis today

The authored values and the current rollout configuration make its effect
unreachable in the shipped ladder, while the engine floor can make it live.

Whether to wire it into the non-rollout scorer or remove it is a product/design
decision coupled to the ladder-authority decision. Do not tune around it as if it
were already a working shipped axis.

### The middle-rung inversion was traced to utility progression, not reflexes

Matched experiments isolated the problematic ordering to the relevant utility
weights rather than reaction/APM/noise. Candidate progression changes should be
measured as progression changes; do not restart the search over every difficulty
parameter.

### Rollout-specific Dodge/Shield shadow work is low priority while rollout is off

Do not invest in richer rollout shadow modelling merely to improve code that the
shipped ladder currently disables. Reopen when a chosen ladder/profile makes the
rollout a player-visible capability.

### The thin Robot stand-ins are not a representative final roster

They lack the special-kit shape present in authored fighters. Rig conclusions
about match pace or move use must state whether the compared fighters have real
kits.

The actual Robot special identities are a maintainer/content decision, not an AI
architecture question.

## Current work

### F1 — resolve the ladder authority decision

Choose whether the authored content ladder is the required game authority or
whether a reusable engine-floor ladder remains a supported production policy.

Whichever survives:

- every game composition must have one unambiguous source;
- tools print that source;
- fallback behavior cannot turn a failed authored load into a different silent
  experiment.

### F2 — resolve `read_weight`

After F1, choose one:

- integrate it into a scorer that is actually active at the authored rungs and
  add behavior acceptance; or
- remove the field/rollback/config surface if the chosen ladder leaves it inert.

Do not keep an authored difficulty knob that looks live but has no reachable
reader.

### F3 — make evaluation rosters representative

For product calibration, use fighters whose kits represent the game being
shipped. If stand-ins remain useful as controls, label them as controls.

Once Robot kits are decided, add them to the representative ladder matrix rather
than retroactively treating old stand-in tables as roster evidence.

### F4 — finish the utility progression decision

Measure candidate `frame_advantage` / `expected_payoff` progressions against the
representative roster/scenarios.

Acceptance is not merely “rung N beats rung N-1.” Check that:

- ordering improves across representative fixtures;
- play does not collapse into one option;
- pace remains acceptable;
- no-cheat constraints remain unchanged.

### F5 — placement race is correctness, not AI tuning

The `t3` question—whether a match can present a tick in which the followed body
has not been placed—belongs to runtime/presentation correctness. Keep its
reproduction and owner in the relevant planning/queue item; do not compensate in
fighter scoring.

### F6 — move-distribution readability

⭐⭐ **STEP 1 IS DONE, 2026-09-10, and it produced a concrete missing term.**
D-BRAIN-MENU found that `attack_kit_of` resolves presses with
`move_for_directional_verb` while the press road calls
`move_for_attack(.., RUNNING)` — so the kit is MISLABELED while a body runs, and
eighteen of eighteen shipped fighters have a dash attack no CPU can reach.

Making the kit truthful, measured on the duel rig (pirate admiral, rung 9, 3613
ticks, `decided None`, 2 knockouts both ways):

| kit | seat 0 | seat 1 | hitstun ticks |
|---|---:|---:|---|
| mislabeled | 1.26 | 1.07 | [525, 324] |
| truthful | 0.47 | 0.86 | [81, 191] |

⚠ **I FIRST WROTE THAT "the CPU never stops running", THEN MEASURED IT AND IT IS
FALSE.** Stance instrumented on the same duel:

| kit | seat 0 running/grounded | seat 1 | grounded ticks |
|---|---|---|---|
| mislabeled | 258/1908 = **14%** | 353/1822 = **19%** | 1908 / 1822 |
| truthful | 402/1334 = **30%** | 569/1457 = **39%** | 1334 / 1457 |

At HEAD these bodies run 14–19% of grounded time. What the trace shows is a
FEEDBACK LOOP: making the dash attack reachable doubles the running fraction and
cuts grounded time by a third, with damage falling alongside.

⭐⭐ **THE COUPLING BETWEEN THE TWO SCORERS IS REAL, AND ON THIS ROSTER IT IS
INERT — I PROPOSED IT AS THE EXPLANATION AND THE MEASUREMENT REFUSED IT.**
`generate_options` calls `movement_options(&view, situation, !lifts.is_empty())`,
so the ATTACK KIT reaches movement scoring through `lifting_candidates`. That is
the only such wire, and it is the right first suspect. MEASURED across all 19
shipped fighters: that boolean is the SAME standing and running for every one of
them. ⇒ **The wire exists and never fires, so it cannot be what moved the
bodies.** Pinned by `authored_movesets::stance_coupling::no_shipped_fighter_changes_its_lift_availability_with_stance`.

⛔⛔ **BUT IT FIRES SPECTACULARLY FOR ONE WRONG VERSION OF THE FIX, AND THAT IS
THE FULL EXPLANATION OF A FAILURE THIS PAGE SHOULD RECORD.** The first attempt
also redirected SPECIAL to ATTACK while running, which the press road never does.
Poisoning the guard with exactly that mistake reddens EVERY fighter: `bob` loses
`steam_lift`, `carl_stargan` `starstuff`, `goblin` `scramble_leap`, the oni
`smoke_fold`, and so on — **because a fighter's lifting move IS its up-special.**
Collapsing specials strips every body's RECOVERY out of its own kit while
running, `!lifts.is_empty()` goes false, and movement scoring changes for a body
that no longer believes it can get home. That is why the buggy version reddened a
driven body's top speed and a dismount's recovery.

⇒ So the section's "independent scorers" premise stands for the shipped roster,
with a named exception: the wire is one boolean, it is inert today, and a fighter
whose only lifting move is a tilt or a smash would make it live.

⭐⭐ **THE MOVE DISTRIBUTION ITSELF, WHICH IS WHAT THIS SECTION ASKS FOR IN THE
FIRST PLACE.** Starts counted per `(move_id, MovePlayback::instance)` so a
self-cancel into the same move counts twice, same duel:

| | HEAD (mislabeled) | truthful kit |
|---|---|---|
| seat 0 | 54 starts / 12 distinct — `pirate_grab` 13, **`jab` 10**, `grapeshot` 9, `dash_attack` 5 | 43 starts / 11 — `grapeshot` 9, `call_the_shark` 5, **`pirate_fthrow` 5, `pirate_pummel` 5**, `pirate_grab` 4 |
| seat 1 | 27 starts / 12 — `call_the_shark` 5, `grapeshot` 5, **`jab` 4**, `dash_attack` 3 | 39 starts / 13 — `grapeshot` 9, `call_the_shark` 6, `pirate_grab_dash` 5, `dash_attack` 3 |

⛔ **THE OBVIOUS SUSPECT IS REFUTED TOO: THE CPU DOES NOT SPAM THE DASH ATTACK.**
It falls out of seat 0's top eight entirely and holds at 3 for seat 1 — fewer
than under the mislabeled kit for seat 0. Anyone reasoning that a newly reachable
move gets over-selected should stop here; it does not.

⇒ **What actually moves is `jab` and the GRAB CHAIN.** Jab leaves the running
menu by construction, and both seats shift toward grab → pummel → throw: seat 0's
`pirate_grab` falls 13 → 4 while `pirate_fthrow` and `pirate_pummel` arrive at 5
each, i.e. the grabs it does start now CONVERT instead of being re-thrown away.
Seat 1 starts MORE moves (27 → 39) and deals LESS damage.

⭐⭐ **DAMAGE ATTRIBUTED BY MOVE — the instrument named above, built and run, and
it ISOLATES the drop.** Joined on `ResolvedBodyHit::attacker_move_instance`, so
the resolved amount lands on the use that earned it rather than on whatever the
body is playing a frame later. `+0 unclaimed` in both runs, which is also an
independent check that the occurrence threading covers this road:

| | HEAD (mislabeled) | truthful kit |
|---|---|---|
| seat 0 | **122** — dash_attack 36, **jab 33**, grapeshot 18, tilt_forward 14, tilt_up 12, air_forward 9 | **44** — grapeshot 18, heave_to 10, dash_attack 9, air_up 7 |
| seat 1 | **157** — **jab 126**, air_down 31 | **25** — heave_to 10, dash_attack 9, tilt_up 6 |

⇒ **JAB IS 159 OF 279 DAMAGE (57%) AT HEAD AND DEALS ZERO UNDER THE TRUTHFUL
KIT.** The whole drop is jab's contribution disappearing. That is the isolation
the previous paragraph asked for, and it retires "grab sequences deal less per
minute" as the explanation.

⛔⛔ **AND IT CORRECTS THIS PAGE'S OWN HEADLINE. The CPU was ALREADY PERFORMING
DASH ATTACKS.** Seat 0 dealt 36 damage with `pirate_admiral_dash_attack` under
the MISLABELED kit. "Eighteen of eighteen fighters have a dash attack no CPU can
reach" is false as stated: the kit's enumeration cannot reach it, but the press
the brain issues produces it anyway, because the press road resolves the stance
itself. ⇒ The defect was never unreachability — it is purely the MISLABEL, the
brain scoring one move's frame data while the body performs another.

⚠ **THE REMAINING QUESTION IS SHARPER THAN THE ONE IT REPLACES, and one more
measurement has narrowed it to something genuinely odd.** The truthful kit
removes jab from the RUNNING menu only; standing menus are byte-identical, and
these bodies stand 60–70% of their grounded time. So jab's contribution should
fall by roughly a third.

⛔⛔⛔ **AND ALL OF THE ABOVE IS ONE RUNG. THREE RUNGS SAY THE EFFECT IS
RUNG-DEPENDENT, AND HELPFUL AT THE BOTTOM.** Same duel, same fighter, `RUNG` 6 instead of 9:

| rung | HEAD | truthful | jab damage, HEAD → truthful |
|---|---|---|---|
| **9** | 1.26 / 1.07 | **0.47** / 0.86 | 159 → **0** |
| **6** | 1.56 / 1.69 | 1.52 / 1.39 | 68 → **161** |

Jab, HEAD → truthful: 159 → **0** at rung 9; 68 → **161** at rung 6.

| rung | HEAD | truthful | verdict |
|---|---|---|---|
| **3** | 0.78 / 0.61 | **0.86 / 0.76** | truthful kit is BETTER on both seats |
| **6** | 1.56 / 1.69 | 1.52 / 1.39 | −3% / −18%, both far above the 0.5 gate |
| **9** | 1.26 / 1.07 | **0.47** / 0.86 | −63% / −20%; the only rung that FAILS |

⇒ **The direction is consistent and it is not "the fix is a regression": the
higher the rung, the worse the truthful kit does, and at the bottom of the ladder
it HELPS.** That is a coherent story about a weaker brain benefiting from an
honest menu and a stronger one being disturbed by it — and it is still one fight
per rung, so it is a direction, not a curve.

⚠ Rung 12 is not a sample: the ladder has no such entry, both fighters failed to
seat, and the test's own 600-tick floor REFUSED to report rather than divide by
zero ticks. The floor doing its job is why that run is absent from the table
instead of sitting in it as a number.

At rung 6 the truthful kit costs ~3% and ~18% (both far above the 0.5 threshold),
lands MORE knockouts (2 against 1), and **jab is the single biggest damage source
UNDER the truthful kit** (124 for seat 1). The rung-9 collapse — damage halving,
jab to zero — is not a property of the kit change. It is one fight.

⇒ **So "the truthful kit halves CPU damage" must not be quoted without its rung.**
The fix still fails the shipped acceptance test, which is calibrated at rung 9 and
is the gate; but the reason to hold it is now "one rung regresses and we do not
know why", not "the fix makes the CPUs worse". Those justify different next work.

⚠ n=2. Two rungs disagree, so the next measurement is MORE SAMPLES — other rungs,
other fighters — before any mechanism is fitted to either. Everything in the two
paragraphs below was derived from the rung-9 fight alone and is retained only as
a description of that fight.

⛔ **AT RUNG 9, JAB IS STARTED ZERO TIMES.** Printing the FULL distribution rather
than a top-eight: seat 0 starts 11 distinct moves and seat 1 starts 13, and `jab`
is in neither list. Against 10 and 4 starts at HEAD. So this is not "jab lands
less" — the brain stops CHOOSING it, on ticks where it is still on the menu.

⇒ **The next instrument is the kit AT THE MOMENT OF DECISION**: log what
`generate_options` was handed and what it picked, on the ticks where the body is
standing. Two candidate readings and no evidence between them yet — either jab is
absent from the standing kit for a reason the stance probe cannot see, or it is
present and out-scored. ⚠ One thing worth checking first because it is cheap and
would explain a scoring shift on EVERY tick: `power` is normalised by
`kit_max_damage`, computed per tick over the CURRENT kit, so changing the kit's
membership re-prices every candidate in it — not only the ones that changed.

⚠ Do not skip to a fifth story. Four have been proposed on this item and three
are measured false.

⚠ Three mechanisms were proposed before this one and all three are measured
false: "the CPU never stops running" (14–19%), the `lifts` coupling (inert), and
dash-attack spam (refuted above). Do not adopt a fifth story without an
instrument behind it.

⇒ **The candidate term is an opportunity term on MOVEMENT: the value of standing
still is the best standing attack it unlocks** — offered as a hypothesis, not a
conclusion, because the loop above has to be broken before any weight is fitted. Its zero case is a body already
standing, or one whose best standing option scores no better than the dash
attack; its nonzero case is a running body in range of a foe with a stronger
committed option available. That is step 3's "one explicit policy term with clear
zero/nonzero cases", and the table above is the step-4 measurement to repeat.

⚠ **THE RIG IN THIS DOCUMENT CANNOT REFEREE IT.** `brain::fighter::evaluation`
builds a synthetic kit (`rig_uptilt`, `rig_smash`) with no dash stance and
measures APM and distinct frames, not damage. The duel rig
(`smash_cpus_damage_each_other`) is the instrument that sees this, and any
sentence sending a reader to the evaluation rig for a move-distribution question
is sending them somewhere that cannot answer.

A fighter repeatedly selecting one converted/dash move can arise from independent
movement and attack scorers rather than the moveset itself.

Before adding randomness or per-move caps:

1. trace the scored movement + attack pair;
2. identify the missing opportunity/commitment term;
3. add one explicit policy term with clear zero/nonzero cases;
4. measure move distribution and match quality.

## No-cheat acceptance

Brain difficulty/profile code must remain unable to:

- multiply damage directly;
- read privileged future state;
- teleport/skip physical execution;
- bypass actor-control budgets/cooldowns;
- change deterministic physics merely because the opponent difficulty is high.

Prefer structural API boundaries that make these operations unavailable over a
long list of tests that merely hopes nobody adds one.

## Exit

This plan can leave active architecture status when:

1. one ladder authority is selected and reported by every rig/tool;
2. every authored difficulty axis has a reachable, tested meaning or is removed;
3. representative fighter kits/scenarios are used for product calibration;
4. the mid-ladder utility progression is acceptable across the representative
   matrix;
5. fixed-seed determinism and no-cheat boundaries remain intact;
6. remaining changes are ordinary fighter/product tuning rather than unresolved
   AI architecture.

Use git history for the removed 2026-08-31 through 2026-09-04 matrices,
statistical arms, rejected hypotheses and investigation chronology.

## Brain policy stays outside combat ownership

The [responsibility map](architecture-responsibility-map.md) treats combat-adjacent
brain code as decision policy over an authored capability menu. A scoring function
can consume combat/action facts without owning damage, capture or live actor
mutation. Do not absorb the fighter brain into a generic combat context to reduce
imports.

A4 preserves the normal accepted-control/action road for both human and CPU input.
A11/A12 ensure the authored techniques and flows that the menu advertises are
actually installed and valid. Selection tests must distinguish a legal but poorly
scored move from content that cannot execute; difficulty should not conceal either
with character-specific scripts.
