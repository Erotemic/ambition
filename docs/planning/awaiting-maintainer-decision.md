# Awaiting a maintainer decision

Only questions whose next step is **Jon's product/authoring judgement** belong
here. Engineering questions go to the queue/tracks; answered questions move to
[`maintainer-decisions.md`](maintainer-decisions.md). The pre-prune investigation
record is archived at
[`../archive/planning-superseded/2026-08-13/awaiting-maintainer-decision.md`](../archive/planning-superseded/2026-08-13/awaiting-maintainer-decision.md).

## Open decisions — 7

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

### 6. How long should a dropped held weapon persist? (former D50)

The lifetime bug is fixed for ability/currency/health drops: the entity and its
visual now share room scope. The remaining laser-sword observation is a product
rule for **held-item drops** after a fight:

- disappear when leaving the room;
- remain in the world when returning; or
- use another explicit persistence policy.

Whichever rule is chosen, simulation entity and presentation must share the same
lifetime.

### 7. Do we want a gravity-camera mode that follows the controlled body's reference frame? (former D64)

This is a new relativity-facing feature, not a bug fix. If wanted, its input and
camera semantics should be expressed in the controlled body's local reference
frame rather than adding a privileged global/player frame. Confirm the feature
before opening an implementation campaign.
