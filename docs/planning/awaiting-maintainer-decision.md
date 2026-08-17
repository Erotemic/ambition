# Awaiting a maintainer decision

Only questions whose next step is **Jon's product/authoring judgement** belong
here. Engineering questions go to the queue/tracks; answered questions move to
[`maintainer-decisions.md`](maintainer-decisions.md). The pre-prune investigation
record is archived at
[`../archive/planning-superseded/2026-08-13/awaiting-maintainer-decision.md`](../archive/planning-superseded/2026-08-13/awaiting-maintainer-decision.md).

## Open decisions — 11

### 1. Projectile collision: authored hurt volume or coarse body box? (former D23)

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

⇒ **what is left for you is only the precision half**, which is the genuine feel
call: should a bolt respect the authored hurt volume, or keep hitting the coarse
body box? The invulnerable-window and corpse cases no longer ride on it.


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

### 6. What should fighter-vs-fighter hit emphasis do without the primary local seat? (former D114)

⛔⛔ **THIS DECISION WAS ANSWERED BY IMPLEMENTATION ON 2026-08-17, AND THE OPTION
TAKEN IS THE ONE THIS ROW TOLD US NOT TO TAKE. Please confirm or revert.**

`818218949` (*"Both roads spend hitlag, so a CPU-versus-CPU hit freezes
somebody"*) added to the actor road:

```rust
let sim_dt = if combat.is_in_hitlag() { 0.0 } else { dt };
```

⇒ that is **a direct per-body zero-dt**, which the paragraph below calls an
experiment that *"made AI-vs-AI bouts degenerate"* and says explicitly: **do not
reintroduce that fix.** The commit does not mention this row, so the prohibition
was not weighed — it was not overruled, it was unseen. ⚠ and this row had already
named that risk exactly: *"guessing it decides feel by refactor."*

⭐ **what is genuinely different this time, and why it may nonetheless be right:**

```text
+153 lines  features/enemies/integration/hitlag_tests.rs   (new, with the fix)
green       a_second_match_on_the_same_stage_counts_in_and_ends  — CPU vs CPU,
            a CPU-produced launch still spends a stock and the match still ENDS
```

⇒ so the bout does not degenerate in the sense a test can see: it still
terminates. ⚠ **but "degenerate" in the original report was a FEEL word**, and no
test in this repository can tell a good AI-vs-AI bout from a bad one — which is
precisely why this was your decision and not an agent's.

▢ **the ask is small: play one CPU-versus-CPU match and say whether it feels like
the bad experiment.** If yes, the revert is one line at
`features/enemies/integration.rs`. If no, this row closes and the third option it
proposes (extending the global 0.125 beat) never needs trying.


`BodyCombat::hitstop_timer` is armed for every body, but the actor road does not
freeze its integration from that timer. A direct per-body zero-dt experiment was
already tried and made AI-vs-AI bouts degenerate, so **do not reintroduce that
fix**.

Choose the desired feel for a landed hit between two fighters where neither is
the primary local controlled body:

- no extra freeze beyond today's timers/presentation;
- a proper-time/per-body treatment designed at the ADR 0011 seam; or
- extend the existing global 0.125 hit-emphasis beat to any seated-fighter hit.

The third is the smallest Smash-oriented experiment; it has not been tried.

⭐ **this decision gates TIME INTEGRATION, and nothing wider (narrowed
2026-08-14).** The controlled and actor roads still have two body integrators, and
unifying them means merging their limbs: hitlag-dt gating and ledge carry are the
home road's, the flight limb is the actor road's. Whether the merged integrator
freezes an actor body on its own hitstop IS this question. The same choice decides
whether the three per-population calls to `decay_reaction_timers` can become one
system, since the controlled site decays on `frame_dt` and the other two on sim
`dt`. Answering it unblocks both; guessing it decides feel by refactor.

⛔ **it does NOT block controlled/AI contract convergence, which was the reading
for a day and was wrong.** The `ActorControl` producers converged on 2026-08-14
without touching either integrator: one `tick_controlled_brains` translates
participant control for any controlled body, and `tick_actor_brains` skips
player-brained bodies. Control authority and time integration were separable, and
the only thing that had joined them was the sentence that named them together.

### 7. How long should a dropped held weapon persist? (former D50)

The lifetime bug is fixed for ability/currency/health drops: the entity and its
visual now share room scope. The remaining laser-sword observation is a product
rule for **held-item drops** after a fight:

- disappear when leaving the room;
- remain in the world when returning; or
- use another explicit persistence policy.

Whichever rule is chosen, simulation entity and presentation must share the same
lifetime.

### 8. Which platform-fighter verbs does each creature author?

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

▢ **what is left is exactly the part this row already calls yours**: the
per-creature absence list — which creature should NOT shield, dodge, ledge-grab
or double-jump, so that the omission means something.


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

### 9. What should the per-turn suite actually run? (measured 2026-08-14)

**471 tests are hidden behind features in eight crates, nothing runs them
automatically, and every one of them is green today.** So this is a question
about future regressions, and the price is small but not zero.

⭐⭐ **NARROWED 2026-08-17 by D160, which changed the premise.** The project gate
now runs `cargo test --workspace --lib`, so **every crate's BARE lib suite is
watched** — that half of "nothing runs them automatically" is no longer true.
⇒ **what stays unwatched is only the WITH-FEATURES DELTA**, which is what the
table below actually measures: `ambition_input` 54 → 115, `ambition_audio`
25 → 64, `ambition_touch_input` 4 → 45. ⚠ note the delta is where the interesting
tests live — a crate whose bare suite is 4 and whose real suite is 45 is being
watched at under a tenth of its coverage.

⇒ so the decision is now the smaller one: **is the feature-gated delta worth a
second gate pass, and if so which features?** — not "should anything run at all".

⭐⭐ **AND HERE IS WHAT THE DELTA ACTUALLY HOLDS — a named consequence rather
than a count (2026-08-17).** `game/ambition_demo_mary_o_app/tests/painted_blocks_still_change_their_art.rs`
opens with `#![cfg(feature = "visible")]`, so **the whole file** is in the
unwatched half. What it guards is a bug Jon reported and we fixed:

```text
a_question_block_in_the_painted_cavern_wears_its_own_art
a_painted_block_nobody_dresses_keeps_its_flat_quad
the_invisible_brick_triggers_from_below
a_discovered_hidden_block_reveals_itself
```

⇒ the file exists because *"one line in `level_1_2()` opted every block in the
cavern out of art updates, permanently"* — and **a regression of exactly that
would not be caught by the gate today.** ⚠ Jon's observations file still carries
open block-art items in the same area, so this is a live surface, not a settled
one.

⭐ that makes the decision concrete: the `visible` feature on the two demo apps
is the highest-value single addition, because it is where the ART assertions
live and art is what Jon reports.

`scripts/feature_gated_tests.py` says 24 crates hide 629 tests. Eight were run
explicitly at HEAD:

| crate | bare | with its features |
|---|---|---|
| `ambition_demo_mary_o_app` | 31 | 45 (`visible`, 9.7s) |
| `ambition_demo_sanic_app` | 25 | 45 (`visible`, 4.9s) |
| `ambition_touch_input` | 4 | 45 (`mobile_touch`) |
| `ambition_audio` | 25 | 64 |
| `ambition_portal2d_presentation` | 16 | 45 (`effect_view_cones`) |
| `ambition_input` | 54 | 115 (`input`) |
| `ambition_game_shell` | 45 | 70 (`basic_presentation`) |
| `ambition_dialog` | 30 | 42 |

The ones that matter most for the reports you actually file are the demo apps'
`visible` guards: they are the only thing in the repo asserting what a block
LOOKS like, and D64's row was opened because a *"a discovered hidden block pays
out invisibly"* report turned out to be already-fixed-and-never-run.

**Why this is yours and not mine.** Both runners are decisions you made:

- `.github/workflows/test.yml` is `on: workflow_dispatch` — disabled 2026-05-07,
  *"no need to churn the servers with rust CIs until we have something we really
  need github action testing for."*
- `scripts/gate_suite.py` runs only `cargo test -p ambition_app --test app_it`,
  shrunk on your measurement — *"I want to bias towards running less tests to
  balance out the agent urge to run more."*

Quietly enlarging the gate you shrank would be disobeying that ruling while
looking careful, so:

- **Leave it as-is** — the guards run only when an agent is doing visual work and
  remembers. That is today, and it is why the stale report survived.
- **Add the two demo `visible` runs to the FULL path of `gate_suite.py`**
  *(recommended)* — ~15s of test time, but a distinct build configuration from
  the gate's, so the first run of a turn that touches source pays a compile. It
  buys the only automated watch on presentation art.
- **Re-enable CI for these** — costs you nothing per turn and catches things a
  day later; needs the 2026-05-07 ruling revisited.
- **Something narrower** — e.g. run them only when `game/ambition_demo_*` or
  `crates/ambition_render` changed, which is the cheap targeted version and needs
  a path rule in `gate_suite.py`.

---

### 10. Camera shake is measured in "px" and behaves as world units

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

⚠ **I did not change it.** The zoom range in the shipped game is narrow enough
that nobody has reported it, and picking silently would be choosing a feel.

### 11. Two views need per-view world-space entities, or a policy that picks one

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

⇒ **the engineering default, taken unless you say otherwise: duplicate per
view**, because it is the only option that does not have to be undone. ⚠ what is
genuinely yours is not this fork but the LAYOUT POLICY above it — shared, fixed
split, or adaptive-with-hysteresis — and no agent should invent that enum.

⚠ two related shapes found while landing M2's first half, both left alone: with
several cameras, label layout and nameplates fall back to a **world-origin**
focus (`Vec2::ZERO`) rather than declining to draw — silent-wrong where the rest
of this seam is loud-wrong — and `MainCameraEntity` is a **seventh** process-global
"the main camera" resource that split-screen will have to answer for.

### 12. ✔ CLOSED 2026-08-17 — the `ambition_map_assets` submodule pushes fine

⭐ **verified from inside the VM rather than assumed**: the submodule's local
HEAD is `4fb0c03`, `git ls-remote origin HEAD` answers with the same sha, and
`origin/HEAD..HEAD` is empty. Nothing is stranded and a fresh clone resolves.

⇒ this row duplicates the credential outage closed on 2026-08-15 further down
this file (*"five of five submodules answer `git ls-remote`, none is ahead of its
`origin/main`"*) — Jon provisioned the aliases and this one was covered. It is
kept, struck through, because the CONSEQUENCE it describes is the one worth
remembering: a superproject gitlink can point at a commit that exists only in one
working tree, and nothing in the superproject's own green push says otherwise.

⚠ **the original text follows, for the failure mode.**

⚠ **environment, not design — it needs your credentials, not a decision.**

`game/ambition_map_assets` commit `73baab9` ("The two patrollers name their path
instead of spelling it") is committed locally and **could not be pushed**. Its
`origin` is a rewritten SSH host (`git@aivm-cred-git-…:Erotemic/ambition_map_assets.git`),
and both that and the plain `https://github.com/Erotemic/ambition_map_assets.git`
in `.gitmodules` fail with *"Please make sure you have the correct access rights"*.
The superproject's own remote pushes fine.

⇒ **consequence:** the superproject gitlink at `c28414a0d` and later points at a
map-assets commit that exists **only in this working tree**. A fresh clone will
not resolve it until that submodule commit is pushed. The two migrated worlds
(`intro.ldtk`, `sandbox.ldtk`) are the content at risk.

⛔ **I did not work around this** — a workaround here means either rewriting a
remote or vendoring assets out of their submodule, and both are yours to decide.
Everything else in that slice is pushed and green.

### 13. ✔ MOSTLY CLOSED 2026-08-17 — the suite is green and CI now watches it

⭐ **measured, not assumed**: `cargo test -p ambition_workspace_policy` is
**34 passed / 0 failed**, `engine_policies` among them, and the five rules this
row named are all still IN `engine.toml` (187 rules total) — so the twelve
violations were FIXED rather than waived away.

⭐⭐ **and item 1 was fixed the way this row asked for**, which is the part worth
noting: the `gate_portal` determinism flag was a false positive on code that
collected and then sorted, and the row said *"a waiver would be the wrong answer
here — make `phases` a `BTreeMap` so ordered iteration is a property of the
type."* It is now `BTreeMap<String, GatePortalPhase>`, and the file carries a
comment guarding against a revert to `HashMap`.

⭐ **the second half — "nothing watches it" — was still true this morning.** Not
`run_tests.py`, not CI, not the goal guard. It now runs in CI's `engine-tests`
job. That job rather than the headless one because the crate inspects the
workspace as parsed manifests and source text and links no production crate (its
own manifest: *"running the policy suite must not compile `ambition_app`"*), so
it needs a toolchain and nothing else, and it costs ~5s.

▢ **what is genuinely left for you** is only the original question this row
asked — whether every one of the 187 rules deserves enforcement — and it is much
cheaper to answer now that the answer costs nothing: they all pass.

⚠ the original text follows.

### 13. The workspace policy suite is red, and nothing watches it

⚠ **measured 2026-08-15: `cargo test -p ambition_workspace_policy` reports 12
violations in the `engine` scope**, and that suite is **not** among the goal
guard's 13 checks. Three separate slices landed violations in one day and nothing
caught them, which is why this is a decision rather than a bug report.

The violations are not one problem. They sort into three kinds, and only you can
say which deserve enforcement:

1. ⭐ **a false positive on correct code.** `engine.determinism` flags
   `gate_portal.rs:197` for iterating a std hash container — but that site
   `collect`s and then **sorts**, which is deterministic. ⇒ the honest fix is
   structural: make `phases` a `BTreeMap` so ordered iteration is a property of
   the type rather than a discipline the next editor can drop. **A waiver would
   be the wrong answer here.**
2. **genuinely pre-existing debt** — `movement-model-is-never-optional`,
   `player-fallback-update-documented`, `pose-writes-are-authority-only` all name
   files untouched today.
3. **boundary rules with real content** — `runtime-manifest-deny` and
   `runtime-source-no-upper` say `ambition_platformer2d_runtime` must not name
   `ambition_platformer2d_ldtk`. ⚠ **that edge predates today** (it is in the
   manifest at the pre-merge commit), so the rule has been violated for a while.

⇒ **the decision: should this suite join the goal guard?** ⛔ I did not add it —
the guard's check list is yours, and adding a red check would stop every
autonomous run until the twelve are cleared. The alternative is to treat the
suite as advisory and fix the twelve on their own merits.

#### ⭐ UPDATE 2026-08-16 (D134): the twelve are ZERO, so the objection above no longer applies

**The reason not to add it was that it was RED.** It is not: `cargo test -p
ambition_workspace_policy` is **34/34, 0 violations**. The decision is still
yours — I did not touch `.goal/active.json` — but here is everything it costs,
measured rather than estimated, so the call is cheap to make:

```text
what it costs      6.2 s / 9.7 s / 8.6 s wall, warm, three consecutive runs
                   (the suite's own libtest line reports 5.2–9.9 s)
what it REbuilds   nothing. `ambition_workspace_policy` links no production
                   crate — it reads the repository as data — so an engine source
                   edit does not invalidate its build. The cost above is
                   essentially all runtime, on every turn, forever.
against            check [4] `cargo test -p ambition_app --test app_it` = 158.6 s
                   ⇒ adding it is a ~5% increase on the guard's dominant check
```

⚠ **and "nothing watches it" was half wrong, which changes the shape of the
question.** `./run_tests.sh` DOES run it — the backbone job is
`cargo test --workspace` and the suite is workspace member `tests/ambition_workspace_policy`
(verified with `./run_tests.sh --list`). What it is absent from is the *per-turn*
gate: the goal guard's 8 checks, `scripts/gate_suite.py` (which runs only
`-p ambition_app --test app_it`), and AGENTS.md's stated gate. ⇒ the real question
is not "is it checked at all" but **"is it checked on the turn that breaks it"** —
and the answer today is no, which is how one facade deletion on 2026-08-15 turned
into seven red sites nobody saw.

⇒ **what I did instead, being non-blocking:** AGENTS.md's Verification section now
names the suite and the three change-shapes that redden it, with the cost above.
That is a documented pre-landing step, not a gate. If you want the gate, the line
is:

```json
{"name": "the ADR-backed workspace policy suite is green", "cmd": "$HOME/.cargo/bin/cargo test -p ambition_workspace_policy --quiet"}
```

⭐ **the three kinds above, resolved** — and note that only ONE of the twelve was
debt in the ordinary sense:

1. ✔ **kind 1 was right, and it was the only assessment that needed re-checking.**
   `phases` is a `BTreeMap` now (the site had moved to `:199`). The hash order
   never reached an observable — the `collect` was immediately `sort_by`'d — so
   this was a hazard removed, not a defect fixed. The test that guarded it was
   itself weak (it would have stayed green through a revert to `HashMap`, because
   only the checksum was asserted); it now asserts the CONTAINER's key order and
   was poison-tested red against `HashMap`.
2. ✔ **kind 2 split three ways rather than being three of a kind.**
   `movement-model-is-never-optional` was a REAL ADR 0024 §1 violation and was
   hiding a second one the rule could not spell (`Option<&ae::MotionModel>` in
   `perception_body_for`, whose `None` arm read a missing component as
   `AxisSweptMotion::default()`) — both non-optional now.
   `player-fallback-update-documented` was **deleted**: `333c48376` deleted its
   subject (the slot board and the `PlayerSlot` anchor) months before the rule
   noticed. The two pose/velocity writes were **policy imprecision** — a pre-spawn
   `ActorClusterSeed` and an off-sim `BodyClusterScratch`, neither of which has an
   authority to route through — and both now say their state at construction.
3. ✔ **kind 3 was the interesting one and it went the other way.** `cargo tree`
   settles it: `ambition_platformer2d_ldtk`'s entire transitive closure contains
   ZERO occurrences of `ambition_platformer2d_runtime` or
   `ambition_platformer2d_actor_monolith`, and the monolith — long allowed — has
   linked it directly all along. **The edge is downward, and the two rules were
   stating one wrong fact twice.** They were changed, with the argument written
   into their own rationale fields. ⚠ the concern they were groping at is real and
   survives as **queue D135**: `PlatformerSessionWorld` carries a format-specific
   `LdtkRuntimeIndex` field that five RON-only games fill with `::default()`.

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
