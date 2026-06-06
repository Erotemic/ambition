# Applying the final plugin-refactor overlay

This overlay is a normal file overlay, so it can add and replace files but it
cannot delete stale files. The final state intentionally replaces the legacy flat
portal module with a `portal/` module directory.

Run these commands from any shell:

```bash
cd ~/code/ambition
rm -f crates/ambition_sandbox/src/portal.rs
unzip -o ~/Downloads/ambition-plugin-refactor-final-with-snapshots-overlay.zip -d ~/code/ambition
```

The delete is safe before or after extraction, but doing it before extraction
keeps the checkout from ever containing both module sources at once.

## Stale file removed by the snapshot series

```text
crates/ambition_sandbox/src/portal.rs
```

## Files intentionally not touched

The overlay intentionally avoids the parallel cube-menu/inventory work:

```text
crates/ambition_inventory_ui/**
crates/ambition_sandbox/src/oot_cube.rs
crates/ambition_sandbox/src/oot_cube_app.rs
```

## Suggested verification

```bash
cargo fmt --all
cargo test -p ambition_sandbox architecture_boundaries
cargo test -p ambition_sandbox room_scoped portal_lab_usable gravity_room_reachability
```

`cargo` and `rustfmt` were not available in the agent container, so those checks
must be run in a normal Rust development environment.
