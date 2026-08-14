# Awaiting a maintainer decision

Only questions whose next step is **Jon's product/authoring judgement** belong
here. Engineering questions go to the queue/tracks; answered questions move to
[`maintainer-decisions.md`](maintainer-decisions.md). The pre-prune investigation
record is archived at
[`../archive/planning-superseded/2026-08-13/awaiting-maintainer-decision.md`](../archive/planning-superseded/2026-08-13/awaiting-maintainer-decision.md).

## Open decisions — 8

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

### 5. What should fighter-vs-fighter hit emphasis do without the primary local seat? (former D114)

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

### 6. How long should a dropped held weapon persist? (former D50)

The lifetime bug is fixed for ability/currency/health drops: the entity and its
visual now share room scope. The remaining laser-sword observation is a product
rule for **held-item drops** after a fight:

- disappear when leaving the room;
- remain in the world when returning; or
- use another explicit persistence policy.

Whichever rule is chosen, simulation entity and presentation must share the same
lifetime.

### 7. Which platform-fighter verbs does each creature author?

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
