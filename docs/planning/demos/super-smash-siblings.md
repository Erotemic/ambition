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
| 3 | Local play | ✔ **met** — driven end-to-end through the real screen, not the model |
| 4 | Stage breadth | ◐ **was NOT met; worked 2026-09-04 and now reachable in play** — `smash_platform_stage()` adds the genre's drop-through tiers, `SmashStageChoice` selects it, and a stage button on the select screen cycles it. Two stages is not yet *"several"* |
| 5 | Match completeness | ◐ **partial, 3 of 4** — stage select landed 2026-09-04; rule selection is still absent as UX |
| 6 | CPU adoption | ◐ **partial, and now measurable for the first time** |

⭐ **Checkpoint 2 is worth reading before anyone plans roster work.** The charter's
hard architectural claim — *"distinct reusable move semantics without character-ID
engine branches"* — HOLDS under measurement. Searching engine crates for a
gameplay branch on a character id (`== "george"`, `character_id ==`, a `matches!`
on a fighter name) returns **nothing**; the single hit anywhere near it is
`character_catalog/registry.rs:212`, which dedupes catalog registration and is not
a gameplay branch. Twenty-one movesets and no engine knows any of their names.

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
pressable by any seat. ⚠ **Two stages is not "several"**, and rule selection is
still absent, so neither checkpoint is closed — but the mechanism both of them
were waiting on is built, tested and poison-verified, and the next stage costs a
`RoomSpec` and one enum variant.

⛔ **The confounder this was found through, which stands regardless.**
`SMASH_STAGE_ROOM_ID` is one constant, `smash_stage()` is one function, and there
is no stage-select concept in the demo at all — the select screen's cursor targets
are exactly `Portrait`, `RoleButton`, `Start`, `PagePrev`, `PageNext`, `Back`
(`select_screen/layout.rs:359`). So the checkpoint's *"several authored stages
change spacing/recovery decisions"* has no customer to change: **every measurement
this project has ever taken of spacing, recovery and edgeguarding — the whole
ladder rig included — was taken on one stage layout.** That is a confounder in
existing results, not only a missing feature.

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
longer: at the shipped clock and the shipped rows it ranks `3 vs 1` and `5 vs 3`
significantly. See [`engine/fighter-brain.md`](../engine/fighter-brain.md) and
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
