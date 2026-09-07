# Planning status

This page is a **short orientation snapshot**, not a test-history ledger. Live
work is in [`queue.md`](queue.md); unresolved maintainer calls are in
[`awaiting-maintainer-decision.md`](awaiting-maintainer-decision.md); durable
architecture belongs in focused owner documents.

**Reference source:** `625fa79af45e6eff40cbefabd8cdae33c5b5e9db`,
reviewed 2026-09-07. Re-measure before quoting these numbers on a newer head.

## Current architecture posture

The prerequisite authority work required before serious capability/crate
composition has substantially converged:

```text
F1  construction inversion             crossed
B   control/custody authority           crossed
D   rollback composition               crossed
E   scoped ruleset policy               crossed

C1  public scheduling vocabulary       one known regression
C2  capability/crate composition       active
```

The C1 regression is narrow: Smash's mark-clock contributor directly orders
itself after `sim_view::rebuild_body_clocks_view`. The queue requires replacing
that concrete foreign-function edge with public body-clock scheduling vocabulary.

## Actor monolith

Current nontrivial module SCCs:

```text
11  abilities, actor_spawn, character_runtime, construction, control,
    features, items, projectile, session, shrine, world

 2  assets, character_sprites
```

The next four cuts are already designed in
[`engine/actor-monolith-work-frontier.md`](engine/actor-monolith-work-frontier.md):

```text
P1  character_runtime -> features       expected 11 -> 9
P2  projectile -> features              expected  9 -> 8
P3  shrine -> session                   expected  8 -> 7
P4  construction -> world               expected  7 -> 6
```

After P4, implementation stops until the six-module hard-core edge ledger is
complete. Do not choose another cut merely because it has a low reference count.

## Composition measurements

At the reference head:

```text
capability/ruleset foreign private ordering      1
composition foreign private ordering            73
foreign system installations                   174
mechanically reducible installation blocks       2
irreducible composition blocks                  39
```

The target is **correct ownership**, not zero host/composition code. A block that
truly decides how independent capabilities compose belongs in composition.

Owner: [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md).

## Current correctness front

The highest-priority current issues are deliberately few:

1. delayed mark attribution must survive elimination/despawn of the marking body;
2. body-owned drawable geometry needs a semantic finalized phase before portal
   candidate publication;
3. hit-flash needs same-frame visibility restoration on far-side -> near-side;
4. the body-clock contribution must stop naming a foreign private reset system.

The broad problems those fixes grew out of are already closed and should not be
reopened without new evidence:

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
