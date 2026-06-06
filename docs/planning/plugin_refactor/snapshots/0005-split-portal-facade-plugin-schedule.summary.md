# 0005 — split portal facade, plugin, and schedule files

Converted the monolithic portal source file into a real same-crate module shell
without mechanically splitting portal behavior yet.

Why this matters:

- Replaces `src/portal.rs` with a `src/portal/` facade module.
- Adds `portal/mod.rs` as the public API surface for portal components,
  resources, helpers, and plugins.
- Adds `portal/plugin.rs` with a top-level `PortalPlugin` and the existing
  simulation registration moved behind `PortalSimulationPlugin`.
- Adds `portal/schedule.rs` with portal-owned `PortalSet` labels so later
  patches can order against portal semantics instead of concrete functions.
- Leaves the old implementation body mostly intact in `portal/implementation.rs`
  to keep this step architectural rather than semantic.
- Updates guardrails so future overlays preserve the facade/plugin/schedule
  split and still order portal transit after `ItemPickupSet` instead of
  `ground_item_physics`.

Overlay deletion caveat:

Normal overlay ZIP extraction cannot delete files. This snapshot intentionally
removes the legacy flat portal module, so run this cleanup before or after
extracting the overlay:

```bash
rm -f ~/code/ambition/crates/ambition_sandbox/src/portal.rs
```

Without that removal, Rust will see both `src/portal.rs` and `src/portal/mod.rs`
and report a duplicate module-source error for `portal`.

Main files:

- `crates/ambition_sandbox/src/app/plugins.rs`
- `crates/ambition_sandbox/src/portal/mod.rs`
- `crates/ambition_sandbox/src/portal/plugin.rs`
- `crates/ambition_sandbox/src/portal/schedule.rs`
- `crates/ambition_sandbox/src/portal/implementation.rs`
- `crates/ambition_sandbox/tests/architecture_boundaries.rs`
- `docs/architecture/architecture-boundaries.md`
