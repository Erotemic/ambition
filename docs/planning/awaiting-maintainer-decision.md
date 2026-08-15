# Awaiting a maintainer decision

Only questions whose next step is **Jon's product/authoring judgement** belong
here. Engineering questions go to the queue/tracks; answered questions move to
[`maintainer-decisions.md`](maintainer-decisions.md). The pre-prune investigation
record is archived at
[`../archive/planning-superseded/2026-08-13/awaiting-maintainer-decision.md`](../archive/planning-superseded/2026-08-13/awaiting-maintainer-decision.md).

## Open decisions — 11

### 1. Projectile collision: authored hurt volume or coarse body box? (former D23)

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
each other?"** ⚠ the volume is at 93% with ~137 GB free and the existing target
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

⇒ engine-side this is a decision-model investigation (a twelve-tick search is
choosing to leave the stage), and it blocks ladder calibration entirely. The
question for you is priority: is CPU quality on the path to what Smash is for,
or is it acceptable that CPUs are currently sparring partners that suicide?
Detail in [`engine/fighter-brain.md`](engine/fighter-brain.md).

### 6. What should fighter-vs-fighter hit emphasis do without the primary local seat? (former D114)

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

### 12. The `ambition_map_assets` submodule cannot be pushed from this machine

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

## `ambition_sfx_renderer` cannot be pushed, and `main` already points into it

**Blocked 2026-08-15, and it needs credentials rather than a decision about the
code.** `git push` in `tools/ambition_sfx_renderer` fails with *"make sure you
have the correct access rights"* against
`git@aivm-cred-git-d8c7161d54bc:Erotemic/ambition_sfx_renderer.git`. Its sibling
submodules push fine today (`ambition_map_assets` and the sprite renderer both
went out this session), so this is one credential alias, not the mechanism.

⛔ **the consequence is already published.** `main` records `b61ee24` for that
submodule while its `origin/main` is still `bbfe0f9`, so a fresh clone's
`git submodule update` cannot resolve the pointer. I pushed the superproject
anyway — the commits existed locally either way, and holding my work back would
not have unpublished yours — but a clone is broken until the push lands.

⇒ **what is needed:** provision the credential, then push that submodule. No
repository change is required, and ⛔ do not "fix" it by rolling the pointer
back: `bb2d5950f` and `2a5705839` depend on the content in `b61ee24`.
