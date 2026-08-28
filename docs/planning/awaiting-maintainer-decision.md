# Awaiting a maintainer decision

Only questions whose next step is **Jon's product/authoring judgement** belong
here. Engineering questions go to the queue/tracks; answered questions move to
[`maintainer-decisions.md`](maintainer-decisions.md). The pre-prune investigation
record is archived at
[`../archive/planning-superseded/2026-08-13/awaiting-maintainer-decision.md`](../archive/planning-superseded/2026-08-13/awaiting-maintainer-decision.md).

⭐⭐ **AND JON ANSWERED TWO OF THEM THE SAME DAY — 6 AND 9 ARE CLOSED**, both
recorded verbatim in [`maintainer-decisions.md`](maintainer-decisions.md). Their
sections below are kept as answered records, not as questions:

- **6 (hitlag)** — the landed fix STANDS and the old *"do not reintroduce a
  per-body zero-dt"* prohibition is **superseded**. Hitlag is a body semantic and
  must not depend on which control road a body is on. ⛔ a future feel complaint
  is answered by DURATION/SHAPE, never by restoring the asymmetry.
- **9 (per-turn suite)** — the per-turn gate STAYS SMALL, deliberately. ⛔ do not
  add `cargo test --workspace --lib` to `gate_suite.py`; it is a pre-push tier.

⛔⛔ **BOTH had been mis-stated by an agent-closed ledger row first, and that is
the pattern to watch.** 6 was answered by an implementation that never read this
file, so a written prohibition was *unseen rather than overruled*. 9 had inherited
a false premise from D160's premature closure (*"the project gate now runs
`cargo test --workspace --lib`"* — it did not; D160 added a pre-push paragraph to
`AGENTS.md`). ⇒ **a row's premise is worth re-checking against the tree, not
against the row that claims to have moved it.**

⚠ **the same items also live in
[`JONS_OBSERVATIONS_BUGS_AND_ISSUES.md`](JONS_OBSERVATIONS_BUGS_AND_ISSUES.md)**
and the two files did not reference each other, so a settled decision kept
reading there as an unfixed bug. Cross-links added for 1, 4 and 7.

⛔ **A CLOSED ROW HERE IS A RECEIPT TOO.** An answered decision keeps Jon's
ruling verbatim, the consequence, and any standing prohibition — not the
investigation that led to the question. Same rule as
[`README.md`](README.md#queue-contract); on 2026-08-17 this file was **739 lines
for 9 open questions**, and the four answered ones held a third of it.

## Open decisions

### 34. DO WE WAKE **TUMBLE**? — THREE MECHANICS ARE PROXYING FOR IT

⭐ **NOTHING LOOKS WRONG TODAY; this is here because a review asked and the
answer is taste, not correctness.** Two grounded bodies that meet at walking
speed stop where they meet — that is what body contact is FOR. The question is
only about the END of a knockback: once a launched body has decayed below walking
speed, `body_contact` can no longer tell it from someone walking, so a neighbour
stops it dead.

```text
today   grounded, |v| <= max_run_speed  ⇒  treated as locomotion  ⇒  resisted
        at resistance == 1.0 the body stops AT contact — the rest of the
        slide is cancelled, not slowed
```

⛔ **THE OBVIOUS FIX WAS MEASURED AND IT DOES NOT COVER THIS.**
`knockdown::owns_control` is FALSE exactly when this fires — a decayed launch is
neither a knockdown nor a tumble — so gating on it changes nothing.

⛔ **AND THE KERNEL CANNOT ASK THE QUESTION.** There is no hitstun in
`AxisManeuverState`; hitstun lives monolith-side in the hit reaction. *"Did this
motion come from a hit"* is not askable at this seam without threading a new fact
down, and THAT is the price — not three lines.

⛔⛔ **AND IT IS NOT ONE CONSUMER. MEASURED 2026-08-26 — THREE SITES ASK THE SAME
QUESTION AND ALL THREE ANSWER IT WITH A MAGNITUDE:**

```text
integration.rs:862   the INITIAL DASH   `if along.abs() > want.abs()` — don't touch
integration.rs:987   the SHIELD BRAKE   `if along.abs() <= max_run_speed` — brake
body_contact.rs      CONTACT RESISTANCE  faster than one walk-tick is "not walking"
```

⇒ each carries its own comment saying *"anything faster than its own run is
somebody else's velocity — a LAUNCH"*, each was written after a real regression
that deleted knockback, and **each fails in exactly the same place**: a launch
that has DECAYED below the threshold is indistinguishable from walking. This is
one missing fact wearing three thresholds.

⭐⭐ **THE FACT EXISTS, THE GENRE NAMES IT, AND THIS KERNEL ALREADY IMPLEMENTS
IT: TUMBLE.** A tumbling body is carrying somebody else's velocity BY DEFINITION,
at any magnitude, which is precisely what all three are guessing at.
`movement/knockdown.rs::launch_into_tumble` is the one entry point and the whole
knockdown → tech → getup cycle is built behind it.

⛔⛔ **AND I FIRST WROTE THAT IT IS DORMANT. IT IS NOT — CORRECTED 2026-08-26 BY
RUNNING THE GAME.** The Smash stage authors `tumble_speed: 500.0`
(`game/ambition_demo_smash/src/lib.rs:275`), and `match_report -- 30 --runs 3`
measures **121–203–401 tumbling ticks per run**. The claim came from D241 (*"the
only 500.0 in the tree is a unit-test fixture"*) and I repeated it without
running a match. ⇒ **in SMASH the fact is live and the three sites could read it
today.**

⭐ **SO THE REAL SHAPE IS A SPLIT BETWEEN THE TWO GAMES, which is a better
question than the one this row opened with:**

```text
SMASH      tumble_speed = 500.0, authored by the stage — tumble is REAL
AMBITION   DEFAULT_TUNING.tumble_speed = 0.0, and a test PINS that zero
           (`smash_roster_movesets.rs:857`) because moving it would change
           Mary-O, the explorer and every wandering enemy at once
KERNEL     the three magnitude bounds live in the SHARED movement kernel,
           used by both
```

⇒ **the question is whether a kernel ownership bound may read a fact that one
game authors and the other deliberately does not:**

```text
(a) LEAVE THE PROXIES      the magnitude works in both games and is wrong in
                           the same small way in both. Nothing looks wrong today
(b) READ TUMBLE WHEN IT
    IS THERE               `tumbling || knockdown` when the body has it, the
                           magnitude when it does not — correct in Smash
                           immediately, unchanged in Ambition
(c) AUTHOR TUMBLE FOR
    AMBITION TOO           one number, and it re-tunes every wandering enemy;
                           the pinning test above is the ledger of what breaks
```

⚠ **(b) is the cheap one and it is NOT a refactor-answer to a feel question** —
it makes the kernel read the authored fact where the fact exists. It is here
rather than shipped because *which games get a floor game* is yours.

⭐ **AND A MEASUREMENT WORTH HAVING EITHER WAY: `downed 0–0–0`.** Across three
30-second matches, tumble happened constantly and **nobody was ever knocked
down**. A tumbling CPU acts out with jump or attack the moment helplessness ends,
so it never lands while helpless. That may be correct for CPU-vs-CPU and wrong
for a human — worth one playtest before anybody tunes the knockdown window.

⚠ **NOT BLOCKING ANYTHING.** Closed out of [D179 in `queue.md`](queue.md) (whose
other half shipped), and it is the same question as **(20)** in D238 / **#20** in
D241 — those two rows are blocked on this answer, not on a number.

### 33. WHAT DOES A RECHARGING WEAPON LOOK LIKE?

⭐ **THE MECHANIC IS SHIPPED; THIS IS THE ART.** Your own ruling on the ranged
cadence ended with *"give recharge enough presentation that an unavailable shot is
legible"*, and that half is not built. `BodyMelee::ranged_cooldown` is now the
authored, per-weapon truth (`RangedActionSpec::refire_s`) and **nothing draws
it**.

⛔ **THE "BUTTON DOES NOTHING" HALF IS ALREADY ANSWERED, so this is only about
seeing it.** A press that arrives during recharge is refused BEFORE
`proposer.spend`, so the ordinary combat buffer keeps re-proposing and starts the
move the moment the weapon returns — the normal short buffering you asked for,
not a queue.

⭐ **THE GENRE PUTS IT ON THE CHARACTER, not on the HUD** — Samus's charge glow,
ROB's fuel gauge, Mega Man's arm cannon. That is research rather than taste, and
it narrows the question to WHICH channel:

```text
(a) a per-character VFX row on the muzzle, driven by the recharge fraction
    → reads at a glance, costs one authored effect per ranged character
(b) a tint/overlay pose on the firing limb while the weapon is down
    → free for every ranged body, weakest read on a busy stage
(c) a HUD element beside the fighter's percent
    → strongest read, and the least like the genre
```

⚠ **NOT BLOCKING ANYTHING.** The cadence is correct without it; a player simply
cannot see why a press is waiting. ⇒ pick a channel and it is content work.

⭐ **NONE OTHERWISE. 2026-08-24: Jon's W8 playtest message closed the last one** and said
so explicitly — *"There are no unresolved maintainer design questions in this
feedback. Continue implementation rather than stopping for another decision
round."* The answered records follow, newest first.

### 32. ✔ ANSWERED 2026-08-24 — THERE IS NO STANDARD ADULT HEIGHT

> There is **no standard adult height**. Do not introduce `ADULT_HEIGHT` or
> normalize humanoids to one number. `robot_v3` should remain approximately 48
> units and should intentionally read as **shorter than most other characters**.
> The recent normalization pass pulled too many characters toward Robot v3's
> size, which made the cast generally too small.
>
> Correct principle: `CharacterDefinition → character-authored stature →
> congruent render + hurt/body geometry`, not `adult/humanoid/category → shared
> height constant`.
>
> Do not mechanically scale all 48-unit characters upward by the same ratio. Give
> characters intentional relative stature; leave ambiguous characters unchanged
> until visually reviewed.

⛔⛔ **THE QUESTION'S PREMISE WAS THE MISTAKE.** This section asked for a number
or a ratio and offered three shapes of answer, and Jon rejected the shape: a
category that produces a height is exactly the shared default decision 30 already
ruled against, one layer up. Stature is a per-character authored fact, and the
only measurement below that still matters is the one saying **38 of 45 characters
are 48.0 because nothing ever authored anything else** — that is a cast of
UNAUTHORED characters, not a cast of agreeing ones.

⇒ ⛔ **AND THE FIX IS NOT A SWEEP.** "Scale everything that is 48" would re-make
the same error with a different constant. Author the characters whose stature you
can actually reason about, one at a time, and leave the rest.

The measurement that produced the question is kept below because it is still the
inventory of what is unauthored.

⭐ **ONE NUMBER, OR A PRINCIPLE THAT PRODUCES THEM — the rest is content work
that can proceed the moment it exists.** This is not "which characters look
wrong": that is measured and listed below. It is the per-character number, which
you already ruled belongs to height rather than to the art (decision 30 —
*"height owns world size"*).

**What is measured** (`print_the_two_render_size_publishers --ignored`, 2026-08-24):

```text
38 of 45 rendered characters are EXACTLY 48.0 world px tall
  player_robot_v3 (the chibi protagonist) sets that number and IS 48
  standing at the same height: npc_viking_warrior · npc_viking_shieldmaiden
  npc_raid_enforcer · npc_salvage_guard · npc_olivia · npc_trent · npc_victor
  npc_ramen_nujan · npc_sybil · npc_vera_ruin · … and a sandbag, and solid_snake
```

`CharacterBodyKind::Standard` answers 48 and **nothing in the cast has ever
overridden it**. The three rows that DO author a height are all `Wide` — which has
no default — so each transcribed its own measured size to keep its output
identical. ⇒ **there is no chosen adult height anywhere in the tree**, and an
agent inventing a band from those three transcriptions would be reading a number
off the population and calling it an authority, which is the mistake decision 30
already corrected once.

**Why it needs you.** The direction is objective (an adult reads taller than a
chibi robot) and the magnitude is taste — and it is not free: `collision = body ×
scale`, so declaring a height changes that character's HURTBOX as well as its
render. That is the height contract working as designed, and it is a feel change
on shipped content.

⛔⛔ **AND THE ANSWER-SHAPE THIS SECTION ORIGINALLY OFFERED IS SUPERSEDED BY
DECISION 32 (2026-08-24), which is recorded below.** It asked for *"a number for
an adult human in this cast"* or *"a ratio to the protagonist, applied by
fiction"* — and decision 32 rejected exactly that shape: *"There is no standard
adult height. Do not introduce `ADULT_HEIGHT` or normalize humanoids to one
number… Give characters intentional relative stature; leave ambiguous characters
unchanged until visually reviewed."*

⇒ **SO WHAT IS STILL OPEN IS MUCH NARROWER: SIX NAMED CHARACTERS.**
`npc_pirate_admiral`, `cutlass_viper`, `lookout`, `navigator`, `quartermaster`,
`raider` — the cove pirates that read exactly as tall as the chibi protagonist.
Authoring one `standing_height` each is per-character stature, which is what
decision 32 asks for; it is not a cast sweep.

⭐ **AND YOUR OWN REPORT ALREADY CARRIES A NUMBER — it is the confirmation that
is missing, not the number.** *"the other pirates need to probably scale up 2x"*
⇒ 96 against the robot's 48. Say the word and it is six one-line content edits.

⚠ **IT IS NOT FREE, and that is the whole reason it waits.** `collision = body ×
scale`, so a declared height moves the HURTBOX as well as the render — six
pirates twice as tall are six pirates twice as easy to hit. That is the height
contract working as designed, and it is a feel change on shipped content.

⚠ **acceptance is your three reports** — the snake and AI slop, Sanic in his own
game, the cove pirates against the robot — not a number in a table. Tracked by
D165, whose measurement section carries the same data.

▢ **AND ONE QUESTION DECISION 32 DID NOT REACH:** 17 catalog rows author no
`body_kind` at all, and `Wide`/`Floating`/`Crawler` (27 rows) have no shared unit
BY DESIGN. Should they get one, or does a sprawled quadruped legitimately have no
"standing height"? Nothing is blocked on this; it decides whether the shared unit
is the cast's road or the humanoids'.

▢ **TWO LOOK-AT-IT CALLS THE MEASUREMENTS ARE ALREADY WAITING ON (promoted from
D165, 2026-08-28).** Both have their numbers; neither has an answer, and both have
sat in the execution ledger where a taste call reads as work nobody got to.

```text
THE SLOP     writing the AUTHORITY (`kin.size` + `BodyBaseSize`) instead of the
             mirror makes every slop 28 × 18.2 rather than 73.9 × 48 — a 2.64×
             shrink, in a level you play. ⛔ THE DEFECT AND THE SIZE ARE TWO
             THINGS: that the authored value never took effect is a bug; how big
             a slop should be is yours. The fix is one line and is deliberately
             not taken until you have said which size you want.
THE SNAKE    `snake_body_width()` derives from `mary_o_body_width()`, so the
             one-brick rescale halved it with nothing saying so: `world_per_pixel`
             0.35 → 0.182, collision 41 × 18 → 21.3 × 9.5 — 0.30 tiles tall.
             ⭐ the ratchet beside it could not notice: it pins the quad/body
             RATIO, which is scale-invariant, so it read 2.46× before and after.
             Whether a third-of-a-tile snake still reads as an enemy is yours,
             and the constant to change, if any, is HERS.
```



## ✔ ANSWERED 2026-08-23 — yes, the rollback wire format may grow

The match-level impact hitstop needed a new rollback-canonical match global,
which the shrink-only ratchet forbade. **Jon's reviewer approved the growth and
changed the policy**, and the reasoning is worth keeping because it is about the
guard's ancestry rather than about this one type:

> The ratchet has outlived the architectural condition it was designed to
> protect. It began as `central-rollback-ownership-may-not-grow`, where "only
> shrink" made sense during rollback-registration decentralization. Turning that
> MIGRATION constraint into a permanent prohibition on new canonical gameplay
> state is a different policy. Once impact hitstop changes the simulation clock
> it is rollback-relevant gameplay truth — the forced-rollback divergence
> confirms that. Hiding the value in an already-registered resource to avoid a
> new entry would make the architecture worse merely to satisfy the ratchet.

⇒ `rollback-wire-format-is-frozen` is now
`rollback-wire-format-changes-are-declared`: it still catches drift in both
directions and still demands stale entries be pruned, but growth is legitimate
when the baseline and `GGRS_ROLLBACK_SCHEMA_VERSION` move in the same commit. No
per-type waiver was created.

⭐ **AND THE IMPLEMENTATION IMPROVED ON THE REVIEW.** I had built a decrementing
tick counter; the ruling was to store an ABSOLUTE `until_tick` against `SimTick`,
which is already rollback state and already advances while `sim_dt == 0`. That
deletes the decay system outright, makes overlapping connects a deterministic
`max`, and keeps the property the whole design turns on: the hold cannot freeze
its own expiry.

⚠ the exit oracle then caught one more thing — a plain `rollback_resource_clone`
gives a PRESENCE-only probe, so a diverging `until_tick` would have been
invisible. It registers with a checksum projection.

## ✔ ANSWERED 2026-08-23 — George keeps 11 / 21

> Removing the hidden 1.6× while baking its existing result into the authored
> numbers is the right cleanup. If George feels too strong, retune 11/21
> deliberately afterward. I would not restore 7/13 merely because those were the
> misleading source numbers.

The receipt below records what the hidden multiplier was and how it was found.

### ✔ George Booul's `bivalence` lost 1.6x, and that was a real number in the match

`smash_charge_mult` had two payers. The second read the multiplier off how far a
move's clock had run through its leading Startup window — and a strike volume
only ever spawns INSIDE an Active window, which begins where that Startup window
ends, so the fraction was always clamped to full. Every use of a move with a
multiplier that never entered charge mode landed at the FULL multiplier, every
hit. That road is deleted; `MoveCharge` is now the only thing that pays.

`bivalence` is `Feel::Special`, so it never took the smash gesture and could
never EARN the 1.6 it authored. It was paid anyway. Its authored 7 / 13 damage
was 11 / 21 in every match played to date.

**Nothing has changed about how hard he hits.** The 1.6 is baked into the
authored numbers — damage 7→11 and 13→21, knockback 100→160 and 170→272, which
is exactly what the runtime was computing — so this George is the George that
has been fighting. ⭐ that is not a cosmetic choice: dropping the multiplier
without baking it took him out of every recovery situation he had, and
`the_cpu_throws_its_authored_recovery_during_a_match` went red over 1800 ticks
with him otherwise fighting normally. A damage change of that size moves who
gets launched offstage, which moves which situations arise at all.

**The open question is whether 11 / 21 is what you want him to hit for**, now
that it is written down instead of applied invisibly:

- leave it — this is the balance the demo has actually been tuned around;
- retune to the numbers as they READ — 7 / 13 is a much weaker neutral special,
  and the fighter brain will pick it less;
- make it genuinely chargeable — a held Special is a real thing the genre has,
  and it would need its own explicitly named policy rather than the smash's.

⛔ not answerable by refactor, and not a genre-research question either: it is
what this fighter should hit for.

### 1. ✔ ANSWERED 2026-08-17 — a bolt hits what a sword hits (former D23)

`projectile/systems.rs` now resolves victims through **`StrikeVictim`**, the
same named role melee uses, owned by `ambition_combat::hitbox` beside the
victim-geometry rule.

```text
INTANGIBILITY   ✔ CLOSED — a body carrying an EMPTY `DamageableVolumes` list
                  now offers NO target, so a bolt no longer lands on (and is
                  eaten by) a body a sword passes straight through
PRECISION       ✔ CLOSED 2026-08-22 — `step_projectiles` asks
                  `victim.reached_by(&kin.aabb().into())`, the same
                  `strike_reaches_victim` rule melee uses
```

⭐⭐ **RULED: the projectile respects the AUTHORED HURT VOLUME — the same geometry
melee uses.** One victim-geometry rule for everything, so a crouching or
ledge-hanging fighter reads the same to a bolt and to a sword, and an authored
hurtbox finally means one thing.
⛔⛔ **this is a real feel change on shipped content, and it is intended**: a shot
that connects today against a body whose authored volume is tighter than its AABB
will start missing. That is the point, not a regression to file.
⚠ per-volume overlap now runs on every projectile tick — **measure it rather than
assuming it is free**, and say so at the loop.

✔ **BUILT 2026-08-22.** The two checks collapsed into one: `reached_by` answers
intangibility for free, and `is_intangible`'s own doc says a caller that asks it
*"must not ask twice"*. Pinned by
`a_bolt_misses_the_gap_in_an_authored_silhouette` — same body, same bolt, same
position, only the published rectangle moves — and falsified by restoring the
coarse box, which reddens it while the two sibling bolt tests stay green. The
cost note is at the loop as asked: `strike_reaches_victim` walks a 1–2 volume
list where an AABB test stood, so it is a constant factor on a loop already
bounded by live shots × candidate victims.

⚠ **AND THE SHARED SILHOUETTE DID NOT MAKE PROJECTILE COLLISION CONTINUOUS —
two correctness gaps stay open, both verified against HEAD 2026-08-22** (raised
by a GPT review; re-read at the loop rather than taken on trust):

1. **Endpoint-only.** The step integrates first (`body.pos += body.vel * dt`)
   and then tests `victim.reached_by(&kin.aabb().into())` at the RESULTING box.
   Nothing sweeps the shape along its displacement, so a fast enough bolt can
   cross a narrow hurt volume between ticks. ⚠ **not urgent on shipped content**:
   the default projectile speed is ~360 px/s, about 6 px per 60 Hz tick, far
   under any authored volume. It becomes real with faster bolts, smaller hurt
   parts, or a lower tick rate.
2. **First victim wins.** `for victim in &victims { … break; }` takes the first
   qualifying body with no geometric or authored ordering. ⭐ **and the precise
   claim matters**: the PROJECTILES are already sorted by a global spawn
   sequence (`ordered.sort_by_key(|(_, seq)| *seq)`), so this is not a desync —
   every peer shares the spawn history and iterates the same way. What it is, is
   ARBITRARY: when a bolt overlaps two valid bodies on one tick, which one takes
   it is decided by archetype order rather than by anything a designer chose.

⇒ **they are ONE slice, and in that order**: sweeping the projectile against
`DamageableVolumes` produces a time-of-impact, and a time-of-impact is exactly
the arbitration key gap 2 lacks — nearest TOI, with the existing spawn sequence
as the stable tie-break. ⛔ do not fix 2 alone by sorting on distance-to-centre;
that is a proxy for the number the sweep would give for free.

### 2. ✔ ANSWERED 2026-08-22 — advance the measurement-submodule pointer periodically

⭐ **Jon, verbatim:** *"It doesn't matter, as long as advance it every so often."*

⇒ the ruling is that `dev/ambition_dev_measurements` must not go STALE
INDEFINITELY, and nothing finer. Bump the superproject pointer when convenient —
a batch of accepted measurements, or a repo citation that should stay checkable.

⛔ leaving it permanently pinned is refused. ⛔ do not build a policy, a cadence
or a check around this.

### 3. ✔ ANSWERED 2026-08-22 — disable rust-analyzer; no second target directory

⭐⭐ **Jon, verbatim:** *"Disable RA. I don't need it. If you aren't using it then
its bloat."*

⇒ ⭐ **he answered a question that was not asked.** Both offered answers spent
something — ≈35–100 GB of disk for a second check directory, or a standing build
tax from sharing one — and turning the consumer OFF spends neither.

⇒ the measured contention disappears with the process rather than being routed
around: rust-analyzer's `cargo check --workspace` restarted every ~50s and took
the target-directory lock each time, turning a 21-second gate into 1m26s of work
spread over ~9 minutes of `Blocking waiting for file lock on build directory`.

⛔ do NOT set `rust-analyzer.cargo.targetDir`, and ⛔ do not propose a second
target directory again on throughput grounds — the throughput problem has no
source once RA is off.

⚠ **the conditional in his answer is worth honouring**: agent sessions do have an
`mcp__rust-analyzer__*` server wired in. If an agent workflow starts depending on
it, that is a NEW fact and this decision should be revisited rather than quietly
worked around.

### 4. ✔ ANSWERED 2026-08-22 — it was Mary-O, and it is believed resolved

⭐ **Jon, verbatim:** *"It was maryo, but I think it was resolved."*

⇒ the row retires. Mary-O's three death routes — hit, timeout, and
pit/hazard/kernel reset — are each covered and each returns the body to spawn,
and the pit fixture re-arms a spent question block; nothing was reproducible
against them because the defect had already been fixed.

⛔ do NOT move the investigation to Ambition or Sanic — he named the host.
⛔ do not change Mary-O's proven replay path chasing this.

⚠ *"I think"* is the confidence: a restart failure observed again is a NEW report
with fresh evidence, not this row reopened.

### 5. ✔ ANSWERED 2026-08-22 — correct the level-1 CPU for feel

⭐⭐ **RULED: the easiest rung is bad at FIGHTING, not self-destructive.** A
level-1 CPU losing all three stocks to ITSELF inside a minute reads as broken
rather than easy, and the easiest rung is the one a new player meets first.

⭐ **the cost is small and already located**: the gap between a rung that self-KOs
and one that does not is a single authored field. Nothing else in the ladder
needs to move.

⛔⛔ **THE ROW'S ORIGINAL EVIDENCE IS RETRACTED AND MUST NOT BE RE-CITED.**
`0.84%` was **84%** — `damage_percent` returns a RATIO and the rig printed it
under a literal `%`. The CPUs always fought hard, and the corrected ladder
DISCRIMINATES: peak damage rises monotonically with the rung and the higher rung
out-damages the lower on all four pairs (3v1 48:45, 9v6 193:158). Confirmed in
the shipped composition by `app_it::smash_cpus_damage_each_other`, which states
its units in the assertion message.

⛔ the *"upper half self-destructs"* claim was exactly BACKWARDS: at 15 seeds
level 9 never self-KOs and every rung below it does, so self-preservation already
improves with difficulty. ⛔ the `rollout_depth` diagnosis is falsified by its own
instrument — depth 0 and depth 12 are flat — do not reopen it.

⚠ three seeds says something different from fifteen; fifteen is the number that
answers.

### 14. ✔ ANSWERED 2026-08-22 — 56 px stands, the crown rises, and the walk dip gets a real fix

**a. ✔ THE SHARED COLLISION WIDTH STAYS 56 px ON BOTH FORMS.** Identical-width-
for-every-form and box-inside-the-drawing are together decided by the NARROWEST
form, and the one-brick short form draws 60 px wide — so the old 64 collided on
empty air beside her. The grown form's box narrows too, and that is the accepted
price of the identical-width rule. ⛔ widening the short form's ART to ~68 px to
recover it was DECLINED.

**b. ✔ THE SHORT FORM'S CROWN RISES ~6 px — the art moves to meet the box.** The
box top comes from the height contract (short × 2 = grown exactly), not from the
art, so closing the +6 px of empty air above her hat is a redraw rather than an
acceptance. ⚠ **this knowingly moves the 40/40/20 head/body/legs split he
specified** — the head grows against the other two — and the trade is taken so her
silhouette matches her collision and ceiling contact reads correctly.
⚠ unchanged elsewhere: grown 0 px headroom, fire −14 px (its flame frills clear
the box on purpose).

**c. ✔ THE WALK DIP GETS THE POSE FIELD — a torso lower that does NOT move
`foot_y`.** The stride's extension is currently spelled as a downward
TRANSLATION, which is why every walk frame puts her foot below her own idle line:
small dips +0.50 to +1.00 units, grown +0.33 to +1.17, so she walks through the
floor by up to a fifth of a tile and the renderer's clipping warning is the canvas
noticing. The three authored numbers are `walk#0 leg_back_dy = 1.0`,
`walk#2 leg_front_dy = 1.0`, and `walk#1 bob = 0.4` feeding `foot_y = 30.2 + bob`.

⛔ **do not zero them.** Clamping removes the stride extension and the mid-stride
bob rather than respelling them, and flattens the walk he tuned.

⭐ the new field is engine vocabulary, not a Mary-O patch: every future pose gets
the right way to say *"the body dips, the contact point does not"*.

⚠ this changes an animation he has already seen and approved, taken deliberately;
⚠ the grown form's dip is PRE-EXISTING — those frames came through the rig
refactor byte-identical — so the fix corrects shipped behaviour on both forms.

### 15. ✔ ANSWERED 2026-08-22 — impact hitstop is a bounded MATCH-LEVEL request

⭐⭐ **Jon, verbatim:** *"Use a bounded match-level request emitted by a
successful connect. Keep slot-0 time control for genuinely participant-specific
affordances such as bullet time and blink hold. Impact hitstop belongs to combat
presentation and should work without any PrimaryPlayer. The request expires using
unscaled time; the clock returns to normal because arbitration has no remaining
freeze request, so there is no explicit hand-back path capable of leaving the
world at 0.0 forever."*

⇒ ⭐⭐ **the question's framing was rejected.** All three options offered an OWNER
(nobody / the most recent connect / the framed fighter); the answer is that
impact hitstop has NO owner — a connect emits a bounded request and the match
arbitrates.

⛔⛔ **the 0.0-forever failure is designed out rather than guarded against.** The
request expires on UNSCALED time and normal pace is the ABSENCE of any live
request, so there is no hand-back path a missing `PrimaryPlayer` can fail to
walk. The 2026-08-07 freeze (*"the characters are just stuck in air"*) cannot
recur in this shape.

⛔ **bullet-time and blink-hold do NOT move with it** — they are per-PARTICIPANT
feel affordances by ADR 0010/0011 and stay slot-keyed.

⚠ CPU-vs-CPU then screen-freezes correctly with nobody playing, which is the case
that exposed this.

### 16. ✔ ANSWERED 2026-08-22 — the layout tool owns position, and ownership follows the LAYOUT MODE

⭐⭐ **Jon, verbatim:** *"choose 1, with one refinement: position ownership should
follow the layout mode"*.

⇒ where the tool computes placement — Free layout, arranged by `world auto-layout`
from the LoadingZone graph — the area spec DROPS `world_x`/`world_y` and stops
claiming something it lost.

⭐ **the refinement is the load-bearing half**: ownership is a property of the
world's LAYOUT MODE, not a project-wide constant. A world whose placement is not
computed from a graph is a different case and must not have the field stripped by
the same sweep. ⇒ what follows is engineering, per mode: which modes compute
placement, and what `level diff-specs` compares in each.

⛔ **do not bulk-rewrite the 52 drifting specs to silence the check.**
Re-recording live values is the option NOT taken; doing it anyway answers this by
accident.

⚠ 13 of the 52 are a usage limit rather than drift — specs for levels in another
world file, which the command cannot see because it takes one `--ldtk`. A second
flag is warranted either way.

### 17. ✔ ANSWERED 2026-08-22 — `DebugLabel` is debug, it keeps shipping, and edge-exit labels are proximity-gated

⭐⭐ **Jon on the name, verbatim:** *"Its debugging, but we don't need to stop
shipping it, we are nowhere close to a real exploration game. This is part of the
scaffold and prototype, and if a GPT review claimed otherwise, then it was out of
line."*

⇒ the name is honest and stays. The density measurement — 14 DebugLabels in
`gate_stack_lower`, 13 in `drain_alley`, 12 rooms carrying both sources —
describes a PROTOTYPE, not a defect.

⛔ **do not rename it to signage, do not gate it out of the build, and do not run
a triage pass over the authored labels.**

⛔⛔ **the standing lesson is the last clause, and it generalises**: *"we are
nowhere close to a real exploration game"* answers a whole CLASS of polish
findings. A reviewer measuring a prototype against a shipped game is out of line,
and a finding of that shape is weighed against the project's stage before it is
filed. ⚠ for the record this one came from the D161 density measurement rather
than from a GPT review; the ruling on its class stands either way.

⭐⭐ **Jon on the zone labels, verbatim:** *"Proximity gate them. Note, different
games will want different behaviors here, and it should be easy to have them
always on, vs proximity, vs dont use them at all."*

⇒ the 24 always-on EdgeExit labels join the 127 Doors on the proximity gate.

⭐⭐ **the second sentence is the engine half and outranks the first**: label
visibility is a THREE-VALUED POLICY the consuming game selects — always on ·
proximity · off — with proximity as Ambition's choice. ⛔ do not ship a second
hardcoded rule; a game that wants always-on labels must not edit engine code.

### 18. ✔ ANSWERED 2026-08-22 — a hit's art follows BOTH the victim's material and the blow's strength

⭐⭐ **RULED: both.** `ImpactMaterial::{Flesh, Robot, Metal}` is the VICTIM's fact
and picks the family; soft-vs-hard is the ATTACKER's fact and picks the intensity.

⇒ the two vocabularies are joined and neither is subordinate. That is what
explains all four shipped rows — `hit_soft` / `hit_hard` / `hit_metal` /
`hit_energy` — which material alone could only explain three of.

⛔ **the cost is a MESSAGE change, not a lookup.** The material lives on the
victim's `HurtFeedback` and `VfxMessage::Impact` carries a position and nothing
else, so ~10 emitters must start saying what was hit and how hard.

⚠ the shipped row set is not a full cross product. A combination with no authored
row needs a STATED fallback — the untextured yellow quad that started this is
exactly what a silent one looks like.

### 19. ✔ ANSWERED 2026-08-22 — the sheet registry keys by FILE ROOT

⭐⭐ **RULED: key by FILE ROOT.** A file root names a PRODUCT — one published
page — which is what the registry actually serves, and it is the key
`record_index()` and the character-geometry road already use.

⇒ the 148 root==target sheets are unaffected; the 48 that share a rig adapter
(robot 18, toon 16, goblin 9, sandbag 3, ninja 2) each get their own key, so the
last-manifest-wins class becomes IMPOSSIBLE rather than reported. `robot` stops
losing its own 256x256 sheet to `tech_bro_disruptor`, `goblin` to
`ranged_skirmisher`, `sandbag` to `sandbag_full_review`.

⛔ **"retire the stale manifest" was never available for two of the three** —
`tech_bro_disruptor` and `ranged_skirmisher` are distinct characters legitimately
declaring a shared rig target, so there is no pair to retire.

⛔⛔ the standing principle this executes, from the 2026-08-18 review: *"Do not
let a sprite-renderer target string accidentally become the durable identity of a
character package."* `CharacterId` is semantic and durable; a renderer target is
an authoring choice; a sheet file root is a product. ⚠ promoting the key to
`CharacterId` was offered and NOT taken — the registry is a product lookup.

⚠ this changes what a shared engine resource returns for 48 files. Every live
consumer looks up a name where root == target, so nothing shipped should move;
verify that rather than assume it. Measurement: [`../../dev/reviews/sheet-target-collisions-2026-08-19.md`](../../dev/reviews/sheet-target-collisions-2026-08-19.md).

### 6. ✔ ANSWERED 2026-08-17 — hitlag freezes the body that is in it (former D114)

⭐⭐ **Jon, verbatim:** *"keep the landed fix and overrule the old prohibition …
**hitlag is a combat/body semantic, not something that should depend on whether a
body happens to occupy the primary local-control road** … Keep `sim_dt = 0.0`
during that body's hitlag. Mark the old prohibition superseded. **If hitlag later
feels too sticky, tune its duration/shape rather than restoring a
controlled-body/actor asymmetry.**"* Recorded in
[`maintainer-decisions.md`](maintainer-decisions.md); `818218949` is the code.
⚠ the process failure is not excused by the outcome: the commit consulted neither
document that had previously warned against this fix, so the prohibition was
*unseen rather than overruled*.

⭐⭐ **AND ANSWERING THIS UNBLOCKED D117, which is the consequence to act on.**
This decision gated TIME INTEGRATION and nothing wider: the controlled and actor
roads still have two body integrators, and unifying them means merging their
limbs — hitlag-dt gating and ledge carry are the home road's, the flight limb is
the actor road's. The ruling answers *"does the merged integrator freeze an actor
body on its own hitstop?"* **yes, on both roads.** ⇒ D117's last structural item
is now executable, and so is folding the three per-population
`decay_reaction_timers` calls into one system (the controlled site decays on
`frame_dt`, the other two on sim `dt`).

### 7. ✔ ANSWERED 2026-08-17 — a dropped weapon persists PER ITEM, not by one rule

The lifetime bug is fixed for ability/currency/health drops: the entity and its
visual now share room scope.

⭐⭐ **RULED: authored per item.** A story or unique weapon stays in the world
where it fell; an ordinary dropped one is room-scoped like the other drops.
⭐ consistent with the same day's inventory ruling, where UNIQUENESS is what
decides whether a thing needs its own identity.

⛔⛔ **AND IT PROMOTES A KNOWN RESIDUE INTO A PREREQUISITE.** A minted instance
**not in a hand** at save time — lying in a room, in flight — is undescribed and
lost today, because the description remembers no POSITION (D133's open item). A
persisting dropped weapon IS that case, so it must be built first rather than
noted beside. ⚠ it also needs a per-item authoring field that does not exist yet.
⛔ **whatever is built, simulation entity and presentation share ONE lifetime** —
that was the original defect and it is not re-litigated by the persistence rule.

### 8. ⏸ DEFERRED 2026-08-17 — the absence list waits for a bigger cast

⭐⭐ **THE FORK IS DECIDED AND PINNED — reconciled 2026-08-17. Only the absence
list below is still yours.** The recommended option, *universal baseline with
absences authored*, was taken during the D146/D151 campaign:

```text
the grant arm is DELETED      MatchAbilities::apply now reads
                              authored.unwrap_or(AbilitySet::NONE);
                              the doc still names unwrap_or(self.permitted)
                              as "a migration bridge ... until today"
the baseline is AUTHORED      smash fighters state the full ground kit, with
                              fly / blink / dash omitted DELIBERATELY and each
                              omission carrying its reason
```

⭐ **and the population is guarded, not assumed** —
`smash_roster_movesets.rs` walks every id the character grid offers, computes
`effective_abilities(authored, rules)` and requires each to equal
`SMASH_FIGHTER_KIT`. ⚠ it also carries the two guards that stop it going
vacuous: **at least 8 fighters must resolve** (*"the host is not composing the
cast and this test is about to prove nothing"*) and **at least one must author a
kit that DIFFERS** from the stage's, or the union is a no-op and the test would
pass on the old mask too.

⏸ **DEFERRED BY JON 2026-08-17.** Fourteen fighters is a small sample and the kits
were only just completed, so everyone keeps the uniform effective kit and
**personality comes from MOVESETS rather than from missing verbs** until enough
matches have been played to know who feels wrong.
⭐ nothing is blocked and nothing is lost: the grant scaffold is already deleted,
so an omission MEANS something the day one is authored — the mechanism is ready
and only the content decision waits. ⛔ do not propose an absence list unprompted,
and ⛔ do not author an absence for balance reasons in the meantime.
⛔ refused for cause, and not on the table again: letting the mode grant a body a
verb it lacks — it would delete the invariant
`a_match_cannot_grant_a_verb_the_character_does_not_have` pins.

**The part that is genuinely yours** is the per-creature absence list — can a
goblin double-jump, can a crawler ledge-grab. The engine has no opinion and
should not invent one.

### 9. ✔ ANSWERED 2026-08-17 — what the per-turn suite should run (asked 2026-08-14)

⭐⭐ **Jon, verbatim:** *"keep the per-turn gate small … do not add `cargo test
--workspace --lib` to every turn. The workspace lib suite remains a required
pre-push/finalization check. Likewise, **feature-gated suites should be run when
the affected subsystem is touched, not wholesale every turn**."*

```text
per-turn EXECUTABLE gate   gate_suite.py → cargo test -p ambition_app --test
                           app_it. ⛔ DO NOT GROW IT.
pre-push / finalization    cargo test --workspace --lib. Required ≠ gated.
touched the subsystem      that crate's feature-gated suite. ⛔ never wholesale.
```

⭐ **what the third tier means in practice — 24 crates hide 629 tests**, and the
delta is where the interesting ones live (`ambition_input` 54 → 115,
`ambition_audio` 25 → 64, `ambition_touch_input` 4 → 45). So *"I ran `--workspace
--lib`"* is not evidence about a crate whose real suite is ten times its bare one.
**Two named consequences, both live surfaces:**
* `demo_mary_o_app/tests/painted_blocks_still_change_their_art.rs` is
  `#![cfg(feature = "visible")]` in its entirety — the only thing in the repo
  asserting what a block LOOKS like, and it exists because one line opted every
  block in a cavern out of art updates. ⇒ run `--features visible` before and
  after any Mary-O visual work.
* `ambition_conversation` gates `dialog` behind `ui`, which holds **both**
  authored-logic Yarn falsifiers. `cargo test -p ambition_conversation --features
  ui --lib` is the only command that compiles them.

### 10. ✔ ANSWERED 2026-08-17 — shake stays constant IN THE WORLD; the name changes

`CameraShakeState::amplitude_px` was added straight to the camera's translation
in WORLD units, so the on-screen displacement scaled with `orthographic_scale` —
**the same hit shakes the screen less the further the camera is pulled out**. The
field's own name said "px" while the behaviour was world units.

⭐⭐ **RULED: constant in the world — keep the behaviour, fix the NAME.** A shake
is a physical displacement of the viewpoint, so a camera showing more world
registers it as smaller; a camera that pulls out as a fight grows should calm the
screen rather than thrash it harder.
⇒ **what changes is `amplitude_px` → `amplitude_world`, and
`HIT_SHAKE_GAIN_PX_PER_S` with it.** ⛔ the maths is not touched.
⭐ and the rename is now unambiguous rather than merely tidier: **one world unit
IS a base-grid pixel**, so `_px` would stay permanently confusable while `_world`
says exactly which quantity this is.

### 11. ✔ ANSWERED 2026-08-17 — split-screen layout is ADAPTIVE WITH HYSTERESIS

⭐⭐ **RULED 2026-08-17: adaptive with hysteresis** — one shared framing while
participants are close, splitting into viewports as they separate, with hysteresis
so it cannot flap at the boundary. Recorded in
[`maintainer-decisions.md`](maintainer-decisions.md).
⇒ **and that settles the engineering fork under it by implication.** A layout that
can split at ANY MOMENT cannot be served by one set of world-space entities, so
**duplicate per view** is the only surviving option.

```text
label_layout.rs   per-view projections   d09229ceb (2026-08-15)
nameplates.rs     per-view projections   d09229ceb
view_isolation.rs isolate by RELATIONSHIP, not identity   b732e5d6a
parallax.rs       per-view projections   IN THE WORKING TREE, UNCOMPILED (2026-08-20)
```

⚠ **the parallax row is written but not yet built.** `mirror_parallax_layers_per_view`
gives each live view its own panel set (same `PresentedForView` vocabulary as the two
label families), `sync_parallax_layers` places each panel against the camera that draws
it and sizes it from that view's `CameraViewport`, and a panel whose view or camera
cannot be resolved is HIDDEN rather than left at the world origin. `view_isolation`
grew `ProjectionRestingLayers` so a collapsing split returns the backdrop to the private
parallax layer instead of layer 0 (layer 0 would hand it to every portal capture).
⛔ **the consequence to look at first**: panel extent now tracks the resolved gameplay
rectangle, so any composition whose gameplay rect is not 1600x900 gets a differently
framed backdrop — `capture_scene` goldens included.

⚠ **the distance threshold and the hysteresis band are FEEL values Jon has not
named** — measure them against a real two-player session rather than picking
constants.
⛔ **adaptive layout promotes the silent-wrong fallback into a real defect**, and
under this policy several cameras is the ordinary case rather than the exception.
The two the row originally named are CLOSED: label layout and nameplates stopped
inventing a `Vec2::ZERO` focus in `d09229ceb` — they iterate views, so every
iteration holds a real `CameraViewState` and there is no branch left to invent one
in. The one that was still open was `parallax.rs`, which did not invent a focus but
LEFT a position at the origin: `.single()` on the main camera returned
`Err(MultipleEntities)` the instant a second camera existed, and every screen-sized
backdrop panel stayed at its spawn transform. It declines now.
✔✔ **BOTH LATENT ITEMS ARE CLOSED — verified 2026-08-22.**
* the never-written `CameraViewState` cannot happen: it is spawned WITH the view
  in `camera_snapshot.rs`'s view bundle, under its own comment *"a reader must
  never see a frame where the view exists and its state does not"*.
* `MainCameraEntity` no longer loses a writer silently — `publish_main_camera`
  is **first-writer-wins, LOUDLY**, `tracing::error!`ing the second rig and
  saying to address several rigs by `MainCamera` + `PresentsView` instead. And
  the production reader is gone: the kaleidoscope scrim *"used to borrow the
  gameplay camera through the `MainCameraEntity` resource"*, while
  `camera_follow` resolves each camera through its own `PresentsView` link.

~~▢ what is still latent: a view whose `CameraViewState` was never written (no
`camera_follow` in the composition, or a view bound to no camera yet) reports a
default focus, which IS the world origin — the fallback survives as a component
DEFAULT rather than as an `unwrap_or`. It is invisible today because such a view's
projections are isolated onto a band no camera renders, and it becomes visible the
moment anything draws for a view before its camera resolves.~~
⚠ `MainCameraEntity` is a SEVENTH process-global *"the main camera"* resource that
this layout has to answer for. Census 2026-08-20: **two writers**
(`ambition_render::platformer_presentation::spawn_main_camera`,
`ambition_app::app::scene_setup::host_presentation_scaffold`), both inserting
unconditionally beside a `PresentsView` link that refuses — so two rigs is
last-writer-wins, silently. **One production reader**,
`retarget_kaleidoscope_scrim`, which points the cube's dim-scrim at it with
`UiTargetCamera`; under a split that dims ONE viewport rectangle instead of the
screen, and it wants a DISPLAY answer rather than a view answer. **Two test
readers** (`mary_o`/`sanic` `ov1_draws_the_world`) whose assertion messages are
already stale — they claim camera-follow and the portal viewer resolve through it,
and neither has since D116 M2.

### 12. ✔ CLOSED 2026-08-17 — the `ambition_map_assets` submodule pushes fine

⭐ verified from inside the VM rather than assumed: local HEAD and
`git ls-remote origin HEAD` agree and `origin/HEAD..HEAD` is empty. Jon had
provisioned the credential aliases; this row duplicated the outage closed
2026-08-15 further down.
⛔ **the consequence worth remembering**: a superproject gitlink can point at a
commit that exists only in one working tree, and **nothing in the superproject's
own green push says otherwise**. ⇒ never resolve a rejected submodule push by
rolling the superproject pointer back — the pointer is the symptom, the
credential is the cause.

### 13. ✔ CLOSED 2026-08-17 — the workspace policy suite is green and CI watches it

⭐ measured: `cargo test -p ambition_workspace_policy` is **34 passed / 0
failed**, and the five rules this row named are all still IN `engine.toml` (187
rules) — so the twelve violations were FIXED rather than waived away. It now runs
in CI's `engine-tests` job (~5s; that job because the crate parses manifests and
source text and links no production crate).
⭐⭐ **item 1 was fixed the way the row asked**: the `gate_portal` determinism
flag was a false positive on code that collected and then sorted, and the row
said *"a waiver would be the wrong answer — make `phases` a `BTreeMap` so ordered
iteration is a property of the TYPE."* It is, and the file guards against a
revert.
▢ **what is genuinely left for you** is the row's original question — whether all
187 rules deserve enforcement — now much cheaper to answer, because they all pass.
⛔ **the shape that made this expensive**: a suite nothing runs is a suite that
goes red and stays red, and both failures were guards that were CORRECT when
written and became wrong when a rule moved under them. ⇒ **when you add a check,
name the TIER that runs it.**

### 28. ⏸ PARKED 2026-08-20 — can a SPLIT SCREEN demo relativity at all, and does TwinTrack want a rollback host?

⛔ **TwinTrack is PUT DOWN, and neither question below is open work.** Jon,
2026-08-20, right after confirming the second seat drives on hardware: *"I want
to put twintrack down for now and get back to the main games. Twin track is fun,
but not the primary target."* The analysis is recorded because it was measured,
not because it is queued — do not pick it up as a task.

Jon, 2026-08-20, after driving the second seat for the first time: *"I'm not
sure we can even really demo relativity with this split screen"*. He is right,
and the limit is the construction rather than the presentation.

**Both panes render one instant of the simulation's coordinate time.** They are
two positions on ONE simultaneity surface, so what they can honestly disagree
about is optics — retarded images, aberration, Doppler, each twin's own clock
reading — and every one of those the per-observer views already get right. What
they cannot disagree about is SIMULTANEITY, which is the entire content of the
twin paradox. Two panes side by side read to a viewer as *two frames' views of
now*, and they are one "now" drawn twice.

```text
keep it as OPTICS      the exhibit is honest about what it shows: light delay,
                       aberration and Doppler, and the paradox stays narrated
make each pane a SLICE each pane resamples every worldline where it crosses THAT
                       observer's constant-time surface. The same event then sits
                       at different positions in the two panes, they disagree
                       about where the other twin is "right now", and the
                       disagreement FLIPS at turnaround — the paradox, visible
```

⭐ **the data for the second already exists**: `WorldlineTracked2d` /
`WorldlineHistoryView2d` store the history a slice would resample. It is not a
camera change — a pane stops being a VIEW and becomes a SLICE.

**And a second question rides along.** The shipped `ambition_app` is a
`SimulationHost::Rollback` host for every route, and `visible_composition.rs`
says why: not netplay, but a stable timestep (`check_distance = 0`, rollback
dormant). TwinTrack wants that timestep MORE than a platformer does — proper
time integrated at a variable `dt` makes clocks disagree for reasons that are
not physics — and wants the rollback half not at all. The half it does not want
charges a real price: a GGRS session's handle count is decided once at session
start and is never resized, which is exactly why the laboratory twin was inert
in the game while the standalone `fixed_tick` binary drove her fine.

```text
one host, always   simplest; a route that declares seating must do so BEFORE the
                   session starts, which is what landed 2026-08-20
per-route host     TwinTrack takes `fixed_tick`, Smash keeps rollback. ⚠ the
                   schedule is chosen at BUILD time, before any sim plugin
                   registers, so this is a real design slice and not a flag
```

⇒ the seating declaration landed either way and is not waiting on this: Smash
genuinely wants rollback AND has two seats, and "who is playing" had been
declarable only from inside the rollback backend.

### 29. ✔ ANSWERED 2026-08-22 — sweep the crates a CARVE touches, not the workspace

⭐⭐ **RULED: the gate stays `cargo check -p ambition_app --all-targets`.** A carve
additionally compiles `--all-targets` on each crate it touched.

⇒ that targets the actual failure mode: a carve moving a type or a function is
exactly the change that breaks a SIBLING crate's test build, and the app gate
builds none of them. The two 2026-08-21 breakages were both from that day's
carves; a sweep of all 54 crates found zero others, so this is INCIDENTAL and
correlated with carving rather than endemic rot.

⛔ **do not widen to `--workspace --all-targets` or `--workspace --tests`.**
Measured: the 12-crate touched sweep took ~10 minutes cold, the full 54 is
proportionally worse and would have bought nothing, and `--workspace --tests` has
FILLED THIS DISK here.

⚠ D33 is a carving campaign, so this is a live discipline, not a precaution.
⛔ a third instance of a red library test build must not be treated as a surprise.

### 20. ✔ ANSWERED 2026-08-22 — a boss's hoard is per-boss eventually; currency for the demo

⭐⭐ **Jon, verbatim:** *"Decide per boss, for the current demo, the reward can
just be currency. In the real game we will give different rewards."*

⇒ ⭐ **the shape is per-boss; the SCHEDULE is not.** The eight
`PickupKind::Custom` ids in `boss_profiles.ron` — `pirate_hoard`, `gnu_scroll`,
`noodly_relic`, `trex_bone_relic`, `collapsed_relic`, `divergence_shard`,
`stack_frame_relic` and one more — become currency now, so a defeated boss stops
paying nothing. The per-boss answer (an ability, a relic item, a quantity) is
authored later, one boss at a time, in the real game.

⛔ **do not invent eight item definitions to close this** — that option was not
taken, and inventing them is authoring content policy at an engine seam.
⛔ do not read the currency answer as the final one; it is the demo's placeholder.

⚠ the mechanism is already loud: a `Custom` payload reaching the grant warns with
the id and says nobody was awarded it, so a boss whose real reward has not been
authored is visible rather than silent. ⛔ the silent `_ => {}` that swallowed it
is what let eight shipped bosses drop empty treasure without a line of evidence.

### 21. ✔ ANSWERED — separating control authority from AI policy: TAKE THE BREAK (option A)

**The question: may `Brain` lose its `Player(PlayerSlot)` variant, given that
doing so changes the rollback wire format the absence contract freezes?**

The 2026-08-19 review calls this the next major actor-monolith seam:
`Brain::Player(PlayerSlot)` combines two different ideas — *which participant
drives this body* and *which brain backend this body uses* — so possession
transfers an AI-backend variant in order to transfer control authority. The
review explicitly REJECTS the `Brain::Capability(BrainId)` + registered-dispatch
direction (a dynamic executable service locator) and asks for a typed
decomposition instead.

**The evidence, measured rather than estimated.** `Brain::Player` has ~115
references; **85 of them are comments**. The real code surface is about thirty
sites, and most are the enum's own methods:

```text
brain/mod.rs          9   the enum's own dispatch/label/compare methods
possession.rs         3   inserts Brain::Player(PRIMARY) — the control TRANSFER
dormancy.rs           4   "is this a participant-driven body"
causal.rs             6   test fixtures
avatar/bundles.rs     1   the home avatar spawns with it
prepared_match.rs     1   ControlAuthority::LocalInput -> Brain::Player
opening.rs, acting.rs 2   "which slot drives this body" (already asked once each)
input_adapter.rs      1
```

⭐ **and the concept is already half-named**: `character_runtime::ControlAuthority`
exists, with a `LocalInput { source, channel }` variant that is lowered INTO
`Brain::Player`. The carve would make that lowering unnecessary rather than
introduce a new idea. `Brain` would then hold a single variant
(`StateMachine(StateMachineCfg)`), which is the review's "AI policy/state remains
a domain-owned typed component" reached by deletion.

**⛔⛔ THIS WAS NEVER A DECISION, AND ASKING IT WAS THE DEFECT.** The row was
filed 2026-08-19 offering Jon three options — take the break / add first / defer
— on the premise that *"the cost lands on save/peer compatibility, which is yours
to spend."* **That premise was already false, by a standing ruling twelve days
older** ([`maintainer-decisions.md`](maintainer-decisions.md), 2026-08-08), which
answers it in the imperative:

> *"I'm not concerned with saved replays and net play right now. We need to
> maintain no backwards compatibility there and we can say that the latest build
> is only compatible with itself… In fact we don't have any net play yet.
> **Agents have asked me this question in different variants many times and it's
> not worth perseverating on.**"*
>
> ⛔ *"so rename crates freely, change registrations freely, bump the schema
> version and move on — no migration, no compatibility shim, no deferral of a
> good change to protect a replay that does not exist."*

⇒ **Option A, by the ruling. There is no migration to write and nothing to
protect.** Bump the version, re-record the three baselines, move on.

⚠ **and note what the ruling itself predicted.** Jon flagged this as a PROCESS
problem, not a technical one — *"agents have asked me this question in different
variants many times"* — and this row is another instance of exactly that,
dressed as an architecture decision. ⛔ **before filing anything under "the cost
lands on compatibility", grep `maintainer-decisions.md` first.** A wire-format
question has a standing answer and re-asking it costs a session of a maintainer's
attention on a settled point.

⛔ the one thing from the original framing that still stands, because it is about
CORRECTNESS rather than compatibility: **do not leave two writable sources of
"who drives this".** That is the `ScriptedControl`/`ControlHolds` breach shape —
a derived fact and its source disagreeing, resolved by whoever writes next. The
landed slice avoids it by being a pure DERIVE with one source, which is why it
was safe to land ahead of the deletion.

⭐ **SLICE 1 IS LANDED (2026-08-20).**
`control::DrivingParticipant(PlayerSlot)` is the fact by itself, DERIVED at the
time by `project_driving_participant` from `Brain::Player` plus
`PossessionState` — so it added no snapshot entry and no ENCODED bytes, and none
of A/B/C applied to it. ⚠ **slice 2 below changes that**: with the variant gone
the component is authored state and is REGISTERED.

⛔ **it DID need a version bump, and the reason is worth knowing before the next
derive.** `rollback_coverage` offers three outcomes — registered, DECLARED
derived, or waived — and "it is genuinely a derive" is the JUSTIFICATION for the
second, not a substitute for taking it: a reprojected component that says so
nowhere fails all three, which cost eight coverage tests plus the exit oracle.
And the declaration's `detail` string reaches `schema_dump` and is hashed into
`schema_fingerprint`, so **declaring a derive moves the schema even though the
component encodes not one byte.** v52 → **v56**. `ActingParticipant::driving_slot` now
reads it instead of matching `Brain::Player`, which is the first consumer moved
off the conflated enum. The answer is identical today (one system asserts
exactly one body carries `Brain::Player(PRIMARY)`, so brain and derive agree by
construction); what changed is that the reader no longer knows where the answer
is spelled.

⛔ **this is NOT option B.** B's danger is two WRITABLE sources of "who drives"
disagreeing, which is the `ScriptedControl`/`ControlHolds` breach. A derive has
exactly one source of truth by construction, so there is no window where
possession can be expressed two ways.

⚠ the precedent is **`InCustodyOf`** (`declare_rollback_derived_component`), and
`InCustodyOf` ALONE. `ScriptedControl` is *registered* (`rollback_component_clone`)
— the opposite shape — and an earlier version of this paragraph cited both.

⛔⛔ **and a name was already taken.** `character_runtime::prepared_match::`
`ControlAuthority` exists, and is re-exported from the `ambition_platformer2d`
SDK, for a DIFFERENT fact: what a roster SEAT attaches
(`LocalInput { channel, source }` or `Brain { profile }`). That is a binding SPEC
read once at match preparation; the new component is a body's live driver,
re-derived every tick. The paragraph above this one used to call the existing
type "already half-named" toward this concept — it is not, and building the new
fact under that name would have put two meanings on one word in one crate.

⭐⭐ **SLICE 2 IS LANDED (2026-08-20): `Brain::Player(PlayerSlot)` IS GONE.**
`Brain` is `StateMachine(StateMachineCfg)` and nothing else — it is a
ONE-VARIANT enum today, deliberately left as an enum because collapsing it into
a struct is a separate decision with its own reader churn.

⚠ **`DrivingParticipant` stopped being a DERIVE in the same change, and it had
to.** The declaration's own justification was *"reprojected from `Brain::Player`
and possession every tick"* — half of that upstream no longer exists, so the seat
a participant drives from is now authored at the spawn/seat site and lives in that
component and nowhere else. It is `rollback_component_clone` /
`actor.driving_participant`, and a rewind that did not carry it would restore a
body nobody drives. `derived.driving_participant` leaves the schema in the same
commit. **v56 → v58** (57 skipped so a concurrent lane and this one could not take
one number).

⭐ **and there is still exactly ONE writer.** `project_driving_participant` stopped
deriving from `Brain` and became a RECONCILE: while `PossessionState::possessed`
is live it takes the primary seat off `PossessionState::home` and puts it on the
driven body, and it hands the seat back — and clears `home` — when the possession
ends or the driven body vanishes. Outside a possession it does nothing at all,
which is what keeps it from having an opinion about a seated versus fighter.
`possession.rs` states the decision and writes no seat; the spawn/seat sites
author the initial value and never touch it again.

⛔ **`restore_brain` and `restore_scope` are DELETED.** Nothing is displaced by a
possession any more — the driven body keeps its own policy for the whole
possession and resumes deciding with it the instant the seat leaves — so there is
nothing to put back. Two follow-on simplifications fell out of that, and both are
behaviour changes worth knowing: a `BrainCommand` that lands mid-possession now
applies to the LIVE brain instead of only updating the source it would resume
into, and `reconcile_brain_bindings` no longer skips a driven body. Both used to
skip because the live brain was the driver's; it is the body's own again.


### 22. ✔ RESOLVED 2026-08-19 — external-consumer enemy authoring follows the post-D73 character seam

The external-consumer sentinel had not compiled since D73 deleted the roster.
**RULED: a third party authors an enemy as a `CharacterDefinition`, with
controller policy in `BrainProfile`, and the placement names the required
`CharacterId`.** The umbrella exports the small authored vocabulary needed to
state those facts, so the fixture still depends on `ambition_platformer2d` alone.

```text
body        max_health, run_speed, move_style, contact strength/damage
controller  Wanderer, patrol/chase effort, aggro radius, attack range
placement   OnRoomReenter (the EnemySpawnSpec default)
```

`CharacterRosterFragment`, `CharacterRosterAppExt`, and
`register_character_roster_fragment` are gone from the fixture, and the staged
spawn names `OUTLANDER_SENTRY_CHARACTER_ID` directly.
⭐ **guarded by** a fixture test that reads the prepared character back through
the public umbrella and pins the migrated body and controller values — this
preserves the sentinel's purpose: a public SDK break the shared workspace cannot
see still fails in the independent consumer.

### 23. ✔ CLOSED 2026-08-20 — it was the missing spacing primitive, and body contact turned all seven green

⭐⭐ **THE QUESTION BELOW IS SUPERSEDED — §25 answers what to BUILD.** Jon,
2026-08-20: *"the limit cycle is very plausibly exposing a missing physical
spacing primitive rather than a brain defect. The agent was right to stop rather
than compensating for missing contact by making the AI stranger."* ⇒ the change
to make is opt-in body contact in the movement sweep (§25), NOT a spawn asymmetry
and NOT more randomness.

⭐⭐ **AND THE RE-MEASUREMENT IS IN: `smash_it` 26/7 → 34/0** (`da884be08`). All
seven — both repertoire guards and all five `the_stage_kills` — pass with fighters
that are solid to each other, and **nothing in the brain moved**. Jon's reading of
the limit cycle was right.

⚠ **the order this was taken in is the part worth keeping.** The diagnosis below
was made in a match running NO smash combat rules, so it was a CANDIDATE and was
recorded as one; `a1c251b44` closed the route-entry hole and the suite went five
red → **seven**, because two repertoire guards had been green only while staling
and the rest of the ruleset were absent. The capability was then built because
the GENRE wants it and Jon had ruled on it — not to fix these tests — and only
then re-measured. A build aimed at the seven would have been tuned until they
passed, and would have proved nothing about either.

⛔⛔ **MEASURED 2026-08-20: THOSE FIVE TESTS RUN A MATCH WITH NO SMASH COMBAT
RULES AT ALL, AND THAT INVALIDATES EVERY DIAGNOSIS TAKEN IN THEM — INCLUDING THE
LIMIT-CYCLE ONE BELOW.**

Probed inside a live `the_stage_kills` run, every tick for 2398 ticks:

```text
jostle_accel        = 0      declared 600.0 by smash
crouch_cancel_scale = 1.0    declared 0.85 by smash   ← the BASELINE
DeclaredCombatRules present = false
```

⭐ **the cause is one call site.** `smash_declared_combat_rules()` is invoked
only inside `start_the_battle_when_asked` — the system that starts a battle
**from the select screen**. All five of these tests reach the match by writing
`ShellCommand::GoTo(SMASH_GAMEPLAY_ROUTE)` directly, which is a real shell road
and skips that system entirely.

⇒ **so meteor lock, rage, staling, crouch cancel, grab depth, SDI, the parry
timing and jostle are ALL inert in this suite.** Every one of them was built and
gated on evidence taken somewhere else; none of them has ever been exercised
here. A fighter in these tests is playing the undeclared baseline ruleset.

⚠ **and the limit cycle is still real but its explanation is not settled.** The
measurement that produced it — two brains closing, passing through, overshooting
to opposite edges forever — was taken in this same ruleless match. Jostle may
well be the missing mechanic; it cannot be shown to be by a suite where jostle
is switched off.

⇒ **the next step is a FIXTURE repair, not a feature.** Either these tests enter
through select the way a player does, or they declare the ruleset explicitly and
say why. ⛔ do not "fix" them by relaxing an assertion: the standing rule is to
repair an unrealistic fixture to model production construction, and a match with
no declared rules is exactly that.

⚠ **the open question this raises for the ENGINE, not the test**: can production
reach `SMASH_GAMEPLAY_ROUTE` without passing through select — a resume, a debug
route, a deep link? If it can, this is a live defect and not a fixture problem,
and the declaration belongs on ROUTE ENTRY rather than on leaving the lobby.

**The question: what breaks the two CPUs' mirror, given that the fix the code
prescribes is a per-seat spawn ASYMMETRY and seat placement is deliberately
SYMMETRIC?**

`smash_it::the_stage_kills` has **five failing tests**, and nothing was running
that suite — the queue still records it as *"17 tests, all green"* (2026-08-17).
Bisected in the main tree:

```text
951806c9e  (before the legality filter)   2 failed
39b5a739a  "An attack the body cannot      5 failed   ⇐ this commit owns three
            BEGIN is not an option"
main today                                 5 failed
main with `legality_of` forced to `Now`    2 failed   ⇐ mechanically confirmed
```

⇒ **`39b5a739a` owns exactly three**: `two_cpus_wearing_one_character_stop_being_a_perfect_reflection`,
`every_live_fighter_stays_inside_the_frame`, `the_camera_closes_no_faster_than_it_opened`.
The other two (`a_match_whose_last_loser_is_removed_still_decides`,
`the_framing_centre_absorbs_an_elimination_instead_of_cutting`) predate it and
are **not yet attributed**.

⛔ **the filter is not "wrong", and this is not a request to revert it.** It is
the review's own first ask, and its measurement stands: wasted mid-smash grab
presses went 33 → 0. What it could not see is that its instrument counted GRABS,
so a second-order effect on ordinary attacks was invisible to it.

**What the failures actually say.** Not "the fighters stand still" — measured,
they are inside a running move 80% of body-frames and moves are short (max
0.70s). Every failing assertion reports **exactly 0.0**: the frame never widened
by 0.0, the cast's centre never jumped by 0.0, 0 body-frames outside the room.
And the mirror test reports the pair equal-and-opposite to **0.00012 px for 1077
ticks**. ⇒ they are fighting hard and *perfectly synchronised*, so nothing
separates them, nobody is launched differently, and no camera or elimination
follows.

⭐ **the leading mechanism, stated as a hypothesis because only the cause is
measured**: `brain_builders::fighter_cognition_seed` records that the RNG stream
has *"exactly ONE consumer in the fighter brain — press-timing jitter, only on a
decision that commits to an attack"*. A filter that drops candidates removes
committing decisions, so the two streams advance in lockstep and the seed never
separates anybody. The CAUSE is mechanical (forcing `Now` restores the mirror
test); the WHY above is not yet instrumented.

⛔⛔ **AND THE SAME FIVE GUARDS BROKE ONCE BEFORE, FROM A DIFFERENT CAUSE, AND
THAT FIX WAS REVERTED FOR IT.** The seed note: *"per-participant DECISION PHASE
… BROKE FIVE behavioural guards in `the_stage_kills`: a 0-4 tick offset changed
whether attacks connect at all — 'the brain travels but never commits'. Too high
a price."* So this suite is the tripwire for exactly this class, and it caught
this change too — it simply was not being run.

⭐⭐ **MEASURED 2026-08-20, AND IT CHANGES THE DIAGNOSIS: they are not fighting
hard and synchronised. They exchange ONCE and then never touch again.**

`a_match_whose_last_loser_is_removed_still_decides`, instrumented per body:

```text
tick  120   s0 pct=0  x=224      s1 pct=0  x=416
tick  240   s0 pct=0  x=387      s1 pct=0  x=253     closing
tick  360   s0 pct=19 x=416      s1 pct=19 x=224     ONE exchange, and they SWAPPED SIDES
tick  480   s0 pct=19 x=326      s1 pct=19 x=314     12px apart — passing THROUGH
tick 1800   s0 pct=19 x=236      s1 pct=19 x=404     still 19, still oscillating
```

⇒ **damage goes 0 → 19 in a single window near tick 300 and is FLAT for the
remaining 75 seconds.** Every post-hit timer is `0.00` the whole time —
`damage_invuln_timer`, `recoil_lock_timer`, `hitstun_timer` — so nothing is
latched and nobody is invulnerable. They simply stop connecting.

⛔ **the shape is a LIMIT CYCLE, and it is the thing to explain**: each brain
closes on the other, the two bodies PASS THROUGH each other, both overshoot to
opposite stage edges, both turn around, and it repeats forever, perfectly
anti-phase about the stage centre. The one exchange is the first meeting; after
that the crossing happens too fast for anything to land.

⭐⭐ **AND THIS IMPLICATES §25 (JOSTLE) DIRECTLY.** With body-vs-body contact,
two fighters closing on each other STALL where they meet — which is where
attacks land. Without it there is no such thing as being in front of somebody,
so a closing brain's reward is to sail past them. ⇒ the fairness question in §25
and this suite's five reds may be ONE question, and jostle may be the fix for
both.

⚠ **and one premise above is now doubtful.** These two seats wear DIFFERENT
characters (`smash_duelist_a` / `smash_duelist_b`) at the same level, so their
`fighter_cognition_seed` values genuinely differ — `preserves_mirror_symmetry`
only collapses the seat suffix, not two different character ids. Yet the motion
is still perfectly anti-phase. ⇒ seed divergence is not reaching MOTION at all,
only the press timing that never fires, so "the two streams advance in lockstep"
understates it: even two separated streams produce mirrored bodies.

⚠ **TWO BRAIN EXPLANATIONS ELIMINATED 2026-08-20, so nobody re-treads them:**

```text
"the hysteresis locks it in Approach through the band"   ⛔ NO — mode.rs:111
    already exempts Engage: `dwell < MIN && candidate != Engage`. Engage was
    never blocked. (The arithmetic that suggested it is still worth knowing:
    two fighters closing at a combined 540px/s cross the whole 57.6px engage
    band in ~0.1s, against a 0.18s dwell. It just does not apply to Engage.)

"the brain closes at full speed because Walk has no throttle"   ⛔ NO —
    smash/emit.rs:62 already emits `WALK_SPEED_PX_S / SPRINT_SPEED_PX_S` as a
    partial axis, so a walking brain asks for a fraction of its own top speed.
```

⇒ **the brain's vocabulary is not obviously the defect**, which strengthens the
reading that the missing thing is BODY CONTACT rather than a decision error. ⚠ a
related arithmetic worth keeping either way: a jab's 0.05s startup at a combined
540px/s closing speed is 27px of approach during startup alone, against a 36px
reach — so a swing begun in range lands where the target no longer is. Startup
frames assume a target cannot pass through you.

⇒ **why this needs you rather than a fix from me.** The note prescribes the
answer: *"what would actually move it is asymmetric CIRCUMSTANCES, not more
randomness — a per-seat spawn offset"*, and it bans a third randomness fix. But
`respawn_placement` is **deliberately symmetric** — *"seats alternate outward
from the centre … the arrangement is symmetric at any roster size and no seat is
privileged"* — so the prescribed asymmetry contradicts a fairness property
somebody chose on purpose. Inventing an asymmetry is a competitive-balance
decision, not a compile fix.

⚠ **RE-MEASURED 2026-08-20 on the `smash-parity` lane: still exactly 28 passed /
5 failed, same five.** The shield-as-a-resource, the taunt, the footstool and two
new anim reads all land on top of this suite and move the count by nothing. ⇒ the
five are a STABLE tripwire rather than a drifting one, and a lane adding combat
features is not what disturbs them — which is worth knowing before anybody reads
a change in the count as noise.

Options as I see them: (a) accept a small deliberate per-seat offset and record
why fairness tolerates it; (b) give the jitter a consumer that fires on every
decision rather than only on a committing one — explicitly banned by the note,
so only with your override; (c) let the two CPUs mirror and retune the three
guards to measure something a mirrored match can show; (d) revisit whether the
legality filter should admit an action the body could begin within N frames
(`BufferableSoon`, which `39b5a739a` names and defers to `BodyActionBuffer`).

⭐⭐ **MEASURED 2026-08-19, and it changes what the answer can be: they are NOT
one mind played twice.** The phrase in the failing assertion is wrong, and the
options above should be read with these numbers rather than with it. Probed on
the real demo app, two `smash_duelist_a` CPUs at rung 5:

```text
seat 0 noise  0x1fe5e72e2d1e8e0a   seat 1 noise  0x20e5e8c52d1e8e0a
                    ^ the streams DIFFER — and only in the high 32 bits, so the
                      low half printing identical is the seeding, not a bug
|sample| 0.511 / 0.104 · 0.491 / 0.464 · 0.703 / 0.933 · 0.150 / 0.458
                    ^ genuinely different draws, at the SAME tick count: the two
                      states stay a constant offset apart all match, so both
                      seats consume one sample per decision, together
```

⇒ **the seeding works and the noise is live. What is one frame wide is its
EFFECT.** `jitter = (|sample| · execution_noise · interval).round()`, clamped to
`interval - 1`. At rung 5 the engine formula gives `execution_noise = 0.275`
(not the ladder's 0.20 — the demo composes no ladder) and
`DEFAULT_DECISION_INTERVAL_TICKS = 5`, so the expression is
`round(|sample| · 1.375)` ∈ **{0, 1}**. The whole execution-noise budget of a
level-5 fighter is ONE FRAME of press delay, differing between the seats on
roughly half the draws.

⚠ **and the match really is a fight, which rules out the whiff explanations.**
Over ~580 ticks the two close to **1.32 px** apart (attack range is 48), live
hitboxes exist on **46** ticks with **2** at once, and `HitboxHits` records
**2 landed hits**. Teams are `seat 1` / `seat 2` — different, so `MatchTeam`
correctly says Foe — while both bodies are `ActorFaction::Player`, which is the
case `team_allows_damage` exists for and it is working. Nothing is being
refused: they approach, swing, connect, and take mirrored knockback, so the
reflection survives combat rather than never reaching it.

⇒ **so option (b) is worse than the note already says.** It is not merely banned;
the evidence says a third randomness fix would buy at most a frame against a
symmetry that survives two fighters hitting each other. The live question is
whether the asymmetry comes from CIRCUMSTANCES — option (a) — or whether the
guards should stop asking a symmetric stage for an asymmetric outcome —
option (c). ⛔ I did not touch `respawn_placement`: that is the fairness property
somebody chose, and choosing against it is yours.

⚠ **and one process finding regardless of the answer**: `smash_it` is not in the
per-turn gate, so a behavioural suite went five-red across at least two
regressions without anything saying so. `cargo test --workspace --lib` and
`-p ambition_app --test app_it` do not reach it.

### 25. ✔ ANSWERED AND BUILT 2026-08-20 — body contact belongs in the SWEEP, as an OPT-IN capability, not a global property of every body

⭐ **SHIPPED in `da884be08`** as `ambition_platformer2d_core::movement::body_contact`
— a constraint on the motion a body PROPOSED, applied before the world sweep and
writing no position, so nothing is teleported apart. `BodyContact { resistance }`
is presence-as-opt-in; the smash stage grants it to its cast and calls the result
jostle, and the engine does not know that word. See queue D172 for the three
rules it had to learn and for what it did to the smash suite. ▢ the resistance
NUMBER (0.85) is a feel choice nobody has measured, and airborne contact is
deliberately not in the first slice.

⭐⭐ **JON'S RULING, VERBATIM.** Keep these words; the paraphrase loses the
constraints.

> **AVOID PUSHOUT does not prohibit prospective body contact in the movement
> sweep.** A sweep that constrains proposed motion before integration is the
> correct mechanism. It must not perform after-the-fact positional separation.
>
> **Add explicit, typed participation in body-vs-body contact. Do not make all
> `Body`s globally solid.** Smash fighters opt into grounded lateral body
> contact; existing Ambition NPCs remain unchanged unless their composition
> explicitly opts them in later.
>
> The core mechanism should not be named or conditioned on Smash/jostle. The
> movement layer owns a generic body-contact capability/policy; Smash uses that
> capability to express jostle.

⛔⛔ **THREE IMPLEMENTATION CONSTRAINTS, in his words, "because otherwise a
superficially working solution could create worse problems".**

> **First, do not simply insert every moving body's current AABB into the
> existing static-wall sweep.** Two moving bodies require a deterministic
> pair/relative-motion calculation. If A sweeps against B's old position and then
> B sweeps against A's updated position, iteration order becomes physics. Resolve
> the pair from a common tick snapshot/proposed deltas, with stable ordering such
> as `SimId`, and constrain both bodies consistently.

> **Second, Smash jostle should initially be grounded lateral contact, not
> "fighters are full solid platforms."**
> ```text
> fighter → fighter while grounded   lateral crossing is constrained
> fighter above fighter              does not land on the other's head as geometry
> airborne fighters                  do not suddenly become wall/ceiling geometry
> ```
> So I would not blindly reuse the entire world-solid AABB semantics on both
> axes. The reusable primitive is something like an opted-in **lateral body
> blocker/contact participant**, not "all bodies become walls."

> **Third, preserve the AVOID PUSHOUT behavior for pre-existing overlap.** If two
> bodies somehow begin a tick overlapping because of spawn, transfer, teleport,
> etc., the contact solver should not teleport them apart. It should permit
> separating/non-deepening motion and prevent ordinary locomotion from driving
> them farther through each other.

⚠ **AND THE SCOPE OF THE FIRST SLICE IS BOUNDED, deliberately:**

> I would also **not require a complete rigid-body/jostle impulse solver in this
> slice**. The immediate semantic requirement is that two opted-in grounded
> fighters cannot simply exchange sides by running through one another. … More
> sophisticated weight-dependent pushing or contact momentum can be added as a
> separately authored response if gameplay needs it.

⇒ **the instruction, verbatim:**

> **Proceed with option (c), but make body contact an explicit opt-in movement
> capability/policy rather than a global property of `Body`. Smash fighters opt
> into grounded lateral body blocking; ordinary Ambition NPCs keep their existing
> pass-through behavior. Resolve moving-body pairs deterministically from common
> pre-integration motion, do not treat one moving body as static based on ECS
> iteration order, and never use post-overlap positional pushout. Do not make
> fighters vertically solid to one another.**

⭐ **and he expects this to close §23 as well**: *"the limit cycle is very
plausibly exposing a missing physical spacing primitive rather than a brain
defect. The agent was right to stop rather than compensating for missing contact
by making the AI stranger."*

---

**The original question, kept for the reasoning that led here.**


⛔⛔ **BUILT IT, MEASURED IT, REVERTED IT — the ACCELERATION form CANNOT WORK,
and that changes the question (2026-08-20).**

I built jostle exactly as this row proposes — a `DeclaredCombatRules::jostle_accel`
applied as an opposing force to overlapping grounded bodies, never writing a
position, so AVOID PUSHOUT is untouched. Five unit tests, all green. On the real
stage it does **nothing**, and the probe says why in one column:

```text
tick  240   s0 x=387 vx=-270      s1 x=253 vx=270
tick  480   s0 x=326 vx=-270      s1 x=314 vx=270    overlapping, still ±270
tick 1200   s0 x=236 vx=-270      s1 x=404 vx=270
```

⇒ **`vx` is EXACTLY ±`max_run_speed` on every sample.** The horizontal law is
`approach(along, run * max_run_speed, accel * dt)` — a velocity **TARGET**, not
an accumulation — so any delta added before the kernel is overwritten by the
kernel on the same tick, for as long as the brain holds a direction. A force
cannot survive a law that re-derives velocity from input every frame.

⭐⭐ **the unit tests passed because they had no movement kernel in them.** They
spawned two bodies, ran the one system, and read the velocity it wrote — the
exact "a test that SUPPLIES the precondition cannot prove the mechanism reaches
production" shape. Only the end-to-end run found it.

⇒ **so the real question is not "force or displacement", it is WHERE.** Three
candidates, and the third looks right:

```text
(a) write the position          ⛔ forbidden by AVOID PUSHOUT, and correctly so
(b) reduce the RUN TARGET when blocked   a movement-kernel change: the law would
                                         have to know another body is there
(c) put bodies in the COLLISION SWEEP    what the sweep already does for walls —
                                         it CLAMPS motion rather than writing a
                                         position, so it is pushout-free by
                                         construction, and it is how the genre
                                         actually works
```

⚠ **(c) has real blast radius and is why this is still your call**: making bodies
solid to each other is a movement-kernel change that reaches every NPC in
Ambition, not only fighters, so it needs a gate and a decision about which
populations collide. That is a bigger question than "may fighters jostle", and it
is the one worth answering.



**The mechanic.** Every platform fighter pushes two grounded bodies apart when
they occupy the same space — Ultimate calls it jostle, and without it two
fighters stand inside each other and the stage's spacing game stops existing.
It is the last unbuilt row in the smash inventory's Movement section
(`docs/planning/demos/smash-parity-inventory.md`).

**Why it is a decision and not research.** The genre's answer is not in doubt:
bodies push each other apart, symmetrically, proportional to overlap. What is in
doubt is whether Jon's own standing rule forbids it. The rule, as recorded:

> **AVOID PUSHOUT.** Almost never artificially push a body out of geometry …
> pushout corrupts position/reversibility and papers over the real bug. Emerging
> at the face + carrying momentum is the intended physical behavior.

⭐ **the reading that says jostle is FINE**: the rule names *geometry*, and every
case behind it was a CORRECTION — an NPC embedded in a wall, a body straddling a
closing portal. Jostle is neither. It is a designed, symmetric, momentum-carrying
force between two LIVE bodies, which is much closer to the "intended physical
behavior" the rule is protecting than to the correction it forbids.

⚠ **the reading that says it is not**: it is still a position written by
something other than the body's own velocity, and the rule's stated cost —
*"corrupts position/reversibility"* — applies to any such write. Under rollback
that cost is not rhetorical.

**A third option, and probably the honest one:** jostle as an ACCELERATION rather
than a displacement — two overlapping bodies each take a small push-apart
velocity, and the kernel integrates it like any other force. Position is never
written, reversibility is untouched, and the visible behaviour is the genre's.
It is slower to separate than a displacement, which in this genre reads as weight
rather than as a bug.

⭐⭐ **JON, 2026-08-20, verbatim:**

> The no pushout rule I think is for portals, because I wanted them to be
> elegant. For bodies I think it might be ok. This isn't a hack, it is a game
> feel feature. If ultimate does it they must have rollback code for it. This is
> something that games will want, so we should be able to express it. It should
> never be a mandatory part of the movement kernel though. It should be
> composable and not add to tech dept.

⇒ **the rule's SCOPE was the thing in question and it is narrower than both
readings above assumed.** AVOID PUSHOUT is about PORTALS and the elegance of
emerging at a face carrying momentum. It was never a statement about two live
bodies, so the whole "does it extend" framing was the wrong question — and both
readings recorded above spent their effort on it.

⛔⛔ **THE BINDING CONSTRAINT IS NOT WHETHER, IT IS WHERE.** *"It should never be
a mandatory part of the movement kernel. It should be composable and not add to
tech dept."* So jostle is a body-vs-body PASS that a game opts into — the fourth
beside capture, footstool and the ledge trump — and NOT a term in `step_body`.
A kernel that jostles unconditionally would make every body in every composition
pay for a platform-fighter rule, which is the shape this repo has removed twice
already (the stale-move ring rode the generic movement bundle; the capture
timeout lived on a shared constant).

⚠ **and "games will want this" is a statement about the ENGINE, not about
smash.** The knob belongs where a game declares its rules, so a second game can
turn it on without touching the fighter demo.

### 27. ✔ ANSWERED 2026-08-20 — the parry timing is a KNOB, because the target is smash-LIKE and not Ultimate

⭐⭐ **JON, 2026-08-20, verbatim:**

> Our point is to build a smash-like game, not exactly ultimate. It would be nice
> if there was a set of knobs we could tune to reproduce ultimate, but it doesn't
> have to be ultimate. Reproducing smash 4 or brawl, or melee (bugs are not
> reuqired parity) would be nice too

⇒ **the question was the wrong SHAPE and the answer is neither option.** Ours is
press-timed (Smash 4's), Ultimate's is release-timed, and the ruling is that both
are settings of ONE declared knob — a stage picks which game it reproduces.

⛔ **and this generalises past the parry.** *"Which does the genre do"* is the
wrong question wherever the games differ FROM EACH OTHER; the right one is *"what
is the knob, and what does each setting reproduce"*. Everything filed under that
heading — and the whole `smash-parity-inventory` frontier list — should be read
that way from here.

⚠ **bugs are not required parity.** Melee's wavedash and L-cancel are artefacts
of its physics rather than authored rules; reproducing Melee does not mean
reproducing them.

⭐ **and the follow-up, same day, verbatim:**

> Note, if ultimate does it I do want a setting for get ultimate, so release
> style shielding is in scope as an option.

⇒ **so the release-timed parry is not merely permitted, it is WANTED** — the knob
ships with both settings rather than shipping with one and a note about the
other. ⚠ the general rule that follows: *"Ultimate does it"* is sufficient reason
for a SETTING to exist, whatever the default ends up being.

⚠ **implementation note for whoever builds the second setting**: `parrying()`
currently requires `active`, and a release-timed window is live on frames when
the shield is DOWN — so the term that separates a parry from a held shield would
have to move, and `vulnerability_gate_tests` asserts that term by name.

## ✔ CLOSED 2026-08-15 — every submodule remote is reachable and current

**Was:** `git push` in `tools/ambition_sfx_renderer` failed with *"correct access
rights"*, and `main` already recorded a commit from it, so a fresh clone could
not resolve the pointer. Three more submodules were on the same footing, each
behind its own credential alias.

✔ **Jon provisioned all four.** Verified from inside the VM: five of five
submodules answer `git ls-remote`, none is ahead of its `origin/main`, and every
pointer `main` records exists on its remote.

⭐ **the check worth keeping**, because "ahead: 0" alone is not evidence — a
submodule pushed only from the host reads as current from in here while being
unpublishable:

```sh
git submodule foreach 'git ls-remote --exit-code origin >/dev/null 2>&1 \
    && echo OK || echo NO-ACCESS'
```

⛔ **and the rule that outlives this:** never resolve a rejected submodule push
by rolling the superproject pointer back. The superproject commits depend on the
submodule content; the pointer is the symptom, the credential is the cause.


### 30. ✔ ANSWERED 2026-08-22 — height owns world size; art DENSITY is a separate, declared contract

⭐⭐ **Jon, verbatim:** *"Neither the cast median nor 1.0 is the right authority.
Split this from the height contract. Height owns world size; the measured scale is
just the conversion from source pixels to world size. If consistent source-art
density is desired, give the art/render pipeline an explicit authoring-density
profile and warn when a sheet deviates from that declared profile. Do not infer
the standard from the current cast. Until we have deliberately chosen that
profile, remove the false 1.0 warning rather than replacing it with a
median-based one."*

⇒ ⭐⭐ **this AMENDS his own 2026-08-17 ruling.** *"Warns when the scale drifts far
from 1.0"* rested on a false premise: measured across 95 catalog characters the
scale runs 0.188–0.571, median 0.320, so a band around 1.0 warns on 100% of the
cast — the mirror of a check that cannot fail.

⛔ **DELETE the warn. Do not substitute a median-based one** — a cast-derived
standard drifts as the cast changes, so enough dense-art characters silently move
what counts as normal.

⭐ the density question is real and stays open as a FUTURE contract: an explicit
authoring-density profile, DECLARED once by the art/render pipeline, with sheets
warned against that.

⚠ nothing else in the height contract moves — the authored number still wins and
every character still renders at its declared height.

### 31. ✔ ANSWERED 2026-08-22 — `SeatRawFrames` stays RAW; the split is source-local vs world-dependent

⭐⭐ **Jon, verbatim:** *"Choose 1, but don't encode “portal warp is
proposal-side” into the design. Keep SeatRawFrames genuinely raw/proposed and
stop shape_seat_frame from dual-writing raw + published state. Establish one
canonical per-tick input after the fixed-step/GGRS boundary, then derive
effective controls deterministically from that. Source-local normalization
happens before the boundary; world-dependent semantics such as
portal/reference-frame transforms and fast-fall happen after it. The
post-boundary value does not need to mean “confirmed” in the networking
sense—GGRS may be using predicted remote input—so name it something like
CanonicalSeatInput or TickSeatInput, not necessarily ConfirmedSeatInput."*

⇒ the stage model is taken, with TWO corrections to the reviewer's version:

```text
device sample -> raw seat proposal        source-local normalization ONLY
              -> fixed-step / GGRS boundary
              -> CanonicalSeatInput       (or TickSeatInput)
              -> deterministic derivation portal / reference-frame transforms,
                                          fast-fall, other world-dependent semantics
              -> effective slot controls
```

⛔ **the axis is SOURCE-LOCAL vs WORLD-DEPENDENT, not proposal vs confirmed.**
Portal / reference-frame transforms are world-dependent and belong AFTER the
boundary beside fast-fall — do not place portal warping on the proposal side,
which is what the review proposed.

⛔ **the post-boundary table must not be called "confirmed"**: GGRS may be
feeding PREDICTED remote input, so the name may not claim agreement.

⇒ `shape_seat_frame`'s dual write ends and the three-host (fixed-tick latch /
GGRS / frame-step) knowledge leaves the shaper; `SeatRawFrames` keeps its stated
type contract instead of having it rewritten.

⛔ the reviewer's guard against new `shape_seat_frame` callers stays DECLINED —
a check that counts call sites is source-text meta-test machinery AGENTS.md
forbids, and the standing note predicts an LLM review asking for exactly it.

### 26. ✔ ANSWERED 2026-08-22 — rename the blast zone out of every world (D169)

⭐⭐ **RULED: the full rename, Rust and LDtk in ONE change.**

```text
World { blast_margin, side_blast_margin, ceiling_blast_margin }
  -> World { edges: WorldEdgeMargins { fall: f32, side: Option<f32>, rise: Option<f32> } }
LDtk keys -> fall_out_margin / side_out_margin / rise_out_margin
```

⇒ one field instead of three, named for the AXIS ROLE rather than the genre, and
the kernel destructures it EXHAUSTIVELY so a fourth axis is a compile error rather
than a forgotten comparison — the same shape as `CapabilityLanes`.

⭐ **the MECHANISM was already generic and does not change**:
`apply_world_hazard_gate` computes a per-axis distance past the world AABB and
emits `ResetCause::LeftTheWorld` — Smash loses a stock, Mary-O respawns, Ambition
calls it out of bounds. What leaks is the WORD, and it leaks in the authoring
schema every author meets.

⭐⭐ **it costs NO content migration.** All six shipped worlds
(`sanic_speedway`, `intro`, `sandbox`, `you_have_to_cut_the_rope`,
`hall_of_characters`, `mary_o`) carry all three fields in `defs.levelFields`, and
ZERO levels author a value — 18 schema entries with no data behind any of them.

⛔ **it is one change or it is not worth 206 sites.** The converter reads the
authored key by name, so the struct field and the authored key are ONE name;
renaming the Rust half alone needs a mapping, and a mapping is the shim this
project refuses. Guarded by `a_level_authors_its_own_blast_margin` plus the LDtk
contract prover.

⛔ **`BlockKind` is the plan's other half and is NOT in scope.** Its diagnosis
— one enum mixing contact law, traversal permission, world consequence and contact
affordance — was re-measured as correct, and its trigger has not fired.

## WHICH MOVE GUSTS? — the windbox has a mechanism and no customer (2026-08-25)

The windbox primitive landed (`e06333002`, D215): a volume may now push without
hurting — no hitstun, and optionally repeating so it pushes for as long as you
stand in it. It is guarded by three poisoned tests.

⛔ **NO MOVE USES IT, and that is deliberate rather than unfinished.** Which
fighter gets a gust, and on which move, is a CHARACTER-DESIGN decision — the
kind the W8 list asked not to be invented without direction. The engine question
is answered; this one is yours.

⇒ **what would settle it:** name one move. The genre's own examples are a
lingering wind that shoves an edge-guarding opponent off their spacing, or a
suction that drags one in — the second needs no new mechanic, only a launch
aimed back toward the owner.

⚠ **and until one exists the mechanic is UNADOPTED.** This demo has shipped
three mechanics green and inert (the smash charge, DI, the tech), each caught by
counting in a real match rather than by a unit test — so `match_report` should
show a windbox connecting before this row is called done.

⭐⭐ **THE WINDBOX IS NOT ALONE — an ADOPTION CENSUS of the authored move
vocabulary, taken 2026-08-28 across `ambition_content`, `ambition_demo_smash` and
the shared authoring helpers (comments and tests excluded):**

```text
motion_scale            40   the committed-strike damp; thoroughly adopted
boomerang_return_s      10   the ponytail
WindowTag::Cancelable    5   jab strings and chains
WindowTag::Invuln        5   the Actress's trapdoor + THREE teleports (added
                             2026-08-28; it was ONE until that day)
on_hit: Some(..)         4   still only `technique::POGO_BOUNCE_KEY` behind them
fixed_knockback          3
equips: Some(..)         2   the Admiral's gun-sword, the Polygon's ponytail
stores: true             1   the power ball
with_aim_assist          1   the Officer's shot
WindowTag::Armor         0   ⛔ DORMANT
windbox                  0   ⛔ DORMANT — this row
```

⇒ **two dormant, not one**, and they are the same question with two names: which
fighter takes a hit and keeps swinging, and which one gusts. ⚠ a mechanic at ONE
adopter is barely better — `stores`, `with_aim_assist` and (until this week)
`Invuln` are each one authoring mistake away from looking unused, and an unused
mechanic is one nobody notices breaking.
⭐ **the census is a grep, not a tool**: count the authoring hook in the content
crates. It is worth re-taking whenever a mechanic ships, because *shipping* and
*being used* are the two facts this demo keeps proving are different.

⭐ the sibling parity row *"Vacuum / suction hitboxes"* needs NO further work:
it is the same primitive with the launch aimed inward, so authoring one move
closes both rows.

## ✔ ANSWERED 2026-08-25 — Charge Shot's cadence: three variables, not two

⭐⭐ **JON'S RULING (via GPT 5.6, 2026-08-25), and it rejected the question's
shape.** The two options offered were bundling **three independent variables**:

```text
firing cadence          how often a shot may come out
animation commitment    how long the move owns the body
locomotion freedom      whether the fighter may move/melee between shots
```

> *"Preserve the historically observed ranged cadence and mobility, but make it
> explicit. An accepted authored ranged move guarantees its authored shot. Move
> the old refire floor out of the projectile consumer into explicit
> weapon/action readiness checked before move acceptance. Do not lengthen move
> recovery merely to encode weapon recharge; the fighter should retain the same
> ability to move/melee between shots that existed under the old veto. Give
> recharge enough presentation that an unavailable shot is legible, and use
> normal short input buffering rather than accept-then-veto."*

⛔⛔ **AND A CORRECTION TO THIS FILE'S OWN EARLIER WORDING.** It said *"the 1.1s
constant is a generic legacy ATTEMPT floor, not character balance"*. Only the
first half is true:

```text
architectural origin        a generic legacy spam guard, one layer below authoring
observed consequence        the de facto ~1.1s cadence of every ranged fighter
```

⇒ the architecture goes; **the number stays**, deliberately, as the baseline
per-character tuning starts from. Shipped as
`RangedActionSpec::refire_s` (default `DEFAULT_RANGED_REFIRE_S = 1.1`), checked
in `moveset::weapon_ready` where the move is ACCEPTED and spent in `start_move`;
`ActionRequest::Ranged` now carries a `RangedCommitment` so the projectile
consumer keeps the floor for a controller ATTEMPT and honours a `CommittedMove`.

⭐ **MEASURED AFTER LANDING, and it pays neither of the costs the two rejected
options did.** Same instrument (`duel_arena`), same fighters:

```text
option (a) short move, no floor    melee 0        fighters stopped meleeing
option (b) stretch to 1.10s        locomotion.x = 0   recovery damped steering
SHIPPED                            PCA melee 36, robot melee 23, hp 60→34 / 60→51
```

⇒ MORE melee than either, because a refused move no longer costs the fighter a
windup: the press is refused before `proposer.spend`, so the ordinary combat
buffer keeps re-proposing and starts the move the moment the weapon returns.

▢ **STILL OWED — the presentation half of the ruling.** *"Give recharge enough
presentation that an unavailable shot is legible."* `BodyMelee.ranged_cooldown`
is now the authored, per-weapon truth and nothing draws it; a muzzle/charge
indicator is a separate lane and is not in this change.

## ✔ ANSWERED 2026-08-25 — Charge Shot plays its release and fires nothing

⭐ JON: *"An accepted authored move should guarantee its authored fire event. If
0.58s is too fast, encode the desired cadence at move acceptance/authoring rather
than vetoing the projectile downstream."* ⇒ (b), plus the observation that the
1.1s constant ORIGINATED as a generic legacy ATTEMPT floor — ⛔ which is not the
same as saying it carried no balance, and the section above corrects that. See
D239 item 31 for what measuring it turned up: 22 of 28 authored ranged events
were being refused game-wide, so the floor had become the cadence.

## 2026-08-25 — Charge Shot plays its release and fires nothing: which fix?

A second Charge Shot thrown at the earliest legal moment produces no projectile.
The move is `0.58s` long and fires at `0.26s`; the projectile consumer imposes a
hidden `1.1s` refire. So the second shot's authored fire frame lands `0.58s`
after the first and is silently vetoed — the move starts, animates, plays its
release presentation, and nothing comes out.

⛔ NOT A TUNING QUESTION, and that is why it is here: both fixes are correct and
they give different games.

```text
(a) refuse to START Charge Shot until the weapon can fire
    → the 1.1s weapon cooldown stays authoritative
    → nothing about current feel changes
    → the button sometimes does nothing, which reads as unresponsive

(b) an accepted move's authored shot is GUARANTEED
    → the timeline becomes truthful: what plays, fires
    → Charge Shot can fire every 0.58s instead of every 1.1s
    → a real balance change to Projectile Polygon
```

⭐ I lean (b) — a move that visibly commits should not be vetoed halfway — but it
speeds the weapon up by roughly half, and that is your call rather than mine.

⚠ Either way the current behaviour is wrong: accept-then-veto is the bad middle.

## 2026-08-27 — Should the moveset inspector need a GPU? — ✅ DECIDED

⭐ JON DECIDED, same day, and chose a shape none of the three options below had:
*"having some binary that can be called to produce the moveset animation on a
machine that requires a gpu to do it, and then having the animation be generated
on demand, and using a fallback visualization if it wasn't available or we didn't
have the gpu."*

Better than (a), (b) or (c): the CPU tool stays fast and portable, and the real
engine picture becomes an on-demand EXTRA rather than a requirement. Shipped as
`capture_scene --frames N [--stride K]` (the binary that already had the GPU
boot), an `/api/render` route that shells out and caches, and a viewer that falls
back to the derived sprites and SAYS which one is on screen.

⚠ THE DERIVED CURSOR THEREFORE STAYS, and so does its drift risk — it is now the
fallback rather than the only answer. The pose-picker gap named below is closed
only for fighters that have actually been rendered.

The analysis is kept below, because the measurement is why the decision was
possible at all.

## (original question, 2026-08-27)

The Engine Takes view plays a fighter's real sprites, but the FRAME within each
animation is derived by the viewer rather than read from the engine. Jon:
*"I do not like the idea of a reimplementation or duplicate implementation that
can drift."* Agreed — this records what standing on the real implementation
would cost, because the answer changes what the tool IS.

MEASURED, not inferred (`moveset_takes` prints a `[presentation]` census every
run; today it reads `PlayerVisual=0 CharacterAnimator=0 BodyPoseView=0`):

```text
1. `BodyPoseView` is unavailable to a smash fighter in ANY mode.
   `rebuild_body_pose_views` filters `With<PlayerVisual>`, and `PlayerVisual` is
   granted in ONE production place — `session/setup.rs`, to the exploration
   player's avatar. A seated `MatchSeat` fighter never carries it, windowed or
   not. This one is not about headless at all.

2. `CharacterAnimator` needs a render app. It is built by the render layer from
   a loaded `CharacterSpriteAsset`, and `NoWindow` sets `backends: None`, which
   omits the render app BY DESIGN.
```

⛔ THE CHOICE, and it is a question about the tool's AUDIENCE rather than its
code:

```text
(a) the inspector requires a GPU
    → boot `moveset_takes` the way `capture_scene` does (`OffscreenGpu` +
      `build_visible_app_with` + its camera/render-target setup)
    → read `CharacterAnimator::frame` directly; DELETE the viewer's derivation
    → the 56-state pose picker comes free, so a walking or hitstunned fighter
      stops showing IDLE — the largest fidelity gap in the view today
    → takes RENDER every tick instead of only simulating: slower, and it can no
      longer run anywhere the rest of the headless suite runs

(b) the inspector stays CPU-only
    → the derivation stays, and stays a duplicate that can drift
    → its rules are pinned to the animator's today (`duration_secs` is PER
      FRAME; a clip holds its last frame, a pose loops) and nothing enforces
      that they stay pinned
    → non-move poses stay a two-way `jump`/`idle` guess

(c) CPU-only, but the duplication is GUARDED
    → keep the derivation, add a test that runs both cursors over one recorded
      take and asserts they agree frame for frame
    → needs (a) to exist anyway in order to have something to compare against
```

⭐ Worth saying plainly: the drift risk is real but SMALL and slow-moving — the
animator's advance rule is ten lines and has not changed in this campaign. The
fidelity gap that actually misleads today is the POSE PICKER, not the frame
cursor, and only (a) fixes that.

⚠ A GPU requirement is a real cost to name: this tool currently runs in the same
places the headless suite does. Jon: *"It might be the case that we build a tool
so it works on a machine with a gpu. Not sure yet though."* Held here until that
is decided.

## 2026-08-28 — Does an enemy you left in a foreign room STAY there?

**The question, and it is a product call rather than a defect** (D125 says so in
its own words): an actor RELEASED in a room that is not its authored home writes
no `Placed` row today, so it is retired when that room is left and re-authored at
home on re-entry. *"The enemy goes home"* is a defensible rule. *"The enemy stays
where you dragged it"* is the other one.

**What it costs, measured — and the two halves cannot land apart:**

```text
PRODUCER   the placement recorder is items-only; a released body needs the same
           `republish_placements` call
CONSUMER   `construction::relocate_request` returns FALSE for anything but a
           ground item, so an actor request is refused and the body is rebuilt at
           its authored spot with a warn
```

⛔ **adding the producer alone makes every re-entry log a warn and teleport the
actor home anyway.** They land together or not at all.

⭐ **the current refusal is honest, not broken**: the room build already declines
to pretend an unmovable family moved, and says so.

⚠ **why this is here rather than in the ledger**: D125 has carried it as *"still
open — two named pieces"* since 2026-08-19 with the note *"whether an abandoned
enemy should stay put is a product call"*, and a product call sitting in the
execution ledger reads as work nobody has got to. Promoted 2026-08-28.
