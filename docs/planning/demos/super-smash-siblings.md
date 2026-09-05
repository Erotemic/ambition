# Super Smash Siblings — platform-fighter product charter

**State:** ACTIVE product push; serious engine customer and possible future
first-class game.
**Project order:** Ambition remains the flagship and primary product driver.

Super Smash Siblings is a Smash-like platform fighter built from ordinary
Ambition bodies, controls, combat, world geometry, presentation, and content.
It is a product in its own right and a pressure test for reusable engine
capabilities.

## Start here

- **Choose or inspect a feature, and start here:**
  [`smash-parity-inventory.md`](smash-parity-inventory.md) is the canonical
  shipped/partial/absent inventory and records the implementation seam for each
  gap.
- ⛔ **The "current push" was `campaigns/smash-fun-push-2026-08-22.md` and it is
  CLOSED** — corrected 2026-09-03, when this was still the first thing a new
  session was told to implement. That file's own header now reads *"execution
  campaign closed; do not use this file as Smash feature status"* and
  *"replaying that chronology is actively harmful because the parity inventory
  has since been reconciled against HEAD"*. ⇒ Read it only for the standing
  lessons it deliberately retains
  ([`campaigns/smash-fun-push-2026-08-22.md`](campaigns/smash-fun-push-2026-08-22.md)),
  never as a task list. ⚠ The campaign closed itself correctly and pointed here;
  this page is what never learned, which is the direction that rot usually
  travels — the closing document knows, the linking one does not.
- **Change reusable combat semantics:**
  [`../engine/combat-model.md`](../engine/combat-model.md) owns the body-generic
  combat contract. The inventory owns feature priority and gap status.
- **Change fighter decision policy:**
  [`../engine/fighter-brain.md`](../engine/fighter-brain.md) owns broad fighter-AI
  evaluation and calibration. A Smash feature may add the smallest semantic
  observation/option support it needs without starting a second brain stack.
- **Change local multiplayer/view behavior:**
  [`../engine/multiplayer-and-multiview.md`](../engine/multiplayer-and-multiview.md)
  owns participant/view architecture.

The superseded body-generic successor plan is archived. No open Smash work is
owned by that document.

## Product target

The target is **Smash-like**, not byte-for-byte Ultimate. The engine should be
able to express useful rule differences among platform fighters through normal
content or rules knobs when those differences are worth supporting. Physics
bugs do not need parity.

A strong demo should have:

- responsive neutral built from walk/run, attacks, shield, grab, dodge, jump,
  recovery, ledges, tech, and launch;
- several fighters whose movement and move kits feel materially different;
- readable hit, launch, defense, invulnerability, KO, respawn, and victory
  presentation;
- multiple local human participants plus CPU participants through the same body
  control model;
- several stages with meaningful platform/geometry differences;
- enough rules, character-select, stage-select, results, training, and rematch
  UX that the demo feels like a game rather than a mechanics harness.

The exhaustive feature surface lives in the parity inventory. Do not maintain a
second backlog here.

## Architecture contract

1. **Fighters are ordinary bodies.** Do not add a fighter-only body ontology or
   a second movement/combat implementation.
2. **Human and CPU fighters obey the same simulation rules.** Controllers
   provide intent; they do not define different physics or combat semantics.
3. **Feature-driven engine work is allowed now.** A missing mechanic may add a
   small reusable semantic in the domain that already owns it. Do not wait for
   the actor-monolith carve, simulation-phase migration, capability/runtime
   composition cleanup, or public-facade cleanup merely to ship a cleanly owned
   Smash feature.
4. **Do not hide real engine work in the demo.** The inventory marks small
   reusable additions as `E1`, coordinated engine campaigns as `E2`, and work
   that should wait as `WAIT`.
5. **Simulation owns gameplay truth.** Rendering, shaders, particles, audio,
   cameras, and HUD consume resolved facts/events rather than reconstructing
   charge, vulnerability, shield, hit, or launch rules.
6. **Prefer one reusable semantic over one fighter exception.** Add autolink,
   hitbox arbitration, capture acquisition policy, or a locomotion phase when a
   real move needs it; do not branch on character identity.
7. **Do not pre-generalize.** Stances, status frameworks, cinematic supers, and
   other broad systems wait until a concrete fighter or ruleset defines the
   actual requirement.

## Engine capabilities consumed

Smash already consumes ordinary engine support for:

- prepared character definitions and ordinary actor construction;
- shared movement, collision, body contact, jump, dodge, knockdown, tech, ledge,
  and recovery mechanics;
- move timelines, hurt geometry, hit volumes, damage, knockback, DI, hitlag,
  hitstun, shields/parry, capture, pummel, and throws;
- participant/action routing for human and AI-controlled bodies;
- fighter-brain profiles through ordinary control intent;
- item identity/custody, pickup, held use, throw, and world-item physics;
- world/stage geometry, blast zones, and kinematic stage objects;
- deterministic/headless simulation and rollback state;
- shared VFX, audio, camera, HUD, and multi-view presentation infrastructure.

The inventory records which additional semantics are genuinely missing.

## What Smash owns

Smash owns product policy and content:

- stocks, timer, sudden death, stamina/time variants, teams, item rules, and
  other match rules;
- roster declaration, CPU-fill/difficulty policy, character-select and
  stage-select UX;
- stage layouts and platform-fighter-specific stage policy;
- percent presentation, stocks HUD, respawn-platform behavior, results, rematch,
  victory presentation, announcer/fanfare, and other ceremony;
- fighter move content, frame data, balance, pose selection, character audio,
  and game feel;
- which reusable mechanics earn engine implementation and how the demo tunes
  them.

A product-owned rule may consume reusable engine primitives without moving the
named Smash policy into core.

## Character and stage composition

A match selects authored character identities that preparation resolves to the
complete body/kit consumed by ordinary construction. Hosted Smash may use
characters installed by Ambition; a standalone build installs the content it
wants through supported provider/SDK seams.

Stages should use supported world tooling. Moving platforms, hazards, one-way
platforms, collision geometry, blast zones, and camera bounds should remain
ordinary world/stage concepts rather than a parallel Smash scene format.

## Multiplayer

Local and future network participants feed the same participant/control model.
Arena matches normally choose one shared framing policy, but that is a
presentation choice rather than an engine-wide single-camera rule.

## Product checkpoints

⚠ **MEASURED 2026-09-04 against the workspace. These six were the charter's
falsifiable core and NONE of them carried a status**, so the same rule the
parity inventory's primitive table now carries applies here: an unmarked
checkpoint meant nobody had checked, not that it was open. Method for each is
named inline; a number here travels with the search that produced it.

| # | checkpoint | verdict |
|---|---|---|
| 1 | Core fight | ✔ **met** — the mechanics it lists are the parity inventory's P01–P14, now 14 of 14 measured: ten shipped, three partial, **zero absent** |
| 2 | Roster depth | ✔ **met, and it is the strongest of the six** — **21** authored movesets (19 in `ambition_content`, 2 smash-local) and **zero** character-ID gameplay branches in any engine crate |
| 3 | Local play | ✔ **met** — driven end-to-end through the real screen, not the model. ⭐ **The evidence has a name and it was not written down**: `smash_tool select-walkthrough` drives the actual screen headlessly — a real cursor over `select_screen::layout`'s own rectangles, real presses, and the text read back through the SAME functions the cards render (`role_button_text`, `card_name_text`, `SmashSelect::blocker`), so it cannot show a screen a player would not see. ⚠ It is the ONE instrument for this surface and it appears in no planning page except the CLI-collapse campaign, so select-screen work does not find it. ⓘ Complements `capture_scene --route smash_select`, which photographs the screen; this prints what the screen BELIEVES. |
| 4 | Stage breadth | ✔ **MET 2026-09-04 — THREE stages, and they differ on two independent axes.** `smash_stage()` flat; `smash_platform_stage()` the same floor with drop-through tiers; `smash_narrow_stage()` two thirds the ground with the blast envelope **unchanged**. ⭐ The third is the one that makes "several" mean something: holding the margins fixed while shrinking the platform moves every blast line from 1.000 / 1.125 / 0.875 platform-widths to **1.750 / 1.688 / 1.313**, so it is an edgeguard-and-recovery stage rather than the same stage smaller. ⛔ Not tuned and not balanced — authored to CHANGE the decisions the rig measures, which is what this row asks for |
| 5 | Match completeness | ✔ **MET 2026-09-04, 4 of 4** — stage select landed, and rule selection landed with it: a `Stocks:` cycle beside the stage cycle, any seat, 1 / 3 / 5. ⭐ The rule itself was already shipped (`MatchRules::stocks`); what was missing was a customer, which is this page's own standing lesson about the primitive table |
| 6 | CPU adoption | ◐ **partial, and the USE half is now measured — it is the weaker half.** The charter asks that the brain *"can use and answer"* the roster's mechanics. ⛔ **It does not use them**: a 120s census of George on the shipped ladder records **16 of his 28 authored moves never starting once** — all three smashes, all three tilts, three of five aerials — with the dash attack at **81%** of starts. ⭐ Root cause traced to source and it is not a Smash-only AI problem, which is the charter's actual concern: movement and attack are scored INDEPENDENTLY and **neither axis declines on score**, so the fighter moves whenever it can and attacks whenever it can, and 73 of 81 attack decisions land on a tick it also chose `Approach` — which drives a run, where a neutral press converts to the dash attack. ⇒ Indexed as decision 5 in [`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md). ✔✔ **THE *ANSWER* HALF IS NOW MEASURED TOO — 2026-09-04, from SOURCE, and it is the STRONGER half.** Method, because the number travels: diff `SelfView`'s fields against `PerceivedActor`'s in `ambition_characters/src/perception.rs`, then count brain readers of the fields that exist ONLY on the opponent view — those two counts are unambiguous, where a field on both views cannot be attributed to self or foe by name alone. **`SelfView` 24 fields, `PerceivedActor` 17.** ⭐ **The brain DOES answer the roster's mechanics**: `shield_raised` and `ledge_hanging` are opponent-only and both are read in production — `foe.shield_raised && foe.on_ground` gates the guard feature (`brain/fighter/options.rs:525`) and `foe.ledge_hanging` is half of the `EdgeGuard` classification (`brain/fighter/situation.rs:154`), with 10 and 3 references respectively. ⛔ **The one genuine gap is CAPTURE, and it is precise: a fighter cannot perceive a capture it is not part of.** `captured`, `captured_for`, `holding_captive` and `pummels_landed` are published on `SelfView` **only**, and `BodyPhase`'s six variants (`Neutral`, `Hitstun`, `AttackStartup`, `AttackActive`, `AttackRecovery`, `Shielding`) include none for it — so the state is absent as a field AND as a phase. ⚠ **In 1v1 that is not a gap**: *"I am captured"* is self-visible, so escaping is answerable (though no captor id is carried, and with one opponent none is needed). ⇒ **It is a gap at 3+ seats, which the demo supports** — `MAX_SMASH_SEATS = 4` — where a grab between two other fighters is the genre's clearest punish window and is invisible. ⓘ And `capture_value` is set from the MOVE's verb (`crates/ambition_characters/src/brain/fighter/options.rs:407`, `match c.binding.verb`), never from opponent state: the brain knows grabbing is worth something, and never that somebody IS grabbed. |

⭐ **Checkpoint 2 is worth reading before anyone plans roster work.** The charter's
hard architectural claim — *"distinct reusable move semantics without character-ID
engine branches"* — HOLDS under measurement. Searching engine crates for a
gameplay branch on a character id (`== "george"`, `character_id ==`, a `matches!`
on a fighter name) returns **nothing**; the single hit anywhere near it is
`character_catalog/registry.rs:212`, which dedupes catalog registration and is not
a gameplay branch. Twenty-one movesets and no engine knows any of their names.

⛔ **AND CHECKPOINT 2 IS THE ONE MOST LIKELY TO BE MISREAD, so its reference point
belongs next to its number.** *"21"* counts **authored movesets**. It does not
count what a player is offered, and the two are far apart:

| what is counted | number |
|---|---:|
| authored movesets (checkpoint 2's subject) | **21** |
| seats in the **standalone** demo | **3** |
| seats in the **composed** app | **≥8** |
| *distinct contracts behind the standalone 3* | **2** |

⇒ The standalone demo's three seats are George plus **two stand-ins that share one
contract** — `fighter_moveset()` — and that contract has **no special button**.
Measured by press rather than by binding (2026-09-04): the stand-ins answer
nothing on **15** `(base, direction, stance)` presses against George's **7**,
George's set is a strict **subset** of theirs, and the surplus is **eight, every
one a `special`**. ⭐ **So the stand-in is George's genre shape with the special
button removed** — guarded by
`the_stand_in_is_george_s_genre_shape_with_the_special_button_removed` in
`game/ambition_demo_smash/src/moveset.rs`.

⚠ **Both statements are true and neither weakens the other.** The architectural
claim this checkpoint actually tests — *distinct move semantics with no
character-ID engine branches* — holds exactly as measured, and 21 authored
movesets is the right evidence for it. ⇒ What the number does **not** license is
the reading *"the roster is in good shape"*, because roster DEPTH and roster
REACH are different quantities and this checkpoint only ever measured the first.
The reach question is open with Jon in
[`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md), and it
is a small question: four named special slots on one shared contract.

### Checkpoint 4, worked 2026-09-04 — the platforms a platform fighter had none of

⭐ **The engine ships one-way platforms in full and the demo used NONE of them.**
`BlockKind::OneWay`, `Block::one_way`, `resolve_one_way_hit`, a
`drop_through_timer`, and BOTH authored gestures — down+jump
(`wants_drop_through`) and the platform-fighter's own guard+down
(`wants_platform_drop`, whose doc reads *"on a surface that can be left
downward"*, i.e. it was written for this). Measured: **zero** occurrences of
`one_way` or `drop_through` anywhere in `ambition_demo_smash`. The same
shipped-primitive-with-no-customer shape the parity table found, on the feature
the genre is named after.

⇒ `smash_platform_stage()` is that customer: the solid floor plus three
drop-through tiers. Authored as a SECOND stage rather than an edit to
`smash_stage()`, because changing the stage everyone plays is Jon's design call,
and because every spacing/recovery/edgeguard number recorded so far was taken on
the flat block — a second layout gives that corpus something to be compared
against instead of invalidating it.

⛔ **THE TIER HEIGHTS I FIRST CHOSE WERE SCENERY, and this is the reusable
lesson.** Picked by eye: 132px and 250px. The engine states the arc on
`FighterBodyAuthoring::jump_speed` — apex is `v²/(2·gravity)` — and with the
shipped defaults (`GRAVITY` 2250, `JUMP_SPEED` 630, `DOUBLE_JUMP_SPEED` 520) a
single jump rises **88.2px** and an air jump taken exactly at the apex reaches
**148.3px**. So 250 was 100px above anything the roster can reach, and 132 was
inside the ceiling by 16px — a frame-perfect input. **Neither would have failed
anything**: the stage renders perfectly and the platform is simply unusable.
Shipped heights are 64px (a comfortable single jump) and 120px (needs the air
jump, 28px of margin), and the guard recomputes both from the engine constants so
retuning gravity reddens it rather than stranding a tier.

⇒ **And it IS reachable in play, same day.** I recorded that it was not, on the
grounds that `smash_prepared_session_world` is a plain `fn` with no world access.
That was wrong about the seam: `PlatformerExperienceAuthoring::install` takes
`S: IntoSystem<(), PreparedPlatformerSource, _>` and its own doc says the source
*"may read the provider's own resources"* — built for exactly this. So the choice
is an ordinary resource (`SmashStageChoice`, defaulting to `Flat`), the source
system reads it, and **both** stages go into the `RoomSet` with the choice
picking the starting one — returning only the chosen room would look identical on
any geometry assertion while making the other unreachable to anything that later
moves between them.

⇒ The player-facing half is `SelectTarget::Stage`, a cycle button beside START,
pressable by any seat. ⚠ **Two stages is still not "several"** — checkpoint 4
stays open and the next stage costs a `RoomSpec` and one enum variant.
✔✔ **A THIRD STAGE LANDED 2026-09-04, so checkpoint 4 closes too.**
`smash_narrow_stage()` is the flat block at two thirds the width —
**320px, ten 32px tiles** — with `FALL`/`SIDE`/`CEILING_BLAST_MARGIN_PX`
**unchanged**. ⭐ **The unchanged envelope is the design.** Those margins were
chosen so the PLATFORM carries Final Destination's normalized proportions, so
scaling them with the width would give the same stage smaller and change no
decision. Held fixed, they move the blast lines from `1.000 / 1.125 / 0.875`
platform-widths to `1.750 / 1.688 / 1.313` — less ground, same envelope, ledges
nearer the centre, longer offstage relative to what a fighter is returning to.
⇒ A third stage rather than an edit to either existing one, for the reason
`smash_platform_stage` already gives: **every recorded spacing, recovery and
edgeguard number was taken on `smash_stage`**, and the flat-versus-platforms
comparison on this page was taken on that geometry. Adding beside them costs no
recorded number its meaning. ⓘ Guarded by
`the_stage_choice_decides_which_stage_the_match_prepares`, which now asserts the
three share a blast envelope — so a fourth stage that moved one could not be
compared with the corpus and this test says so.

✔✔ **RULE SELECTION LANDED 2026-09-04, so checkpoint 5 is closed**:
`SelectTarget::Stocks`, a `Stocks: 1 / 3 / 5` cycle to the LEFT of the stage
cycle, pressable by any seat for the same reason the stage is — how long the
match runs is a decision about the MATCH, and gating it on card zero is the
player-one-centric shape Jon rejected.
⭐⭐ **AND THE CHANGE THAT MADE IT TESTABLE WAS NOT THE BUTTON.**
`apply_smash_match_rules` stated `STARTING_STOCKS` as a constant, so **no test at
any level could tell a wired button from an unwired one** — a control that
cycles a resource nothing reads looks completely correct on screen, label and
all. It takes the count as an ARGUMENT now, which forces both roads into it to
name the number: `smash_roster` and `SmashSelect::roster_seeded`, the road a
player actually travels. ⛔ That two-roads split is the same one that once let
the admiral reach the match unable to board its own summon, and this file's
`apply_smash_match_rules` records it.
⇒ Guarded end-to-end by `the_stocks_button_sets_the_count_the_published_match_is_played_at`,
which presses the real button on the real screen, starts the match, and reads the
count off the **published roster**. Poison-verified: making the rules ignore the
argument leaves the button and its label correct and turns exactly that
assertion red — *"the lobby said one stock and the published match says Some(3)"*.

⛔ **The confounder this was found through, which stands regardless.**
⛔ **AS WRITTEN, AND BOTH HALVES ARE NOW FALSE — kept because the checkpoint below
records when each stopped being true.** *"`SMASH_STAGE_ROOM_ID` is one constant,
`smash_stage()` is one function, and there is no stage-select concept in the demo
at all — the select screen's cursor targets are exactly `Portrait`, `RoleButton`,
`Start`, `PagePrev`, `PageNext`, `Back`. So the checkpoint's "several authored
stages change spacing/recovery decisions" has no customer to change: every
measurement this project has ever taken of spacing, recovery and edgeguarding —
the whole ladder rig included — was taken on one stage layout."*

✔ **The target list gained `SelectTarget::Stage` on 2026-09-04**
(`game/ambition_demo_smash/src/select_screen/layout.rs:379`, `targets()`), with a
second room to choose — which the Checkpoint 5 bullet below already says, two
paragraphs from a sentence claiming there is no stage-select concept at all.

✔ **And "one stage layout" stopped being true the same day.** Both stages have now
been measured against each other twice: once at 60 seconds and once at the shipped
480-second clock. ⚠ **The two runs disagree, and the long one wins** — the short
one said the tiers *"roughly halve the lethality"*, and at a clock long enough for
bouts to finish, stocks-left is `0.00` on BOTH stages and the tiers instead take
**1.83× as long** to reach the same end. See
[`../engine/fighter-brain.md`](../engine/fighter-brain.md).

⇒ **So the confounder this paragraph named is real and is now partly measured**,
which is a better position than either the original claim or a silent deletion of
it.

⇒ **Checkpoint 5, itemised**, since "partial" hides which half: stage select
**landed 2026-09-04** (`SelectTarget::Stage`, and a second room to choose);
rule selection **absent as UX** though
`MatchRules` exists as data (`ambition_match/src/prepared.rs:193` — stocks,
abilities, body) so the seam is there and nothing drives it; results/rematch
**present** (`coming_back_to_the_select_screen_offers_a_fresh_match`); training/
tuning **present** through the rig and probe tools.

⇒ **Checkpoint 6** now points at a finished measurement rather than a feeling:
the primitive table's ten shipped rows are what the brain can use, and its named
partials (`P02` fixed reactions and set knockback, `P10`'s unpublished tech
result, `P11`'s 2-of-6 capture roads) are exactly what it cannot answer yet. ⚠ The claim that *"the
ladder rig cannot currently rank skill either"* was true when written and is no
longer: at the shipped clock and the shipped rows it separates `3 vs 1` and
`5 vs 3` from the rest. ⛔ **The word *significantly* stood here and is ON HOLD** —
the rig printed its direction from pooled medians and its qualifier from a sign
test that discarded direction, so a cell could read decisive in the direction its
own evidence did not support. ✔✔ Fixed 2026-09-04 (`36dd9a248`) **and re-run the same day: the four cells came
back UNCHANGED**, so `5 vs 3` is confirmed rather than merely restated. `median()`
was corrected in the same pass and the survival/damage columns moved slightly; the
verdicts did not.
the supersession note below.

⛔ **In short, so nobody quotes a superseded number from here — this one moved
three times in a day.** The original *"35 of 36 verdicts inside the seed spread"*
used survival-until-a-cap as the verdict, which saturates at both ends and pays
for passivity. An outcome verdict (stocks taken, then damage dealt) resolved the
18 uninformative cells and read 24 : 12 toward the LOWER rung. ⛔ **That skew was
the SEAT.** Under `--paired` — each seed run twice with the rungs swapped between
seats — it becomes **16 : 19**, 14 of 36 cells change verdict, and the sign test
goes from suggestive to nothing.

⛔⛔ **AND IT MOVED A FOURTH AND FIFTH TIME LATER THE SAME DAY. EVERYTHING ABOVE
THIS LINE IS SUPERSEDED — the paragraph that follows is the current reading.**

⚠ The `6 vs 5` finding above was real about the ENGINE FLOOR and is **not** the
defect. Two things were wrong with the instrument underneath it: the rig read
`FighterBrainProfile::for_level` instead of the shipped
`fighter_brain_ladder.ron`, and its bouts ran **60 seconds against a shipped match
of 480**. ⇒ On a clock that short no bout can END, so stocks tie in every cell and
every verdict falls through to the damage tiebreak. Both are fixed (`--ladder
PATH`, and the clock now reads `SMASH_TIME_LIMIT_TICKS`).

⭐⭐ **THE CURRENT PICTURE — the shipped rows, the shipped clock, bouts that
resolve.** Replicated at 12 and 28 seeds with every verdict identical:

| cell | verdict |
|---|---|
| 3 vs 1 | ✔ higher outfights |
| **5 vs 3** | ⛔ **LOWER outfights — the one established defect** |
| 6 vs 5 | *(within spread)* |
| 9 vs 6 | *(within spread)* |

⛔⛔ **THE WORD *"established"* IS ON HOLD AS OF 2026-09-04.** A review found the
rig's `report_row` carries **two authors of one row's meaning**: the printed
direction comes from POOLED MEDIANS, while the `(within spread)` qualifier comes
from a PAIRED, DAMAGE-ONLY sign test that **discards which direction won**
(`k = positives.max(negatives)`). ⇒ A row can print `LOWER outfights` with the
qualifier removed while its own test is significant for HIGHER, and it looks
exactly like a row where the two agree.

⚠ **And note what does NOT rescue it: replication.** *"12 and 28 seeds, every
verdict identical"* is real and it is the wrong reassurance here — a defect in how
a row is composed is **deterministic**, so it reproduces perfectly at every seed
count. ⇒ Replication tests sampling noise; it cannot see a systematic fault, and
citing it against one is the same mistake as trusting a null from an instrument
that could not resolve its subject.

⭐ **What survives untouched is the MECHANISM**, because it never routed through
the qualifier: stocks level at `2 : 2`, the lower rung dealing more damage, and
`frame_advantage` + `expected_payoff` isolated **byte-for-byte** as reproducing
the whole effect. ⇒ Read `5 vs 3` as *"a defect with a named cause and an
unconfirmed significance label"*, not as withdrawn — and re-take it first.
✔✔ **RE-TAKEN 2026-09-04 AND THE HOLD IS LIFTED** (`36dd9a248` fixed the rig,
`fdbc77b1c` records the re-run): the four cells came back with the same verdicts
in the same directions, so `5 vs 3` carries its significance label again.

⚠ **The descriptive columns were re-taken with them, and the post-fix numbers are
in [`../engine/fighter-brain.md`](../engine/fighter-brain.md) under THE DEFINITIVE
RUN.** `median()` had returned the upper-middle order statistic on an even sample;
corrected, every column on that table moves and none by much — largest move
**360% → 349%**, survival by under a second and a half, verdicts unchanged.
⇒ Any stocks or damage figure quoted ON THIS PAGE is still the pre-fix generation
unless it says otherwise; prefer the fighter-brain table when the digits matter.


⇒ **`6 vs 5` is NOT the bad rung; `5 vs 3` is.** The rollout story does not apply
to it either — the shipped ladder sets `rollout_depth: 0` on all nine rows, so no
player has ever met the L3 search or the `Dodge`/`Shield` suppression. ⭐ The cause
is `frame_advantage` + `expected_payoff` jointly, isolated byte-for-byte, and
holding that pair flat above rung 3 removes the inversion without flattening the
rung.

⭐ **So the scoreboard HAS now been pointed at the fighter a player fights**, which
is what the sentence here used to say it had not. See
[`engine/fighter-brain.md`](../engine/fighter-brain.md) for the tables and
[`../awaiting-maintainer-decision.md`](../awaiting-maintainer-decision.md) for
the design question the measurement cannot answer.

⚠ **Five supersessions in one day is the story worth keeping**, and it is why the
superseded text stays above rather than being deleted: every one was the same
class — the instrument's configuration differed from the shipped game's, and only
the instrument was ever read.


1. **Core fight:** attacks, shield, grab, dodge, movement, launch, recovery,
   ledges, tech, stocks, respawn, and readable feedback support a fun short
   match.
2. **Roster depth:** several fighters exercise distinct reusable move semantics
   without character-ID engine branches.
3. **Local play:** two or more human participants can join, select fighters, and
   complete matches alongside CPUs.
4. **Stage breadth:** several authored stages change spacing/recovery decisions,
   including at least one kinematic-platform customer.
5. **Match completeness:** stage select, rule selection, results/rematch, and
   training/tuning support make iteration and ordinary play coherent.
6. **CPU adoption:** the fighter brain can use and answer the mechanics that
   define the current roster without a Smash-only AI stack.

## Exit

Smash has graduated from acceptance demo to a strong game slice when adding a
fighter, stage, or match rule normally means authoring content or extending one
reusable semantic owner; CPU and human fighters obey the same body laws; the
same characters remain ordinary Ambition characters outside the ruleset; and a
short local match is fun without developer interpretation of what the systems
are doing.
