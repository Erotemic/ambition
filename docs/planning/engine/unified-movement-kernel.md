# Unified movement kernel — remaining work

> **Verified against `cecd01ca` (2026-08-13).** The frame-aware movement kernel,
> typed resolution seams, rollback registration, surface-momentum operations,
> portal transit geometry, and the previously listed residual items 1–4 are
> implemented or refuted. The full architecture/migration record is archived at
> [`../../archive/planning-superseded/2026-08-13/engine/unified-movement-kernel.md`](../../archive/planning-superseded/2026-08-13/engine/unified-movement-kernel.md).

## Remaining

- ▢ **Block ↔ chain crawl transfer.** A crawler attached to a block surface does
  not transfer directly onto an overlapping chain surface (or vice versa)
  without detaching first. Define the shared attachment-transfer rule so the
  two authored surface domains can hand off continuously without introducing a
  second crawler controller.

- ▢ **Exercise portal transit inside authored gravity zones.** The code resolves
  projectile gravity per body and portal transit itself is pure portal geometry,
  but current portal rooms do not nest gravity zones. Add a behavioral exercise
  if/when a room authors that combination; there is no known porting bug to fix.
