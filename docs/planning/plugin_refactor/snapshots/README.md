# Plugin refactor snapshot diffs

This directory records the patch trail for the overlay produced by this agent run.
The final overlay is still the source of truth for the target tree state; these
snapshots are review breadcrumbs so another agent can inspect how the final state
was assembled.

## Overlay cleanup commands

Normal ZIP overlays cannot delete stale files. Snapshot 0005 removes the legacy
flat portal module, so apply this cleanup when using the latest overlay:

```bash
rm -f ~/code/ambition/crates/ambition_sandbox/src/portal.rs
```

This command is safe to run before or after extracting the overlay. It prevents
Rust from seeing both `src/portal.rs` and `src/portal/mod.rs` for the same module.

The first three snapshots are retroactive reconstructions of the previously
shipped steps 1-5 overlay. Snapshot 0004 is the first follow-up work, and snapshot 0005 continues the portal module shell split.


For copy/paste apply instructions and the stale-file manifest, see
[`APPLY_OVERLAY.md`](APPLY_OVERLAY.md).

## Snapshot index

| Snapshot | Purpose |
| --- | --- |
| `0001-docs-inventory-baseline` | Add generated ECS inventory baselines and planning notes that capture the starting point for the refactor. |
| `0002-architecture-boundary-guardrails` | Add ADR/architecture docs and the first architecture-boundary tests. |
| `0003-proto-runtime-lifecycle-schedule-and-subsystem-ownership` | Add the same-crate proto-runtime lifecycle API, schedule vocabulary, room-scoped spawn migrations, and first module-owned portal/item plugins. |
| `0004-extract-runtime-raycast-seam-and-set-ordering` | Move plain solid raycasts into `platformer_runtime::collision`, migrate non-portal callers, and replace the portal-to-item concrete ordering pin with `ItemPickupSet`. |
| `0005-split-portal-facade-plugin-schedule` | Replace flat `portal.rs` with a `portal/` facade module, move plugin registration into `portal/plugin.rs`, and add portal-owned `PortalSet` labels. |

## How to read these

Each `.patch` is a normal `git show --binary` snapshot diff. Apply them in
number order to the original uploaded source archive to reproduce the final code
state, except that this `snapshots/` directory itself is only included in the
final overlay as review documentation.
