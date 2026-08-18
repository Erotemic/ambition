# Awaiting a maintainer decision

Only questions whose next step is **Jon's product/authoring judgement** belong
here. Engineering questions go to the queue/tracks; answered questions move to
[`maintainer-decisions.md`](maintainer-decisions.md). The pre-prune investigation
record is archived at
[`../archive/planning-superseded/2026-08-13/awaiting-maintainer-decision.md`](../archive/planning-superseded/2026-08-13/awaiting-maintainer-decision.md).

⭐ **Every row re-checked against the tree on 2026-08-17.** Two closed (12: the
submodule pushes fine; 13: the policy suite is green AND now runs in CI), and
five had moved without anyone noticing — 1 is half-landed, 5's headline is
falsified by a capture and re-measured by its own rig, 8's fork was decided and
pinned by the D146 campaign, 11 is two-thirds executed.
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

## Open decisions — 5 (§1, §6, §7, §9, §10, §11, §12 and §13 are ANSWERED; §8 is DEFERRED)

### 1. ✔ ANSWERED 2026-08-17 — a bolt hits what a sword hits (former D23)

⭐⭐ **HALF OF THIS IS ALREADY RESOLVED — reconciled 2026-08-17; the row reads as
fully open and is not.** `projectile/systems.rs` now resolves victims through
**`StrikeVictim`**, *"the SAME NAMED ROLE melee uses"*, owned by
`ambition_combat::hitbox` beside the victim-geometry rule.

```text
INTANGIBILITY   ✔ CLOSED — a body carrying an EMPTY `DamageableVolumes` list
                  now offers NO target, so a bolt no longer lands on (and is
                  eaten by) a body a sword passes straight through
PRECISION       ▢ OPEN — the overlap test is still the coarse `victim.aabb`
```

⭐ the file says so itself: *"the INTANGIBILITY half of that is closed in the
loop below; the precision half is still open, and says so there."* ⚠ and the
comment records why the old claim was false: the tuple that would have carried
`DamageableVolumes` **had run out of arity**, so *"the claim was never anything
but prose"* — sharing the type is what made it checkable.

⭐⭐ **RULED: the projectile respects the AUTHORED HURT VOLUME — the same geometry
melee uses.** One victim-geometry rule for everything, so a crouching or
ledge-hanging fighter reads the same to a bolt and to a sword, and an authored
hurtbox finally means one thing.
⛔⛔ **this is a real feel change on shipped content, and it is intended**: a shot
that connects today against a body whose authored volume is tighter than its AABB
will start missing. That is the point, not a regression to file.
⚠ per-volume overlap now runs on every projectile tick — **measure it rather than
assuming it is free**, and say so at the loop.


Current source still has the split: projectile collision uses the victim's coarse
`CenteredAabb`, while melee/feature reach can use the body's published authored
silhouette. Boss projectiles are excluded from the coarse-body path because a
composite boss envelope is too broad.

Choose one:

- **Authored hurt volume** — projectiles use the same published body geometry as
  the other damage families. This also permits retiring the anonymous boss
  `HitTarget::UnresolvedFeatures` path.
- **Coarse body box** — preserve today's projectile feel and keep the two damage
  geometry laws intentionally distinct.

This is feel, not missing engineering evidence.

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

**b. Her one-brick box has 6 px of empty air above her hat.** The box top is set
by the height contract (small is one brick, grown is two, so short height ×2 =
grown height exactly), not measured off the art. MEASURED: grown form 0 px of
headroom, fire form −14 px (its flame frills clear the box on purpose), short
form **+6 px**. So she is drawn very slightly shorter than one brick and will
bump a ceiling with the air over her head.

⇒ closing it means either raising her crown ~6 px — which moves the 40/40/20
head/body/legs split you specified — or accepting the 6 px. The test bounds it at
8 px so it cannot quietly grow while you decide.

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
this session printed `Blocking waiting for file lock on build directory`. ⚠ the volume is at 93% with ~137 GB free and the existing target
dir is 106 GB, so this genuinely does not fit twice — which is why it is your
call and not an agent's. A cheaper variant if the disk answer is no: leave the
directory shared and accept that only one builder makes progress at a time.

### 4. Mary-O restart report: which game, and roughly when? (former D68)

Current Mary-O tests cover all three death routes — hit, timeout, and
pit/hazard/kernel reset — and each returns the body to spawn; the pit fixture also
re-arms a spent question block. The remaining observation cannot be reproduced
from current Mary-O mechanics.

Needed fact: **was the report actually in Mary-O, and was it before or after
2026-08-08?** If it was Ambition or Sanic, investigation should move to that
host instead of changing Mary-O's proven replay path.

### 5. Smash CPUs walk off the stage at every difficulty — is this news? (measured 2026-08-14)

Not a decision so much as a finding you should see before it gets designed
around. Two independent rigs agree: a Smash duelist loses all three stocks to
ITSELF, at 0% damage, at every authored rung. In a real duel neither fighter
exceeds 0.84% peak damage — they never hit each other; the "outlast" numbers the
ladder rig reports are measuring who walked off later.

The clean A/B, same level-9 profile with only `rollout_depth` varied: **depth 0
survives 47.8s, depth 12 survives 7.4s.** The L3 rollout is enabled
automatically at level ≥ 6, so the upper half of the ladder is the half that
self-destructs fastest.

⛔⛔ **HALF OF THIS IS FALSIFIED BY A PHOTOGRAPH — 2026-08-17.** The claim above
is *"they never hit each other"*, evidenced as **peak 0.84% damage**. A capture
of a live two-CPU match this morning shows:

```text
George Booul     180%   ·  3/3 stocks
Pirate Admiral   124%   ·  3/3 stocks
```

⇒ **the CPUs now hit each other constantly.** That is not a mystery: the
measurement predates D155 (every authored `launch_dir` was inverted and a
tumbling launch resolved as a landing), D114 (hitlag reached only the avatar
road, so a CPU-versus-CPU hit froze nobody) and D157. Damage was landing all
along; almost nothing about it worked.

⚠ **the OTHER half is untested and is the part that mattered** — whether a
level-9 duelist still walks off the stage, and whether `rollout_depth` still
inverts the ladder (depth 0 survived 47.8 s, depth 12 survived 7.4 s). Nothing
since has touched the decision model, so the finding is *plausibly* intact — but
the rig that produced it was measuring a fighter that could not deal damage, and
a search that now has real hits to weigh may choose differently.

✔ **RE-RUN 2026-08-17. The finding HOLDS in direction and is much smaller in
size — and the headline sentence is now false at depth 0.**

```text
level  first_self_KO   survived   stocks_lost   peak%
9/d0      none in 60s     >60s          0         0%     ⭐ no self-KO at all
9/d12          21.8s      >60s          2         0%
                        (was: d0 survived 47.8s · d12 survived 7.4s)
```

⇒ **the rollout still kills the fighter and depth 0 still does not**, so *"a
twelve-tick search is choosing to leave the stage"* stands. But d12's first
self-KO moved 7.4 s → **21.8 s**, and **d0 now loses ZERO stocks** where this row
says *"a Smash duelist loses all three stocks to ITSELF at every authored
rung."* ⚠ that sentence is now wrong at d0 and right everywhere the shipped
ladder applies depth, which is **level ≥ 6** — so the upper half of the ladder is
still the half that self-destructs, exactly as the row argued.

⛔⛔ **AND THE `peak%` COLUMN IS 0% BY CONSTRUCTION — do not read it as evidence
about duels.** The harness prints its own warning: *"opponent cannot attack, so
every loss is a self-KO."* ⇒ this rig **cannot** speak to whether CPUs damage each
other, and it **cannot** speak to decision 6's hitlag question either, because
with an inert opponent there are no hits and therefore no hitlag. The
`180% / 124%` capture above remains the evidence for the first; decision 6 still
needs a played match.

▢ what stays open is the row's actual question — **is CPU quality on the path to
what Smash is for** — now costed better: the gap between a rung that self-KOs and
one that does not is a single authored field. ⚠ the 3/3 stocks in that capture are also a
data point in the other direction: at 180% neither had died to anything, itself
included.

⇒ engine-side this is a decision-model investigation (a twelve-tick search is
choosing to leave the stage), and it blocks ladder calibration entirely. The
question for you is priority: is CPU quality on the path to what Smash is for,
or is it acceptable that CPUs are currently sparring partners that suicide?
Detail in [`engine/fighter-brain.md`](engine/fighter-brain.md).

### 6. ✔ ANSWERED 2026-08-17 — hitlag freezes the body that is in it (former D114)

⭐⭐ **Jon, verbatim:** *"keep the landed fix and overrule the old prohibition …
**hitlag is a combat/body semantic, not something that should depend on whether a
body happens to occupy the primary local-control road** … Keep `sim_dt = 0.0`
during that body's hitlag. Mark the old prohibition superseded. **If hitlag later
feels too sticky, tune its duration/shape rather than restoring a
controlled-body/actor asymmetry.**"* Recorded in
[`maintainer-decisions.md`](maintainer-decisions.md); `818218949` is the code.
⛔⛔ **the three options this row used to offer are VOID** — every one preserved a
per-road distinction, and the distinction WAS the defect.
⭐⭐ **the lesson is about EVIDENCE, not hitlag.** Two documents independently
warned against this fix, and both were measured on a build where every authored
launch direction was inverted and a tumbling launch resolved as a landing (D155)
— i.e. where nobody was ever knocked anywhere. **A feel verdict inherits the
build it was formed on**, and D155 invalidated every judgement that predates it.
⚠ the process failure is not excused by the outcome: the commit consulted neither
document, so the prohibition was *unseen rather than overruled*.

⭐⭐ **AND ANSWERING THIS UNBLOCKED D117, which is the consequence to act on.**
This decision gated TIME INTEGRATION and nothing wider: the controlled and actor
roads still have two body integrators, and unifying them means merging their
limbs — hitlag-dt gating and ledge carry are the home road's, the flight limb is
the actor road's. *"Does the merged integrator freeze an actor body on its own
hitstop?"* **was** this question, and the ruling answers **yes, on both roads**.
⇒ D117's last structural item is now executable, and so is folding the three
per-population `decay_reaction_timers` calls into one system (the controlled site
decays on `frame_dt`, the other two on sim `dt`).

### 7. ✔ ANSWERED 2026-08-17 — a dropped weapon persists PER ITEM, not by one rule

The lifetime bug is fixed for ability/currency/health drops: the entity and its
visual now share room scope. The remaining laser-sword observation is a product
rule for **held-item drops** after a fight:

⭐⭐ **RULED: authored per item.** A story or unique weapon stays in the world
where it fell; an ordinary dropped one is room-scoped like the other drops.
⭐ consistent with the same day's inventory ruling, where UNIQUENESS is what
decides whether a thing needs its own identity.

⛔⛔ **AND IT PROMOTES A KNOWN RESIDUE INTO A PREREQUISITE.** A minted instance
**not in a hand** at save time — lying in a room, in flight — is undescribed and
lost today, because the description remembers no POSITION (D133's open item). A
persisting dropped weapon IS that case, so it must be built first rather than
noted beside.
⚠ it also needs a per-item authoring field that does not exist yet.
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

⇒ so *"two of fourteen author, and the grant scaffold cannot be deleted while
the rest do not"* is no longer the situation: the scaffold is gone and the
EFFECTIVE set is uniform across the cast whether or not a given character
authors one.

⏸ **DEFERRED BY JON 2026-08-17.** Fourteen fighters is a small sample and the kits
were only just completed, so everyone keeps the uniform effective kit and
**personality comes from MOVESETS rather than from missing verbs** until enough
matches have been played to know who feels wrong.
⭐ nothing is blocked and nothing is lost: the grant scaffold is already deleted,
so an omission MEANS something the day one is authored — the mechanism is ready
and only the content decision waits. ⛔ do not propose an absence list unprompted,
and ⛔ do not author an absence for balance reasons in the meantime.


**Authoring verbs is currently a nerf, and that is why only two characters do
it.** A seat's abilities are the character's authored set ∩ the mode's declared
set. The intersection is right and is pinned — a ruleset may forbid, and may
never hand a body a verb it lacks. The consequence is that a character stating
`basic() + attack`, everything its old archetype row actually said, LOSES
shield, dodge, ledge-grab, double-jump and dash, because today it receives those
from the `(None, mode) => mode` grant. Two of fourteen fighters author an
`AbilitySet`; the grant scaffold cannot be deleted while the rest do not.

Choose one:

- **Universal baseline, absences authored** *(recommended — this is genre
  research, not taste)*. Every seated fighter authors the full platform-fighter
  set, because in this genre every fighter shields, dodges, ledge-grabs,
  double-jumps and dashes; a creature that should NOT have one omits it
  deliberately, and the omission then means something. Character ∩ mode =
  baseline, so authoring costs nothing and the grant arm loses its last consumer.
- **Per-creature verb sets, no baseline** — the grant arm stays indefinitely and
  the mask keeps punishing whoever authors first.
- ⛔ **Let the mode grant verbs a body lacks** — listed to be refused: it deletes
  the invariant `a_match_cannot_grant_a_verb_the_character_does_not_have` pins.

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

⛔⛔ **this row carried a FALSE PREMISE for a day** — *"the project gate now runs
`cargo test --workspace --lib`"*, inherited from D160's premature closure. It
never did; what landed was a pre-push paragraph in `AGENTS.md`. ⇒ **a row's
premise is worth re-checking against the tree, not against the row that claims to
have moved it.**

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

**Found 2026-08-15 while closing D118's C4.** Not a bug report — a feel question
with two defensible answers, which is why it is yours.

`CameraShakeState::amplitude_px` is added straight to the camera's translation:

```rust
transform.translation.x = x + shake_offset.x;
```

That translation is in WORLD units, and the camera's `orthographic_scale` is what
converts world to screen. So the on-screen displacement is
`amplitude_px / orthographic_scale` — **the same hit shakes the screen less the
further the camera is pulled out**, and more when it is zoomed in. Ambition's
observed gameplay scale is 0.5, so a shake authored at the hub reads at roughly
double strength there compared with a zoomed-out framing.

`hit_shake_amplitude` is documented in `HIT_SHAKE_GAIN_PX_PER_S`, and the field
is named `_px`, so the NAME says screen pixels while the behaviour says world
units. One of the two is wrong and I cannot tell which from the code.

- **Constant on screen** — divide the offset by `orthographic_scale`. A hit feels
  identical however the camera is framed, which is what "px" promises and what
  most action games do.
- **Constant in the world** *(what ships today)* — a shake is a physical
  displacement of the viewpoint, so a distant camera showing more world naturally
  registers it as smaller. Defensible, and arguably better for a camera that
  zooms out during a big fight: the screen does not thrash harder as the stakes
  rise.
- **Rename and keep the behaviour** — if world-units is what you want, the field
  is `amplitude_world` and the constant is not `PX_PER_S`. Cheapest option, and
  it stops the next reader making my mistake.

⭐⭐ **RULED: constant in the world — keep the behaviour, fix the NAME.** A shake
is a physical displacement of the viewpoint, so a camera showing more world
registers it as smaller; a camera that pulls out as a fight grows should calm the
screen rather than thrash it harder.
⇒ **what changes is `amplitude_px` → `amplitude_world` and
`HIT_SHAKE_GAIN_PX_PER_S` with it.** ⛔ the maths is not touched.
⭐ and the rename is now unambiguous rather than merely tidier: by the same day's
ruling **one world unit IS a base-grid pixel**, so `_px` would stay permanently
confusable while `_world` says exactly which quantity this is.

### 11. ✔ ANSWERED 2026-08-17 — split-screen layout is ADAPTIVE WITH HYSTERESIS

⭐⭐ **TWO THIRDS OF THIS WAS TAKEN AND EXECUTED ON 2026-08-15 — the row reads as
fully open and is not (reconciled 2026-08-17).** The stated engineering default
below, *duplicate per view*, was implemented for two of the three systems:

```text
label_layout.rs   per-view projections   d09229ceb (2026-08-15)
nameplates.rs     per-view projections   d09229ceb
view_isolation.rs isolate by RELATIONSHIP, not identity   b732e5d6a
parallax.rs       ⛔ NOT DONE
```

⚠ **`parallax.rs` still has no view concept at all** — zero mentions of a view
id, `.single()` on the main camera, and a viewport built from the constants
`WINDOW_W`/`WINDOW_H` rather than from the camera it is drawing for. So it is not
merely un-duplicated; it could not serve a second view of a different size.

⇒ **what is left of this decision is parallax alone**, and the default already
applied twice says what to do with it. ⭐ the genuinely open question is the one
the row itself flags at the end — the LAYOUT POLICY above the fork — not this.

⚠ **noted rather than asked, and D116 M2 proceeds without the answer** — the
first two items of M2 landed 2026-08-14 and do not depend on this.

Three draw systems are genuinely per-view — foreground/parallax, label layout and
nameplates — and each builds **one** set of world-space entities: one `Transform`
per world label, per nameplate, per parallax layer. Naming which view they serve
is not the blocker. Per-view *correctness* needs one of:

- **duplicate per view** — each view owns its own label/nameplate/parallax
  entities. The general answer, and what shared / fixed-split / BG3-adaptive all
  eventually need, since a second view is a count rather than a special case.
  Costs entities per view.
- **pick one view** — keep one set, drive it from a designated view. Smaller, but
  it re-centralises the thing D116 exists to decentralise, and the next slice
  undoes it.
- **stop here** — two views are already structurally real (distinct transforms,
  distinct viewports), and the three systems refuse loudly at two cameras. Return
  when a product need for split-screen actually arrives.

⭐⭐ **RULED 2026-08-17: adaptive with hysteresis** — one shared framing while
participants are close, splitting into viewports as they separate, with hysteresis
so it cannot flap at the boundary. Recorded in
[`maintainer-decisions.md`](maintainer-decisions.md).
⇒ **and that settles the engineering fork under it by implication.** A layout that
can split at ANY MOMENT cannot be served by one set of world-space entities, so
**duplicate per view** is the only surviving option and *pick one view* is dead.
`parallax.rs` must gain a view concept: it has none today, `.single()`s the main
camera, and builds its viewport from `WINDOW_W`/`WINDOW_H` rather than from the
camera it draws for.
⚠ **the distance threshold and the hysteresis band are FEEL values Jon has not
named** — measure them against a real two-player session rather than picking
constants.
⛔ **adaptive layout promotes the silent-wrong fallback into a real defect**: with
several cameras, label layout and nameplates fall back to a **world-origin** focus
(`Vec2::ZERO`) instead of declining to draw, and under this policy several cameras
is the ordinary case rather than the exception. ⚠ `MainCameraEntity` is a SEVENTH
process-global *"the main camera"* resource that this layout has to answer for.

⚠ two related shapes found while landing M2's first half, both left alone: with
several cameras, label layout and nameplates fall back to a **world-origin**
focus (`Vec2::ZERO`) rather than declining to draw — silent-wrong where the rest
of this seam is loud-wrong — and `MainCameraEntity` is a **seventh** process-global
"the main camera" resource that split-screen will have to answer for.

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

## ✔ CLOSED 2026-08-15 — every submodule remote is reachable and current

**Was:** `git push` in `tools/ambition_sfx_renderer` failed with *"correct access
rights"*, and `main` already recorded a commit from it, so a fresh clone could
not resolve the pointer. Probing the rest found three more on the same footing,
each behind its own credential alias.

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
