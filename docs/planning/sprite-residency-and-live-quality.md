# Sprite residency and live quality — consolidated

**State:** forward work moved to
[`engine/asset-preparation-and-residency.md`](engine/asset-preparation-and-residency.md).

The remaining useful requirements are:

- semantic character/asset identity must survive quality-tier changes;
- live quality changes must converge in both directions in a real rendered
  session;
- roster/room demand should precede first visible use;
- residency ownership and budgets must be explicit before adding general
  eviction machinery;
- asset/device materialization bursts need rendered measurements, not headless
  proxies.

The detailed historical step ledger is intentionally removed from live planning.
Git history retains it. Keep this forwarding receipt until Phase 2 can update the
standing references in the control-plane files.
