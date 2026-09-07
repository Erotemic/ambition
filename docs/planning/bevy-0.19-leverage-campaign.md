# Bevy 0.19.1 leverage campaign — completed receipt

**State:** COMPLETE since 2026-08-31. This path remains only as a stable historical
pointer for source/docs that cite the campaign. Do not add current work here.

## Durable outcomes

The campaign evaluated which Bevy 0.19 facilities should replace or simplify
Ambition-specific infrastructure. The durable results were:

- host-global diagnostics were preferred over gameplay-session-local debug UI;
- Bevy's FPS/diagnostic overlay facilities replaced redundant custom display
  where their semantics matched Ambition's needs;
- selected Ambition censuses were exposed through diagnostics;
- F1/debug rendering moved toward text/gizmo/debug presentation rather than
  persistent gameplay entities;
- custom menu text-height policy that duplicated Bevy behavior was removed;
- resource/component semantics were audited rather than blindly converted;
- explicit render recovery remained an Ambition-owned requirement;
- `SettingsPlugin` was evaluated and **not** adopted merely because Bevy offered
  it;
- the migrated post-processing path was GPU-verified and its no-op path removed.

The campaign's governing rule remains useful:

> adopt an upstream primitive when it owns the same semantics; do not migrate
> merely to reduce local code or because a new Bevy API exists.

## Current owners

Current diagnostics, iteration and rendering work belongs in:

- [`engine/performance-and-iteration.md`](engine/performance-and-iteration.md)
- [`engine/render-animation-and-vfx.md`](engine/render-animation-and-vfx.md)
- [`engine/capability-and-runtime-composition.md`](engine/capability-and-runtime-composition.md)
- [`queue.md`](queue.md)

Git history preserves the original A-H work plan, experiments, corrections and
validation matrix.
