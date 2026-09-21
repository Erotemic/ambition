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

⛔⛔ **THIS RULE STOOD FOR A WEEK WITH NOTHING BEHIND IT, AND THE ONE THING THAT
LOOKED LIKE ITS EVIDENCE WAS A DIFFERENT MEASUREMENT.** `ladder-rig`'s `--rungs`
flag documented `--rungs 6,6` as the null — *"do two IDENTICAL fighters split
evenly?"* — but equal rungs do not make equal fighters. With no
`--character`/`--opponent` the row seats the demo's two default ids, and those
are two different bodies: MEASURED 2026-09-21, `smash_duelist_a` wears
`player_robot_v3` (256px frames, 133 authored animations, body bbox 57x91) and
`smash_duelist_b` wears `player_robot_v2` (64px, 42 animations, bbox 17x37).
Same eight hitboxes, so the same active volumes — a different hurtbox, and 91
animations one of them does not author. That row returns `LOWER outfights
[3:12 = 80%, p=0.035]`: a real fighter result, which read as a null would have
condemned the instrument.

⭐⭐ **THE NULL CONTROL NOW EXISTS AND ITS FIRST RUN FAILED.** One rung, one
fighter in both seats, paired by exchanging the two seats' NOISE STREAMS — the
only thing left that tells the seats apart once the rung and the body are the
same:

```bash
cargo run --release -p ambition_demo_smash_app --bin smash_tool -- ladder-rig \
  --rungs 6,6 --paired --seeds 40 \
  --character smash_duelist_a --opponent smash_duelist_a \
  --ladder game/ambition_content/assets/data/fighter_brain_ladder.ron
```

| fighter (both seats) | seat 0 : seat 1 | tied | p |
| --- | --- | --- | --- |
| `smash_duelist_a` | 17 : 6 | 17 | **0.035** |
| `smash_duelist_b` | 11 : 4 | 25 | 0.118 (within spread) |
| `smash_george_booul` | 12 : 8 | 20 | 0.503 (within spread) |
| **pooled** | **40 : 18 (69%)** | 62 | **0.0054** |

⇒ **Seat 0 takes 69% of decided pairs at rung 6, and all three fighters lean the
same way.** `--paired` cancels this on the rung and fighter arms, which is what
those arms are for; what is new is its SIZE. An UNPAIRED row does not cancel it,
unpaired is the DEFAULT, and every ladder number recorded before `--paired`
existed was unpaired — so discount those by roughly a 69:31 seat term rather
than by nothing.

⚠ **NOT THE PLACEMENT, and that was checked rather than assumed.**
`ambition_demo_smash::respawn_placement` alternates seats outward from the stage
centre, so seats 0 and 1 sit at ±32px of a symmetric 480px platform, and the
initial seating is the same call. Decision order within a tick is the obvious
remaining candidate and has **not** been measured. Named, not chased.

⭐ The ties are the arm's own evidence: 62 of 174 pairs came out exactly level,
which is what exchanging one term and nothing else should do to a third of
seeds. An arm that returned no ties would be swapping more than it claimed.

### Use enough clock for the outcome being measured

A short clock that leaves many unresolved bouts measures pace/partial damage,
not final match strength. When the product question is “which fighter wins,” use
the shipped/appropriate match clock and report unresolved bouts explicitly.

### Scenario premises must be real

A static opponent does not measure reaction delay. An unarmed stand-in does not
measure special-move choice. A recovery fixture must include the relevant body
capabilities.

If the premise is missing, fix the fixture rather than interpreting the table.

### ⛔⛤ A mirror match is a room, and a narrow menu turns it into a limit cycle

Measured 2026-09-20 on `every_fighter_on_the_grid_can_fight_its_mirror`.

Two copies of one brain at one rung see mirrored worlds and rank the same menu
the same way. While the menu is WIDE, execution noise and changing situations
hold the two seats apart and the bout looks like a fight. Once the menu narrows
to one or two moves, the pair locks into a DETERMINISTIC LIMIT CYCLE: it repeats
a period exactly, so the move either lands every time or never — and "never" is
what it printed. `medic` threw `medic_tourniquet` 177 times over 3600 ticks at
alternating gaps of 9–10px and 153–162px with `dy` zero throughout, and
`LandedBodyHit` at **zero**. `pointed_polygon` ran the same 127-tick cycle.

⇒ **A `0%` row in a mirror is a QUESTION, NOT A VERDICT.** Seat somebody else
opposite before believing it. `AMBITION_GRID_FOE=smash_george_booul` on the same
three fighters, same clock: 80%, 88% and 17% dealt, on 23, 24 and 13 distinct
moves. All three fight; the mirror was the thing that could not.

⚠ This is the third cause of a zero in that table, and the first two — a fighter
that cannot be seated, and a fighter with no repertoire — look identical in the
column. The narrowing tools are `AMBITION_GRID_ONLY`, `AMBITION_GRID_FOE` and
`AMBITION_GRID_TRACE`, which prints each move start with both axes of the gap
and the running `landed`/`resolved`/`blocked` tallies. **`landed` is what
separates "chose badly" from "chose well and missed"**; the aggregate row cannot.

### ⛔⛤ A brain that reasons about a stale world as if it were the present swings where somebody was

Measured 2026-09-20. `DelayedPerception` is the no-cheat contract made
structural — `reaction_ms` says how late the brain sees the world — and the
delay was invisible to everything downstream: a consumer received a
`WorldView` and had no way to ask how old it was, so every geometric question
in the option layer was asked of the past.

It went unnoticed for as long as attack admission forgave three times a move's
reach. **Making that rule honest is what made the staleness matter**, and the
reading that found it contradicts the rule's own ceiling: `medic_tourniquet`
reaches 80px and is admitted only out to 104px, yet a `medic` mirror STARTED it
at a real gap of 153.6px, 177 times in 3600 ticks, with `LandedBodyHit` at zero
for the whole bout. A move cannot be admitted past its ceiling — what was
admitted was a remembered opponent. At rung 5 the arithmetic is
`reaction_ms: 300` (eighteen ticks) plus the move's own startup: about 100px of
walking against a 24px slack.

⇒ `Perceived::staleness_s()` publishes it, read off the buffered views' own
`sim_time` so no tick rate has to be agreed on and warm-up reports itself
honestly. Attack admission carries the foe forward at the relative velocity the
view reports, over `staleness + startup`.

⚠ **SCORING IS DELIBERATELY NOT LED, and that is the line that protects §1.3.**
`reach_fit` is a judgement about VALUE; `reaction_ms` is the shipped difficulty
axis; a brain that predicted perfectly everywhere would flatten the ladder.
Admission is the one judgement here about a moment in the FUTURE — the tick the
hitbox opens — so it is the one that leads. Anyone widening this should say
which rung the prediction is supposed to be wrong at, and by how much.

### ⛔⛤ The tick a move goes live is not the tick its hitbox opens

Reviewed 2026-09-20, one day after the section above landed. The lead is over
`staleness + startup_s`, and `startup_s` is derived as *"time until the first
Active window"* with a fallback: **a move with no Active window at all reports
its whole duration.** Every ranged move in the game is that shape, because the
projectile is the attack — so the moment `hazard_reach` put them on the menu,
the lead began aiming them past the opponent.

```text
polygon_projectile_charge_shot   throws 0.26s   startup_s 0.58s
polygon_ponytail_boomerang       throws 0.16s   startup_s 0.40s
polygon_lay_bomb                 throws 0.18s   startup_s 0.46s
```

At 200px/s of closing speed the first two aim 64px and 48px long, both wider
than the 24px slack the admission rule is tuned around. ⇒
`MoveFrameData::threat_live_at_s` answers the question the lead is actually
asking, folded from the three roads that can threaten — an Active window's own
`start_s`, a `Ranged` or hazardous `Effect` event's `at_s`, and the window
carrying a hazardous sustain — and `None` where the move offers the opponent
nothing.

⚠ **THE GENERAL SHAPE, STATED ONCE SO IT IS NOT RE-LEARNED:** a field derived
with a fallback answers TWO questions, and the second answer is a guess wearing
the first one's units. `startup_s`'s fallback is a sensible answer to *"how
long am I committed"* and a wrong answer to *"when does this become dangerous"*.
Give the second question its own field rather than the first field a second
meaning.

### ⛔⛤ And a hazard is not a threat where it is thrown

The third correction to the same lead, one day after the second, and the first
one that is not a substitution. Leading by `startup_s` over-led by the whole
move; leading by `threat_live_at_s` is exact for a SWING and under-leads
anything that travels by the whole of its flight. `director_train_of_thought`
crosses 671px at 300px/s, so against somebody 400px away the shot lands about
1.3 seconds after it leaves the hand.

⭐ **THE FIELD THAT MAKES THE THIRD ANSWER POSSIBLE WAS BEING COMPUTED AND
DISCARDED.** `hazard_reach_of` evaluated `speed × lifetime` and kept only the
product, so the one quantity a consumer needs to convert a distance into a
TIME was thrown away inside the derivation that had it.

⛔⛤ **AND THE FIRST REPAIR KEPT IT AS A PAIR OF SCALARS, WHICH THE SECOND
REVIEW OF THE SAME DAY SENT BACK.** This section said
`MoveHazard::Spawned { reach, speed }` and `thrown_at + min(gap, reach) /
speed` for one commit. A speed beside a time is a distance only while the
motion is UNIFORM, and two of the four shapes the roster ships are not: a
boomerang decelerates to a standstill at its turnaround (the ponytail covers
40px of centre travel in 0.111s where the average said 0.186s) and a laid bomb
does not travel at all — it sits on a four-second fuse. ⇒ The hazard carries
its LAW: `ThreatTravel::{Straight, Boomerang, Placed}`, answering
`travel_to(distance)` and `None` for a distance it never covers. A new travel
shape is a new variant, never a new scalar on an existing one.

⚠ **ONE FIXED-POINT PASS, AND THE ERROR IS DIRECTIONAL ON PURPOSE.** The
flight time depends on the gap at arrival, which depends on the flight time.
Measuring the gap at the throw and flying for that long is exact against a
standing opponent and under-leads a retreating one — which refuses a shot
rather than throwing one that lands behind them, and refusing is the cheaper
mistake for the same reason the admission rule is absolute.

⛔ **AND A FUSE IS NOT A FLIGHT.** `travel_to` answers *"where do I point this
so it lands on them"* and `detonates_by_s` answers *"when can it hurt anybody
at all"*; feeding the second to the aim lead carried the opponent forward four
seconds of walking and cost Projectile Polygon her bomb outright. A placed
object is aimed nowhere and travels for zero. Nothing prices the fuse yet, and
that is recorded on the type rather than patched into the lead.

### ⛔⛤ A placeholder is a request, and a request needs an owner

`MoveEventKind::Ranged` fires whatever `RangedActionSpec` the BODY carries, and
a catalog derivation has no body to ask. The answer was
`RANGED_ACTION_REACH = 1000` — wider than any stage this game ships — folded
into the same `max` as measured bolt and bomb flights, with nothing in the
value saying which numbers were real. Its own doc named the cost: *"a CPU that
fires from further away than its shot can carry."*

⇒ **The catalog states the REQUEST.** `MoveHazard::OwnersRangedAction` is not
a distance; the accessors answer it with the standing constant so an unjoined
reader is unchanged, and the layer that CAN see the body replaces the variant
outright. That layer is `attack_kit_of`, which is already the one that joins a
grab to its capture params — so the join has a home rather than a new one.

⚠ **THE PRECEDENCE IS THE RUNTIME'S, COPIED RATHER THAN INVENTED:** what the
move EQUIPS, then the body's standing kit. The admiral's side-B draws
`admiral_gun_sword` and fires that; reading the body alone would describe a
different shot for the one move in the game that brandishes.

⚠ **AND NO RESOLVABLE WEAPON LEAVES THE REQUEST STANDING.** A move authored to
fire a weapon its body does not have cannot produce a shot at all, and *"should
this be pressed"* is a question for the layer that decides pressing — not a
reach of zero invented at the join.

⛔⛤ **AND THE FIRST JOIN GOT THE BOOMERANG WRONG BY EXACTLY A FACTOR OF TWO**,
which is the same lesson one layer down: `boomerang_return_s` is a TIME and
`speed` is a SPEED, and their product is not the distance, because the shot is
decelerating the whole way out. `ProjectileFlight::boomerang`'s own doc states
the displacement — `v0·t − v0·t²/2·out_s`, i.e. `v0 · out_s / 2` at the
turnaround. The ponytail reaches **73px**, not 146. ⇒ Read the flight's
ARITHMETIC, not its two numbers; and the hazard's `speed` is published as the
AVERAGE over that leg, so `reach / speed` is the time it actually takes, which
is what the admission lead divides by.

### ⛔⛤ And the payoff gate was shut on every launcher in the game

Reviewed 2026-09-21, and it is the THIRD reader of `startup_s` to be caught by
the same fallback. `expected_payoff` is a move's power gated by whether it fits
the opening — `frame_advantage(startup_s, their_commitment, startup_s)` — and
`startup_s` is *"time until the first Active window"* falling back to the whole
DURATION when there is none, which is every launcher. So the gate asked whether
the opponent was committed for longer than the attacker's entire animation, and
`expected_payoff` was structurally zero for the whole projectile half of the
roster whatever a shot dealt.

⭐ **THE MEASUREMENT IS WHAT MADE THIS VISIBLE, AND IT WAS TAKEN FOR THE OTHER
HALF.** Giving `MoveHazard::Spawned` its `damage` moved exactly **one of
twenty-one** grid rows, and that row moved through the NORMALISER — a laid
bomb's 12 becoming the largest thing in an aerial kit — rather than through any
launcher being priced. A number that reaches the kit and cannot reach a decision
is a mechanism wired to nothing; the sweep is what told the difference.

⇒ **The gate asks when the move CONNECTS**: `arrival_of` for a hazard, which is
the same throw-plus-flight the admission rule already computes one block down,
and `threat_live_at_s` otherwise — identical to `startup_s` for an ordinary
strike by construction, so only launchers move. The three closures that answer
it (`threat_at`, `lead_of`, `arrival_of`) moved above the scoring, because two
consumers asking the same question must not be two derivations of it.

⚠ **THE GRID CANNOT WITNESS THE GATE, AND THE CROSSOVER SAYS WHY.**
`connects_at` for `polygon_projectile_charge_shot` is `0.26 + (gap − 10)/540`,
equal to its `startup_s` of `0.58` at a gap of **183px**: more generous inside
that, stricter beyond it, and all 21 rows bit-identical either way. The gate
only opens against an opponent committed for longer than the arrival, and that
window never co-occurred with a launcher being the best option on the menu.

⚠ **AND THE ASYMMETRY THAT IS LEFT IS NAMED RATHER THAN PATCHED.** A launcher
has `coverage: None`, so its `reach_fit` is ZERO at every range while its
payoff is now paid in full — a swing's worth is discounted by the gap and a
shot's is not. That is the missing term, and it is a feature (hazard coverage),
not another scalar.

### ⛔⛤ The resolver stales every landing and the chooser could not see it

Reviewed 2026-09-21. `apply_hitbox_damage` resolves a landing as
`damage × stale_scale(n)` and its launch's percent term as
`victim_percent_knockback_scale × knockback_stale_scale(..)`. The brain's
`LaunchLaw` carried only the first factor of that second product and no damage
staling at all — while `LaunchConditions::growth_scale`'s own doc had named
both factors since the launch law was collapsed into one copy. **A
specification written on the type and one term carried in the value.**

At the smash stage's declared `stale_step 0.05 / stale_floor 0.55 /
stale_knockback_influence 0.30`, a move landed nine times recently deals
**55%** of what `expected_payoff` priced it at, and its percent term keeps
**86.5%** of its growth.

⇒ `AttackCandidate::wear` carries `MoveWear { damage, launch_growth }`,
resolved by `attack_kit_of` from the body's own `BodyStaleMoves` ring and the
stage's `ResolvedCombatTuning`. ⛔ It is NOT part of `MoveFrameData`: that is a
pure derivation of a `MoveSpec` and two bodies holding one moveset wear their
moves differently — the same distinction `max_damage` keeps against a
hazard's damage, one layer down.

⛔ **THE TWO HALVES ARE SEPARATE BECAUSE THE RESOLVER'S SPLIT IS.** The damage
answer is spent whole; the launch answer is that same weakening attenuated by
the declared influence, applied to the PERCENT TERM and never to `base` — a
worn move throws a FRESH opponent exactly as far as it always did. Collapsing
them is what once threw away half of everything at high percent and stopped
the stock ending, and a test that priced the launch with the DAMAGE scale
passed a first, looser version of this arm: the band it checked admitted the
confusion the two fields exist to prevent, so the arm pins the ratio the
fixture's own arithmetic predicts.

⭐ **AND IT IS WHAT MADE THE HAZARD-DAMAGE HALF WHOLE.** Pricing what a move
deals without pricing what repeating it costs moved Projectile Polygon's row
down 40 points; with staling in the kit her row is **bit-identical to the
pre-change run**. Her charge shot lands, so it stales, and her launchers
price below her up smash again.

### ⛔⛤ Recovery authority is a travel number and answers a movement question

Reviewed 2026-09-20. `AuthoredRecoveryRoute::SustainedAuthority { seconds,
reach }` was folded into `hazard_reach` on the argument that a summon holding
ground for `seconds` makes the opponent the same offer a bolt does. Its one
production instance refutes it: `call_the_shark`'s authoring says *"There is no
hurtbox on this up-b, it's purely a mobility special"*, the summoned shark is
`Neutral` and deals no contact damage, and the `reach` is authored as half the
RIDE's straight-line distance.

⇒ **Two independent questions, two numbers.** Recovery authority answers *"what
movement does this give me"*; hazard reach answers *"what can this do to
them"*. A future summon that does both states its offensive half as an effect,
like every other hazard the catalog prices. The travel half is read on the
motion road through `RecoveryRoute::carry`, which is the number both carrying
kinds already publish.

⚠ **AND THE MOTION SCORE HAD TO LEARN LENGTH BEFORE IT COULD RECEIVE IT.** It
was `alignment × speed / the kit's fastest speed`, which contains no length, so
the gap magnitude cancels: a full-strength recovery was worth the same against
an opponent 5px away as against one 900px away. It is now a symmetric tent over
`travelled / gap`, which is dimensionless for the same reason the speed share
was — a ratio of two lengths — while actually depending on how far there is to
go.

### ⛔⛤ A number published for one question does not answer a neighbouring one

Three findings in two days are the same shape, and the shape is worth naming
once. `RecoveryRoute::carry` answers the RECOVERY planner: *"this gets you home
from within this far."* A `SustainedAuthority` ride's `carry` happens to answer
*"how far can I travel toward anything"* too, because the rider steers it. A
`Teleport`'s does NOT: `phase_shift` is authored *"aimed, like every recovery:
the stick, then straight up"*, so it goes where the MOVE aims and not where the
reader wants to go.

⭐ **MEASURED, AND THE MEASUREMENT IS WHY THIS IS A RULE AND NOT A PREFERENCE.**
Pricing a teleport as travel toward the opponent cost `player_robot_v3` his
match at two different prices — 27%/22% on 9 distinct moves with
`phase_shift×186`, then 32%/39% on 11 with ×54 — against 225%/223% on 19 with
the move offered nowhere. **Two prices, one outcome ⇒ the defect is not the
price.** When a change that should be a tuning knob produces the same failure at
both ends of its range, stop tuning and ask what the number means.

⇒ Before spending a published scalar on a new question, read the sentence its
owner wrote about it. `reach` on a summon is *how far the admiral rides*;
`startup_s` on a hitless move is *the whole duration*; `distance` on a teleport
is *how far the jump is, in whatever direction the move picks*. Each of the
three read plausibly and was wrong.

### ⛔⛤ A factor common to two candidates still reorders them if it rides only part of the expression

The brain ranked its finishers with `base + growth × victim_damage` and its own
doc justified the omission: the ruleset's percent scale, the per-`base`
steepening, the victim's weight and rage are *"COMMON to every candidate one
attacker weighs against one opponent, so none of them can reorder a kit"*.

**Common is not the same as uniform.** Each of those factors multiplies the
PERCENT TERM and leaves `base` alone, so they scale one part of each line and
not the other — which moves where two lines CROSS:

```text
d* = (b₂ − b₁) · weight / (growth_scale · growth_base · (g₁ − g₂))
```

Every omitted factor appears in the crossover. George Booul's forward smash
`(185, 3.45)` and up smash `(178, 6.28)` cross at about 2 points of victim
damage under the identity law and have ALREADY crossed there under the smash
stage's declared `1.25` against a `0.85`-weight body.

⇒ The test for "can this factor be dropped from a ranking" is not *is it the
same for every candidate* but *does it multiply the whole expression*. A factor
that divides the candidates' expressions into scaled and unscaled parts is a
reordering factor however common it is. See
`ambition_entity_catalog::launch`, which now owns the one copy of the law, and
`WorldView::launch_law`, which is how a brain receives the half of it that
belongs to the stage.

⚠ **AND THE ONE FACTOR STILL MISSING IS NAMED RATHER THAN ARGUED AWAY:**
per-move staleness. The runtime folds a move's own usage history into
`growth_scale`; a brain with no usage memory cannot, so a repeated finisher is
priced a little high. That is a reordering factor by the rule above, which is
why it is written down as owed instead of dismissed as common.

### ⛔⛤ Two ways to author one number are two authorings, and one shared function is not enough to share them

`knockback_growth` has two authoring roads and they mean different things.
`Some(g)` states a growth, and `Some(0.0)` is the documented way to author a
FIXED launch — a windbox that throws the same distance at 0% and at 200%.
`None` says *"the ruleset decides"*, and the hit resolver answers it with
`base × DeclaredCombatRules::knockback_growth`.

Moving the arithmetic into one function did not make the two sides agree,
because each side still COLLAPSED the `Option` before calling it — and they
collapsed it differently. The hit resolver spent the ruleset's fallback; the
catalog's `LaunchEnvelope::with_volume` spent `unwrap_or(0.0)`. Raised by
review 2026-09-20, one commit after the shared law landed.

It is live on the shipped roster and it is not small. Two moves author `None`:
`cellular_pulse` at base 140 and `performer_trapdoor` at base 150. The smash
stage declares `knockback_growth: 0.02` and a percent scale of `1.25`, so at
100% against the reference body the stage throws the pulse **490px/s** while
the brain priced it **140** — and, reading it as a set launch, declined its
rage as well. The brain called the roster's two ruleset-scaling specials set
knockback and ranked them as pokes.

⇒ **THE COLLAPSE BELONGS TO THE LAW, NOT TO ITS CALLERS.**
`launch_speed(base, growth: Option<f32>, conditions)` takes the `Option` and
`LaunchConditions` carries `ruleset_growth`, so there is exactly one place
where `None` becomes a number and it is inside the thing both sides call.
`LaunchEnvelope` keeps the `Option` on its flat line rather than resolving it
at authoring time, and `grows()` became `grows_under(conditions)` because the
question genuinely has two answers: a `None` volume is a set launch in an
undeclared world and a percent-scaling one on a stage that declares a fallback.

⚠ **THE GENERAL SHAPE, which is worth more than the fix:** a field whose
`Option` is resolved by a FALLBACK is answering two questions — *what did the
author state* and *what does this world make of it* — and any reader that
collapses it early has silently answered the second one on its own. The
symptom is indistinguishable from agreement, because both sides produce a
plausible number in the same units.

### ⛔⛤ A scalar pair can only describe uniform motion, and half the hazards in the game are not uniform

`MoveHazard { reach, speed }` was enough for a bolt and wrong for everything
else the roster already ships. A boomerang decelerates to a standstill at its
turnaround, so its average speed is exact at maximum range and nowhere else —
Projectile Polygon's ponytail covers 40px of centre travel in 0.111s where the
average said 0.186s, which at a 200px/s closing speed is 15px of excess lead,
the width of `ADMISSION_SLACK_PX`. A laid bomb does not travel at all and sits
on a four-second fuse, and `speed: 0.0` was documented as *"the whole reach is
available the moment it exists"*.

⇒ The type carries the LAW and answers `travel_to(distance)`, which is what
every consumer of the pair was computing anyway. A shape that cannot reach a
distance returns `None` rather than a number, because a consumer that falls
back to a number is leading its aim at a shot that cannot land.

**The rule for the next shape: a new travel law is a new variant, not a new
scalar on an existing one.** The four that exist are the four that are
authored.

### ⛔⛤ When a thing arrives and when it can hurt you are two questions, and one function answering both spent a fuse as a flight time

Teaching the laid bomb its fuse was right. Feeding that fuse to the AIM LEAD
was not, and the grid found it in one sweep: the lead carries the opponent
forward at the velocity last seen, and four seconds of that is arithmetic
about a walk nobody takes. Her bomb reaches 72px, so at any walking speed the
extrapolated opponent is outside it — `projectile_polygon` went 144/228 to
131/183 and lost two distinct moves. The move was deleted, not corrected.

⚠ **AND THE FIRST EXPLANATION WAS PLAUSIBLE AND WRONG.** The reflex was to
cap the lead at the width of the room: a 480px stage cannot contain 800px of
walk. Against the real numbers it cannot be the mechanism — 72px of reach is
exceeded by four seconds at 18px/s, and a stage-width cap only bites above
120px/s. Reverting an unmeasured repair is cheaper than keeping a plausible
knob that would have been load-bearing for the wrong reason.

⇒ `travel_to` is the aiming question and `live_at_s` is the fuse. A placed
object is aimed nowhere and travels for zero. **Nothing prices the fuse yet,
and that is recorded on the type rather than patched into the lead**: *"will
they be within 72px in four seconds"* is a stage-control question, the same
shape as the counters and buffs deliberately held off the attack ranking until
there is a defensive feature to price them with.

### ⛔⛤ A layer that has proven a press does nothing must not hand the brain a placeholder

`resolve_owners_ranged_action` returned early when neither an equipped weapon
nor the body's standing kit could answer a move's ranged request, which left
`MoveHazard::OwnersRangedAction` standing — and its unjoined reach is the
1000px placeholder, wider than any stage this game ships. The test asserted
that on purpose, reasoning that *"the move fires nothing"* belongs to whoever
decides pressing. The kit builder IS that layer.

⇒ An unanswerable request is no hazard. The candidate stays — a body does not
lose a move because of what it is not carrying — and the offer goes.

### ⛔ A near-miss that is measured on only one axis is not measured

The sweep's `gap` column is `|x0 − x1|`. A pair 10px apart in `x` and 300px
apart in `y` prints `gap 10`, which reads as point-blank and is a juggle. The
column is kept as it is because three recorded sweeps are keyed to its meaning;
the trace carries both axes so a row can actually be read.

## Settled findings that still constrain work

### ⛔⛤ A press rate read off this rig is a reading of `apm_cap`, and the rig's opponent has to come within reach for it to be a reading at all

Measured 2026-09-20, seed `0x5EED`, `brain::fighter::evaluation`.
`ScenarioOutcome::apm` counts ATTACK presses and nothing else, and the option
layer offers an attack only where the move's own region touches the opponent.
Three things made that impossible across most of the suite, each invisible
while the option layer admitted anything within three times a move's reach:

- **The pacing never closed.** The opponent swept ±120px around where the
  fixture put them, and the fixtures are authored 180..620px apart against a
  90px longest move. Eight of the nine scenarios could not contribute one
  press, so the ladder's mean was ONE scenario divided by nine. `play` now
  walks the opponent in to `RIG_ARMS_LENGTH` and back; the fixture still says
  where they start.
- **Every fixture was authored facing away from half its own premise.**
  `SelfView::facing` defaults to `0.0`, the option layer reads that as `+x`,
  and `scenarios::body()` never stated one — so `edgeguard_window`'s *"the
  opponent must come back THROUGH YOU"* was staged back to back. Every
  authored volume is body-local and forward. `suite()` now derives facing from
  the nearest hostile.
- **The rig's `Up` candidate was a third forward poke.** Its coverage box was
  built the same way for every binding, which is the defect `MoveCoverage`'s
  own doc records about George Booul's vertical game, reproduced inside the rig
  meant to catch it.

⭐ **AND WHAT ORDERS THE RUNGS IS THE CAP.** With the fixtures live, null
controls on noise, rollouts and `read_weight` move the curve by at most 0.7
APM; removing `apm_cap` turns it into a SAW:

```text
as shipped   22.7  30.0  31.3  38.0  39.3  46.7  45.3  52.0  46.7
no cap       46.7  52.7  46.7  52.7  46.7  53.3  46.7  53.3  46.7
```

⇒ The decision cadence quantises the press rate and the quantum alternates;
the authored cap is what separates the rungs, and where it stops binding (7 and
9) the saw shows through. `the_ladder_is_ordered_by_press_rate` therefore
claims a resolution of TWO rungs, which the no-cap curve fails at its first
step. See `probe_what_separates_the_rungs` for the controls.

⚠ **AND `ApmLedger::may_press` AVERAGES FROM THE BRAIN'S FIRST TICK**, not over
a window — so a CPU is most constrained at the start of a bout and least
constrained after idling. That is the mechanism behind both the cap's grip here
and the *"a tenth of its own cap"* reading. Recorded, not changed: it is
shipped difficulty behaviour and moving it moves every APM number in the
project.


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

⛔⛔ **A SECOND FIGHTER SAYS THE GATE IS CALIBRATED TO ONE.** `npc_emmy_noether`
at rung 9 — **HEAD 0.28 / 0.44, which ALREADY FAILS the 0.5 threshold**, 0
knockouts, hitstun [38, 59]; truthful kit **0.28 / 0.44, byte-identical**, same
damage-by-move. ⇒ Another shipped fighter fails this acceptance test today with
nothing changed, so "the fix fails the gate at rung 9" is much weaker evidence
than it reads — the gate does not hold across fighters at HEAD, and the fighter
it is calibrated on is the pirate admiral.

⭐ The identical numbers are a CONTROL rather than a null: the change is a no-op
wherever the running stance never triggers, which is what it should be.

⚠ The fix is runnable behind `--features truthful_attack_kit` on
`ambition_platformer2d_actor_monolith` (default off), one code path — with the
feature off the stance flag is a compile-time `false` and the resolver falls
through to today's, so "off" is the shipped behaviour by construction.

⚠ **AND THE HARNESS IS THE APP ACCEPTANCE TEST, NOT `ladder-rig`** —
`smash_cpus_damage_each_other::two_cpus…` with `FIGHTER`/`RUNG`/`TICKS` selecting
the cell. `ladder-rig`'s default duelists bind no `attack_dash` (its own header
says so) and `ambition_demo_smash_app` has no `ambition_content` edge, so the 19
authored movesets are not seatable there at all. An instrument's NAME is not its
population; this was inferred wrongly once already.

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
