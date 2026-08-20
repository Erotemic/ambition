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

## Open decisions — 15 (§1, §6, §7, §9, §10, §11, §12, §13, §22 and §27 are ANSWERED; §8 is DEFERRED)

### 1. ✔ ANSWERED 2026-08-17 — a bolt hits what a sword hits (former D23)

`projectile/systems.rs` now resolves victims through **`StrikeVictim`**, the
same named role melee uses, owned by `ambition_combat::hitbox` beside the
victim-geometry rule.

```text
INTANGIBILITY   ✔ CLOSED — a body carrying an EMPTY `DamageableVolumes` list
                  now offers NO target, so a bolt no longer lands on (and is
                  eaten by) a body a sword passes straight through
PRECISION       ▢ OPEN — the overlap test is still the coarse `victim.aabb`
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

### 2. Advance the measurement-submodule pointer?

`dev/ambition_dev_measurements` contains useful committed measurement history.
The remaining policy question is whether the superproject should advance its
submodule pointer whenever those measurement commits are accepted, or leave the
pointer intentionally pinned. This currently blocks no engine work.

### 3. Give rust-analyzer its own target directory?

Jon's local `.vscode/settings.json` can set:

```json
"rust-analyzer.cargo.targetDir": true
```

This is build-hygiene only. It isolates rust-analyzer artifacts from the normal
Cargo target directory; it is not established as the cause of the old linker
failure.

⭐ **MEASURED 2026-08-14, and it is no longer only hygiene — it is a throughput
cost.** During a long agent session, rust-analyzer's
`cargo check --workspace` restarted roughly **every 50 seconds** and took the
target-directory lock each time. The agent's `cargo check -p ambition_app
--all-targets` — the 21-second gate — took **1m26s of actual work spread across
about 9 minutes of blocking**, and one focused render test took 6 minutes. ⚠
nothing was corrupted and no failure was mysterious; the entire cost was
`Blocking waiting for file lock on build directory`.

⇒ so the question has changed shape. It is not "does this prevent a linker
failure" (unestablished, and probably no) but **"is a second target directory
worth roughly 100 GB to stop the editor and the agents from serialising against
each other?"**

⭐⭐ **THE DISK HALF OF THIS IS RE-MEASURED 2026-08-17, and it moved a lot — in
BOTH directions inside one day.**

```text
2026-08-14   volume 93% full, ~137 GB free, target 106 GB
2026-08-17   volume 68% full,  156 GB free, target 210 GB
             (target/debug/deps 151G · debug/incremental 35G · release 21G)
```

⚠ **the target directory DOUBLED in three days**, so "does not fit twice" is
more true now, not less — even though free space went up. ⚠ and the free number
is volatile rather than stable: the same volume hit **100% full** earlier on
2026-08-17 (a `cargo test --workspace --tests` sweep), and 160 GB was recovered
by deleting `target/debug/incremental`, which has since regrown to 35 GB.

⇒ **so the honest framing for the disk half is not a headroom number but a
policy**: `target/debug/incremental` is a safe, self-refilling 35 GB that can be
deleted at any time, and it is roughly the size a rust-analyzer check directory
would need. ⭐ the throughput half needs no re-measuring — the contention is
still live and still reproducible: a plain `cargo check -p ambition_platformer2d_shared_tangle`
this session printed `Blocking waiting for file lock on build directory` — which
is why it is your call and not an agent's. A cheaper variant if the disk answer
is no: leave the directory shared and accept that only one builder makes
progress at a time.

### 4. Mary-O restart report: which game, and roughly when? (former D68)

Current Mary-O tests cover all three death routes — hit, timeout, and
pit/hazard/kernel reset — and each returns the body to spawn; the pit fixture also
re-arms a spent question block. The remaining observation cannot be reproduced
from current Mary-O mechanics.

Needed fact: **was the report actually in Mary-O, and was it before or after
2026-08-08?** If it was Ambition or Sanic, investigation should move to that
host instead of changing Mary-O's proven replay path.

### 5. Smash CPUs walk off the stage at every difficulty — is this news? (measured 2026-08-14)

⛔⛔ **THE HEADLINE EVIDENCE BELOW WAS A UNIT ERROR AND IS RETRACTED (2026-08-19).
The CPUs fight hard, and they always did.** `0.84%` was **84%**.
`BodyHealth::damage_percent` returns a RATIO — its own doc says *"`1.88` is a
legal answer and is how a HUD prints `188%`"* — and `ladder_rig` printed it under
a literal `%`, so every reading of that column was a hundredth of the truth. The
rig's *"BUT NEITHER LANDED A HIT"* marker was then given a threshold picked to
fit the misreading (`1.0`, which in the column's real units is a full 100% KO
meter), so it fired on genuine duels.

⭐ **the corrected table, same build, same fifteen seeds:**

```text
3 vs 1     48.0% :  45.0%     (was 0.48% : 0.45% — "NEITHER LANDED A HIT")
5 vs 3    111.0% :  98.0%
6 vs 5    139.0% : 101.0%
9 vs 6    193.0% : 158.0%
```

⭐⭐ **and the ladder DISCRIMINATES in the column nobody could read**: peak damage
rises monotonically with the rung, and the higher rung out-damages the lower on
all four pairs — while the time column stays within spread on every row. The
instrument had a working signal the whole time and it was being printed at 1%
scale.

✔ **confirmed independently in the shipped composition**, not just the demo app:
two level-9 CPUs on `npc_pirate_admiral` (a character a player can pick off the
grid) reach **169 damage against a pool of 100** in sixty seconds, spending 575
ticks each in hitstun. Pinned by `app_it::smash_cpus_damage_each_other`, which
states its units in the assertion message so this cannot be misread again.

⛔⛔ **AND THE SELF-KO HALF IS STALE TOO — RE-MEASURED THE SAME DAY AT FIFTEEN
SEEDS, AND IT POINTS THE OTHER WAY.** `ladder_probe --seeds 15` (one fighter,
opponent cannot attack, so every loss is a self-KO):

```text
level  first self-KO              stocks lost
    1  23.2s [17.7-46.3] +1 never      3
    3  16.3s [12.0-45.3] +8 never      1
    5  21.7s [16.0-46.5] +7 never      1
    6  16.3s [14.5-27.1] +2 never      2
    9  none in 60s                     0
  9/d0  none in 60s                    0
 9/d12  none in 60s                    0     ⭐ the A/B is FLAT
```

⇒ **the `rollout_depth` diagnosis is falsified by its own instrument**: depth 0
and depth 12 both survive the clock with zero self-KOs, so *"a twelve-tick search
is choosing to leave the stage"* no longer holds. And the row's other sentence —
*"the upper half of the ladder is the half that self-destructs"* — is now exactly
backwards: **level 9 never self-KOs and every rung below it does.**
Self-preservation now improves with the rung, which is the direction a difficulty
ladder is supposed to run.

⚠ **three seeds would have said something else**, as this rig's own header warns:
at the default 3, level 3 and level 9 both showed *no self-KO* and level 5 showed
one. Fifteen is the number that answers.

▢ **what is genuinely left for you is one design question, much smaller than the
row it came from**: a level-1 CPU loses all three stocks to itself inside a
minute. That may be exactly right for the easiest rung — a bad opponent should
be bad — or it may read as broken rather than easy. That is taste, not
engineering.

▢ what stays open is the row's actual question — is CPU quality on the path to
what Smash is for — now costed better: the gap between a rung that self-KOs and
one that does not is a single authored field.

⇒ engine-side this is a decision-model investigation, and it blocks ladder
calibration entirely. The question for you is priority: is CPU quality on the
path to what Smash is for, or is it acceptable that CPUs are currently sparring
partners that suicide? Detail in [`engine/fighter-brain.md`](engine/fighter-brain.md).

### 14. Two things the one-brick rescale forced, both wanting your eye

Both fell out of the rig refactor and are recorded rather than guessed at,
because each trades against something you tuned by looking at her.

**a. The shared collision width went 64 px → 56 px.** Your ruling is one width
for every form (*"we keep the width of collision the same for big and small"*),
and the sheet's own guard wants the box narrower than the drawing so she never
collides on her hat or her sleeves. Those two together are decided by the
NARROWEST form, and the one-brick short form's whole drawing is **60 px** wide —
so the old 64 collided on empty air beside her. 56 clears her and still hugs the
grown torso (~62 px).

⚠ **the cost is that the grown form's box narrowed too**, which is the price of
the identical-width rule. The alternative is widening the short form's ART to
~68 px, and her width is exactly what you tuned by eye, so it is your call.

**b. Her one-brick box has 6 px of empty air above her hat.** (Her world size
is settled: `SMALL_FORM_HEIGHT = T`, one tile small and two grown, whole suite
green — the "level-wide rescale" this was blocked on was a unit error, not
content. This is only about the 6 px inside her own sheet.) The box top is set
by the height contract (small is one brick, grown is two, so short height ×2 =
grown height exactly), not measured off the art. MEASURED: grown form 0 px of
headroom, fire form −14 px (its flame frills clear the box on purpose), short
form **+6 px**. So she is drawn very slightly shorter than one brick and will
bump a ceiling with the air over her head.

**c. Every walk frame puts her foot BELOW her standing line — on both forms.**
MEASURED: small dips +0.50 to +1.00 units under its own idle foot line, grown
+0.33 to +1.17. She walks through the floor by up to a fifth of a tile, and the
renderer's clipping warning on those frames is just the canvas noticing (the
small form has zero headroom, so any dip is also amputated). ⚠ pre-existing on
the grown form — its walk frames came through the rig refactor byte-identical.
⭐ **the cause is three authored numbers, not a mystery** — `SHORT_POSES["walk"]`
and its tall twin:

```text
walk#0  leg_back_dy  = 1.0   the trailing leg is pushed DOWN a unit
walk#2  leg_front_dy = 1.0   mirrored, same push
walk#1  bob          = 0.4   and `foot_y = 30.2 + bob`, so the DIP moves her FEET
```

⇒ the stride's extension is being spelled as a downward translation, and the
mid-stride dip lowers the contact point instead of the body above it. Both read
fine in isolation; both put the foot under the floor.

⇒ **listed rather than done, because it changes an animation you have seen** —
and because the honest fix for the dip needs a pose field that lowers the TORSO
without moving `foot_y`, which does not exist yet. Say the word and it is small.

⇒ closing (b) means either raising her crown ~6 px — which moves the 40/40/20
head/body/legs split you specified — or accepting the 6 px. The test bounds it at
8 px so it cannot quietly grow while you decide.

### 15. Whose hitstop owns the SCREEN when nobody is playing?

Small, and it only exists in CPU-vs-CPU. ⛔ **not a defect** — I filed it as one
("presentation hitstop is slot-0 only") and corrected it the same day, because
the part that matters already works: D114 made both movement roads spend hitlag,
so **both bodies stop on a connect**. That is the impact you see.

What slot 0 additionally requests is the SIM CLOCK freezing — particles, VFX,
other bodies — a flourish on top. And slot 0 is right for that system: its other
arms are bullet-time and blink-hold, which are per-PLAYER feel affordances by
ADR 0010/0011. A second player emits its own intent against its own clock.

⚠ **in a CPU-vs-CPU match there is no `PrimaryPlayer` at all**, so nobody asks.
That file already carries your 2026-08-07 freeze from exactly this shape — a
paused match forced the clock to zero, nobody was left to ask for the neutral
pace back, and the world ran at scale 0.0 forever (*"the characters are just
stuck in air"*). ⇒ whatever answers this must also be able to hand the clock
back.

Defensible answers, none obviously right:

```text
nobody's           CPU matches simply never screen-freeze — simplest, and the
                   bodies still stop, so the hit still reads
the most recent    whichever fighter just connected owns the freeze
the framed one     the fighter the camera is following owns it
```

⇒ recorded rather than guessed. If you have no view, "nobody's" is the one that
cannot regress the 0.0-forever failure, because it never asks.

### 16. Who owns a level's POSITION — its area spec, or the layout tool?

`level diff-specs` exists to catch an area spec drifting from its live level. It
was reading none of its subjects (YAML-only loader, every area spec is RON), and
fixing that turns it on: **52 specs differ, 2 match, 78 coordinate mismatches**,
some very large — `volatile_cache`'s spec says `world_x = 72000`, the live level
is at `2048`.

⚠ that is almost certainly not 52 broken levels. `world auto-layout` arranges
Free-layout levels by their LoadingZone graph, and the tool's own message says
*"live LDtk wins"* — so the specs' coordinates look like initial placements that
stopped being authoritative and nobody re-recorded.

```text
specs own position    re-record the live values into 52 specs; drift is then real
layout owns position  drop world_x/world_y from area specs; the graph decides
                      placement and the spec stops claiming something it lost
```

⇒ **the check is fixed but deliberately NOT in CI**, because bulk-rewriting 52
specs to silence it would answer this by accident. ⛔ 13 of the 52 are a
different thing — specs for levels in another world file, which the command
cannot see because it takes one `--ldtk`; that is a usage limit, worth a second
flag whichever way you rule.

### 17. How much floating text should a room show? (D161's residue)

Every loading zone now carries authored prose instead of an id, so the original
complaint is gone. What the measurement turned up on the way is a different
question: **rooms are dense with floating text, from two independent sources.**

```text
room                    DebugLabels   always-on zone labels
gate_stack_lower             14              3
drain_alley                  13              2
combat_calibration_lab        8              2
first_system_boss             6              1
intro_wake_room               2              1
...12 rooms carry both; 3 more have a zone label and no signage
```

⚠ **`DebugLabel` is doing player-facing work** — *"creator's basement lab"* and
*"→ corridor"* in the opening room are DebugLabels with `category: Custom`, not
debug output. So the name says one thing and the usage says another, and nothing
decides which rooms are dressed for a player.

Two questions, either answerable in a sentence:

```text
is DebugLabel authored SIGNAGE or a debug affordance?
  — if signage, it wants a better name and a pass for the rooms with 13 of them
  — if debug, those rooms are showing debug text to players

should a NON-Door zone draw an unconditional label at all?
  — a Door's nameplate is proximity-gated and clearly wants its prose
  — 24 of the 151 named zones are EdgeExit and draw theirs always, beside
    signage that may already say it (the other 127 are Doors)
```

⇒ no longer urgent — nothing on screen is an id — so this is polish, recorded
rather than guessed.

### 18. Should a hit's ART know what it hit? (D128 defect 6's residue)

The untextured quad you photographed is fixed: `VfxMessage::Impact` — the most
drawn effect in the game, written by every actor hit, projectile hit, pickup and
grapple — was a bare yellow rectangle, and it now draws the shipped `hit_soft`
row at 0.6 x `FX_DEFAULT_WORLD_SIZE` (~33 world units against a 46-unit fighter).

What that turned up is a question, not a defect: **two vocabularies already exist
for this and have never been joined.**

```text
the engine ships     hit_soft   hit_hard   hit_metal   hit_energy     (generic_action_fx)
the sim already has  Flesh      Robot      Metal                      (ImpactMaterial)
```

⛔ **I did not join them, deliberately**, for two reasons worth your ruling:

```text
PLUMBING   the material lives on the VICTIM's `HurtFeedback`, and
           `VfxMessage::Impact` carries a position and nothing else — so this is
           a message change touching ~10 emitters, not a lookup
TASTE      `hit_hard` has no material at all; it reads as a STRENGTH distinction
           (a jab vs a smash), which is the attack's fact, not the victim's.
           So "material picks the row" only explains three of the four
```

⇒ one sentence settles it: **does a hit's art follow the body being hit, the
strength of the blow, both, or neither?** ⚠ *"neither"* is a real answer — one
spark for every hit is what fighting games mostly do, and it is what ships today.

### 19. Should the sheet registry key by TARGET or by FILE ROOT? (D162's residue)

Nothing is visibly broken and this is not urgent — it is a one-line ruling that
closes a whole class, and the measurement is already done.

Two lookups exist for the same baked sheets, and they key differently:

```text
record_index()                  keys by FILE ROOT  — 196 unique keys, no ambiguity
                                (used by posed_body_geometry + the animation road)
SheetRegistry::from_baked_table keys by TARGET     — 5 keys claimed by 48 files
```

The 48 are sheets authored against a shared rig adapter: **robot 18, toon 16,
goblin 9, sandbag 3, ninja 2.** For those five keys the target-keyed registry
cannot answer *"give me sheet X"* — whichever manifest loads last wins, and three
of them currently take the key away from a same-named character's own sheet.

⛔⛔ **CORRECTED 2026-08-19 — THE COUNT WAS RIGHT AND ALL THREE NAMED WINNERS
WERE WRONG.** This row said `robot_archivist` over `robot`,
`goblin_brute_hammer` over `goblin` and `sandbag_armored_review` over `sandbag`.
In each case that is the **first** non-own claimant, not the last — the row read
the collision list with last-wins inverted. Measured against the real generated
baked table (812 entries, 166 targets, 39 geometry-differing collisions — the
same 39 the crate's own comment records):

```text
robot    18 claimants   own 256x256  LOSES to tech_bro_disruptor  215x256
goblin    9 claimants   own 239x253  LOSES to ranged_skirmisher   235x229
sandbag   3 claimants   own 128x128  LOSES to sandbag_full_review 256x256
toon     16 claimants   not a catalog id — no character resolves by it
ninja     2 claimants   not a catalog id
shrine    2 claimants   the SAME file via two directories, identical geometry
```

⚠ **the table sorts by file root with `_spritesheet` STRIPPED**, which is what
makes `robot` sort before `robot_archivist`; modelling the sort with the suffix
attached inverts exactly these three and reproduces the original mistake. ⇒ read
the generated table, not a model of it.

⭐ **and "stale manifest" is the wrong frame for two of the three.**
`tech_bro_disruptor` and `ranged_skirmisher` are distinct characters whose
manifests declare a shared rig target; neither file is stale and there is no
pair to retire. Only `sandbag`'s three are one character's own variants. The
full write-up is [`../../dev/reviews/sheet-target-collisions-2026-08-19.md`](../../dev/reviews/sheet-target-collisions-2026-08-19.md).

⭐ **it appears harmless today**: every consumer of the target-keyed resource
(shrine, slash, projectile, boss) looks up a name where root == target, and the
character-geometry road uses the file-root index and cannot collide.

```text
switch to file root   the 148 root==target sheets are unaffected; the 48 each get
                      their own key; the class becomes impossible
leave it              keep the reporter that now names the three, and retire a
                      stale manifest per pair by hand as they appear
```

⇒ I did not take it: it changes what a shared engine resource returns for 48
files, on inference rather than on a stated intent for what that key MEANS.

⭐⭐ **AND THE 2026-08-18 REVIEW SUPPLIES THE MISSING INTENT — read this before
ruling, because it reframes the question from plumbing to identity.** Its words:
*"Renderer target names, sheet/file roots, generated asset IDs, and canonical
`CharacterId` are different namespaces. Do not let a sprite-renderer target
string accidentally become the durable identity of a character package.
Character identity should remain stable and semantic; renderer targets/products
are implementation/presentation identities associated with it."*

```text
CharacterId        semantic, durable, the character package's name
renderer target     an authoring/presentation choice — which generator drew it
sheet file root     a PRODUCT — one published page
generated asset id  a build artifact's name
```

⇒ **on that reading the two options stop being symmetric.** Keying a shared
engine resource by TARGET is precisely letting a renderer-side string act as the
durable identity of a thing the engine looks up — and the 48 collisions are that
mistake becoming visible, since a shared rig adapter is an authoring detail that
48 different characters happen to share. Keying by FILE ROOT names a product,
which is what the registry actually serves.

⚠ **still not taken, and now for a better-stated reason**: the review argues the
PRINCIPLE, and the ruling also decides what `SheetRegistry` is FOR — a
product lookup or a character lookup. If it is meant to be a character lookup, the
right fix is neither key but a `CharacterId`, and that is a bigger change than
either row above.
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
parallax.rs       ⛔ NOT DONE — must gain a view concept: no view id, `.single()`s
                   the main camera, and builds its viewport from
                   `WINDOW_W`/`WINDOW_H` rather than from the camera it draws for
```

⚠ **the distance threshold and the hysteresis band are FEEL values Jon has not
named** — measure them against a real two-player session rather than picking
constants.
⛔ **adaptive layout promotes the silent-wrong fallback into a real defect**: with
several cameras, label layout and nameplates fall back to a **world-origin** focus
(`Vec2::ZERO`) instead of declining to draw, and under this policy several cameras
is the ordinary case rather than the exception. ⚠ `MainCameraEntity` is a SEVENTH
process-global *"the main camera"* resource that this layout has to answer for.

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

### 20. ▢ NEW 2026-08-19 — eight boss chests are authored with treasure that does not exist

**The wiring half is FIXED and is not the question.** `ChestFeature::reward()`
had zero callers: every chest in the game opened, sparked, played its sound,
announced *"opened X"* and granted nothing, because `open_ecs_chests` asked for
`With<ChestFeature>` and never `&ChestFeature`. It now routes the payload through
the same `grant_pickup` the walk-over pickup uses — **130 authored chests in the
shipped world start paying out**: 104 health chests, 13 `ability:test_key`, 13
`flag:opened_basement_story_chest`.

▢ **what is left is a CONTENT call and it is yours.** The eight boss reward
chests in `boss_profiles.ron` are authored `PickupKind::Custom(..)` —
`pirate_hoard`, `gnu_scroll`, `noodly_relic`, `trex_bone_relic`,
`collapsed_relic`, `divergence_shard`, `stack_frame_relic` and one more — and
**each of those ids appears in that one file and nowhere else in the tree**: no
item, no ability, no flag, no catalog row. `Custom` has no reader in the engine
at all. So a defeated boss still drops a chest that pays nothing, and closing the
wiring did not change that.

⇒ **the question is what a boss's hoard IS**, which is a design answer rather
than an engineering one. The three shapes available today:

```text
an ABILITY      the boss teaches a verb — the north star's "every upgrade a
                theorem" beat, and the road bosses already use for
                `reward_ability` beside the chest
a QUANTITY      currency/health — cheap, works now, says nothing about the boss
a NEW ITEM      each relic becomes a real catalog item with art and a use;
                the most work and the only one that makes the names mean
                something
```

⚠ **not guessed at in the meantime, deliberately.** Inventing eight item
definitions would be authoring content policy at an engine seam. What landed
instead is that the unspendable case is now LOUD: a `Custom` payload reaching the
grant warns with the id and says nobody was awarded it, so this stops being
invisible. ⛔ the silent `_ => {}` that swallowed it is what let eight shipped
bosses drop empty treasure without a single line of evidence.


### 21. ▢ NEW 2026-08-19 — separating control authority from AI policy breaks a FROZEN wire format

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

**⛔ WHY THIS IS A DECISION AND NOT A TASK.** `Brain` is encoded in the rollback
snapshot, and `rollback-wire-format-is-frozen` (363 stable names, 118 encoded
types) is an absence contract that currently HOLDS. Removing a variant changes
that format. The options:

```text
A  take the break     one migration, the contract's baseline is re-frozen in the
                      same commit, and every peer/save from before it is
                      incompatible. Cleanest end state.
B  additive first     add the control-authority component, leave Brain::Player in
                      place reading it, migrate consumers, delete the variant in a
                      later break. Two flag days instead of one, and a window
                      where two things answer "who drives this".
C  not now            the seam stays named and unbuilt until a netplay/save
                      compatibility break is happening anyway for other reasons.
```

⚠ **I am not choosing.** Every option is defensible and the cost lands on save/
peer compatibility, which is yours to spend. ⛔ option B is the one that looks
safest and is not: a window where possession can be expressed two ways is exactly
the state the `ScriptedControl`/`ControlHolds` breach came from — a derived fact
and its source disagreeing, resolved by whoever writes next.

⭐ **SLICE 1 IS LANDED (2026-08-20), and it needed no decision.**
`control::DrivingParticipant(PlayerSlot)` is the fact by itself, **DERIVED** each
tick by `project_driving_participant` from `Brain::Player` plus
`PossessionState` — so it adds no snapshot entry and no ENCODED bytes, and none
of A/B/C applies to it.

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

**What STILL needs the decision**: deleting the `Brain::Player` variant and
`PossessionState::restore_brain`/`restore_scope`. That is the wire-format break,
and it is exactly A/B/C above. Everything up to it can proceed by moving readers
onto the derive one at a time.


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

### 23. ▢ NEW 2026-08-19 — five `the_stage_kills` guards are RED, three of them from the legality filter, and the prescribed fix contradicts a deliberate fairness property

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

### 25. ▢ NEW 2026-08-20 — does "AVOID PUSHOUT" cover fighter-vs-fighter JOSTLE?

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

⇒ **the question**: is the third option acceptable, or does the rule extend to
body-vs-body entirely and jostle should not be built? ⛔ not answered by refactor
— I have left the row `▢` rather than guessing at a rule Jon stated twice.

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


### 26. Rename the blast zone out of every world's authoring schema? (D169)

⭐ **the measurement first, because it changes the question the plan asked.**
`world-geometry-and-spatial-semantics.md` argues the engine provides a bespoke
platform-fighter primitive that Smash should instead declare over generic
geometry. Measured at HEAD, the MECHANISM is already generic:
`apply_world_hazard_gate` computes a per-axis distance past the world AABB and
emits `ResetCause::LeftTheWorld` — *"policies flag; the body's owner applies its
reset policy"* — so Smash loses a stock, Mary-O respawns, and Ambition calls it
out of bounds, all from one engine fact. `blast_margin`'s own doc already says
it: *"a platformer's pit depth and a platform fighter's blast zone — the same
number, and it belongs to the STAGE."*

⇒ **what leaks is the WORD, and it leaks furthest in the place you actually
meet it.** All six shipped LDtk worlds carry all three fields in
`defs.levelFields`:

```text
sanic_speedway · intro · sandbox · you_have_to_cut_the_rope
hall_of_characters · mary_o          blast_margin, side_blast_margin, ceiling_blast_margin
```

⭐⭐ **and ZERO levels author a value** — 18 schema entries, no data behind any
of them. So this costs no content migration. Every author of every world is
shown three platform-fighter fields nobody has ever filled in.

**The decision is yours because the `.ldtk` files are yours.** The converter reads
the authored key by name, so the struct field and the authored field are ONE name:
renaming the Rust half alone needs a mapping, and a mapping is the shim this
project refuses. It is one change or it is not worth 206 sites.

**What I would do, if you want it done:**

```text
World { blast_margin, side_blast_margin, ceiling_blast_margin }
  -> World { edges: WorldEdgeMargins { fall: f32, side: Option<f32>, rise: Option<f32> } }
```

One field instead of three, named for the axis role rather than the genre, and
the kernel destructures it EXHAUSTIVELY so a fourth axis is a compile error
rather than a forgotten comparison — the same shape as `CapabilityLanes`. The
LDtk keys become `fall_out_margin` / `side_out_margin` / `rise_out_margin`, which
is a `defs.levelFields` rename in six files and no value to carry.

⚠ **options if you would rather not:** (a) leave it — the leak is lexical and the
engine is correct, and the doc comments already explain the generic meaning to
anyone who reads them; (b) rename the Rust struct only and accept the mapping,
which I do not recommend; (c) do the whole rename, which is mechanical and
guarded by `a_level_authors_its_own_blast_margin` plus the LDtk contract prover.

⛔ **`BlockKind` is the plan's other half and is NOT this.** Its diagnosis — one
enum mixing contact law, traversal permission, world consequence and contact
affordance — was re-measured as correct, and its trigger has not fired. Nothing
here proposes touching it.
