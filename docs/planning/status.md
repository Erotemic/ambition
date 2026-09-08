# Planning status

This page is a **short orientation snapshot**, not a test-history ledger. Live
work is in [`queue.md`](queue.md); unresolved maintainer calls are in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md); durable
architecture belongs in focused owner documents.

**Reference source:** `54d99e7fb` (the head reviewed as `a38e1bf0f` on
2026-09-07 plus the actor-spawn boundary correction committed right after it).
Re-measure before quoting these numbers on a newer head.

## Current architecture posture

The prerequisite authority work required before serious capability/crate
composition has substantially converged:

```text
F1  construction inversion             crossed
B   control/custody authority           crossed
C1  public scheduling vocabulary       crossed
D   rollback composition               crossed
E   scoped ruleset policy               crossed

C2  capability/crate composition       active
```

C1 closed when the body-clock view published `BodyClockViewSet::{Reset, Contribute}`
and Smash registered against it; the ratchet reads zero capability/ruleset
private orderings.

## Actor monolith

Current nontrivial module SCCs (`scripts/measure_kernel_module_graph.py --scc`):

```text
 9  abilities, construction, control, features, items, projectile,
    session, shrine, world

 2  assets, character_sprites
```

P1 landed (settlement state moved to `ambition_match`, wire IDs preserved) and
actor spawning was extracted to the crate `ambition_platformer2d_actor_spawn`, so
`actor_spawn` is no longer a module in this graph and `character_runtime` is no
longer in a cycle. The remaining designed cuts, in
[`engine/actor-monolith-work-frontier.md`](engine/actor-monolith-work-frontier.md):

```text
P2  projectile -> features              expected  9 -> 8
P3  shrine -> session                   expected  8 -> 7
P4  construction -> world               expected  7 -> 6
```

**The module graph cannot see a crate boundary.** The first spawn carve took the
live actor view (`ActorMut`, `ActorClusterQueryData`), the damage i-frame constant,
in-place provocation, the fighter-ladder projection and the dismounted-rider
rebuild out with it, and 11 -> 9 stayed green while the kernel imported its own
tick-time vocabulary from a crate whose contract said "spawn". Those went back to
the kernel (`crate::actor_clusters`, `features/ecs/actors/provoke.rs`,
`features/ecs/{fighter_ladder,dismounted_rider}.rs`); pickup/chest bundles went to
`features/feature_bundles.rs`. `scripts/tests/test_actor_spawn_boundary.py`
states the boundary from the spawn side: no query view, no timing constant, one
system, no pickup/chest, and live kernel roads consume only builders.

After P4, implementation stops until the six-module hard-core edge ledger is
complete. Do not choose another cut merely because it has a low reference count.

## Composition measurements

At the reference head:

```text
capability/ruleset foreign private ordering      0
composition foreign private ordering            73
foreign system installations                   175
mechanically reducible installation blocks       3
irreducible composition blocks                  38
```

The target is **correct ownership**, not zero host/composition code. A block that
truly decides how independent capabilities compose belongs in composition.

Owner: [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md).

## Current correctness front

No correctness regression from the 2026-09-07 reviews is open. The next
engineering action is architecture (P2 in the queue), not a fix.

Closed at the reference head and not to be reopened without new evidence:

- delayed mark attribution after the marking body is gone (`SeatCredit` stand-in,
  never the victim; the stand-in carries no `MatchSeat`);
- body-owned drawable geometry finalized before portal publication
  (`BodyOwnedDrawableSync`);
- hit-flash same-frame near-side visibility (the owner asserts `Visible` each frame);
- C1 body-clock contribution (`BodyClockViewSet::Contribute`);
- mark stock lifetime;
- exact mark fuse duration;
- player-readable mark clock;
- general non-Sprite portal clipping through `DeclaredFrame`;
- map visited-state save/session lifetime;
- installer mutation-runner target/headroom preflight;
- mount/possession multi-claim authority;
- Mary-O's competing logical/trimmed render-basis placement.

## Presentation posture

Body-owned presentation should follow one pipeline:

```text
simulation/read-model fact
    -> body-owned drawable + PresentationOf(body)
    -> drawable geometry finalized
    -> portal candidate publication
    -> per-pane compositing
    -> final draw
```

The remaining mark-clock and hit-flash queue rows are synchronization defects at
this boundary, not arguments for another overlay-specific portal mechanism.

Owner: [`engine/render-animation-and-vfx.md`](engine/render-animation-and-vfx.md).

## Performance and asset posture

Performance claims require an executable measurement with the intended scenario,
hardware and cache state. Source-only or unsupported scenario substitutions must
report **unmeasured**, not pass.

Asset quality and residency are separate concerns:

- selected quality tier owns source-pixel expectations;
- runtime residency owns when prepared/device assets remain live;
- authored character size owns world dimensions;
- trim/packing must not change semantic frame placement.

Owners:
[`engine/performance-and-iteration.md`](engine/performance-and-iteration.md) and
[`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md).

## Fighter-brain posture

The fighter brain owns generic scoring/selection over the authored capability
menu. It must not grow per-character move scripts to compensate for menu/scoring
shape. Difficulty should change decision quality/behavior intentionally rather
than create accidental self-destruction.

Owner: [`engine/fighter-brain.md`](engine/fighter-brain.md).

## Planning hygiene

The planning tree is a control plane, not an archive:

- delete completed queue rows;
- remove answered questions after recording the ruling;
- retire completed campaign diaries to short receipts;
- keep measurements with the tool/owner that can reproduce them;
- do not create maintainer-named scratchpads or agent review dumps;
- use Git history when old reasoning is needed.

The retired maintainer-named observation dump must not be recreated under a
new name. Direct maintainer reports are triaged immediately into an owner doc,
queue row, decision, or durable ruling.

## Where to look next

- **Next engineering action:** [`queue.md`](queue.md)
- **Maintainer questions:** [`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md)
- **Durable rulings:** [`maintainer-decisions.md`](maintainer-decisions.md)
- **Architecture tracks:** [`tracks.md`](tracks.md)
- **Actor decomposition:** [`engine/actor-monolith-decomposition.md`](engine/actor-monolith-decomposition.md)
- **Smash parity:** [`demos/smash-parity-inventory.md`](demos/smash-parity-inventory.md)
